mod core;
mod features;
mod modules;
mod shared;

use crate::core::config::Config;
use crate::core::openapi::{ApiDoc, SwaggerInfoModifier};
use crate::core::{database, middleware};
use crate::features::admin::{routes as admin_routes, AdminService};
use crate::features::auth;
use crate::features::auth::clients::LogtoAuthClient;
use crate::features::auth::routes as auth_routes;
use crate::features::auth::services::{AuthService, TokenService};
use crate::features::categories::{routes as categories_routes, CategoryService};
use crate::features::citizen_report_agent::{
    create_tool_registry, routes as citizen_agent_routes, AgentRuntimeService, ConversationService,
    ThreadAttachmentService,
};
use crate::features::contributors::{routes as contributors_routes, ContributorService};
use crate::features::dashboard::{routes as dashboard_routes, DashboardService};
use crate::features::expectations::{routes as expectations_routes, ExpectationService};
use crate::features::files::{routes as files_routes, FileService};
use crate::features::logto::token_manager::LogtoTokenManager;
use crate::features::rate_limits::{
    routes as rate_limits_routes, RateLimitConfigService, RateLimitService,
};
use crate::features::regions::{routes as regions_routes, RegionService};
use crate::features::reports::{
    routes as reports_routes, ClusteringService, ExtractionService, GeocodingService,
    RegionLookupService, ReportJobService, ReportProcessor, ReportService,
};
use crate::features::tickets::{routes as tickets_routes, TicketService};
use crate::features::users::{
    clients::logto::LogtoUserProfileClient, routes as users_routes, services::UserProfileService,
};
use axum::{middleware::from_fn, Router};
use balungpisah_adk::Storage;
use std::sync::Arc;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

fn main() -> anyhow::Result<()> {
    // Build Tokio runtime with configurable worker threads
    let worker_threads = std::env::var("TOKIO_WORKER_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
        });

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(worker_threads * 4)
        .enable_all()
        .build()?;

    runtime.block_on(async_main(worker_threads))
}

async fn async_main(worker_threads: usize) -> anyhow::Result<()> {
    // Load .env file BEFORE initializing logger so RUST_LOG is available
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env().map_err(|e| anyhow::anyhow!(e))?;

    // Log system info
    let available_cpus = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);
    tracing::info!(
        "System info: available_cpus={}, tokio_worker_threads={}, pid={}",
        available_cpus,
        worker_threads,
        std::process::id()
    );

    tracing::info!("Configuration loaded successfully");

    // Create database connection pool
    let pool = database::create_pool(&config.database).await?;
    tracing::info!("Database connection pool created");

    // Run migrations automatically
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;
    tracing::info!("Database migrations completed successfully");

    // Initialize auth
    let jwks_client = Arc::new(auth::JwksClient::new(
        &config.auth.issuer,
        config.auth.jwks_cache_ttl,
    ));
    let jwt_validator = Arc::new(auth::JwtValidator::new(
        jwks_client,
        config.auth.issuer.clone(),
        config.auth.audience.clone(),
        config.auth.jwt_leeway,
    ));
    tracing::info!("Auth configuration initialized");

    // Initialize Logto Token Manager (used by multiple services)
    let logto_token_manager = Arc::new(LogtoTokenManager::new(config.logto_m2m.clone()));
    tracing::info!("Logto token manager initialized");

    // Initialize auth services (for register/login via Logto token exchange)
    let logto_auth_client = Arc::new(LogtoAuthClient::new(Arc::clone(&logto_token_manager)));
    let token_service = Arc::new(TokenService::new(
        config.auth_token.clone(),
        Arc::clone(&logto_token_manager),
    ));
    let auth_service = Arc::new(AuthService::new(
        Arc::clone(&logto_auth_client),
        Arc::clone(&token_service),
    ));
    tracing::info!("Auth service initialized (with Logto token exchange)");

    // Initialize users service (reusing LogtoTokenManager)
    let logto_user_profile_client = Arc::new(LogtoUserProfileClient::new(Arc::clone(
        &logto_token_manager,
    )));
    let user_profile_service = Arc::new(UserProfileService::new(logto_user_profile_client));
    tracing::info!("User profile service initialized");

    // Initialize MinIO client for storage
    let minio_client = Arc::new(
        modules::storage::MinIOClient::new(config.minio.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize MinIO client: {}", e))?,
    );
    // Ensure bucket exists (create if not)
    minio_client
        .ensure_bucket_exists()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to ensure MinIO bucket exists: {}", e))?;
    tracing::info!(
        "MinIO client initialized for bucket: {}",
        minio_client.bucket_name()
    );

    // Initialize File Service
    let file_service = Arc::new(FileService::new(pool.clone(), Arc::clone(&minio_client)));
    tracing::info!("File service initialized");

    // Initialize Region Service
    let region_service = Arc::new(RegionService::new(pool.clone()));
    tracing::info!("Region service initialized");

    // Initialize Expectation Service (for landing page)
    let expectation_service = Arc::new(ExpectationService::new(pool.clone()));
    tracing::info!("Expectation service initialized");

    // Initialize Contributor Service
    let contributor_service = Arc::new(ContributorService::new(pool.clone()));
    tracing::info!("Contributor service initialized");

    // Initialize Category Service
    let category_service = Arc::new(CategoryService::new(pool.clone()));
    tracing::info!("Category service initialized");

    // Initialize Ticket Service
    let ticket_service = Arc::new(TicketService::new(pool.clone()));
    tracing::info!("Ticket service initialized");

    // Initialize Report Services
    let report_service = Arc::new(ReportService::new(pool.clone()));
    let report_job_service = Arc::new(ReportJobService::new(pool.clone()));
    let geocoding_service = Arc::new(GeocodingService::new());
    let clustering_service = Arc::new(ClusteringService::new(pool.clone()));
    let region_lookup_service = Arc::new(RegionLookupService::new(pool.clone()));
    tracing::info!("Report services initialized");

    // Initialize Dashboard Service
    let dashboard_service = Arc::new(DashboardService::new(pool.clone()));
    tracing::info!("Dashboard service initialized");

    // Initialize Rate Limit Services
    let rate_limit_config_service = Arc::new(RateLimitConfigService::new(pool.clone()));
    let rate_limit_service = Arc::new(RateLimitService::new(
        pool.clone(),
        Arc::clone(&rate_limit_config_service),
    ));
    tracing::info!("Rate limit services initialized");

    // Initialize Admin Service
    let admin_service = Arc::new(AdminService::new(pool.clone()));
    tracing::info!("Admin service initialized");

    // Initialize Citizen Report Agent Services
    // ADK uses a separate database for conversation storage
    let tensorzero_client =
        balungpisah_adk::TensorZeroClient::new(&config.agent_gateway.tensorzero_url)
            .map_err(|e| anyhow::anyhow!("Failed to create TensorZero client: {}", e))?;

    let adk_storage = Arc::new(
        balungpisah_adk::PostgresStorage::connect_url(&config.agent_gateway.database_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to ADK database: {}", e))?,
    );
    tracing::info!(
        "ADK database connection established: {}",
        config
            .agent_gateway
            .database_url
            .split('@')
            .next_back()
            .unwrap_or("***")
    );

    // Run ADK migrations
    tracing::info!("Running ADK database migrations...");
    adk_storage
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("ADK migration failed: {}", e))?;
    tracing::info!("ADK database migrations completed successfully");

    // Initialize Extraction Service (uses TensorZero + ADK for LLM calls)
    let extraction_service = match ExtractionService::new(
        pool.clone(),
        &config.agent_gateway.tensorzero_url,
        config.agent_gateway.openai_api_key.clone(),
        config.agent_gateway.model_name.clone(),
        Arc::clone(&adk_storage),
    ) {
        Ok(service) => {
            tracing::info!("Extraction service initialized (TensorZero + OpenAI)");
            Some(Arc::new(service))
        }
        Err(e) => {
            tracing::warn!(
                "Extraction service not available: {}. Ticket processing worker will not run.",
                e
            );
            None
        }
    };

    // Spawn background processor workers (if extraction service is available)
    if let Some(extraction_svc) = extraction_service {
        // NOTE: TicketProcessor disabled - ticket workflow replaced by direct report creation
        // The ticket feature is kept for historical data but no longer processes new tickets.
        // See: citizen_report_agent -> ReportProcessor workflow

        // Spawn Report Processor Worker (new workflow)
        // NOTE: Clustering disabled - reports use regional hierarchy instead
        let report_processor = ReportProcessor::new(
            pool.clone(),
            Arc::clone(&extraction_svc),
            Arc::clone(&geocoding_service),
            Arc::clone(&report_service),
            Arc::clone(&report_job_service),
            Arc::clone(&region_lookup_service),
        );
        tokio::spawn(async move {
            report_processor.run().await;
        });
        tracing::info!("Report processor worker spawned");
    }

    // Create tool registry with database pool for ticket creation
    let tool_registry = create_tool_registry(Arc::new(pool.clone()));
    tracing::info!(
        "Agent tool registry initialized with {} tools",
        tool_registry.names().len()
    );

    let agent_runtime_service = Arc::new(AgentRuntimeService::with_tools(
        tensorzero_client,
        Arc::clone(&adk_storage),
        config.agent_gateway.openai_api_key.clone(),
        config.agent_gateway.model_name.clone(),
        tool_registry,
    ));
    let conversation_service = Arc::new(ConversationService::new(Arc::clone(&adk_storage)));
    let thread_attachment_service = Arc::new(ThreadAttachmentService::new(
        pool.clone(),
        Arc::clone(&minio_client),
        Arc::clone(&adk_storage),
    ));
    tracing::info!(
        "Citizen report agent services initialized (TensorZero: {})",
        config.agent_gateway.tensorzero_url
    );

    // Build application router with dynamic swagger config
    let swagger_modifier = SwaggerInfoModifier {
        title: config.swagger.title.clone(),
        version: config.swagger.version.clone(),
        description: config.swagger.description.clone(),
    };

    let mut openapi = ApiDoc::openapi();
    swagger_modifier.modify(&mut openapi);

    // Build swagger router
    let swagger = if let Some(credentials) = config.swagger.credentials() {
        tracing::info!("Swagger UI basic auth enabled");
        Router::new()
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
            .layer(from_fn(middleware::basic_auth_middleware(Arc::new(
                credentials,
            ))))
    } else {
        tracing::info!("Swagger UI basic auth disabled (no credentials configured)");
        Router::new().merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
    };

    // Protected routes (require JWT authentication)
    let protected_routes = Router::new()
        .merge(auth_routes::protected_routes(Arc::clone(&auth_service)))
        .merge(users_routes::routes(user_profile_service))
        .merge(regions_routes::routes(region_service))
        .merge(files_routes::routes(file_service))
        .merge(tickets_routes::routes(Arc::clone(&ticket_service)))
        .merge(reports_routes::routes(
            Arc::clone(&report_service),
            Arc::clone(&clustering_service),
        ))
        .merge(citizen_agent_routes::routes(
            Arc::clone(&agent_runtime_service),
            Arc::clone(&conversation_service),
            Arc::clone(&thread_attachment_service),
            Arc::clone(&rate_limit_service),
        ))
        .merge(rate_limits_routes::admin_routes(Arc::clone(
            &rate_limit_config_service,
        )))
        .nest(
            "/api/admin",
            admin_routes::routes(Arc::clone(&admin_service)),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            jwt_validator.clone(),
            middleware::auth_middleware,
        ));

    // Simple health check endpoint (no auth required)
    async fn health_check() -> axum::http::StatusCode {
        axum::http::StatusCode::OK
    }
    let health_route = Router::new().route("/health", axum::routing::get(health_check));

    // Public routes (no auth required)
    let public_routes = Router::new()
        .merge(auth_routes::public_routes(auth_service))
        .merge(expectations_routes::routes(expectation_service))
        .merge(contributors_routes::routes(contributor_service))
        .merge(categories_routes::routes(category_service))
        .merge(dashboard_routes::routes(Arc::clone(&dashboard_service)));

    let app = Router::new()
        .merge(swagger)
        .merge(protected_routes)
        .merge(public_routes)
        .merge(health_route)
        .layer(middleware::cors_layer(
            config.app.cors_allowed_origins.clone(),
        ))
        // Propagate X-Request-Id to response headers
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(middleware::MakeSpanWithRequestId)
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        // Generate X-Request-Id using UUID v7 (or use client-provided one)
        .layer(SetRequestIdLayer::x_request_id(middleware::MakeRequestUuid));

    // Start server
    let addr = config.app.server_address();
    let socket_addr: std::net::SocketAddr = addr
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

    // Use socket2 for TCP listener configuration
    let socket = socket2::Socket::new(
        socket2::Domain::for_address(socket_addr),
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )?;

    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_nodelay(true)?;

    socket.set_recv_buffer_size(256 * 1024)?;
    socket.set_send_buffer_size(256 * 1024)?;

    #[cfg(target_os = "linux")]
    {
        let keepalive = socket2::TcpKeepalive::new()
            .with_time(std::time::Duration::from_secs(60))
            .with_interval(std::time::Duration::from_secs(10))
            .with_retries(3);
        socket.set_tcp_keepalive(&keepalive)?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let keepalive = socket2::TcpKeepalive::new().with_time(std::time::Duration::from_secs(60));
        socket.set_tcp_keepalive(&keepalive)?;
    }

    socket.set_nonblocking(true)?;
    socket.bind(&socket_addr.into())?;
    socket.listen(65535)?;

    let listener = tokio::net::TcpListener::from_std(socket.into())?;
    tracing::info!("Server listening on {}", format!("http://{}", addr));
    tracing::info!(
        "Swagger UI available at {}",
        format!("http://{}/swagger-ui/", addr)
    );

    axum::serve(listener, app).await?;

    Ok(())
}

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Architecture Overview

This is a production-ready Rust API boilerplate built with Axum 0.8, featuring a feature-based architecture suitable for small to medium projects. The codebase follows a clean separation of concerns with three main layers:

### Core Layer (`src/core/`)
Infrastructure components used across the application:
- **config.rs**: Environment-based configuration with structured config objects (AppConfig, DatabaseConfig, AuthConfig, SwaggerConfig)
- **database.rs**: PostgreSQL connection pooling via SQLx with configurable pool settings
- **error.rs**: Centralized error handling with `AppError` enum that implements `IntoResponse` for consistent API error responses
- **middleware.rs**: Request middleware including JWT auth middleware and basic auth for Swagger
- **extractor.rs**: Custom Axum extractors (e.g., `ValidatedJson` for automatic validation)

### Shared Layer (`src/shared/`)
Common utilities and types:
- **types.rs**: Standard API response wrapper (`ApiResponse<T>`) with success/error formatting and pagination metadata
- **constants.rs**: Application-wide constants (e.g., `DEFAULT_PAGE_SIZE`, `MAX_PAGE_SIZE`)

### Features Layer (`src/features/`)
Feature-based modules following a consistent structure with plural subfolder naming. Each feature contains:
- **dtos/**: Request/response DTOs with validation via `validator` crate
- **models/**: Database models (SQLx row types)
- **services/**: Business logic and database operations (takes `PgPool`, returns `Result<T>`)
- **handlers/**: HTTP handlers (Axum route handlers) with OpenAPI documentation via `utoipa` macros
- **routes.rs**: Route registration that returns an Axum `Router`
- **test_utils.rs**: Shared test utilities for the feature
- **mod.rs**: Module exports

### Authentication System
The application uses JWT authentication with JWKS validation against a Logto OIDC provider:

- **JwksClient** (`auth/jwks.rs`): Fetches and caches public keys from the OIDC provider's JWKS endpoint
- **JwtValidator** (`auth/validator.rs`): Validates JWT tokens by:
  1. Extracting the key ID from the JWT header
  2. Fetching the corresponding public key from JwksClient
  3. Validating the token signature, issuer, audience, and expiration
  4. Parsing custom claims (org_id, role, organization metadata)
- **Middleware** (`core/middleware.rs`): `auth_middleware` extracts the JWT from the Authorization header, validates it, and injects the `AuthenticatedUser` into request extensions
- **Extractors** (`core/extractor.rs`): `AuthUser` extractor retrieves the authenticated user from request extensions for use in handlers
- **Guards** (`auth/guards.rs`): `RequireSuperAdmin` extractor provides role-based access control by checking if the user has the super_admin role

Protected routes are registered with `.route_layer(axum::middleware::from_fn_with_state(jwt_validator, middleware::auth_middleware))` in main.rs:170-177.

## Development Commands

For detailed command reference, see [docs/QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md).

### Build & Run
```bash
make run          # Run application (migrations run automatically on startup)
make build        # Build release binary
make dev          # Run with auto-reload (requires cargo-watch)
```

### Testing
```bash
make test         # Run all tests in parallel
cargo test test_name -- --nocapture  # Run specific test with output
```

Tests use `#[sqlx::test]` for isolated database per test. See [docs/development/testing.md](docs/development/testing.md).

### Code Quality
```bash
make fmt          # Format code with rustfmt
make clippy       # Run linter (treats warnings as errors)
cargo sqlx prepare  # Generate offline query metadata for CI/CD
```

### Database Management
```bash
make migrate-run         # Run pending migrations
make migrate-add NAME=<name>  # Create new migration file
make db-reset            # Drop and recreate database
```

See [docs/development/migrations.md](docs/development/migrations.md) for migration details.

## Configuration

All configuration is loaded from environment variables (`.env` file in development):

### Required Variables
- `DATABASE_URL`: PostgreSQL connection string
- `LOGTO_ISSUER`: OIDC issuer URL (e.g., https://auth.example.com/oidc)
- `LOGTO_AUDIENCE`: API audience identifier (e.g., https://api.example.com/api)

### Optional Variables (with defaults)
- `HOST`: Server host (default: 127.0.0.1)
- `PORT`: Server port (default: 3000)
- `RUST_LOG`: Log level (default: info,balungpisah_core=debug)
- `DB_MAX_CONNECTIONS`: Connection pool size (default: 10)
- `DB_ACQUIRE_TIMEOUT_SECS`: Pool acquire timeout (default: 5)
- `DB_IDLE_TIMEOUT_SECS`: Connection idle timeout (default: 600)
- `DB_MAX_LIFETIME_SECS`: Connection max lifetime (default: 1800)
- `JWKS_CACHE_TTL`: JWKS cache duration in seconds (default: 3600)
- `JWT_LEEWAY`: JWT validation leeway in seconds (default: 60)
- `SWAGGER_USERNAME`: Swagger UI username (default: admin)
- `SWAGGER_PASSWORD`: Swagger UI password (default: admin)
- `SWAGGER_TITLE`: API title in Swagger (default: "Balungpisah API")
- `SWAGGER_VERSION`: API version (default: "0.1.0")
- `SWAGGER_DESCRIPTION`: API description

Configuration is parsed via the `Config::from_env()` method which returns structured config objects. See `.env.example` for reference.

## Key Patterns

### Error Handling
- Use `AppError` enum for all error types
- Services return `Result<T>` (type alias for `std::result::Result<T, AppError>`)
- `AppError` automatically converts to HTTP responses with appropriate status codes
- Database errors are caught and sanitized to prevent leaking internal details (see error.rs:41-47)

### Service Layer Pattern
Services are initialized with `PgPool` and registered in main.rs as `Arc<Service>`:

```rust
pub struct DemoService {
    pool: PgPool,
}

impl DemoService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, dto: CreateDto) -> Result<ResponseDto> {
        // Business logic and database queries here
    }
}
```

Initialize in main.rs:
```rust
let demo_service = Arc::new(DemoService::new(pool.clone()));
```

### Handler Pattern
Handlers use Axum extractors and return `Result<Json<ApiResponse<T>>, AppError>`:

```rust
#[utoipa::path(
    post,
    path = "/api/demos",
    request_body = CreateDemoDto,
    responses(
        (status = 201, description = "Demo created", body = ApiResponse<DemoResponseDto>),
        (status = 400, description = "Validation error")
    )
)]
pub async fn create_demo(
    State(service): State<Arc<DemoService>>,
    ValidatedJson(dto): ValidatedJson<CreateDemoDto>,
) -> Result<Json<ApiResponse<DemoResponseDto>>, AppError> {
    let demo = service.create(dto).await?;
    Ok(Json(ApiResponse::success(demo, None)))
}
```

### Protected Route Pattern
Handlers requiring authentication use the `AuthUser` extractor:

```rust
pub async fn protected_handler(
    AuthUser(user): AuthUser,  // Automatically validated by auth middleware
    State(service): State<Arc<SomeService>>,
) -> Result<Json<ApiResponse<T>>, AppError> {
    // user is AuthenticatedUser with org_id, role, etc.
    service.do_something(user.org_id).await?;
    Ok(Json(ApiResponse::success(result, None)))
}
```

For role-based access control, use guards:
```rust
pub async fn super_admin_only_handler(
    RequireSuperAdmin(user): RequireSuperAdmin,  // Only allows users with super_admin role
    // ... rest of handler
) -> Result<Json<ApiResponse<T>>, AppError> {
    // ...
}
```

### Adding New Features
1. Create feature directory: `src/features/<feature_name>/`
2. Create subfolders: `dtos/`, `models/`, `services/`, `handlers/`
3. Implement the standard files within each subfolder (with mod.rs for each), plus routes.rs, test_utils.rs, and mod.rs at feature level
4. Register module in `src/features/mod.rs`
5. Initialize service in `main.rs` and merge routes into the router
6. Add paths and schemas to the `#[openapi]` macro in main.rs for Swagger documentation

See [docs/development/adding-features.md](docs/development/adding-features.md) for detailed step-by-step guide.

### Database Queries (SQLx)
**Always prefer compile-time verified queries** using macros (`query!`, `query_as!`, `query_scalar!`) over runtime-checked functions (`query`, `query_as`):

```rust
// PREFERRED: Compile-time verified (errors caught at build time)
sqlx::query_as!(
    CitizenProfile,
    r#"SELECT id, name FROM profiles WHERE user_id = $1"#,
    &user_id
)

// AVOID: Runtime-checked (errors only caught at runtime)
sqlx::query_as::<_, CitizenProfile>("SELECT id, name FROM profiles WHERE user_id = $1")
    .bind(&user_id)
```

For boolean EXISTS queries, use type annotation:
```rust
sqlx::query_scalar!(
    r#"SELECT EXISTS(SELECT 1 FROM users WHERE id = $1) as "exists!""#,
    &user_id
)
```

Other guidelines:
- For dynamic queries (e.g., filtering with optional params), use `QueryBuilder`
- Always use parameterized queries to prevent SQL injection
- Run `cargo sqlx prepare` after adding/modifying queries to update offline cache

### Testing
- Service tests go in `services/<name>_service_test.rs` files
- Handler tests go in `handlers/<name>_handler_test.rs` files
- Use `#[sqlx::test]` for tests requiring database access
- Seed test data with helper functions in `test_utils.rs`
- Use `-- --nocapture` flag to see print statements in tests
- The `#[sqlx::test]` macro provides an isolated `PgPool` per test

### OpenAPI Documentation
- Use `#[utoipa::path]` macro on handlers to generate Swagger docs
- Register paths in the `#[openapi(paths(...))]` macro in main.rs
- Register schemas in the `#[openapi(components(schemas(...)))]` macro
- Swagger UI is available at `/swagger-ui` (protected by basic auth)
- OpenAPI JSON spec is at `/api-docs/openapi.json`

## Documentation Structure

The project documentation is organized as follows:

### Root Level
- **README.md** - Project overview, quick start, and links to detailed docs
- **CLAUDE.md** - This file, guidance for AI assistants

### Developer Documentation (`docs/development/`)
- **getting-started.md** - Complete setup guide from prerequisites to running app
- **architecture.md** - System architecture, design patterns, and best practices
- **testing.md** - Comprehensive testing guide with patterns and examples
- **git-workflow.md** - Git branching strategy and deployment workflow
- **adding-features.md** - Step-by-step guide to adding new features
- **migrations.md** - Database migration guide with SQLx

### Reference Documentation
- **docs/QUICK_REFERENCE.md** - Command cheatsheet for common operations

### Feature Planning (`docs/feat/`)
- Contains implementation plans and design documents for features
- Organized by feature number (e.g., `001-auth-logto-oidc/`)

When helping users, refer them to the appropriate documentation:
- Setup issues → `docs/development/getting-started.md`
- Architecture questions → `docs/development/architecture.md`
- Testing help → `docs/development/testing.md`
- Adding features → `docs/development/adding-features.md`
- Quick commands → `docs/QUICK_REFERENCE.md`

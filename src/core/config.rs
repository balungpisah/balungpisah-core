use std::env;
use std::time::Duration;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub swagger: SwaggerConfig,
    pub logto_m2m: LogtoM2MConfig,
    pub minio: MinIOConfig,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
    pub max_request_body_size: usize,
    pub frontend_url: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub issuer: String,
    pub audience: String,
    pub jwks_cache_ttl: Duration,
    pub jwt_leeway: Duration,
}

#[derive(Debug, Clone)]
pub struct SwaggerConfig {
    pub username: Option<String>,
    pub password: Option<String>,
    pub title: String,
    pub version: String,
    pub description: String,
}

/// Configuration for Logto Machine-to-Machine (M2M) authentication
/// Used for fetching management API tokens from Logto
#[derive(Debug, Clone)]
pub struct LogtoM2MConfig {
    pub client_id: String,
    pub client_secret: String,
    pub resource: String,
    pub scope: String,
    pub token_url: String,
    pub api_base_url: String,
}

/// MinIO/S3 storage configuration for file uploads
#[derive(Debug, Clone)]
pub struct MinIOConfig {
    /// MinIO/S3 endpoint URL
    pub endpoint: String,
    /// Public endpoint URL for publicly accessible files (optional, defaults to endpoint)
    pub public_endpoint: String,
    /// Access key for authentication
    pub access_key: String,
    /// Secret key for authentication
    pub secret_key: String,
    /// Bucket name for storing files
    pub bucket: String,
    /// AWS region (for S3 compatibility)
    pub region: String,
    /// Prefix for public files (e.g., "public")
    pub public_prefix: String,
    /// Prefix for private files (e.g., "private")
    pub private_prefix: String,
    /// Presigned URL expiry time in seconds
    pub presigned_url_expiry_secs: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        // Load .env file if exists, ignore if not found (optional for production)
        if let Err(e) = dotenvy::dotenv() {
            // Only error if it's not "file not found" - that's acceptable
            if !e.to_string().contains("not found") {
                eprintln!("Warning: Error loading .env file: {}", e);
            }
        }

        Ok(Config {
            app: AppConfig::from_env()?,
            database: DatabaseConfig::from_env()?,
            auth: AuthConfig::from_env()?,
            swagger: SwaggerConfig::from_env()?,
            logto_m2m: LogtoM2MConfig::from_env()?,
            minio: MinIOConfig::from_env()?,
        })
    }
}

impl AppConfig {
    const DEFAULT_MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB

    pub fn from_env() -> Result<Self, String> {
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|e| format!("Invalid PORT: {}", e))?;

        // Parse CORS allowed origins from comma-separated string
        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "*".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let max_request_body_size = env::var("MAX_REQUEST_BODY_SIZE")
            .unwrap_or_else(|_| Self::DEFAULT_MAX_REQUEST_BODY_SIZE.to_string())
            .parse::<usize>()
            .map_err(|_| "MAX_REQUEST_BODY_SIZE must be a valid number".to_string())?;

        let frontend_url =
            env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        Ok(Self {
            host,
            port,
            cors_allowed_origins,
            max_request_body_size,
            frontend_url,
        })
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl DatabaseConfig {
    // Default values for database connection pool (conservative defaults for small-medium apps)
    const DEFAULT_MAX_CONNECTIONS: u32 = 10;
    const DEFAULT_MIN_CONNECTIONS: u32 = 1;
    const DEFAULT_ACQUIRE_TIMEOUT_SECS: u64 = 5;
    const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 600; // 10 minutes
    const DEFAULT_MAX_LIFETIME_SECS: u64 = 1800; // 30 minutes

    pub fn from_env() -> Result<Self, String> {
        let url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set".to_string())?;

        let max_connections = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| Self::DEFAULT_MAX_CONNECTIONS.to_string())
            .parse::<u32>()
            .map_err(|_| "DB_MAX_CONNECTIONS must be a valid number".to_string())?;

        let min_connections = env::var("DB_MIN_CONNECTIONS")
            .unwrap_or_else(|_| Self::DEFAULT_MIN_CONNECTIONS.to_string())
            .parse::<u32>()
            .map_err(|_| "DB_MIN_CONNECTIONS must be a valid number".to_string())?;

        let acquire_timeout_secs = env::var("DB_ACQUIRE_TIMEOUT_SECS")
            .unwrap_or_else(|_| Self::DEFAULT_ACQUIRE_TIMEOUT_SECS.to_string())
            .parse::<u64>()
            .map_err(|_| "DB_ACQUIRE_TIMEOUT_SECS must be a valid number".to_string())?;

        let idle_timeout_secs = env::var("DB_IDLE_TIMEOUT_SECS")
            .unwrap_or_else(|_| Self::DEFAULT_IDLE_TIMEOUT_SECS.to_string())
            .parse::<u64>()
            .map_err(|_| "DB_IDLE_TIMEOUT_SECS must be a valid number".to_string())?;

        let max_lifetime_secs = env::var("DB_MAX_LIFETIME_SECS")
            .unwrap_or_else(|_| Self::DEFAULT_MAX_LIFETIME_SECS.to_string())
            .parse::<u64>()
            .map_err(|_| "DB_MAX_LIFETIME_SECS must be a valid number".to_string())?;

        Ok(Self {
            url,
            max_connections,
            min_connections,
            acquire_timeout_secs,
            idle_timeout_secs,
            max_lifetime_secs,
        })
    }
}

impl AuthConfig {
    // Default values for JWT authentication
    const DEFAULT_JWKS_CACHE_TTL_SECS: u64 = 3600; // 1 hour
    const DEFAULT_JWT_LEEWAY_SECS: u64 = 60; // 1 minute

    pub fn from_env() -> Result<Self, String> {
        let issuer = env::var("LOGTO_ISSUER")
            .map_err(|_| "LOGTO_ISSUER environment variable is required".to_string())?;

        let audience = env::var("LOGTO_AUDIENCE")
            .map_err(|_| "LOGTO_AUDIENCE environment variable is required".to_string())?;

        let jwks_cache_ttl_secs = env::var("JWKS_CACHE_TTL")
            .unwrap_or_else(|_| Self::DEFAULT_JWKS_CACHE_TTL_SECS.to_string())
            .parse::<u64>()
            .map_err(|_| "JWKS_CACHE_TTL must be a valid number".to_string())?;

        let jwt_leeway_secs = env::var("JWT_LEEWAY")
            .unwrap_or_else(|_| Self::DEFAULT_JWT_LEEWAY_SECS.to_string())
            .parse::<u64>()
            .map_err(|_| "JWT_LEEWAY must be a valid number".to_string())?;

        Ok(Self {
            issuer,
            audience,
            jwks_cache_ttl: Duration::from_secs(jwks_cache_ttl_secs),
            jwt_leeway: Duration::from_secs(jwt_leeway_secs),
        })
    }
}

impl SwaggerConfig {
    pub fn from_env() -> Result<Self, String> {
        // Only use credentials if they are non-empty
        let username = env::var("SWAGGER_USERNAME").ok().filter(|s| !s.is_empty());
        let password = env::var("SWAGGER_PASSWORD").ok().filter(|s| !s.is_empty());
        let title = env::var("SWAGGER_TITLE").unwrap_or_else(|_| "Balungpisah API".to_string());
        let version = env::var("SWAGGER_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
        let description = env::var("SWAGGER_DESCRIPTION")
            .unwrap_or_else(|_| "API documentation for Balungpisah".to_string());

        Ok(Self {
            username,
            password,
            title,
            version,
            description,
        })
    }

    /// Returns credentials in "username:password" format if auth is enabled
    pub fn credentials(&self) -> Option<String> {
        match (&self.username, &self.password) {
            (Some(user), Some(pass)) => Some(format!("{}:{}", user, pass)),
            _ => None,
        }
    }
}

impl LogtoM2MConfig {
    pub fn from_env() -> Result<Self, String> {
        let client_id = env::var("LOGTO_M2M_CLIENT_ID")
            .map_err(|_| "LOGTO_M2M_CLIENT_ID environment variable is required".to_string())?;

        let client_secret = env::var("LOGTO_M2M_CLIENT_SECRET")
            .map_err(|_| "LOGTO_M2M_CLIENT_SECRET environment variable is required".to_string())?;

        let resource = env::var("LOGTO_M2M_RESOURCE")
            .unwrap_or_else(|_| "https://default.logto.app/api".to_string());

        let scope = env::var("LOGTO_M2M_SCOPE").unwrap_or_else(|_| "all".to_string());

        // Derive token_url from LOGTO_ISSUER
        let issuer = env::var("LOGTO_ISSUER")
            .map_err(|_| "LOGTO_ISSUER environment variable is required".to_string())?;
        let token_url = format!("{}/token", issuer);

        // Derive api_base_url (remove /oidc from issuer if present)
        let api_base_url = issuer.trim_end_matches("/oidc").to_string();

        Ok(Self {
            client_id,
            client_secret,
            resource,
            scope,
            token_url,
            api_base_url,
        })
    }
}

impl MinIOConfig {
    const DEFAULT_PRESIGNED_URL_EXPIRY_SECS: u32 = 3600; // 1 hour

    pub fn from_env() -> Result<Self, String> {
        let endpoint =
            env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string());

        // Public endpoint defaults to the main endpoint if not specified
        let public_endpoint =
            env::var("MINIO_PUBLIC_ENDPOINT").unwrap_or_else(|_| endpoint.clone());

        let access_key = env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());

        let secret_key = env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());

        let bucket = env::var("MINIO_BUCKET").unwrap_or_else(|_| "balungpisah-uploads".to_string());

        let region = env::var("MINIO_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let public_prefix =
            env::var("MINIO_PUBLIC_PREFIX").unwrap_or_else(|_| "public".to_string());

        let private_prefix =
            env::var("MINIO_PRIVATE_PREFIX").unwrap_or_else(|_| "private".to_string());

        let presigned_url_expiry_secs = env::var("MINIO_PRESIGNED_URL_EXPIRY_SECS")
            .unwrap_or_else(|_| Self::DEFAULT_PRESIGNED_URL_EXPIRY_SECS.to_string())
            .parse::<u32>()
            .map_err(|_| "MINIO_PRESIGNED_URL_EXPIRY_SECS must be a valid number".to_string())?;

        Ok(Self {
            endpoint,
            public_endpoint,
            access_key,
            secret_key,
            bucket,
            region,
            public_prefix,
            private_prefix,
            presigned_url_expiry_secs,
        })
    }
}

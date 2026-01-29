# Architecture Guide

This document explains the architecture and design patterns used in the Service Sql Agents API.

## Overview

The project follows a **feature-based architecture** with clean separation of concerns across three main layers:

```
┌─────────────────────────────────────┐
│         HTTP Layer (Axum)           │  ← Handlers, Routes, Middleware
├─────────────────────────────────────┤
│       Service Layer (Business)      │  ← Business logic, validation
├─────────────────────────────────────┤
│    Repository Layer (Database)      │  ← Database queries, models
└─────────────────────────────────────┘
```

## Project Structure

```
src/
├── main.rs                 # Application entry point, router setup
├── core/                   # Core infrastructure (shared across features)
│   ├── config.rs          # Environment-based configuration
│   ├── database.rs        # PostgreSQL connection pooling
│   ├── error.rs           # Centralized error handling
│   ├── middleware.rs      # Request middleware (auth, logging)
│   └── extractor.rs       # Custom Axum extractors
├── shared/                # Shared utilities and types
│   ├── types.rs          # Common types (ApiResponse, Pagination)
│   └── constants.rs      # Application constants
└── features/             # Feature-based modules (plural naming)
    ├── auth/             # Authentication & authorization
    └── <feature_plural>/ # Each feature follows same structure
        ├── dtos/
        │   ├── <singular>_dto.rs    # Request/response DTOs
        │   └── mod.rs
        ├── models/
        │   ├── <singular>_model.rs  # Database models
        │   └── mod.rs
        ├── services/
        │   ├── <singular>_service.rs # Business logic
        │   ├── <singular>_service_test.rs # Service tests
        │   └── mod.rs
        ├── handlers/
        │   ├── <singular>_handler.rs # HTTP handlers
        │   ├── <singular>_handler_test.rs # Handler tests
        │   └── mod.rs
        ├── routes.rs     # Route registration
        ├── test_utils.rs # Test utilities
        └── mod.rs        # Module exports
```

**Example** (for a "products" feature):
```
features/products/
├── dtos/
│   ├── product_dto.rs
│   └── mod.rs
├── models/
│   ├── product_model.rs
│   └── mod.rs
├── services/
│   ├── product_service.rs
│   ├── product_service_test.rs
│   └── mod.rs
├── handlers/
│   ├── product_handler.rs
│   ├── product_handler_test.rs
│   └── mod.rs
├── routes.rs
├── test_utils.rs
└── mod.rs
```

## Layer Responsibilities

### 1. Core Layer - Infrastructure
- **config.rs** - Environment configuration (`AppConfig`, `DatabaseConfig`, `AuthConfig`)
- **database.rs** - PostgreSQL connection pooling, auto migrations
- **error.rs** - `AppError` enum, HTTP error responses
- **middleware.rs** - JWT auth, basic auth, logging
- **extractor.rs** - `ValidatedJson`, `AuthUser`, role guards

### 2. Shared Layer - Common Utilities
- **types.rs** - `ApiResponse<T>`, pagination types
- **constants.rs** - `DEFAULT_PAGE_SIZE`, `MAX_PAGE_SIZE`

### 3. Features Layer - Business Modules
Each feature (named in plural) contains organized subfolders:
- **dtos/** - Request/response DTOs with validation
- **models/** - Database models (SQLx types)
- **services/** - Business logic with tests
- **handlers/** - HTTP handlers with tests
- **routes.rs** - Route registration
- **test_utils.rs** - Shared test utilities
- **mod.rs** - Module exports

## Authentication System

JWT authentication with JWKS validation (Logto OIDC provider).

**Flow:** Request → `auth_middleware` → `JwtValidator` → `JwksClient` → Validate → `AuthenticatedUser`

**Components:**
- `JwksClient` - Caches public keys from JWKS endpoint
- `JwtValidator` - Validates token (signature, issuer, audience, claims)
- `auth_middleware` - Extracts JWT, injects `AuthenticatedUser` to request
- `AuthUser` extractor - Retrieve user in handlers
- Role guards - `RequireAdmin`, `RequireSuperAdmin`

**Usage:**
```rust
// Protect routes
Router::new()
    .route("/api/protected", get(handler))
    .route_layer(middleware::from_fn_with_state(jwt_validator, auth_middleware));

// In handler
async fn handler(AuthUser(user): AuthUser) -> Result<...> {
    // user.org_id, user.role available
}
```

## Design Patterns

### 1. Service Layer Pattern

Services are initialized with `PgPool` and registered as `Arc<Service>` in main.rs.

```rust
// Service definition
pub struct ProductService {
    pool: PgPool,
}

impl ProductService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, dto: CreateDto) -> Result<ResponseDto> {
        // Business logic here
    }
}

// Registration in main.rs
let product_service = Arc::new(ProductService::new(pool.clone()));
```

### 2. Handler Pattern

Handlers use Axum extractors and return consistent responses.

```rust
#[utoipa::path(
    post,
    path = "/api/products",
    request_body = CreateProductDto,
    responses(
        (status = 201, description = "Product created", body = ApiResponse<ProductResponseDto>)
    )
)]
pub async fn create_product(
    State(service): State<Arc<ProductService>>,
    ValidatedJson(dto): ValidatedJson<CreateProductDto>,
) -> Result<Json<ApiResponse<ProductResponseDto>>, AppError> {
    let product = service.create(dto).await?;
    Ok(Json(ApiResponse::success(product, None)))
}
```

### 3. Error Handling Pattern

All services return `Result<T>` (alias for `std::result::Result<T, AppError>`).

```rust
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    InternalServerError(String),
    DatabaseError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Converts to appropriate HTTP status code
    }
}
```

### 4. DTO Validation Pattern

Request DTOs use `validator` crate for automatic validation.

```rust
#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateProductDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(email)]
    pub email: Option<String>,
}

// Handler uses ValidatedJson extractor
async fn create(
    ValidatedJson(dto): ValidatedJson<CreateProductDto>,
) -> Result<...> {
    // dto is already validated
}
```

## Database Patterns

**Type-Safe Queries:** Use `sqlx::query_as!` for compile-time checks
**Dynamic Queries:** Use `QueryBuilder` for conditional filtering
**Soft Deletes:** Use `deleted_at` column (NULL = active, timestamp = deleted)

## Security Patterns

### 1. Query Safety: Compile-Time vs Runtime Checking

**CRITICAL:** Always prefer compile-time checked queries for security and type safety.

#### Compile-Time Checked Queries (Preferred)

Use `sqlx::query!` or `sqlx::query_as!` macros for all standard queries:

```rust
// ✅ GOOD: Compile-time checked - SQL errors caught at compile time
let agent = sqlx::query_as!(
    SqlAgent,
    r#"
    SELECT id, name, slug, owner, description, is_active,
           database_connection_id, created_at, updated_at
    FROM sql_agents
    WHERE id = $1 AND owner = $2
    "#,
    sql_agent_id,
    owner
)
.fetch_one(&self.pool)
.await?;
```

**Benefits:**
- SQL syntax errors caught at compile time
- Schema changes detected immediately during build
- Column type mismatches prevented
- Typos in table/column names caught early
- Refactoring safety (compiler ensures all queries updated)

#### Runtime Checked Queries (Exceptions Only)

Only use `sqlx::query` or `sqlx::query_as::<_, T>` when absolutely necessary:

```rust
// ⚠️ ACCEPTABLE EXCEPTION: Models with #[sqlx(json)] attribute
// Runtime-checked due to SQLx limitation with JSON fields
let conn = sqlx::query_as::<_, DatabaseConnection>(
    "SELECT id, name, type, host, port, database_name,
     username, password, additional_config, created_at, updated_at
     FROM database_connections WHERE id = $1"
)
.bind(connection_id)
.fetch_one(&self.pool)
.await?;
```

**Acceptable Exception Cases:**
1. **Models with `#[sqlx(json)]` attribute** - SQLx compile-time macros have limitations with JSON type inference
2. **Dynamic `QueryBuilder` queries** - Queries built conditionally at runtime (filtering, pagination)

**Always document exceptions with inline comments explaining why runtime checking is necessary.**

### 2. Type Safety with Custom Enums

When using custom Rust enums mapped to database types, provide explicit type hints:

```rust
// ✅ GOOD: Type hint for custom enum
sqlx::query_as!(
    DatabaseConnection,
    r#"
    SELECT id, name, type as "type: DatabaseType", host, port
    FROM database_connections
    WHERE id = $1
    "#,
    connection_id
)

// ❌ BAD: Missing type hint
sqlx::query_as!(
    DatabaseConnection,
    "SELECT id, name, type, host, port FROM database_connections WHERE id = $1",
    connection_id
)
```

The `"type: DatabaseType"` syntax tells SQLx to map the column to your custom enum type.

### 3. Error Handling Patterns

**Never use `.unwrap()` or `.expect()` in production code** - it creates panic vulnerabilities.

```rust
// ❌ BAD: Panic vulnerability
let db_type: DatabaseType = type_str.parse().unwrap();

// ✅ GOOD: Proper error handling
let db_type: DatabaseType = type_str
    .parse()
    .map_err(|_| AppError::Internal("Invalid database type".to_string()))?;
```

**Pattern for Result conversion:**
```rust
// Converting external errors to AppError
let value = some_operation()
    .map_err(|e| AppError::Internal(format!("Operation failed: {}", e)))?;
```

**Pattern for Option unwrapping:**
```rust
// ❌ BAD: Panic on None
let value = optional_value.unwrap();

// ✅ GOOD: Handle None case
let value = optional_value.ok_or_else(||
    AppError::NotFound("Resource not found".to_string())
)?;
```

### 4. Encryption Key Management

**Never read secrets from environment variables repeatedly** - cache them in configuration.

```rust
// ❌ BAD: Reading env var on every encryption
pub fn encrypt_password(&self, password: &str) -> Result<String> {
    let key = env::var("ENCRYPTION_KEY").unwrap();  // Multiple issues!
    // ...
}

// ✅ GOOD: Key cached in service struct from config
pub struct DatabaseConnectionService {
    pool: PgPool,
    encryption_key: Vec<u8>,  // Cached from EncryptionConfig
}

impl DatabaseConnectionService {
    pub fn new(pool: PgPool, encryption_config: EncryptionConfig) -> Self {
        Self {
            pool,
            encryption_key: encryption_config.key,
        }
    }

    pub fn encrypt_password(&self, password: &str) -> Result<String> {
        // Use cached key
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)?;
        // ...
    }
}
```

**Encryption configuration validation:**
```rust
impl EncryptionConfig {
    pub fn from_env() -> Result<Self, String> {
        let key_str = env::var("ENCRYPTION_KEY")
            .map_err(|_| "ENCRYPTION_KEY must be set")?;

        let key = general_purpose::STANDARD
            .decode(&key_str)
            .map_err(|_| "ENCRYPTION_KEY must be valid base64")?;

        // Validate key size for AES-256
        if key.len() != 32 {
            return Err("ENCRYPTION_KEY must be 32 bytes (256 bits)".to_string());
        }

        Ok(Self { key })
    }
}
```

**Benefits:**
- Validation happens once at startup (fail-fast)
- Better performance (no repeated env lookups)
- Cleaner service code
- Easier testing (inject test keys)

### 5. SQL Injection Prevention

**Always use parameterized queries** - never concatenate user input into SQL:

```rust
// ❌ DANGEROUS: SQL injection vulnerability
let query = format!("SELECT * FROM users WHERE id = {}", user_id);
sqlx::query(&query).fetch_one(&pool).await?;

// ✅ SAFE: Parameterized query
sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;
```

Even with `QueryBuilder` for dynamic queries, always bind parameters:

```rust
// ✅ SAFE: QueryBuilder with bind
let mut query = QueryBuilder::new("SELECT * FROM users WHERE 1=1");
if let Some(name) = filters.name {
    query.push(" AND name = ");
    query.push_bind(name);  // Properly bound
}
```

### 6. Security Checklist for Database Code

When writing database operations, verify:

- [ ] Using `sqlx::query_as!` or `sqlx::query!` (compile-time checked)
- [ ] If runtime-checked, documented reason with comment
- [ ] No `.unwrap()` or `.expect()` calls
- [ ] All user inputs are bound parameters (not concatenated)
- [ ] Sensitive data (passwords, keys) encrypted before storage
- [ ] Encryption keys cached from config, not read from env repeatedly
- [ ] Custom enum types have explicit type hints
- [ ] Proper error handling with `AppError`
- [ ] Owner/organization checks for multi-tenant data access

## Testing Strategy

- **Unit Tests** - Use `#[sqlx::test]` for isolated database per test
- **Integration Tests** - Test HTTP endpoints end-to-end

See [Testing Guide](./testing.md) for details.

## OpenAPI Documentation

Use `#[utoipa::path]` on handlers, register in main.rs `#[openapi]` macro.
Access: `http://localhost:3000/swagger-ui`

## Best Practices

1. **Always use the service layer** - Never put business logic in handlers
2. **Return `Result<T>` from services** - Use `AppError` for errors
3. **Use DTOs for input/output** - Never expose database models directly
4. **Validate at the boundary** - Use `ValidatedJson` for all inputs
5. **Use parameterized queries** - Prevent SQL injection
6. **Test both happy and error paths** - Use `#[sqlx::test]` for DB tests
7. **Document handlers with `#[utoipa::path]`** - Keep OpenAPI docs in sync

## Common Gotchas

- **Don't modify applied migrations** - Create new revert migrations instead
- **Don't use `unwrap()` in production code** - Always handle errors properly
- **Don't hardcode database schemas in tests** - Use actual migrations
- **Don't forget to register new routes** - Add to router in main.rs
- **Don't expose internal errors** - Sanitize errors before sending to client

---

For implementation examples, see [Adding Features Guide](./adding-features.md).

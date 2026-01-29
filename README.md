# Balungpisah API

Production-ready Rust API boilerplate built with Axum, featuring a feature-based architecture for small to medium projects.

## Tech Stack

- **Framework:** Axum 0.8
- **Runtime:** Tokio
- **Database:** PostgreSQL with SQLx
- **Authentication:** JWT/OIDC (Logto)
- **Validation:** Validator
- **Documentation:** OpenAPI (utoipa)
- **Serialization:** Serde
- **Logging:** Tracing

## Quick Start

### Prerequisites

- Rust 1.75+
- PostgreSQL 14+
- SQLx CLI: `cargo install sqlx-cli --no-default-features --features postgres`

### Setup

```bash
# Clone repository
git clone <repository-url>
cd balungpisah-core

# Configure environment
cp .env.example .env
# Edit .env with your database URL and other configs

# Create database and run migrations
createdb balungpisah
sqlx migrate run

# Run application
cargo run
```

Server starts at `http://127.0.0.1:3000`

### Verify Installation

```bash
# Health check
curl http://localhost:3000/health

# Swagger UI (login: admin/admin)
open http://localhost:3000/swagger-ui
```

## Key Features

- 🔐 **JWT Authentication** - OIDC/JWKS validation with Logto
- 🧪 **Comprehensive Testing** - Unit and integration tests with `#[sqlx::test]`
- 📚 **Auto-generated API Docs** - OpenAPI/Swagger UI
- 🔄 **Soft Deletes** - Logical deletion with `deleted_at`
- 🏢 **Multi-tenant Ready** - Organization-scoped data isolation
- 📁 **Feature-based Architecture** - Organized with plural subfolder naming

## Project Structure

```
src/
├── main.rs              # Application entry point
├── core/                # Infrastructure (config, database, error, middleware)
├── shared/              # Common types and utilities
└── features/            # Feature modules (plural naming convention)
    ├── auth/            # Authentication & authorization
    └── <feature>/       # Each feature has subfolders:
        ├── dtos/        # Request/response DTOs
        ├── models/      # Database models
        ├── services/    # Business logic
        ├── handlers/    # HTTP handlers
        ├── routes.rs    # Route registration
        └── mod.rs       # Module exports
```

## Development

### Common Commands

```bash
# Development
make run          # Run application
make dev          # Run with auto-reload (requires cargo-watch)
make test         # Run tests

# Code Quality
make fmt          # Format code
make clippy       # Run linter

# Database
make db-reset     # Reset database
make migrate-add NAME=<name>  # Create migration
```

See [Quick Reference](docs/QUICK_REFERENCE.md) for all commands.

## Documentation

### Getting Started
- 📖 [Getting Started Guide](docs/development/getting-started.md) - Complete setup guide
- 🏗️ [Architecture Guide](docs/development/architecture.md) - System design and patterns
- ✅ [Testing Guide](docs/development/testing.md) - Testing practices
- 🔀 [Git Workflow](docs/development/git-workflow.md) - Branching strategy

### Development
- ➕ [Adding Features](docs/development/adding-features.md) - Step-by-step feature development
- 🗄️ [Migrations Guide](docs/development/migrations.md) - Database migrations
- ⚡ [Quick Reference](docs/QUICK_REFERENCE.md) - Command cheatsheet

## API Endpoints

Access interactive API documentation at `http://localhost:3000/swagger-ui`

**Base endpoints:**
- `GET /health` - Health check
- `POST /api/auth/*` - Authentication (when implemented)

## Configuration

All configuration via environment variables (`.env` file):

```env
# Server
HOST=127.0.0.1
PORT=3000

# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/balungpisah

# Authentication (Logto OIDC)
LOGTO_ISSUER=https://your-logto.com/oidc
LOGTO_AUDIENCE=https://your-api.com/api

# Swagger
SWAGGER_USERNAME=admin
SWAGGER_PASSWORD=admin
```

See `.env.example` for all available options.

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_create_product -- --nocapture

# Run with coverage
cargo test --all-features
```

Tests use `#[sqlx::test]` for isolated databases. See [Testing Guide](docs/development/testing.md).

## License

MIT

---

**For detailed documentation, see the [docs](docs/) directory.**

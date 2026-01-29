# Getting Started

Complete guide to setting up the project on your local machine.

## Prerequisites

- **Rust** 1.75+ ([Install](https://rustup.rs/))
- **PostgreSQL** 14+ ([Install](https://www.postgresql.org/download/))
- **SQLx CLI** - Install with: `cargo install sqlx-cli --no-default-features --features postgres`

## Quick Setup

### 1. Install Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PostgreSQL (choose one)
brew install postgresql@15 && brew services start postgresql@15  # macOS
sudo apt install postgresql && sudo systemctl start postgresql   # Linux
docker run --name postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 -d postgres:15  # Docker

# Install SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres
```

## Project Setup

### 2. Clone and Configure

```bash
git clone <repository-url>
cd balungpisah-core
cp .env.example .env
```

Edit `.env`:
```env
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/db_your_app
LOGTO_ISSUER=https://your-logto-instance.com/oidc
LOGTO_AUDIENCE=https://your-api.com/api
```

### 3. Create Database & Run Migrations

```bash
createdb db_your_app
sqlx migrate run
```

### 4. Build and Run

```bash
cargo build
cargo run
```

Server starts on `http://127.0.0.1:3000`

## Verify Installation

```bash
# Check health
curl http://localhost:3000/health

# Access Swagger UI
open http://localhost:3000/swagger-ui  # Login: admin/admin

# Run tests
cargo test
```

## Development Tools

```bash
# Auto-reload (install cargo-watch first)
cargo install cargo-watch
make dev

# Code quality
make fmt      # Format code
make clippy   # Run linter

# Database
make migrate-add NAME=<name>  # Create migration
make db-reset                 # Reset database
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `connection refused` | Check PostgreSQL: `pg_isready -h localhost -p 5432` |
| `migration checksum mismatch` | Reset DB: `make db-reset` |
| `address already in use` | Kill process: `lsof -i :3000` then `kill -9 <PID>` |
| SQLx offline errors | Run: `cargo sqlx prepare` |

## Next Steps

- [Architecture Guide](./architecture.md) - Understand codebase structure
- [Testing Guide](./testing.md) - Write and run tests
- [Adding Features Guide](./adding-features.md) - Add new features
- [Quick Reference](../QUICK_REFERENCE.md) - Command cheatsheet

.PHONY: help run build test clean migrate-run migrate-revert migrate-info migrate-add db-create db-drop dev check fmt clippy test-watch cross-build docker-build

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

run: ## Run the API server (migrations run automatically)
	cargo run

build: ## Build release binary
	cargo build --release

dev: ## Run in development mode with auto-reload (requires cargo-watch)
	cargo watch -x run

test: ## Run all tests (parallel execution)
	cargo test

test-watch: ## Run tests in watch mode (requires cargo-watch)
	cargo watch -x test

check: ## Check code without building
	cargo check

fmt: ## Format code
	cargo fmt

clippy: ## Run clippy linter
	cargo clippy -- -D warnings

clean: ## Clean build artifacts
	cargo clean

# Database Commands
db-create: ## Create database
	sqlx database create

db-drop: ## Drop database
	sqlx database drop

db-reset: ## Drop and recreate database
	sqlx database drop -y && sqlx database create

# Migration Commands (SQLx)
migrate-run: ## Run pending migrations
	sqlx migrate run

migrate-revert: ## Revert last migration
	@echo "Note: SQLx doesn't have built-in revert. Create a revert migration instead:"
	@echo "  make migrate-add NAME=revert_<feature_name>"

migrate-info: ## Show migration status
	sqlx migrate info

migrate-add: ## Create new migration (use NAME=migration_name)
	@if [ -z "$(NAME)" ]; then \
		echo "Error: NAME is required. Usage: make migrate-add NAME=migration_name"; \
		exit 1; \
	fi
	sqlx migrate add $(NAME)

# Development Helpers
prepare: ## Prepare SQLx offline data for CI/CD
	cargo sqlx prepare

setup: db-create migrate-run ## Setup database (create + migrate)
	@echo "Database setup complete"

# Testing Helpers
test-db-cleanup: ## Clean up orphan test databases
	@echo "Cleaning up test databases..."
	@docker exec shared-postgres psql -U postgres -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname LIKE '_sqlx%';" || true
	@docker exec shared-postgres psql -U postgres -c "DO \$$\$$ DECLARE db_name TEXT; BEGIN FOR db_name IN SELECT datname FROM pg_database WHERE datname LIKE '_sqlx%' LOOP EXECUTE 'DROP DATABASE IF EXISTS ' || quote_ident(db_name); END LOOP; END \$$\$$;" || true
	@echo "Cleanup complete"

# Cross-compilation (for ARM Mac building linux/amd64)
CROSS_TARGET := x86_64-unknown-linux-gnu

cross-build: ## Cross-compile for linux/amd64 (requires cargo-zigbuild)
	@echo "Cross-compiling for $(CROSS_TARGET)..."
	SQLX_OFFLINE=true cargo zigbuild --release --target $(CROSS_TARGET)
	@echo "Binary: target/$(CROSS_TARGET)/release/balungpisah-core"

docker-build: cross-build ## Build Docker image with cross-compiled binary
	@echo "Building Docker image..."
	docker build --platform linux/amd64 -t balungpisah-core:local .
	@echo "Docker image built: balungpisah-core:local"


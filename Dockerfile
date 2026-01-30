# Multi-stage Dockerfile for building from source
# Use this when you clone the repo and want to run with docker compose up
#
# Usage:
#   git clone https://github.com/balungpisah/balungpisah-core.git
#   cd balungpisah-core
#   cp .env.example .env
#   docker compose up -d
#
# For maintainer deployments (pre-built binary), see Dockerfile.deploy

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Copy SQLx offline data if exists (for offline compilation)
COPY .sqlx ./.sqlx

# Touch main.rs to invalidate the dummy build
RUN touch src/main.rs

# Build the actual application
ENV SQLX_OFFLINE=true
RUN cargo build --release

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/balungpisah-core ./balungpisah-core

# Copy migrations for runtime execution
COPY --from=builder /app/migrations ./migrations

# Create non-root user for security
RUN useradd -r -s /bin/false appuser && \
    chown -R appuser:appuser /app
USER appuser

# Expose default port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["./balungpisah-core"]

# ============================================================================
# Resonance API - Multi-Stage Docker Build
# ============================================================================
# Stage 1: Builder - Compile Rust application
# Stage 2: Runtime - Minimal production image
# ============================================================================

# ----------------------------------------------------------------------------
# Stage 1: Builder
# ----------------------------------------------------------------------------
FROM rust:1-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY apps/api/Cargo.toml ./apps/api/
COPY apps/worker/Cargo.toml ./apps/worker/
COPY packages/shared-config/Cargo.toml ./packages/shared-config/

# Create dummy source files for dependency compilation
RUN mkdir -p apps/api/src apps/worker/src packages/shared-config/src \
    && echo 'fn main() {}' > apps/api/src/main.rs \
    && echo 'pub fn dummy() {}' > apps/api/src/lib.rs \
    && echo 'fn main() {}' > apps/worker/src/main.rs \
    && echo 'pub fn dummy() {}' > packages/shared-config/src/lib.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release -p resonance-api \
    && rm -rf apps/api/src apps/worker/src packages/shared-config/src

# Copy actual source code
COPY apps/api/src ./apps/api/src/
COPY apps/api/migrations ./apps/api/migrations/
COPY packages/shared-config/src ./packages/shared-config/src/

# Touch source files to ensure rebuild with actual source
RUN touch apps/api/src/main.rs apps/api/src/lib.rs packages/shared-config/src/lib.rs

# Build the release binary
RUN cargo build --release -p resonance-api

# Verify the binary exists
RUN test -f /app/target/release/resonance-api

# ----------------------------------------------------------------------------
# Stage 2: Runtime
# ----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Labels for container metadata
LABEL org.opencontainers.image.title="Resonance API" \
      org.opencontainers.image.description="Self-hosted music streaming API server" \
      org.opencontainers.image.vendor="Resonance" \
      org.opencontainers.image.source="https://github.com/cjvana/resonance" \
      org.opencontainers.image.licenses="MIT"

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 \
    ffmpeg \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN groupadd --gid 1000 resonance \
    && useradd --uid 1000 --gid resonance --shell /bin/bash --create-home resonance

# Create necessary directories
RUN mkdir -p /app/data /app/music \
    && chown -R resonance:resonance /app

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder --chown=resonance:resonance /app/target/release/resonance-api /app/resonance-api

# Copy migrations for runtime migrations
COPY --from=builder --chown=resonance:resonance /app/apps/api/migrations /app/migrations

# Switch to non-root user
USER resonance

# Environment variables with defaults
ENV RUST_LOG=info \
    PORT=8080 \
    HOST=0.0.0.0

# Expose API port
EXPOSE 8080

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the API server
CMD ["/app/resonance-api"]

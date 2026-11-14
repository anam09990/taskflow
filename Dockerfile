# ==============================================================================
# 1. Builder Stage
# ==============================================================================
FROM rust:slim-bookworm AS builder

WORKDIR /app

# Override release profile settings to stay within Render free tier 512MB RAM limit
ENV CARGO_PROFILE_RELEASE_LTO=false \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=4

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code, migrations, templates, and static assets
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY static ./static

# Compile release binary
RUN cargo build --release

# ==============================================================================
# 2. Runtime Stage
# ==============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies and clean apt cache
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Set working directory and create SQLite data directory
WORKDIR /app
RUN mkdir -p /data

# Copy compiled binary from builder stage to /app/taskflow
COPY --from=builder /app/target/release/taskflow /app/taskflow

# Default environment variables
ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=3000 \
    DATABASE_URL=sqlite:///data/taskflow.db \
    ENVIRONMENT=production

# Expose server port
EXPOSE 3000

# Container entrypoint
ENTRYPOINT ["/app/taskflow"]

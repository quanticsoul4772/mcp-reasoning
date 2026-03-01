# Multi-stage build for minimal image size
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev sqlite-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /build

# Copy dependency manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Copy source code
COPY src src
COPY migrations migrations
COPY benches benches
COPY tests tests

# Build release binary with static linking
# Set SQLX_OFFLINE=true to skip compile-time SQL verification
ENV SQLX_OFFLINE=true
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime image
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache sqlite-libs ca-certificates

# Create non-root user
RUN addgroup -g 1000 mcp && \
    adduser -D -u 1000 -G mcp mcp

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/mcp-reasoning /usr/local/bin/

# Create data directory
RUN mkdir -p /app/data && chown -R mcp:mcp /app

# Switch to non-root user
USER mcp

# Environment defaults
ENV DATABASE_PATH=/app/data/reasoning.db
ENV LOG_LEVEL=info
ENV MCP_TRANSPORT=stdio

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/usr/local/bin/mcp-reasoning", "--health"]

# Expose port for HTTP transport (optional)
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/mcp-reasoning"]

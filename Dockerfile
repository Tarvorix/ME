# Multi-stage build for Machine Empire server
# Stage 1: Build the Rust server binary
FROM rust:1.85-bookworm AS builder

WORKDIR /build

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/server/Cargo.toml crates/server/Cargo.toml
COPY crates/wasm/Cargo.toml crates/wasm/Cargo.toml

# Create stub source files so cargo can resolve deps
RUN mkdir -p crates/core/src crates/server/src crates/wasm/src && \
    echo "pub fn hello() -> &'static str { \"stub\" }" > crates/core/src/lib.rs && \
    echo "fn main() {}" > crates/server/src/main.rs && \
    echo "" > crates/wasm/src/lib.rs

# Pre-build dependencies (cached unless Cargo.toml changes)
RUN cargo build --release -p machine-empire-server 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Build the real server binary
RUN cargo build --release -p machine-empire-server

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash machine-empire

WORKDIR /app

# Copy the built binary
COPY --from=builder /build/target/release/machine-empire-server /app/machine-empire-server

# Set ownership
RUN chown -R machine-empire:machine-empire /app

USER machine-empire

# Expose ports: WebSocket (8080), MCP (8081), HTTP API (8082)
EXPOSE 8080 8081 8082

# Health check via HTTP API
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8082/health || exit 1

ENTRYPOINT ["/app/machine-empire-server"]
CMD ["--ws-port", "8080", "--mcp-port", "8081", "--http-port", "8082"]

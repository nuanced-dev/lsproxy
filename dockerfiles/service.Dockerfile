# Base LSProxy service - lightweight HTTP proxy that orchestrates language containers
# Multi-stage build to minimize image size

FROM rust:1.83.0-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy workspace structure for orchestrator build
# Need to copy all workspace members even though we're only building orchestrator
WORKDIR /usr/src
COPY Cargo.toml Cargo.lock ./
COPY crates/common crates/common/
COPY crates/orchestrator crates/orchestrator/
# Copy wrapper stub to satisfy workspace
COPY crates/wrapper/Cargo.toml crates/wrapper/Cargo.toml
COPY crates/wrapper/src crates/wrapper/src/

# Build release binary from workspace
RUN cargo build --release --bin lsproxy

# Runtime stage - minimal base
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime dependencies only
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /usr/src/target/release/lsproxy /usr/local/bin/lsproxy

# Create workspace directory
RUN mkdir -p /mnt/workspace && \
    chmod 755 /mnt/workspace

# Set workspace as default
WORKDIR /mnt/workspace

# Expose service port
EXPOSE 4444

# Run the service
CMD ["/usr/local/bin/lsproxy"]

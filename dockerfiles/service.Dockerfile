# Base LSProxy service - lightweight HTTP proxy that orchestrates language containers
# Multi-stage build to minimize image size

FROM rust:1.82.0-slim-bookworm AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy source code
COPY lsproxy/Cargo.toml lsproxy/Cargo.lock ./
COPY lsproxy/src ./src

# Build release binary
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
COPY --from=builder /usr/src/app/target/release/lsproxy /usr/local/bin/lsproxy

# Create workspace directory
RUN mkdir -p /mnt/workspace && \
    chmod 755 /mnt/workspace

# Set workspace as default
WORKDIR /mnt/workspace

# Expose service port
EXPOSE 4444

# Run the service
CMD ["/usr/local/bin/lsproxy"]

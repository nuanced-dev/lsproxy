# Base LSProxy service - lightweight HTTP proxy that orchestrates language containers
# Multi-stage build to minimize image size
# Supports linux/amd64 and linux/arm64

FROM --platform=$BUILDPLATFORM rust:1.91.1-slim-bookworm AS builder
ARG BUILDPLATFORM
ARG BUILDARCH
ARG TARGETPLATFORM
ARG TARGETARCH

# Set up cross-compilation tools and target based on build/target platform
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    && case "$TARGETPLATFORM" in \
    "linux/amd64") \
        if [ "$BUILDARCH" = "arm64" ]; then \
            # Cross-compile from arm64 to amd64 \
            dpkg --add-architecture amd64 && \
            apt-get update && \
            apt-get install -y gcc-x86-64-linux-gnu libssl-dev:amd64 && \
            rustup target add x86_64-unknown-linux-gnu && \
            # Configure cargo for cross-compilation \
            echo '[target.x86_64-unknown-linux-gnu]' > /usr/local/cargo/config.toml && \
            echo 'linker = "x86_64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml; \
        elif [ "$BUILDARCH" != "amd64" ]; then \
            echo "Unsupported build architecture for linux/amd64: $BUILDARCH" && exit 1; \
        fi \
        ;; \
    "linux/arm64") \
        if [ "$BUILDARCH" = "amd64" ]; then \
            # Cross-compile from amd64 to arm64 \
            dpkg --add-architecture arm64 && \
            apt-get update && \
            apt-get install -y gcc-aarch64-linux-gnu libssl-dev:arm64 && \
            rustup target add aarch64-unknown-linux-gnu && \
            # Configure cargo for cross-compilation \
            echo '[target.aarch64-unknown-linux-gnu]' > /usr/local/cargo/config.toml && \
            echo 'linker = "aarch64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml; \
        elif [ "$BUILDARCH" != "arm64" ]; then \
            echo "Unsupported build architecture for linux/arm64: $BUILDARCH" && exit 1; \
        fi \
        ;; \
    *) \
        echo "Unsupported target platform: $TARGETPLATFORM (BUILDARCH: $BUILDARCH)" && exit 1 \
        ;; \
    esac && \
    apt-get clean && \
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

# Build lsproxy binary from workspace with cross-compilation support
RUN mkdir -p /usr/src/bin && \
    case "$TARGETPLATFORM" in \
    "linux/amd64") \
        if [ "$BUILDARCH" = "arm64" ]; then \
            # Cross-compile from arm64 to amd64 with OpenSSL configuration \
            PKG_CONFIG_ALLOW_CROSS=1 \
            PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig \
            OPENSSL_DIR=/usr \
            OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu \
            OPENSSL_INCLUDE_DIR=/usr/include \
            cargo build --release --bin lsproxy --target x86_64-unknown-linux-gnu && \
            cp target/x86_64-unknown-linux-gnu/release/lsproxy /usr/src/bin/lsproxy; \
        elif [ "$BUILDARCH" = "amd64" ]; then \
            cargo build --release --bin lsproxy && \
            cp target/release/lsproxy /usr/src/bin/lsproxy; \
        fi \
        ;; \
    "linux/arm64") \
        if [ "$BUILDARCH" = "amd64" ]; then \
            # Cross-compile from amd64 to arm64 with OpenSSL configuration \
            PKG_CONFIG_ALLOW_CROSS=1 \
            PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig \
            OPENSSL_DIR=/usr \
            OPENSSL_LIB_DIR=/usr/lib/aarch64-linux-gnu \
            OPENSSL_INCLUDE_DIR=/usr/include \
            cargo build --release --bin lsproxy --target aarch64-unknown-linux-gnu && \
            cp target/aarch64-unknown-linux-gnu/release/lsproxy /usr/src/bin/lsproxy; \
        elif [ "$BUILDARCH" = "arm64" ]; then \
            cargo build --release --bin lsproxy && \
            cp target/release/lsproxy /usr/src/bin/lsproxy; \
        fi \
        ;; \
    esac

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

# Copy binary from builder (cross-compiled or native)
COPY --from=builder /usr/src/bin/lsproxy /usr/local/bin/lsproxy

# Create workspace directory
RUN mkdir -p /mnt/workspace && \
    chmod 755 /mnt/workspace

# Set workspace as default
WORKDIR /mnt/workspace

# Expose service port
EXPOSE 4444

# Run the service
CMD ["/usr/local/bin/lsproxy"]

# Base Nuanced LSP proxy service - lightweight HTTP proxy that orchestrates language containers
# Multi-stage build to minimize image size
# Supports linux/amd64 and linux/arm64

FROM --platform=$BUILDPLATFORM rust:1.91.1-slim-bookworm AS builder
ARG BUILDPLATFORM
ARG BUILDARCH
ARG TARGETPLATFORM
ARG TARGETARCH

ARG CONTAINER_REGISTRY
ARG LANGUAGE_IMAGE_VERSION
ARG SERVICE_IMAGE_VERSION
RUN test -n "$CONTAINER_REGISTRY" || (echo "Missing required build argument CONTAINER_REGISTRY" ; false)
RUN test -n "$SERVICE_IMAGE_VERSION" || (echo "Missing required build argument SERVICE_IMAGE_VERSION" ; false)

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

# Copy workspace structure for proxy build
# We only need to build the proxy binary, but Cargo workspaces require ALL workspace
# members to be present and valid before building any single member. The root Cargo.toml
# defines a workspace with: common, proxy, and wrapper. Cargo validates that every
# member's source files exist, even when building just one binary.
#
# To avoid copying the full wrapper crate source (which proxy doesn't depend on),
# we create a minimal stub that satisfies Cargo's workspace validation.
WORKDIR /usr/src
COPY Cargo.toml ./
# Cargo.lock is optional - if present, ensures reproducible builds with exact dependency versions.
# The wildcard syntax (Cargo.lock*) allows the build to succeed even if Cargo.lock is not committed.
# For production builds, Cargo.lock should be committed to ensure reproducibility across all builds.
COPY Cargo.lock* ./
COPY crates/common crates/common/
COPY crates/proxy crates/proxy/

# Create minimal wrapper stub to satisfy Cargo workspace requirements.
# The proxy crate does NOT depend on wrapper at runtime - they both depend on common.
# This stub exists only because Cargo requires all workspace members to have valid
# source files before it will build any member. Without this, `cargo build` fails with:
#   "error: failed to read `/usr/src/crates/wrapper/src/lib.rs`"
COPY crates/wrapper/Cargo.toml crates/wrapper/Cargo.toml
RUN mkdir -p crates/wrapper/src && \
    echo '// Minimal stub to satisfy Cargo workspace validation.' > crates/wrapper/src/lib.rs && \
    echo '// The proxy binary does not depend on wrapper - see proxy.Dockerfile for details.' >> crates/wrapper/src/lib.rs && \
    echo 'fn main() {}' > crates/wrapper/src/main.rs

# Build nuanced-lsp-proxy binary from workspace with cross-compilation support
# Uses BuildKit cache mounts to cache Cargo registry and build artifacts across builds.
# These caches persist even with --no-cache flag, significantly speeding up rebuilds.
# Note: Each image uses a distinct cache ID (cargo-registry-proxy, cargo-registry-wrapper)
# to prevent race conditions when building images in parallel.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry-proxy \
    --mount=type=cache,target=/usr/src/target,id=cargo-target-proxy \
    mkdir -p /usr/src/bin && \
    case "$TARGETPLATFORM" in \
    "linux/amd64") \
        if [ "$BUILDARCH" = "arm64" ]; then \
            # Cross-compile from arm64 to amd64 with OpenSSL configuration \
            PKG_CONFIG_ALLOW_CROSS=1 \
            PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig \
            OPENSSL_DIR=/usr \
            OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu \
            OPENSSL_INCLUDE_DIR=/usr/include \
            cargo build --release --bin nuanced-lsp-proxy --target x86_64-unknown-linux-gnu && \
            cp target/x86_64-unknown-linux-gnu/release/nuanced-lsp-proxy /usr/src/bin/nuanced-lsp-proxy; \
        elif [ "$BUILDARCH" = "amd64" ]; then \
            cargo build --release --bin nuanced-lsp-proxy && \
            cp target/release/nuanced-lsp-proxy /usr/src/bin/nuanced-lsp-proxy; \
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
            cargo build --release --bin nuanced-lsp-proxy --target aarch64-unknown-linux-gnu && \
            cp target/aarch64-unknown-linux-gnu/release/nuanced-lsp-proxy /usr/src/bin/nuanced-lsp-proxy; \
        elif [ "$BUILDARCH" = "arm64" ]; then \
            cargo build --release --bin nuanced-lsp-proxy && \
            cp target/release/nuanced-lsp-proxy /usr/src/bin/nuanced-lsp-proxy; \
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
COPY --from=builder /usr/src/bin/nuanced-lsp-proxy /usr/local/bin/nuanced-lsp-proxy

# Create workspace directory
RUN mkdir -p /mnt/workspace && \
    chmod 755 /mnt/workspace

# Set workspace as default
WORKDIR /mnt/workspace

# Expose service port
EXPOSE 4444

# Run the service
CMD ["/usr/local/bin/nuanced-lsp-proxy"]

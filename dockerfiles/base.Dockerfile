# Base image for all LSP server containers
# Contains lsp-wrapper binary and common system dependencies
# Supports linux/amd64 and linux/arm64

FROM --platform=$BUILDPLATFORM rust:1.83.0-slim-bookworm AS builder
ARG BUILDPLATFORM
ARG BUILDARCH
ARG TARGETPLATFORM
ARG TARGETARCH

# Set up cross-compilation tools and target based on build/target platform
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl && \
    case "$TARGETPLATFORM" in \
    "linux/amd64") \
    if [ "$BUILDARCH" = "arm64" ]; then \
    rustup target add x86_64-unknown-linux-gnu && \
    apt-get install -y gcc-x86-64-linux-gnu && \
    echo '[target.x86_64-unknown-linux-gnu]' > /usr/local/cargo/config.toml && \
    echo 'linker = "x86_64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml; \
    elif [ "$BUILDARCH" != "amd64" ]; then \
    echo "Unsupported build architecture for linux/amd64: $BUILDARCH" && exit 1; \
    fi \
    ;; \
    "linux/arm64") \
    if [ "$BUILDARCH" = "amd64" ]; then \
    rustup target add aarch64-unknown-linux-gnu && \
    apt-get install -y gcc-aarch64-linux-gnu && \
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

# Copy workspace structure for wrapper build
# Need to copy all workspace members even though we're only building wrapper
WORKDIR /usr/src
COPY Cargo.toml Cargo.lock ./
COPY crates/common crates/common/
COPY crates/wrapper crates/wrapper/
# Copy orchestrator stub to satisfy workspace (we need ast_grep configs anyway)
COPY crates/orchestrator/Cargo.toml crates/orchestrator/Cargo.toml
COPY crates/orchestrator/src crates/orchestrator/src/

# Build lsp-wrapper binary from workspace
RUN mkdir -p /usr/src/bin && \
    case "$TARGETPLATFORM" in \
    "linux/amd64") \
    if [ "$BUILDARCH" = "arm64" ]; then \
    cargo build --release --bin lsp-wrapper --target x86_64-unknown-linux-gnu && \
    cp target/x86_64-unknown-linux-gnu/release/lsp-wrapper /usr/src/bin/lsp-wrapper; \
    elif [ "$BUILDARCH" = "amd64" ]; then \
    cargo build --release --bin lsp-wrapper && \
    cp target/release/lsp-wrapper /usr/src/bin/lsp-wrapper; \
    fi \
    ;; \
    "linux/arm64") \
    if [ "$BUILDARCH" = "amd64" ]; then \
    cargo build --release --bin lsp-wrapper --target aarch64-unknown-linux-gnu && \
    cp target/aarch64-unknown-linux-gnu/release/lsp-wrapper /usr/src/bin/lsp-wrapper; \
    elif [ "$BUILDARCH" = "arm64" ]; then \
    cargo build --release --bin lsp-wrapper && \
    cp target/release/lsp-wrapper /usr/src/bin/lsp-wrapper; \
    fi \
    ;; \
    esac

# Runtime stage - minimal base without build tools
FROM debian:bookworm-slim AS base-runtime

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install runtime dependencies including Python for ast-grep-cli
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    curl \
    python3 \
    python3-pip \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install ast-grep CLI via pip (simpler and more reliable than binary)
RUN pip3 install --no-cache-dir ast-grep-cli --break-system-packages

# Create workspace directory with proper permissions
RUN mkdir -p /mnt/workspace && \
    chmod 755 /mnt/workspace

# Copy lsp-wrapper binary from builder
COPY --from=builder /usr/src/bin/lsp-wrapper /usr/local/bin/lsp-wrapper
RUN chmod +x /usr/local/bin/lsp-wrapper

# Copy ast-grep configs from builder (located in orchestrator source)
COPY --from=builder /usr/src/crates/orchestrator/src/ast_grep /usr/src/ast_grep

# Default port for LSP wrapper HTTP server
EXPOSE 8080

# Default workspace path (consistent with main LSProxy)
ENV WORKSPACE_PATH=/mnt/workspace

# ENTRYPOINT is set here, CMD will be provided by language-specific images
ENTRYPOINT ["/usr/local/bin/lsp-wrapper"]

# Build-enabled variant for languages that need to compile native extensions
# Use this as the base for:
#   - Ruby (ruby-lsp, sorbet) - compiles native gem extensions
#   - Python (jedi, pyright) - some packages have C extensions
#   - TypeScript/JavaScript (typescript-language-server) - some npm packages build native modules
#   - PHP (intelephense) - may need build tools for extensions
#
# For pure binary/JVM languages (Go, Rust, Java, C#), use base-runtime instead
# to save ~300MB per image
FROM base-runtime AS base-build

# Add build tools for languages that compile native extensions (Ruby, Python, etc.)
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    pkg-config \
    libssl3 \
    build-essential \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Wrapper binary container for binary injection architecture
# This image contains ONLY the lsp-wrapper binary and ast-grep configs
# Language containers mount this via --volumes-from at runtime
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

# Runtime stage - minimal image containing only wrapper binary and configs
FROM debian:bookworm-slim AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# Install minimal runtime dependencies including Python for ast-grep
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    python3 \
    python3-pip \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create directory structure for wrapper binary and configs
# This directory will be mounted into language containers via --volumes-from
RUN mkdir -p /opt/lsp-wrapper/bin /opt/lsp-wrapper/ast_grep

# Install ast-grep CLI via pip (needed by wrapper binary for ast-grep operations)
# Then copy it to /opt/lsp-wrapper/bin so it's available via volume mount
RUN pip3 install --no-cache-dir ast-grep-cli --break-system-packages && \
    cp /usr/local/bin/ast-grep /opt/lsp-wrapper/bin/ast-grep && \
    chmod +x /opt/lsp-wrapper/bin/ast-grep

# Copy ONLY the wrapper binary and ast-grep configs
COPY --from=builder /usr/src/bin/lsp-wrapper /opt/lsp-wrapper/bin/lsp-wrapper
RUN chmod +x /opt/lsp-wrapper/bin/lsp-wrapper

# Copy ast-grep configs from builder
COPY --from=builder /usr/src/crates/orchestrator/src/ast_grep /opt/lsp-wrapper/ast_grep

# Declare volume so --volumes-from can share these directories
VOLUME ["/opt/lsp-wrapper"]

# No ENTRYPOINT - this container is only for volume mounting
# The wrapper binary will be executed in language containers
CMD ["sleep", "infinity"]

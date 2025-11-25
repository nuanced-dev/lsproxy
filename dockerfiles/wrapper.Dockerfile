# Wrapper binary container for binary injection architecture
# This image contains ONLY the lsp-wrapper binary and ast-grep configs
# Language containers mount this via --volumes-from at runtime
# Supports linux/amd64 and linux/arm64

FROM --platform=$BUILDPLATFORM rust:1.91.1-slim-bookworm AS builder
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
# We only need to build the wrapper binary, but Cargo workspaces require ALL workspace
# members to be present and valid before building any single member. The root Cargo.toml
# defines a workspace with: common, proxy, and wrapper. Cargo validates that every
# member's source files exist, even when building just one binary.
#
# To avoid copying the full proxy crate source (which wrapper doesn't depend on),
# we create a minimal stub that satisfies Cargo's workspace validation.
WORKDIR /usr/src
COPY Cargo.toml ./
# Cargo.lock is optional - if present, ensures reproducible builds with exact dependency versions.
# The wildcard syntax (Cargo.lock*) allows the build to succeed even if Cargo.lock is not committed.
# For production builds, Cargo.lock should be committed to ensure reproducibility across all builds.
COPY Cargo.lock* ./
COPY crates/common crates/common/
COPY crates/wrapper crates/wrapper/

# Create minimal proxy stub to satisfy Cargo workspace requirements.
# The wrapper crate does NOT depend on proxy at runtime - they both depend on common.
# This stub exists only because Cargo requires all workspace members to have valid
# source files before it will build any member. Without this, `cargo build` fails with:
#   "error: failed to read `/usr/src/crates/proxy/src/lib.rs`"
COPY crates/proxy/Cargo.toml crates/proxy/Cargo.toml
RUN mkdir -p crates/proxy/src && \
    echo '// Minimal stub to satisfy Cargo workspace validation.' > crates/proxy/src/lib.rs && \
    echo '// The wrapper binary does not depend on proxy - see wrapper.Dockerfile for details.' >> crates/proxy/src/lib.rs && \
    echo 'fn main() {}' > crates/proxy/src/main.rs

# Build nuanced-lsp-wrapper binary from workspace
# Uses BuildKit cache mounts to cache Cargo registry and build artifacts across builds.
# These caches persist even with --no-cache flag, significantly speeding up rebuilds.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cargo-registry-wrapper \
    --mount=type=cache,target=/usr/src/target,id=cargo-target-wrapper \
    mkdir -p /usr/src/bin && \
    case "$TARGETPLATFORM" in \
    "linux/amd64") \
    if [ "$BUILDARCH" = "arm64" ]; then \
    cargo build --release --bin nuanced-lsp-wrapper --target x86_64-unknown-linux-gnu && \
    cp target/x86_64-unknown-linux-gnu/release/nuanced-lsp-wrapper /usr/src/bin/nuanced-lsp-wrapper; \
    elif [ "$BUILDARCH" = "amd64" ]; then \
    cargo build --release --bin nuanced-lsp-wrapper && \
    cp target/release/nuanced-lsp-wrapper /usr/src/bin/nuanced-lsp-wrapper; \
    fi \
    ;; \
    "linux/arm64") \
    if [ "$BUILDARCH" = "amd64" ]; then \
    cargo build --release --bin nuanced-lsp-wrapper --target aarch64-unknown-linux-gnu && \
    cp target/aarch64-unknown-linux-gnu/release/nuanced-lsp-wrapper /usr/src/bin/nuanced-lsp-wrapper; \
    elif [ "$BUILDARCH" = "arm64" ]; then \
    cargo build --release --bin nuanced-lsp-wrapper && \
    cp target/release/nuanced-lsp-wrapper /usr/src/bin/nuanced-lsp-wrapper; \
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
COPY --from=builder /usr/src/bin/nuanced-lsp-wrapper /opt/lsp-wrapper/bin/lsp-wrapper
RUN chmod +x /opt/lsp-wrapper/bin/lsp-wrapper

# Copy ast-grep configs from builder
COPY --from=builder /usr/src/crates/common/src/ast_grep /opt/lsp-wrapper/ast_grep

# Declare volume so --volumes-from can share these directories
VOLUME ["/opt/lsp-wrapper"]

# No ENTRYPOINT - this container is only for volume mounting
# The wrapper binary will be executed in language containers
CMD ["sleep", "infinity"]

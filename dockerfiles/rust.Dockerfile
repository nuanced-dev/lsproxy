# Rust LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Rust and rust-analyzer
FROM rust:1.91.1-slim-bookworm AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install rust-analyzer and rustfmt via rustup
RUN rustup component add rust-analyzer rustfmt

# Runtime stage: Pure Debian base (standalone image with language-specific LSP server)
# Wrapper binary will be mounted at runtime via --volumes-from
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install minimal runtime dependencies
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install minimal Rust runtime (we only need rust-analyzer binary)
# Copy rust-analyzer from builder
COPY --from=builder /usr/local/rustup /usr/local/rustup
COPY --from=builder /usr/local/cargo /usr/local/cargo

# Set Rust environment variables
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH

# Set rust-analyzer log path
ENV RA_LOG="/tmp/rust-analyzer.log"

# Create symlinks in standard PATH location (following TypeScript/Golang pattern)
RUN ln -s /usr/local/cargo/bin/rust-analyzer /usr/local/bin/rust-analyzer && \
    ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="rust"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "rust-analyzer"]

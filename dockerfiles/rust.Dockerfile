# Rust LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Rust and rust-analyzer
FROM rust:1.82.0-slim-bookworm AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install rust-analyzer and rustfmt via rustup
RUN rustup component add rust-analyzer rustfmt

# Runtime stage: Use slim base (Rust doesn't need build tools at runtime)
FROM lsproxy-base:latest

ENV DEBIAN_FRONTEND=noninteractive

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

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "rust-analyzer"]

# Python LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Python and jedi-language-server
FROM debian:bookworm-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install Python and pip (Debian Bookworm includes Python 3.11)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-venv \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create virtual environment and install jedi-language-server
RUN python3 -m venv /opt/jedi-venv && \
    /opt/jedi-venv/bin/pip install --no-cache-dir \
    jedi-language-server

# Runtime stage: Pure Debian base (no dependency on lsproxy-base)
# Wrapper binary will be mounted at runtime via --volumes-from
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install runtime and build dependencies for potential native extensions
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    pkg-config \
    libssl3 \
    build-essential \
    python3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create python symlink for compatibility
RUN ln -sf /usr/bin/python3 /usr/bin/python

# Copy virtual environment from builder
COPY --from=builder /opt/jedi-venv /opt/jedi-venv

# Create symlinks in standard PATH location (following TypeScript/Golang pattern)
# Note: ast-grep will be mounted from wrapper container, so symlink target won't exist at build time
# but will exist at runtime - this is expected behavior
RUN ln -s /opt/jedi-venv/bin/jedi-language-server /usr/local/bin/jedi-language-server && \
    ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="python"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path (must match mount point)
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "jedi-language-server"]

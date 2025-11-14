# C/C++ LSP server container (clangd)
# Uses build tools since C/C++ development needs compilers (gcc, g++, etc.)

# Runtime stage: Pure Debian base (no dependency on lsproxy-base)
# Wrapper binary will be mounted at runtime via --volumes-from
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install runtime and build dependencies for C/C++ development
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    pkg-config \
    libssl3 \
    build-essential \
    clangd \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create symlink for ast-grep in standard PATH location
RUN ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="cpp"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "clangd", "--lsp-arg=--log=info"]

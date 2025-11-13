# TypeScript/JavaScript LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Node.js and typescript-language-server
FROM debian:bookworm-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install Node.js 20.x from NodeSource
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install typescript-language-server and typescript globally
RUN npm install -g typescript-language-server typescript && \
    npm cache clean --force

# Runtime stage: Pure Debian base (no dependency on lsproxy-base)
# Wrapper binary will be mounted at runtime via --volumes-from
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install runtime and build dependencies for potential native npm modules
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    curl \
    gnupg \
    pkg-config \
    libssl3 \
    build-essential \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 20.x runtime
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy global npm packages from builder
COPY --from=builder /usr/lib/node_modules /usr/lib/node_modules

# Create symlinks for binaries (instead of copying them)
# This preserves import.meta.url path resolution
RUN ln -s /usr/lib/node_modules/typescript-language-server/lib/cli.mjs /usr/bin/typescript-language-server && \
    ln -s /usr/lib/node_modules/typescript/bin/tsc /usr/bin/tsc && \
    ln -s /usr/lib/node_modules/typescript/bin/tsserver /usr/bin/tsserver

# Create symlink for ast-grep in standard PATH location
RUN ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="typescript"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "typescript-language-server", "--lsp-arg=--stdio"]

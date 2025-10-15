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

# Runtime stage: Use build-enabled base for potential native npm modules
FROM lsproxy-base-build:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install Node.js 20.x runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
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

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="typescript"

# Set workspace path
WORKDIR /mnt/workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "typescript-language-server", "--lsp-arg=--stdio"]

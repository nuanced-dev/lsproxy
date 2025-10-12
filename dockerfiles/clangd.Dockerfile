# C/C++ LSP server container (clangd)
# Uses build-enabled base since C/C++ development needs build tools (gcc, g++, etc.)
# Note: gcc and g++ are already included via build-essential in lsproxy-base-build

FROM lsproxy-base-build:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install clangd from Debian repos
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    clangd \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "clangd", "--lsp-arg=--log=info"]

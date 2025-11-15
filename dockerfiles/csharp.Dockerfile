# C# LSP server container (csharp-ls)
# Multi-stage build to minimize image size

# Builder stage: Install .NET SDK and csharp-ls
FROM debian:bookworm-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies for dotnet install script and ICU for globalization
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates libicu72 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install .NET SDK 8.0 and 9.0
RUN curl -fsSL https://builds.dotnet.microsoft.com/dotnet/scripts/v1/dotnet-install.sh -o dotnet-install.sh && \
    chmod +x dotnet-install.sh && \
    ./dotnet-install.sh --channel 8.0 --install-dir /opt/dotnet && \
    ./dotnet-install.sh --channel 9.0 --install-dir /opt/dotnet && \
    rm dotnet-install.sh

ENV PATH="/opt/dotnet:/opt/dotnet/tools:${PATH}"
ENV DOTNET_ROOT=/opt/dotnet

# Install csharp-ls globally
RUN dotnet tool install --global csharp-ls

# Runtime stage: Pure Debian base (standalone image with language-specific LSP server)
# Wrapper binary will be mounted at runtime via --volumes-from
# .NET is self-contained, doesn't need build tools
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install minimal runtime dependencies and ICU library for .NET globalization support
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    libicu72 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Copy .NET SDK and tools from builder
COPY --from=builder /opt/dotnet /opt/dotnet
COPY --from=builder /root/.dotnet/tools /opt/dotnet/tools

# Set .NET environment variables
ENV DOTNET_ROOT=/opt/dotnet

# Create symlinks in standard PATH location (following TypeScript/Golang pattern)
RUN ln -s /opt/dotnet/dotnet /usr/local/bin/dotnet && \
    ln -s /opt/dotnet/tools/csharp-ls /usr/local/bin/csharp-ls && \
    ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="csharp"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "csharp-ls"]

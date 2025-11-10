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

# Runtime stage: Pure Debian base (no dependency on lsproxy-base)
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
ENV PATH="/opt/dotnet:/opt/dotnet/tools:${PATH}"
ENV DOTNET_ROOT=/opt/dotnet

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="csharp"

# Add wrapper binary location to PATH (will be mounted from wrapper container)
ENV PATH="/opt/lsp-wrapper/bin:${PATH}"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "csharp-ls"]

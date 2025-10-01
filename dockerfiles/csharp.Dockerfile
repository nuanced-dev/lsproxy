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

# Runtime stage: Use slim base (.NET is self-contained, doesn't need build tools)
FROM lsproxy-base:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install ICU library for .NET globalization support
RUN apt-get update && \
    apt-get install -y --no-install-recommends libicu72 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy .NET SDK and tools from builder
COPY --from=builder /opt/dotnet /opt/dotnet
COPY --from=builder /root/.dotnet/tools /opt/dotnet/tools

# Set .NET environment variables
ENV PATH="/opt/dotnet:/opt/dotnet/tools:${PATH}"
ENV DOTNET_ROOT=/opt/dotnet

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "csharp-ls"]

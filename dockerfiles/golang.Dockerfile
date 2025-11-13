# Golang LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Go and build gopls
FROM debian:bookworm-slim AS builder

ARG GO_VERSION=1.24.2
ENV DEBIAN_FRONTEND=noninteractive

# Install curl for downloading Go
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Download and install Go
RUN curl -fsSL https://go.dev/dl/go${GO_VERSION}.linux-$(dpkg --print-architecture).tar.gz \
    | tar -C /usr/local -xz

ENV GOROOT=/usr/local/go
ENV GOPATH=/tmp/go
ENV PATH=$GOROOT/bin:$PATH

# Build gopls
RUN go install golang.org/x/tools/gopls@latest

# Runtime stage: Pure Debian base (no dependency on lsproxy-base)
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

# Copy Go toolchain from builder (gopls needs this at runtime)
COPY --from=builder /usr/local/go /usr/local/go

# Copy gopls binary from builder
COPY --from=builder /tmp/go/bin/gopls /usr/local/bin/gopls

# Set Go environment variables
ENV GOROOT=/usr/local/go
ENV GOPATH=/home/user/go

# Create symlinks in standard PATH location (following TypeScript pattern)
# Symlink go toolchain binaries so they're accessible even if PATH is overridden
RUN ln -s /usr/local/go/bin/go /usr/local/bin/go && \
    ln -s /usr/local/go/bin/gofmt /usr/local/bin/gofmt && \
    ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="go"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
# gopls args match existing LSProxy configuration
CMD ["--lsp-command", "gopls", "--lsp-arg=-mode=stdio", "--lsp-arg=-vv", "--lsp-arg=-logfile=/tmp/gopls.log", "--lsp-arg=-rpc.trace"]

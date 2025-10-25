# Golang LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Go and build gopls
FROM debian:bookworm-slim AS builder

ARG GO_VERSION=1.23.4
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

# Runtime stage: Use lsproxy-base and copy only what's needed
FROM lsproxy-base:latest

ENV DEBIAN_FRONTEND=noninteractive

# Copy Go toolchain from builder (gopls needs this at runtime)
COPY --from=builder /usr/local/go /usr/local/go

# Copy gopls binary from builder
COPY --from=builder /tmp/go/bin/gopls /usr/local/bin/gopls

# Set Go environment variables
ENV GOROOT=/usr/local/go
ENV GOPATH=/home/user/go
ENV PATH=$GOPATH/bin:$GOROOT/bin:$PATH

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
# gopls args match existing LSProxy configuration
CMD ["--lsp-command", "gopls", "--lsp-arg=-mode=stdio", "--lsp-arg=-vv", "--lsp-arg=-logfile=/tmp/gopls.log", "--lsp-arg=-rpc.trace"]

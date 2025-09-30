# Golang LSP server container
# Based on lsproxy-base with gopls installed

FROM lsproxy-base:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install Go and gopls, then clean up
ARG GO_VERSION=1.23.4
RUN curl -fsSL https://go.dev/dl/go${GO_VERSION}.linux-$(dpkg --print-architecture).tar.gz \
    | tar -C /usr/local -xz && \
    # Set Go environment temporarily
    export GOROOT=/usr/local/go && \
    export GOPATH=/tmp/go && \
    export PATH=$GOPATH/bin:$GOROOT/bin:$PATH && \
    # Install gopls
    go install golang.org/x/tools/gopls@latest && \
    # Move binary to final location
    mv /tmp/go/bin/gopls /usr/local/bin/gopls && \
    # Clean up Go build cache and modules
    rm -rf /tmp/go && \
    rm -rf /root/.cache/go-build

# Set Go environment variables
ENV GOROOT=/usr/local/go
ENV GOPATH=/home/user/go
ENV PATH=$GOPATH/bin:$GOROOT/bin:$PATH

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
# gopls args match existing LSProxy configuration
CMD ["--lsp-command", "gopls", "--lsp-arg=-mode=stdio", "--lsp-arg=-vv", "--lsp-arg=-logfile=/tmp/gopls.log", "--lsp-arg=-rpc.trace"]

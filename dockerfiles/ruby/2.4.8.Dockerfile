# Ruby 2.4.8 LSP server container
# Multi-stage build to minimize image size

# Builder stage: Install Ruby and ruby-lsp
FROM debian:bookworm-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive
ARG RUBY_VERSION=2.4.8

# Install Ruby build dependencies and system libraries for native gems
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config git curl ca-certificates \
    autoconf bison libffi-dev libgdbm-dev libreadline-dev libncurses5-dev \
    libyaml-dev zlib1g-dev libssl-dev \
    libpq-dev default-libmysqlclient-dev libsqlite3-dev \
    libxml2-dev libxslt1-dev libcurl4-openssl-dev \
    imagemagick libmagickwand-dev libvips-dev \
    libjpeg-dev libpng-dev libtiff-dev libwebp-dev \
    protobuf-compiler libc-ares-dev libhiredis-dev \
    && rm -rf /var/lib/apt/lists/*

# Install rbenv and ruby-build
ENV RBENV_ROOT=/opt/rbenv
ENV PATH="$RBENV_ROOT/bin:$RBENV_ROOT/shims:${PATH}"

RUN git clone --depth 1 https://github.com/rbenv/rbenv.git "$RBENV_ROOT" && \
    git clone --depth 1 https://github.com/rbenv/ruby-build.git "$RBENV_ROOT/plugins/ruby-build"

# Install Ruby and ruby-lsp gem
RUN eval "$("$RBENV_ROOT"/bin/rbenv init -)" && \
    rbenv install ${RUBY_VERSION} && \
    rbenv global ${RUBY_VERSION} && \
    rbenv exec gem install ruby-lsp && \
    rbenv rehash

# Runtime stage: Pure Debian base with build tools for native gems
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user
ARG RUBY_VERSION=2.4.8

# Install runtime dependencies AND build tools (needed for native gem compilation)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    build-essential \
    pkg-config \
    libssl3 \
    libffi8 \
    libgdbm6 \
    libreadline8 \
    libncurses6 \
    libyaml-0-2 \
    zlib1g \
    libpq5 \
    libmariadb3 \
    libsqlite3-0 \
    libxml2 \
    libxslt1.1 \
    libcurl4 \
    imagemagick \
    libmagickwand-6.q16-6 \
    libvips42 \
    libjpeg62-turbo \
    libpng16-16 \
    libtiff6 \
    libwebp7 \
    protobuf-compiler \
    libc-ares2 \
    libhiredis0.14 \
    && rm -rf /var/lib/apt/lists/*

# Copy rbenv and Ruby installation from builder
ENV RBENV_ROOT=/opt/rbenv
ENV PATH="$RBENV_ROOT/bin:$RBENV_ROOT/shims:${PATH}"
COPY --from=builder /opt/rbenv /opt/rbenv

# Set language for lsp-wrapper configuration

# Create symlinks in standard PATH location (following TypeScript/Golang pattern)
RUN ln -s ${RBENV_ROOT}/shims/ruby-lsp /usr/local/bin/ruby-lsp && \
    ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true
ENV LSP_LANGUAGE="ruby"


# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace
WORKDIR /mnt/workspace

# Use wrapper ENTRYPOINT (mounted from wrapper container)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "ruby-lsp", "--lsp-arg=--use-launcher"]

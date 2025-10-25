# Ruby 3.4.4 LSP server container
# Multi-stage build to minimize image size
#
# To build for a different Ruby version:
#   1. Copy this file to ruby-X.Y.Z.Dockerfile
#   2. Change RUBY_VERSION below to your desired version
#   3. Build: docker build -f dockerfiles/ruby-X.Y.Z.Dockerfile -t lsproxy-ruby-X.Y.Z:latest .

# Builder stage: Install Ruby and ruby-lsp
FROM debian:bookworm-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive
ARG RUBY_VERSION=3.4.4

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

# Runtime stage: Use build-enabled base for native gem extensions
FROM lsproxy-base-build:latest

ENV DEBIAN_FRONTEND=noninteractive
ARG RUBY_VERSION=3.4.4

# Install Ruby runtime dependencies (runtime versions of the build libs)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libffi8 \
    libgdbm6 \
    libreadline8 \
    libncurses6 \
    libyaml-0-2 \
    zlib1g \
    libssl3 \
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

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "ruby-lsp", "--lsp-arg=--use-launcher"]

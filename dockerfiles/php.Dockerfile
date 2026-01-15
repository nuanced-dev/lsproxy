# PHP LSP server container (Phpactor)
# Multi-stage build to minimize image size

# Builder stage: Install Composer and Phpactor (needs PHP to run composer install)
FROM debian:bookworm-slim AS builder
LABEL org.opencontainers.image.source https://github.com/nuanced-dev/lsp

ENV DEBIAN_FRONTEND=noninteractive

# Install PHP and dependencies needed for Composer
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    php \
    php-xml \
    php-mbstring \
    php-curl \
    php-zip \
    curl \
    ca-certificates \
    git \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install Composer
RUN curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer

# Install Phpactor
RUN cd /usr/src && \
    git clone https://github.com/phpactor/phpactor.git && \
    cd /usr/src/phpactor && \
    composer install --no-dev

# Runtime stage: Pure Debian base (standalone image with language-specific LSP server)
# Wrapper binary will be mounted at runtime via --volumes-from
FROM debian:bookworm-slim
LABEL org.opencontainers.image.source https://github.com/nuanced-dev/lsp

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install runtime and build dependencies for potential PHP extensions
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    pkg-config \
    libssl3 \
    build-essential \
    php \
    php-xml \
    php-mbstring \
    php-curl \
    php-zip \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Copy Composer and Phpactor from builder
COPY --from=builder /usr/local/bin/composer /usr/local/bin/composer
COPY --from=builder /usr/src/phpactor /usr/src/phpactor

# Create symlinks in standard PATH location (following TypeScript/Golang pattern)
RUN ln -s /usr/src/phpactor/bin/phpactor /usr/local/bin/phpactor && \
    ln -s /opt/lsp-wrapper/bin/ast-grep /usr/local/bin/ast-grep || true

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="php"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "phpactor", "--lsp-arg=language-server"]

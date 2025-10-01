# PHP LSP server container (Phpactor)
# Multi-stage build to minimize image size

# Builder stage: Install Composer and Phpactor (needs PHP to run composer install)
FROM debian:bookworm-slim AS builder

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

# Runtime stage: Use build-enabled base for potential PHP extensions
FROM lsproxy-base-build:latest

ENV DEBIAN_FRONTEND=noninteractive

# Install PHP runtime (Phpactor needs PHP to run)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    php \
    php-xml \
    php-mbstring \
    php-curl \
    php-zip \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy Composer and Phpactor from builder
COPY --from=builder /usr/local/bin/composer /usr/local/bin/composer
COPY --from=builder /usr/src/phpactor /usr/src/phpactor

# Add Phpactor to PATH
ENV PATH="/usr/src/phpactor/bin:${PATH}"

# Set workspace path
WORKDIR /workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "phpactor", "--lsp-arg=language-server"]

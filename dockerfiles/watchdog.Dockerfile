# Lightweight watchdog container that monitors the service container
# and cleans up orphaned language server containers if the service dies unexpectedly

FROM alpine:3.19
LABEL org.opencontainers.image.source=https://github.com/nuanced-dev/lsp

# Install Docker CLI (needed to monitor and cleanup containers)
RUN apk add --no-cache docker-cli

# Copy watchdog script
COPY dockerfiles/entrypoints/watchdog-entrypoint.sh /usr/local/bin/watchdog-entrypoint.sh
RUN chmod +x /usr/local/bin/watchdog-entrypoint.sh

# Run watchdog
ENTRYPOINT ["/usr/local/bin/watchdog-entrypoint.sh"]

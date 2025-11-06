# Lightweight watchdog container that monitors the service container
# and cleans up orphaned language server containers if the service dies unexpectedly

FROM alpine:3.19

# Install Docker CLI (needed to monitor and cleanup containers)
RUN apk add --no-cache docker-cli

# Copy watchdog script
COPY scripts/watchdog.sh /usr/local/bin/watchdog.sh
RUN chmod +x /usr/local/bin/watchdog.sh

# Run watchdog
ENTRYPOINT ["/usr/local/bin/watchdog.sh"]

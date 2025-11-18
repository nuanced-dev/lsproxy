#!/bin/sh
# Watchdog script that monitors the parent service container
# and cleans up child language server containers if the parent dies

set -e

PARENT_ID="${PARENT_CONTAINER_ID:-}"

if [ -z "$PARENT_ID" ]; then
    echo "ERROR: PARENT_CONTAINER_ID environment variable not set"
    exit 1
fi

echo "Watchdog: Monitoring parent container $PARENT_ID"

# Monitor parent container
while docker inspect "$PARENT_ID" >/dev/null 2>&1; do
    # Check if parent is still running (not just exists, but running)
    STATUS=$(docker inspect --format='{{.State.Status}}' "$PARENT_ID" 2>/dev/null || echo "gone")

    if [ "$STATUS" != "running" ]; then
        echo "Watchdog: Parent container $PARENT_ID stopped (status: $STATUS)"
        break
    fi

    sleep 2
done

echo "Watchdog: Parent container $PARENT_ID is no longer running"
echo "Watchdog: Performing cleanup (will be no-op if parent already cleaned up)..."

# Find and remove all language server containers spawned by this parent
CHILD_CONTAINERS=$(docker ps -aq --filter "label=nuanced.parent=$PARENT_ID" --filter "label=nuanced.role=language-server")

if [ -n "$CHILD_CONTAINERS" ]; then
    CHILD_COUNT=$(echo "$CHILD_CONTAINERS" | wc -l)
    echo "Watchdog: Found $CHILD_COUNT language server container(s) to clean up"

    for container in $CHILD_CONTAINERS; do
        CONTAINER_NAME=$(docker inspect --format='{{.Name}}' "$container" 2>/dev/null | sed 's/^\///' || echo "$container")
        echo "Watchdog: Stopping and removing $CONTAINER_NAME..."
        docker rm -f "$container" 2>/dev/null || true
    done

    echo "Watchdog: Cleanup complete - removed $CHILD_COUNT container(s)"
else
    echo "Watchdog: No language server containers to clean up (already cleaned by parent)"
fi

# Clean up wrapper container
WRAPPER_CONTAINER=$(docker ps -aq --filter "name=nuanced-lsp-wrapper")
if [ -n "$WRAPPER_CONTAINER" ]; then
    echo "Watchdog: Cleaning up nuanced-lsp-wrapper container..."
    docker rm -f "$WRAPPER_CONTAINER" 2>/dev/null || true
    echo "Watchdog: nuanced-lsp-wrapper container removed"
else
    echo "Watchdog: No nuanced-lsp-wrapper container found"
fi

echo "Watchdog: Exiting"
exit 0

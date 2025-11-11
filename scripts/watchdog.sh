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

# Check if cleanup was already completed by the parent
if docker exec "$PARENT_ID" test -f /tmp/cleanup_complete 2>/dev/null; then
    echo "Watchdog: Clean shutdown detected (cleanup_complete marker found)"
    echo "Watchdog: No emergency cleanup needed, exiting"
    exit 0
fi

# Parent died without cleaning up - emergency cleanup
echo "Watchdog: Parent died unexpectedly, performing emergency cleanup..."

# Find and remove all language server containers spawned by this parent
CHILD_CONTAINERS=$(docker ps -aq --filter "label=lsproxy.parent=$PARENT_ID" --filter "label=lsproxy.role=language-server")

if [ -n "$CHILD_CONTAINERS" ]; then
    CHILD_COUNT=$(echo "$CHILD_CONTAINERS" | wc -l)
    echo "Watchdog: Found $CHILD_COUNT orphaned language server container(s)"

    for container in $CHILD_CONTAINERS; do
        CONTAINER_NAME=$(docker inspect --format='{{.Name}}' "$container" 2>/dev/null | sed 's/^\///' || echo "$container")
        echo "Watchdog: Stopping and removing $CONTAINER_NAME..."
        docker rm -f "$container" 2>/dev/null || true
    done

    echo "Watchdog: Emergency cleanup complete - removed $CHILD_COUNT container(s)"
else
    echo "Watchdog: No orphaned language server containers found"
fi

# Clean up wrapper container
WRAPPER_CONTAINER=$(docker ps -aq --filter "name=lsproxy-wrapper")
if [ -n "$WRAPPER_CONTAINER" ]; then
    echo "Watchdog: Cleaning up lsproxy-wrapper container..."
    docker rm -f "$WRAPPER_CONTAINER" 2>/dev/null || true
    echo "Watchdog: lsproxy-wrapper container removed"
else
    echo "Watchdog: No lsproxy-wrapper container found"
fi

echo "Watchdog: Exiting"
exit 0

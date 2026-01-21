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

# Don't let individual cleanup failures exit the watchdog early, but capture failures
set +e

remove_with_retries() {
    target_id="$1"
    target_name="$2"

    for i in 1 2 3 4 5; do
        if docker rm -f "$target_id"; then
            echo "Watchdog: Removed $target_name on attempt $i"
            return 0
        fi
        echo "Watchdog: ERROR removing $target_name (attempt $i) — retrying in 1s"
        sleep 1
    done

    echo "Watchdog: ERROR removing $target_name after retries"
    return 1
}

cleanup_role() {
    role_filter="$1"
    role_desc="$2"

    for attempt in $(seq 1 30); do
        IDS=$(docker ps -aq --filter "label=nuanced.parent=$PARENT_ID" --filter "label=nuanced.role=$role_filter")
        if [ -z "$IDS" ]; then
            echo "Watchdog: ${role_desc} cleanup complete after ${attempt}s"
            return 0
        fi

        COUNT=$(echo "$IDS" | wc -l)
        echo "Watchdog: Found $COUNT ${role_desc} to clean up (attempt ${attempt})"
        for container in $IDS; do
            CONTAINER_NAME=$(docker inspect --format='{{.Name}}' "$container" 2>/dev/null | sed 's/^\///' || echo "$container")
            echo "Watchdog: Removing $CONTAINER_NAME..."
            remove_with_retries "$container" "$CONTAINER_NAME"
        done

        sleep 1
    done

    echo "Watchdog: ERROR cleanup for ${role_desc} did not finish"
    return 1
}

# Remove language servers first, then wrappers. These loops keep the watchdog
# alive until containers are actually gone (or retries are exhausted).
cleanup_role "language-server" "language server container(s)"
cleanup_role "wrapper" "wrapper container(s)"

echo "Watchdog: Exiting"

# Restore default error handling in case this script is ever sourced
set -e
exit 0

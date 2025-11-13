#!/usr/bin/env bash

set -e

# Emergency cleanup script - removes ALL LSProxy related containers and networks
# Usage: ./scripts/cleanup-all.sh

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  LSProxy Emergency Cleanup${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

echo -e "${YELLOW}This will forcefully remove:${NC}"
echo "  - All running LSProxy containers (service, watchdog, language containers)"
echo "  - All stopped LSProxy containers"
echo

read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Cancelled${NC}"
    exit 0
fi

# Count what we're cleaning up
RUNNING=$(docker ps -q --filter "name=lsproxy-" | wc -l | tr -d ' ')
STOPPED=$(docker ps -aq --filter "name=lsproxy-" | wc -l | tr -d ' ')

echo -e "${BLUE}Found:${NC}"
echo "  Running containers: $RUNNING"
echo "  Total containers: $STOPPED"
echo

# Stop and remove all containers (running and stopped)
echo -e "${BLUE}Removing containers...${NC}"
CONTAINERS=$(docker ps -aq --filter "name=lsproxy-")
if [ -n "$CONTAINERS" ]; then
    echo "$CONTAINERS" | while read container; do
        NAME=$(docker ps -a --filter "id=$container" --format "{{.Names}}" 2>/dev/null || echo "unknown")
        echo -n "  Removing $NAME... "
        if docker rm -f "$container" > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
        else
            echo -e "${RED}✗${NC}"
        fi
    done
else
    echo -e "${YELLOW}  No containers to remove${NC}"
fi

# Final verification
echo
echo -e "${BLUE}Verification:${NC}"
REMAINING=$(docker ps -aq --filter "name=lsproxy-" | wc -l | tr -d ' ')
echo "  Remaining containers: $REMAINING"

echo
if [ "$REMAINING" -eq 0 ]; then
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  ✓ Cleanup complete!${NC}"
    echo -e "${GREEN}=========================================${NC}"
    exit 0
else
    echo -e "${YELLOW}=========================================${NC}"
    echo -e "${YELLOW}  ⚠ Cleanup incomplete${NC}"
    echo -e "${YELLOW}=========================================${NC}"

    echo -e "${YELLOW}Remaining containers:${NC}"
    docker ps -a --filter "name=lsproxy-" --format "  {{.Names}}\t({{.Status}})"

    echo
    echo -e "${YELLOW}Manual cleanup command:${NC}"
    echo "  docker rm -f \$(docker ps -aq --filter \"name=lsproxy-\")"
    exit 1
fi

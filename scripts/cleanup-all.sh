#!/bin/bash

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
echo "  - All running LSProxy containers"
echo "  - All stopped LSProxy containers"
echo "  - LSProxy Docker network"
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

# Remove network
echo
echo -e "${BLUE}Removing network...${NC}"
if docker network ls --format "{{.Name}}" | grep -q "^lsproxy$"; then
    if docker network rm lsproxy > /dev/null 2>&1; then
        echo -e "${GREEN}  ✓ Network removed${NC}"
    else
        echo -e "${YELLOW}  ⚠ Network could not be removed (may still be in use)${NC}"
    fi
else
    echo -e "${YELLOW}  No network to remove${NC}"
fi

# Final verification
echo
echo -e "${BLUE}Verification:${NC}"
REMAINING=$(docker ps -aq --filter "name=lsproxy-" | wc -l | tr -d ' ')
echo "  Remaining containers: $REMAINING"

NETWORK_EXISTS=$(docker network ls --format "{{.Name}}" | grep -c "^lsproxy$" || echo "0")
echo "  Network exists: $NETWORK_EXISTS"

echo
if [ "$REMAINING" -eq 0 ] && [ "$NETWORK_EXISTS" -eq 0 ]; then
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  ✓ Cleanup complete!${NC}"
    echo -e "${GREEN}=========================================${NC}"
    exit 0
else
    echo -e "${YELLOW}=========================================${NC}"
    echo -e "${YELLOW}  ⚠ Cleanup incomplete${NC}"
    echo -e "${YELLOW}=========================================${NC}"

    if [ "$REMAINING" -gt 0 ]; then
        echo -e "${YELLOW}Remaining containers:${NC}"
        docker ps -a --filter "name=lsproxy-" --format "  {{.Names}}\t({{.Status}})"
    fi

    if [ "$NETWORK_EXISTS" -gt 0 ]; then
        echo -e "${YELLOW}Network still exists (may have active connections)${NC}"
    fi

    echo
    echo -e "${YELLOW}Manual cleanup commands:${NC}"
    echo "  docker rm -f \$(docker ps -aq --filter \"name=lsproxy-\")"
    echo "  docker network rm lsproxy"
    exit 1
fi

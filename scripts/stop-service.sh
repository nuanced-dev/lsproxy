#!/bin/bash

set -e

# Stop LSProxy service and clean up all containers
# Usage: ./scripts/stop-service.sh [--force]

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

FORCE=false

# Parse options
while [[ $# -gt 0 ]]; do
    case $1 in
        --force|-f)
            FORCE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --force, -f    Force removal without confirmation"
            echo "  --help, -h     Show this help"
            echo ""
            echo "This script stops and removes all LSProxy containers:"
            echo "  - Service container"
            echo "  - All language containers"
            echo "  - Docker network (if empty)"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Stopping LSProxy Service${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Check if any containers are running
CONTAINERS=$(docker ps -aq --filter "name=lsproxy-")
if [ -z "$CONTAINERS" ]; then
    echo -e "${YELLOW}No LSProxy containers found${NC}"
    exit 0
fi

# Show what will be stopped
echo -e "${BLUE}Containers to stop:${NC}"
docker ps --filter "name=lsproxy-" --format "  {{.Names}}\t({{.Status}})"
echo

# Confirm unless force
if [ "$FORCE" = false ]; then
    read -p "Stop and remove all LSProxy containers? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Cancelled${NC}"
        exit 0
    fi
fi

# Stop and remove containers
echo -e "${BLUE}Stopping containers...${NC}"
for container in $CONTAINERS; do
    NAME=$(docker ps -a --filter "id=$container" --format "{{.Names}}")
    echo -n "  Stopping $NAME... "
    if docker rm -f "$container" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ (failed)${NC}"
    fi
done

# Check if network exists and is empty
if docker network ls --format "{{.Name}}" | grep -q "^lsproxy$"; then
    NETWORK_CONTAINERS=$(docker network inspect lsproxy --format '{{range .Containers}}{{.Name}} {{end}}' 2>/dev/null || echo "")
    if [ -z "$NETWORK_CONTAINERS" ]; then
        echo -e "${BLUE}Removing Docker network...${NC}"
        if docker network rm lsproxy > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Network removed${NC}"
        else
            echo -e "${YELLOW}⚠ Could not remove network (may have active connections)${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ Network has active containers, not removing${NC}"
    fi
fi

# Verify cleanup
REMAINING=$(docker ps -aq --filter "name=lsproxy-" | wc -l | tr -d ' ')
if [ "$REMAINING" -eq 0 ]; then
    echo
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  ✓ All LSProxy containers stopped${NC}"
    echo -e "${GREEN}=========================================${NC}"
    exit 0
else
    echo
    echo -e "${YELLOW}=========================================${NC}"
    echo -e "${YELLOW}  ⚠ Warning: $REMAINING containers remaining${NC}"
    echo -e "${YELLOW}=========================================${NC}"
    echo -e "${YELLOW}Remaining containers:${NC}"
    docker ps -a --filter "name=lsproxy-" --format "  {{.Names}}\t({{.Status}})"
    echo
    echo -e "${YELLOW}Try running with --force:${NC}"
    echo -e "  $0 --force"
    exit 1
fi

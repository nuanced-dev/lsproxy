#!/usr/bin/env bash

set -e

# Stop Nuanced LSP proxy and clean up all containers
# Usage: ./scripts/stop-proxy.sh [--force]

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse options
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            echo "Usage: $0"
            echo ""
            echo "This script stops and removes all Nuanced LSP containers:"
            echo "  - Proxy container"
            echo "  - Watchdog container"
            echo "  - All language containers"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Stopping Nuanced LSP Proxy${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Check if any containers are running
CONTAINERS=$(docker ps -aq --filter "name=nuanced-lsp-")
if [ -z "$CONTAINERS" ]; then
    echo -e "${YELLOW}No Nuanced LSP containers found${NC}"
    exit 0
fi

# Show what will be stopped
echo -e "${BLUE}Containers to stop:${NC}"
docker ps --filter "name=nuanced-lsp-" --format "  {{.Names}}\t({{.Status}})"
echo

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

# Verify cleanup
REMAINING=$(docker ps -aq --filter "name=nuanced-lsp-" | wc -l | tr -d ' ')
if [ "$REMAINING" -eq 0 ]; then
    echo
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  ✓ All Nuanced LSP containers stopped${NC}"
    echo -e "${GREEN}=========================================${NC}"
    exit 0
else
    echo
    echo -e "${YELLOW}=========================================${NC}"
    echo -e "${YELLOW}  ⚠ Warning: $REMAINING containers remaining${NC}"
    echo -e "${YELLOW}=========================================${NC}"
    echo -e "${YELLOW}Remaining containers:${NC}"
    docker ps -a --filter "name=nuanced-lsp-" --format "  {{.Names}}\t({{.Status}})"
    echo
    echo -e "${YELLOW}Some containers could not be stopped${NC}"
    exit 1
fi

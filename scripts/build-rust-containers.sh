#!/bin/bash

set -e

# Build Rust-based containers (wrapper, service, watchdog)
# Usage: ./scripts/build-rust-containers.sh [--use-cache]
#
# By default:
#   - Builds WITHOUT cache (use --use-cache to enable caching)
#   - Builds Rust binaries first before Docker images

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default: no cache
USE_CACHE=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --use-cache)
            USE_CACHE=true
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 [--use-cache]"
            exit 1
            ;;
    esac
done

# Set cache flag for docker builds
CACHE_FLAG=""
if [ "$USE_CACHE" = false ]; then
    CACHE_FLAG="--no-cache"
fi

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Building Rust Containers${NC}"
echo -e "${BLUE}  Cache: $USE_CACHE${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Step 0: Build Rust binaries first
echo -e "${YELLOW}Step 0: Building Rust binaries${NC}"
echo -e "${BLUE}Running cargo build --release...${NC}"
if cargo build --release 2>&1 | tee /tmp/cargo-build.log | tail -5; then
    echo -e "${GREEN}✓ Rust binaries built successfully${NC}"
else
    echo -e "${RED}✗ Failed to build Rust binaries${NC}"
    tail -20 /tmp/cargo-build.log
    exit 1
fi
echo

# Build wrapper image (contains lsp-wrapper binary and ast-grep configs)
# This is mounted into language containers at runtime via --volumes-from
echo -e "${YELLOW}Step 1: Building wrapper image${NC}"

echo -e "${BLUE}Building lsproxy-wrapper...${NC}"
if docker build $CACHE_FLAG -f dockerfiles/wrapper.Dockerfile -t lsproxy-wrapper:latest . > /tmp/build-wrapper.log 2>&1; then
    SIZE=$(docker images lsproxy-wrapper:latest --format "{{.Size}}")
    echo -e "${GREEN}✓ lsproxy-wrapper built successfully ($SIZE)${NC}"
else
    echo -e "${RED}✗ lsproxy-wrapper failed to build${NC}"
    echo -e "${YELLOW}See /tmp/build-wrapper.log for details${NC}"
    tail -20 /tmp/build-wrapper.log
    exit 1
fi
echo

# Build service image (orchestrator)
echo -e "${YELLOW}Step 2: Building service image${NC}"
echo -e "${BLUE}Building lsproxy-service...${NC}"
if docker build $CACHE_FLAG -f dockerfiles/service.Dockerfile -t lsproxy-service:latest . > /tmp/build-service.log 2>&1; then
    SIZE=$(docker images lsproxy-service:latest --format "{{.Size}}")
    echo -e "${GREEN}✓ lsproxy-service built successfully ($SIZE)${NC}"
else
    echo -e "${RED}✗ lsproxy-service failed to build${NC}"
    echo -e "${YELLOW}See /tmp/build-service.log for details${NC}"
    tail -20 /tmp/build-service.log
    exit 1
fi
echo

# Build watchdog image (monitors service container)
echo -e "${YELLOW}Step 3: Building watchdog image${NC}"
echo -e "${BLUE}Building lsproxy-watchdog...${NC}"
if docker build $CACHE_FLAG -f dockerfiles/watchdog.Dockerfile -t lsproxy-watchdog:latest . > /tmp/build-watchdog.log 2>&1; then
    SIZE=$(docker images lsproxy-watchdog:latest --format "{{.Size}}")
    echo -e "${GREEN}✓ lsproxy-watchdog built successfully ($SIZE)${NC}"
else
    echo -e "${RED}✗ lsproxy-watchdog failed to build${NC}"
    echo -e "${YELLOW}See /tmp/build-watchdog.log for details${NC}"
    tail -20 /tmp/build-watchdog.log
    exit 1
fi
echo

echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Rust Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}Container Images:${NC}"
docker images | grep -E "lsproxy-(wrapper|service|watchdog)" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
echo

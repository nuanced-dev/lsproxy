#!/bin/bash

set -e

# Build all language containers and the service container
# Usage: ./scripts/build-all-containers.sh [--parallel]

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PARALLEL=false
if [ "$1" = "--parallel" ]; then
    PARALLEL=true
fi

# Language Dockerfiles
LANGUAGES=(
    "base"
    "python"
    "typescript"
    "rust"
    "golang"
    "java"
    "clangd"
    "csharp"
    "php"
    "ruby-3.4.4"
    "ruby-sorbet-3.4.4"
)

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Building All LSProxy Containers${NC}"
echo -e "${BLUE}  Parallel: $PARALLEL${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

build_container() {
    local lang="$1"
    local dockerfile="dockerfiles/${lang}.Dockerfile"

    if [ ! -f "$dockerfile" ]; then
        echo -e "${YELLOW}Warning: $dockerfile not found, skipping${NC}"
        return 0
    fi

    echo -e "${BLUE}Building lsproxy-${lang}...${NC}"

    if docker build -f "$dockerfile" -t "lsproxy-${lang}:latest" . > "/tmp/build-${lang}.log" 2>&1; then
        local size=$(docker images "lsproxy-${lang}:latest" --format "{{.Size}}")
        echo -e "${GREEN}✓ lsproxy-${lang} built successfully ($size)${NC}"
        return 0
    else
        echo -e "${RED}✗ lsproxy-${lang} failed to build${NC}"
        echo -e "${YELLOW}See /tmp/build-${lang}.log for details${NC}"
        tail -20 "/tmp/build-${lang}.log"
        return 1
    fi
}

# Build service image first (it's used by all language containers)
echo -e "${YELLOW}Step 1: Building service image${NC}"
build_container "service" || { echo -e "${RED}Failed to build service image${NC}"; exit 1; }
echo

# Build language images
echo -e "${YELLOW}Step 2: Building language containers${NC}"

if [ "$PARALLEL" = true ]; then
    echo -e "${BLUE}Building in parallel (see /tmp/build-*.log for progress)${NC}"

    # Build in parallel using background jobs
    pids=()
    for lang in "${LANGUAGES[@]}"; do
        build_container "$lang" &
        pids+=($!)
    done

    # Wait for all builds
    failed=0
    for pid in "${pids[@]}"; do
        if ! wait $pid; then
            failed=$((failed + 1))
        fi
    done

    if [ $failed -gt 0 ]; then
        echo -e "${RED}$failed language containers failed to build${NC}"
        exit 1
    fi
else
    # Build sequentially
    for lang in "${LANGUAGES[@]}"; do
        build_container "$lang" || exit 1
    done
fi

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  All Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}Container Images:${NC}"
docker images | grep "lsproxy-" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
echo
echo -e "${BLUE}Total size:${NC}"
docker images | grep "lsproxy-" | awk '{size+=$7} END {print "  ~" size " (approximate)"}'

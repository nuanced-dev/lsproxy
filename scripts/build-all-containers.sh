#!/bin/bash

set -e

# Build all language containers and the service container
# Usage: ./scripts/build-all-containers.sh [--use-cache] [--sequential]
#
# By default:
#   - Builds WITHOUT cache (use --use-cache to enable caching)
#   - Builds in PARALLEL (use --sequential for sequential builds)
#   - Builds Rust binaries first before Docker images

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default: parallel builds without cache
PARALLEL=true
USE_CACHE=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --sequential)
            PARALLEL=false
            ;;
        --use-cache)
            USE_CACHE=true
            ;;
        --parallel)
            PARALLEL=true
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 [--use-cache] [--sequential]"
            exit 1
            ;;
    esac
done

# Set cache flag for docker builds
CACHE_FLAG=""
if [ "$USE_CACHE" = false ]; then
    CACHE_FLAG="--no-cache"
fi

# Language Dockerfiles (non-Ruby, non-base)
# Note: base images are built separately in Step 1
LANGUAGES=(
    "python"
    "typescript"
    "rust"
    "golang"
    "java"
    "clangd"
    "csharp"
    "php"
)

# Ruby base images (must be built before Sorbet variants)
# Dynamically discover all Ruby versions from dockerfiles/ruby/ directory
RUBY_VERSIONS=()
if [ -d "dockerfiles/ruby" ]; then
    for dockerfile in dockerfiles/ruby/*.Dockerfile; do
        if [ -f "$dockerfile" ]; then
            # Extract version from filename (e.g., 3.4.4 from 3.4.4.Dockerfile)
            version=$(basename "$dockerfile" .Dockerfile)
            RUBY_VERSIONS+=("$version")
        fi
    done
fi

# Ruby Sorbet variants (depend on ruby base images)
# Same versions as Ruby, but from ruby-sorbet directory
RUBY_SORBET_VERSIONS=()
if [ -d "dockerfiles/ruby-sorbet" ]; then
    for dockerfile in dockerfiles/ruby-sorbet/*.Dockerfile; do
        if [ -f "$dockerfile" ]; then
            # Extract version from filename (e.g., 3.4.4 from 3.4.4.Dockerfile)
            version=$(basename "$dockerfile" .Dockerfile)
            RUBY_SORBET_VERSIONS+=("$version")
        fi
    done
fi

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Building All LSProxy Containers${NC}"
echo -e "${BLUE}  Parallel: $PARALLEL${NC}"
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

build_container() {
    local lang="$1"
    local subdir="$2"  # Optional subdirectory (ruby or ruby-sorbet)
    local dockerfile

    if [ -n "$subdir" ]; then
        dockerfile="dockerfiles/${subdir}/${lang}.Dockerfile"
    else
        dockerfile="dockerfiles/${lang}.Dockerfile"
    fi

    if [ ! -f "$dockerfile" ]; then
        echo -e "${YELLOW}Warning: $dockerfile not found, skipping${NC}"
        return 0
    fi

    # For Ruby images, the image name includes the full version
    if [ -n "$subdir" ]; then
        local image_name="lsproxy-${subdir}-${lang}"
    else
        local image_name="lsproxy-${lang}"
    fi

    echo -e "${BLUE}Building ${image_name}...${NC}"

    if docker build $CACHE_FLAG -f "$dockerfile" -t "${image_name}:latest" . > "/tmp/build-${subdir}-${lang}.log" 2>&1; then
        local size=$(docker images "${image_name}:latest" --format "{{.Size}}")
        echo -e "${GREEN}✓ ${image_name} built successfully ($size)${NC}"
        return 0
    else
        echo -e "${RED}✗ ${image_name} failed to build${NC}"
        echo -e "${YELLOW}See /tmp/build-${subdir}-${lang}.log for details${NC}"
        tail -20 "/tmp/build-${subdir}-${lang}.log"
        return 1
    fi
}

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
build_container "service" || { echo -e "${RED}Failed to build service image${NC}"; exit 1; }
echo

# Build watchdog image (monitors service container)
echo -e "${YELLOW}Step 3: Building watchdog image${NC}"
build_container "watchdog" || { echo -e "${RED}Failed to build watchdog image${NC}"; exit 1; }
echo

# Build language images
echo -e "${YELLOW}Step 4: Building non-Ruby language containers${NC}"

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

# Build Ruby base images (must complete before Sorbet variants)
echo -e "${YELLOW}Step 5: Building Ruby base images (${#RUBY_VERSIONS[@]} versions)${NC}"

if [ ${#RUBY_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby versions found in dockerfiles/ruby/, skipping${NC}"
else
    if [ "$PARALLEL" = true ]; then
        echo -e "${BLUE}Building in parallel (see /tmp/build-ruby-*.log for progress)${NC}"

        # Build in parallel using background jobs
        pids=()
        for version in "${RUBY_VERSIONS[@]}"; do
            build_container "$version" "ruby" &
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
            echo -e "${RED}$failed Ruby containers failed to build${NC}"
            exit 1
        fi
    else
        # Build sequentially
        for version in "${RUBY_VERSIONS[@]}"; do
            build_container "$version" "ruby" || exit 1
        done
    fi
fi

echo

# Build Ruby Sorbet variants (depends on Ruby base images)
echo -e "${YELLOW}Step 6: Building Ruby Sorbet variants (${#RUBY_SORBET_VERSIONS[@]} versions)${NC}"

if [ ${#RUBY_SORBET_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby Sorbet versions found in dockerfiles/ruby-sorbet/, skipping${NC}"
else
    if [ "$PARALLEL" = true ]; then
        echo -e "${BLUE}Building in parallel (see /tmp/build-ruby-sorbet-*.log for progress)${NC}"

        # Build in parallel using background jobs
        pids=()
        for version in "${RUBY_SORBET_VERSIONS[@]}"; do
            build_container "$version" "ruby-sorbet" &
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
            echo -e "${RED}$failed Ruby Sorbet containers failed to build${NC}"
            exit 1
        fi
    else
        # Build sequentially
        for version in "${RUBY_SORBET_VERSIONS[@]}"; do
            build_container "$version" "ruby-sorbet" || exit 1
        done
    fi
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

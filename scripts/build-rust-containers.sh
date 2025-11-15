#!/usr/bin/env bash

set -e

# Build Rust-based containers (wrapper, service, watchdog)
# Usage: ./scripts/build-rust-containers.sh [--use-cache] [--multiarch] [--load] [--tag=TAG]
#
# By default:
#   - Builds WITHOUT cache (use --use-cache to enable caching)
#   - Builds for local platform only (use --multiarch for amd64+arm64)
#   - Builds Rust binaries first before Docker images (only for single-arch builds)
#   - Tags images as :latest (use --tag=0.4.0 for custom tag)
#
# Options:
#   --multiarch       Build for both linux/amd64 and linux/arm64
#   --load            Also build and load local platform into Docker (use with --multiarch)
#   --use-cache       Enable Docker build cache
#   --tag=TAG         Tag images with specified tag (default: latest)

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default: no cache, single-arch, no load, latest tag
USE_CACHE=false
MULTIARCH=false
LOAD_LOCAL=false
TAG="latest"

# Parse arguments
for arg in "$@"; do
    case $arg in
        --use-cache)
            USE_CACHE=true
            ;;
        --multiarch)
            MULTIARCH=true
            ;;
        --load)
            LOAD_LOCAL=true
            ;;
        --tag=*)
            TAG="${arg#*=}"
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 [--use-cache] [--multiarch] [--load] [--tag=TAG]"
            exit 1
            ;;
    esac
done

# Set cache flag for docker builds
CACHE_FLAG=""
if [ "$USE_CACHE" = false ]; then
    CACHE_FLAG="--no-cache"
fi

# Set up build command based on multiarch flag
BUILD_CMD="docker build"
PLATFORM_FLAG=""
LOAD_FLAG=""
if [ "$MULTIARCH" = true ]; then
    BUILD_CMD="docker buildx build"
    PLATFORM_FLAG="--platform linux/amd64,linux/arm64"
    # Note: Multi-arch builds are NOT loaded into local Docker daemon
    # They are built and cached, ready for pushing to a registry
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Multi-Arch Rust Containers${NC}"
    echo -e "${BLUE}  Platforms: linux/amd64, linux/arm64${NC}"
    echo -e "${BLUE}  Cache: $USE_CACHE${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
    echo -e "${YELLOW}Note: Multi-arch builds are prepared for publishing but not loaded into local Docker${NC}"
    echo -e "${YELLOW}      Use 'docker buildx imagetools inspect <image>' to verify build${NC}"
    echo -e "${YELLOW}      Use './scripts/publish-images.sh' to push to registry${NC}"
    echo -e "${YELLOW}      Skipping local cargo build (cross-compilation happens in Docker)${NC}"
    echo
else
    BUILD_CMD="docker build"
    LOAD_FLAG=""  # Docker build loads by default
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Rust Containers (Local Platform)${NC}"
    echo -e "${BLUE}  Cache: $USE_CACHE${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo

    # Step 0: Build Rust binaries first (only for single-arch local builds)
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
fi

# Build wrapper image (contains lsp-wrapper binary and ast-grep configs)
# This is mounted into language containers at runtime via --volumes-from
echo -e "${YELLOW}Step 1: Building wrapper image${NC}"

echo -e "${BLUE}Building nuanced-lsp-wrapper:${TAG}...${NC}"
if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAG -f dockerfiles/wrapper.Dockerfile -t nuanced-lsp-wrapper:${TAG} . > /tmp/build-wrapper.log 2>&1; then
    if [ "$MULTIARCH" = true ]; then
        echo -e "${GREEN}✓ nuanced-lsp-wrapper:${TAG} built successfully (multi-arch)${NC}"
    else
        SIZE=$(docker images nuanced-lsp-wrapper:${TAG} --format "{{.Size}}")
        echo -e "${GREEN}✓ nuanced-lsp-wrapper:${TAG} built successfully ($SIZE)${NC}"
    fi
else
    echo -e "${RED}✗ nuanced-lsp-wrapper failed to build${NC}"
    echo -e "${YELLOW}See /tmp/build-wrapper.log for details${NC}"
    tail -20 /tmp/build-wrapper.log
    exit 1
fi
echo

# Build service image (orchestrator)
echo -e "${YELLOW}Step 2: Building service image${NC}"
echo -e "${BLUE}Building nuanced-lsp-proxy:${TAG}...${NC}"
if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAG -f dockerfiles/service.Dockerfile -t nuanced-lsp-proxy:${TAG} . > /tmp/build-service.log 2>&1; then
    if [ "$MULTIARCH" = true ]; then
        echo -e "${GREEN}✓ nuanced-lsp-proxy:${TAG} built successfully (multi-arch)${NC}"
    else
        SIZE=$(docker images nuanced-lsp-proxy:${TAG} --format "{{.Size}}")
        echo -e "${GREEN}✓ nuanced-lsp-proxy:${TAG} built successfully ($SIZE)${NC}"
    fi
else
    echo -e "${RED}✗ nuanced-lsp-proxy failed to build${NC}"
    echo -e "${YELLOW}See /tmp/build-service.log for details${NC}"
    tail -20 /tmp/build-service.log
    exit 1
fi
echo

# Build watchdog image (monitors service container)
echo -e "${YELLOW}Step 3: Building watchdog image${NC}"
echo -e "${BLUE}Building nuanced-lsp-watchdog:${TAG}...${NC}"
if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAG -f dockerfiles/watchdog.Dockerfile -t nuanced-lsp-watchdog:${TAG} . > /tmp/build-watchdog.log 2>&1; then
    if [ "$MULTIARCH" = true ]; then
        echo -e "${GREEN}✓ nuanced-lsp-watchdog:${TAG} built successfully (multi-arch)${NC}"
    else
        SIZE=$(docker images nuanced-lsp-watchdog:${TAG} --format "{{.Size}}")
        echo -e "${GREEN}✓ nuanced-lsp-watchdog:${TAG} built successfully ($SIZE)${NC}"
    fi
else
    echo -e "${RED}✗ nuanced-lsp-watchdog failed to build${NC}"
    echo -e "${YELLOW}See /tmp/build-watchdog.log for details${NC}"
    tail -20 /tmp/build-watchdog.log
    exit 1
fi
echo

echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Rust Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo

if [ "$MULTIARCH" = true ]; then
    echo -e "${BLUE}Multi-arch images built and cached (not loaded into local Docker)${NC}"
    echo

    # If --load was specified, also build local platform and load it
    if [ "$LOAD_LOCAL" = true ]; then
        echo -e "${YELLOW}Also building and loading local platform images...${NC}"
        echo

        # Build wrapper for local platform with --load
        echo -e "${BLUE}Loading nuanced-lsp-wrapper:${TAG} (local platform)...${NC}"
        docker buildx build --load $CACHE_FLAG -f dockerfiles/wrapper.Dockerfile -t nuanced-lsp-wrapper:${TAG} . > /tmp/build-wrapper-local.log 2>&1

        # Build service for local platform with --load
        echo -e "${BLUE}Loading nuanced-lsp-proxy:${TAG} (local platform)...${NC}"
        docker buildx build --load $CACHE_FLAG -f dockerfiles/service.Dockerfile -t nuanced-lsp-proxy:${TAG} . > /tmp/build-service-local.log 2>&1

        # Build watchdog for local platform with --load
        echo -e "${BLUE}Loading nuanced-lsp-watchdog:${TAG} (local platform)...${NC}"
        docker buildx build --load $CACHE_FLAG -f dockerfiles/watchdog.Dockerfile -t nuanced-lsp-watchdog:${TAG} . > /tmp/build-watchdog-local.log 2>&1

        echo -e "${GREEN}✓ Local platform images loaded into Docker${NC}"
        echo
        echo -e "${BLUE}Container Images (Local):${NC}"
        docker images | grep -E "nuanced-lsp-(wrapper|proxy|watchdog)" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
        echo
    fi

    echo -e "${YELLOW}To verify multi-arch builds:${NC}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-wrapper:${TAG}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-proxy:${TAG}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-watchdog:${TAG}"
    echo
    echo -e "${YELLOW}To publish:${NC}"
    echo -e "  ./scripts/publish-images.sh <version> [options]"
else
    echo -e "${BLUE}Container Images (Local):${NC}"
    docker images | grep -E "nuanced-lsp-(wrapper|proxy|watchdog)" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
fi
echo

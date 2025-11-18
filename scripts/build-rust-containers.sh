#!/usr/bin/env bash

set -e

# Build Rust-based containers (wrapper, service, watchdog)
# Usage: ./scripts/build-rust-containers.sh [--use-cache] [--multiarch] [--load] [--tag=TAG] [--registry=REGISTRY]
#
# By default:
#   - Builds WITHOUT cache (use --use-cache to enable caching)
#   - Builds for local platform only (use --multiarch for amd64+arm64)
#   - Builds Rust binaries first before Docker images (only for single-arch builds)
#   - Tags images as :latest (use --tag=0.4.8 for custom tag)
#   - Does NOT push (use --registry to push to ghcr/dockerhub/local)
#
# Options:
#   --multiarch       Build for both linux/amd64 and linux/arm64
#   --load            Also build and load local platform into Docker (use with --multiarch)
#   --use-cache       Enable Docker build cache
#   --tag=TAG         Tag images with specified tag (default: latest)
#   --registry=REG    Push to registry: ghcr, dockerhub, or local (requires authentication)
#   --help, -h        Show help message
#
# Examples:
#   ./scripts/build-rust-containers.sh --tag=0.4.8
#   ./scripts/build-rust-containers.sh --multiarch --tag=0.4.8 --registry=ghcr
#   ./scripts/build-rust-containers.sh --multiarch --load --tag=0.4.8

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default: no cache, single-arch, no load, latest tag, no registry
USE_CACHE=false
MULTIARCH=false
LOAD_LOCAL=false
TAG="latest"
REGISTRY=""  # Options: ghcr, dockerhub, local, or empty for no push

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
        --registry=*)
            REGISTRY="${arg#*=}"
            if [[ ! "$REGISTRY" =~ ^(ghcr|dockerhub|local)$ ]]; then
                echo -e "${RED}Invalid registry: $REGISTRY. Must be ghcr, dockerhub, or local${NC}"
                exit 1
            fi
            ;;
        --help|-h)
            echo "Usage: $0 [--use-cache] [--multiarch] [--load] [--tag=TAG] [--registry=REGISTRY]"
            echo ""
            echo "Options:"
            echo "  --use-cache       Enable Docker build cache (default: disabled)"
            echo "  --multiarch       Build for both linux/amd64 and linux/arm64"
            echo "  --load            Load local platform into Docker (use with --multiarch)"
            echo "  --tag=TAG         Tag images with specified tag (default: latest)"
            echo "  --registry=REG    Push to registry: ghcr, dockerhub, or local"
            echo "  --help, -h        Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 --tag=0.4.8"
            echo "  $0 --multiarch --tag=0.4.8 --registry=ghcr"
            exit 0
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 [--use-cache] [--multiarch] [--load] [--tag=TAG] [--registry=REGISTRY]"
            exit 1
            ;;
    esac
done

# Set cache flag for docker builds
CACHE_FLAG=""
if [ "$USE_CACHE" = false ]; then
    CACHE_FLAG="--no-cache"
fi

# Set up registry configuration and authentication
REGISTRY_PREFIX=""
PUSH_FLAG=""
if [ -n "$REGISTRY" ]; then
    case "$REGISTRY" in
        ghcr)
            REGISTRY_PREFIX="ghcr.io/nuanced-dev/"
            # Check for GITHUB_TOKEN
            if [ -z "$GITHUB_TOKEN" ]; then
                echo -e "${RED}Error: GITHUB_TOKEN environment variable is required for GHCR${NC}"
                echo "Please set GITHUB_TOKEN with write:packages permission"
                echo ""
                echo "To create a token:"
                echo "  1. Go to https://github.com/settings/tokens"
                echo "  2. Generate new token (classic)"
                echo "  3. Select scopes: write:packages, read:packages, delete:packages"
                echo "  4. export GITHUB_TOKEN=your_token_here"
                exit 1
            fi
            # Authenticate to GHCR
            echo -e "${BLUE}Authenticating to ghcr.io...${NC}"
            echo "$GITHUB_TOKEN" | docker login ghcr.io -u nuanced-dev --password-stdin > /dev/null 2>&1
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}✓ Successfully authenticated to GHCR${NC}"
            else
                echo -e "${RED}✗ Failed to authenticate to GHCR${NC}"
                echo "Please check your GITHUB_TOKEN has the correct permissions"
                exit 1
            fi
            ;;
        dockerhub)
            REGISTRY_PREFIX="nuanced/"
            # Check for Docker Hub credentials
            if [ -z "$DOCKER_HUB_USERNAME" ] || [ -z "$DOCKER_HUB_TOKEN" ]; then
                echo -e "${RED}Error: DOCKER_HUB_USERNAME and DOCKER_HUB_TOKEN required for Docker Hub${NC}"
                echo "Please set:"
                echo "  export DOCKER_HUB_USERNAME=your_username"
                echo "  export DOCKER_HUB_TOKEN=your_token_or_password"
                exit 1
            fi
            # Authenticate to Docker Hub
            echo -e "${BLUE}Authenticating to Docker Hub...${NC}"
            echo "$DOCKER_HUB_TOKEN" | docker login -u "$DOCKER_HUB_USERNAME" --password-stdin > /dev/null 2>&1
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}✓ Successfully authenticated to Docker Hub${NC}"
            else
                echo -e "${RED}✗ Failed to authenticate to Docker Hub${NC}"
                exit 1
            fi
            ;;
        local)
            REGISTRY_PREFIX="localhost:5000/"
            # Check if local registry is running
            if ! docker ps | grep -q "registry:2"; then
                echo -e "${YELLOW}Warning: Local registry not detected${NC}"
                echo "To start a local registry:"
                echo "  docker run -d -p 5000:5000 --restart=always --name registry registry:2"
                echo ""
                read -p "Continue anyway? (y/N) " -n 1 -r
                echo
                if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                    exit 1
                fi
            else
                echo -e "${GREEN}✓ Local registry detected at localhost:5000${NC}"
            fi
            ;;
    esac
    PUSH_FLAG="--push"
    echo
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

# Determine full image tag (with or without registry prefix)
WRAPPER_IMAGE_TAG="${REGISTRY_PREFIX}nuanced-lsp-wrapper:${TAG}"
echo -e "${BLUE}Building ${WRAPPER_IMAGE_TAG}...${NC}"

if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAG $PUSH_FLAG -f dockerfiles/wrapper.Dockerfile -t "${WRAPPER_IMAGE_TAG}" . > /tmp/build-wrapper.log 2>&1; then
    if [ -n "$PUSH_FLAG" ]; then
        echo -e "${GREEN}✓ ${WRAPPER_IMAGE_TAG} built and pushed successfully${NC}"
    elif [ "$MULTIARCH" = true ]; then
        echo -e "${GREEN}✓ ${WRAPPER_IMAGE_TAG} built successfully (multi-arch)${NC}"
    else
        SIZE=$(docker images nuanced-lsp-wrapper:${TAG} --format "{{.Size}}")
        echo -e "${GREEN}✓ ${WRAPPER_IMAGE_TAG} built successfully ($SIZE)${NC}"
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

# Determine full image tag (with or without registry prefix)
PROXY_IMAGE_TAG="${REGISTRY_PREFIX}nuanced-lsp-proxy:${TAG}"
echo -e "${BLUE}Building ${PROXY_IMAGE_TAG}...${NC}"

if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAG $PUSH_FLAG -f dockerfiles/service.Dockerfile -t "${PROXY_IMAGE_TAG}" . > /tmp/build-service.log 2>&1; then
    if [ -n "$PUSH_FLAG" ]; then
        echo -e "${GREEN}✓ ${PROXY_IMAGE_TAG} built and pushed successfully${NC}"
    elif [ "$MULTIARCH" = true ]; then
        echo -e "${GREEN}✓ ${PROXY_IMAGE_TAG} built successfully (multi-arch)${NC}"
    else
        SIZE=$(docker images nuanced-lsp-proxy:${TAG} --format "{{.Size}}")
        echo -e "${GREEN}✓ ${PROXY_IMAGE_TAG} built successfully ($SIZE)${NC}"
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

# Determine full image tag (with or without registry prefix)
WATCHDOG_IMAGE_TAG="${REGISTRY_PREFIX}nuanced-lsp-watchdog:${TAG}"
echo -e "${BLUE}Building ${WATCHDOG_IMAGE_TAG}...${NC}"

if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAG $PUSH_FLAG -f dockerfiles/watchdog.Dockerfile -t "${WATCHDOG_IMAGE_TAG}" . > /tmp/build-watchdog.log 2>&1; then
    if [ -n "$PUSH_FLAG" ]; then
        echo -e "${GREEN}✓ ${WATCHDOG_IMAGE_TAG} built and pushed successfully${NC}"
    elif [ "$MULTIARCH" = true ]; then
        echo -e "${GREEN}✓ ${WATCHDOG_IMAGE_TAG} built successfully (multi-arch)${NC}"
    else
        SIZE=$(docker images nuanced-lsp-watchdog:${TAG} --format "{{.Size}}")
        echo -e "${GREEN}✓ ${WATCHDOG_IMAGE_TAG} built successfully ($SIZE)${NC}"
    fi
else
    echo -e "${RED}✗ nuanced-lsp-watchdog failed to build${NC}"
    echo -e "${YELLOW}See /tmp/build-watchdog.log for details${NC}"
    tail -20 /tmp/build-watchdog.log
    exit 1
fi
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
    fi
fi

echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Rust Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo

if [ -n "$PUSH_FLAG" ]; then
    # Images were pushed to registry
    echo -e "${GREEN}Images pushed to ${REGISTRY}:${NC}"
    echo -e "  ${WRAPPER_IMAGE_TAG}"
    echo -e "  ${PROXY_IMAGE_TAG}"
    echo -e "  ${WATCHDOG_IMAGE_TAG}"
    echo
    echo -e "${YELLOW}To verify:${NC}"
    echo -e "  docker pull ${WRAPPER_IMAGE_TAG}"
    echo -e "  docker pull ${PROXY_IMAGE_TAG}"
    echo -e "  docker pull ${WATCHDOG_IMAGE_TAG}"
elif [ "$MULTIARCH" = true ]; then
    if [ "$LOAD_LOCAL" = true ]; then
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
    echo -e "  $0 --multiarch --tag=${TAG} --registry=ghcr"
else
    echo -e "${BLUE}Container Images (Local):${NC}"
    docker images | grep -E "nuanced-lsp-(wrapper|proxy|watchdog)" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
fi
echo

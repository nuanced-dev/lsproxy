#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"

DEFAULT_RUST_TAG="$("$SCRIPT_DIR/util/rust-image-version.sh")"
DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

usage() {
    echo "Usage: $0 [--cache=MODE] [--language-tag=TAG] [--multiarch] [--registry=REGISTRY] [--rust-tag=TAG] [--sequential]"
}

help() {
    echo "Build Rust-based containers (wrapper, proxy, watchdog)"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --cache=MODE          Docker build cache mode: none, docker, gha (default: none)"
    echo "                        - none: disable all caching"
    echo "                        - docker: use default Docker layer caching"
    echo "                        - gha: build without Docker cache to ensure fresh Docker layers,"
    echo "                               but use GitHub Actions cache backend for BuildKit cache"
    echo "                               mounts (Cargo registry and build artifacts). This gives us"
    echo "                               reproducible builds while still caching Rust compilation."
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "                        Language containers use semver (e.g., 1.0.0) for API compatibility"
    echo "  --multiarch           Build for both linux/amd64 and linux/arm64 (default: local platform only)"
    echo "  --registry=REG        Push to registry: ghcr, dockerhub, or local (default: no push)"
    echo "  --rust-tag=TAG        Tag Rust images with specified tag (default: $DEFAULT_RUST_TAG)"
    echo "  --sequential          Build images sequentially (default: parallel)"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --rust-tag=0.4.8 --language-tag=1.1.0"
    echo "  $0 --multiarch --rust-tag=0.4.8 --registry=ghcr"
    echo "  $0 --cache=gha --rust-tag=0.4.8"
}

# Defaults
CACHE_MODE=none
LANGUAGE_TAG=""
MULTIARCH=false
PARALLEL=true
REGISTRY="" # Options: ghcr, dockerhub, local, or empty for no push
RUST_TAG=""

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            help
            exit 0
            ;;
        --cache=*)
            CACHE_MODE="${arg#*=}"
            if [[ ! "$CACHE_MODE" =~ ^(none|docker|gha)$ ]]; then
                echo -e "${RED}Invalid cache mode: $CACHE_MODE. Must be none, docker, or gha${NC}"
                exit 1
            fi
            ;;
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --multiarch)
            MULTIARCH=true
            ;;
        --registry=*)
            REGISTRY="${arg#*=}"
            if [[ ! "$REGISTRY" =~ ^(ghcr|dockerhub|local)$ ]]; then
                echo -e "${RED}Invalid registry: $REGISTRY. Must be ghcr, dockerhub, or local${NC}"
                exit 1
            fi
            ;;
        --rust-tag=*)
            RUST_TAG="${arg#*=}"
            ;;
        --sequential)
            PARALLEL=false
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            usage
            exit 1
            ;;
    esac
done

# Fall back to default tags
RUST_TAG="${RUST_TAG:-$DEFAULT_RUST_TAG}"
LANGUAGE_TAG="${LANGUAGE_TAG:-$DEFAULT_LANGUAGE_TAG}"

# Set up registry configuration and authentication
REGISTRY_PREFIX=""
PUSH_FLAG=""
if [ -n "$REGISTRY" ]; then
    case "$REGISTRY" in
        ghcr)
            REGISTRY_PREFIX="ghcr.io/nuanced-dev/"
            # Check for GITHUB_TOKEN
            if [ -z "${GITHUB_TOKEN:+x}" ]; then
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
            if echo "$GITHUB_TOKEN" | docker login ghcr.io -u nuanced-dev --password-stdin > /dev/null 2>&1; then
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
            if [ -z "${DOCKER_HUB_TOKEN:+x}" ]; then
                echo -e "${RED}Error: DOCKER_HUB_TOKEN required for Docker Hub${NC}"
                echo "Please set:"
                echo "  export DOCKER_HUB_TOKEN=your_token_or_password"
                exit 1
            fi
            # Authenticate to Docker Hub
            echo -e "${BLUE}Authenticating to Docker Hub...${NC}"
            if echo "$DOCKER_HUB_TOKEN" | docker login -u nuanced --password-stdin > /dev/null 2>&1; then
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

# Set up build arguments
BUILD_ARGS=("--build-arg" "RUST_IMAGE_VERSION=$RUST_TAG" "--build-arg" "LANGUAGE_IMAGE_VERSION=$LANGUAGE_TAG")

# Set up build command based on multiarch flag
BUILD_CMD="docker build"
PLATFORM_FLAG=""
if [ "$MULTIARCH" = true ]; then
    BUILD_CMD="docker buildx build"
    PLATFORM_FLAG="--platform linux/amd64,linux/arm64"
    # Note: Multi-arch builds are NOT loaded into local Docker daemon
    # They are built and cached, ready for pushing to a registry
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Multi-Arch Rust Containers${NC}"
    echo -e "${BLUE}  Platforms: linux/amd64, linux/arm64${NC}"
    echo -e "${BLUE}  Cache: $CACHE_MODE | Parallel: $PARALLEL${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
    echo -e "${YELLOW}Note: Multi-arch builds are prepared for publishing but not loaded into local Docker${NC}"
    echo -e "${YELLOW}      Use 'docker buildx imagetools inspect <image>' to verify build${NC}"
    echo -e "${YELLOW}      Use './scripts/publish-images.sh' to push to registry${NC}"
    echo -e "${YELLOW}      Skipping local cargo build (cross-compilation happens in Docker)${NC}"
    echo
else
    BUILD_CMD="docker build"
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Rust Containers (Local Platform)${NC}"
    echo -e "${BLUE}  Cache: $CACHE_MODE | Parallel: $PARALLEL${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
fi

compute_cache_flags() {
    local name="$1"
    case "$CACHE_MODE" in
        none)
            echo "--no-cache"
            ;;
        docker)
            echo ""
            ;;
        gha)
            echo "--no-cache --cache-from type=gha,scope=$name --cache-to type=gha,mode=max,scope=$name"
            ;;
    esac
}

build_image() {
    local name="$1"
    local image_tag="$2"
    local dockerfile="$3"
    local log_file="$4"
    local human_name="$5"

    local CACHE_FLAGS
    CACHE_FLAGS="$(compute_cache_flags "$name")"

    echo -e "${BLUE}Building ${image_tag}...${NC}"
    if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAGS $PUSH_FLAG -f "${dockerfile}" -t "${image_tag}" "${BUILD_ARGS[@]}" . > "${log_file}" 2>&1; then
        if [ -n "$PUSH_FLAG" ]; then
            echo -e "${GREEN}✓ ${image_tag} built and pushed successfully${NC}"
        elif [ "$MULTIARCH" = true ]; then
            echo -e "${GREEN}✓ ${image_tag} built successfully (multi-arch)${NC}"
        else
            local size
            size=$(docker images "${image_tag}" --format "{{.Size}}")
            echo -e "${GREEN}✓ ${image_tag} built successfully ($size)${NC}"
        fi
        return 0
    else
        echo -e "${RED}✗ ${human_name} failed to build${NC}"
        echo -e "${YELLOW}See ${log_file} for details${NC}"
        tail -20 "${log_file}" || true
        return 1
    fi
}

# Build images (wrapper, proxy, watchdog)
echo -e "${YELLOW}Step 1: Building images${NC}"

PROXY_IMAGE_TAG="${REGISTRY_PREFIX}nuanced-lsp-proxy:${RUST_TAG}"
WATCHDOG_IMAGE_TAG="${REGISTRY_PREFIX}nuanced-lsp-watchdog:${RUST_TAG}"
WRAPPER_IMAGE_TAG="${REGISTRY_PREFIX}nuanced-lsp-wrapper:${RUST_TAG}"

if [ "$PARALLEL" = true ]; then
    echo -e "${BLUE}Building in parallel (logs: /tmp/build-*.log)...${NC}"
    declare -a pids names

    (build_image "proxy" "${PROXY_IMAGE_TAG}" dockerfiles/proxy.Dockerfile /tmp/build-proxy.log "nuanced-lsp-proxy") &
    pids+=($!)
    names+=("proxy")

    (build_image "watchdog" "${WATCHDOG_IMAGE_TAG}" dockerfiles/watchdog.Dockerfile /tmp/build-watchdog.log "nuanced-lsp-watchdog") &
    pids+=($!)
    names+=("watchdog")

    (build_image "wrapper" "${WRAPPER_IMAGE_TAG}" dockerfiles/wrapper.Dockerfile /tmp/build-wrapper.log "nuanced-lsp-wrapper") &
    pids+=($!)
    names+=("wrapper")

    failed=0
    for i in "${!pids[@]}"; do
        if ! wait "${pids[$i]}"; then
            failed=1
        fi
    done

    if [ $failed -ne 0 ]; then
        echo -e "${RED}One or more images failed to build${NC}"
        exit 1
    fi
else
    build_image "proxy" "${PROXY_IMAGE_TAG}" dockerfiles/proxy.Dockerfile /tmp/build-proxy.log "nuanced-lsp-proxy" || exit 1
    build_image "watchdog" "${WATCHDOG_IMAGE_TAG}" dockerfiles/watchdog.Dockerfile /tmp/build-watchdog.log "nuanced-lsp-watchdog" || exit 1
    build_image "wrapper" "${WRAPPER_IMAGE_TAG}" dockerfiles/wrapper.Dockerfile /tmp/build-wrapper.log "nuanced-lsp-wrapper" || exit 1
fi
echo

if [ "$MULTIARCH" = true ]; then
    echo -e "${BLUE}Multi-arch images built and cached (not loaded into local Docker)${NC}"
    echo
fi

echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Rust Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo

if [ -n "$PUSH_FLAG" ]; then
    # Images were pushed to registry
    echo -e "${GREEN}Images pushed to ${REGISTRY}:${NC}"
    echo -e "  ${PROXY_IMAGE_TAG}"
    echo -e "  ${WATCHDOG_IMAGE_TAG}"
    echo -e "  ${WRAPPER_IMAGE_TAG}"
    echo
    echo -e "${YELLOW}To verify:${NC}"
    echo -e "  docker pull ${PROXY_IMAGE_TAG}"
    echo -e "  docker pull ${WATCHDOG_IMAGE_TAG}"
    echo -e "  docker pull ${WRAPPER_IMAGE_TAG}"
elif [ "$MULTIARCH" = true ]; then
    echo -e "${YELLOW}To verify multi-arch builds:${NC}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-proxy:${RUST_TAG}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-watchdog:${RUST_TAG}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-wrapper:${RUST_TAG}"
    echo
    echo -e "${YELLOW}To load local platform images, run without --multiarch${NC}"
    echo
    echo -e "${YELLOW}To publish:${NC}"
    echo -e "  $0 --multiarch --rust-tag=${RUST_TAG} --registry=ghcr"
else
    echo -e "${BLUE}Container Images (Local):${NC}"
    docker images | grep -E "nuanced-lsp-(proxy|watchdog|wrapper)" | grep -F "$RUST_TAG" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
fi
echo

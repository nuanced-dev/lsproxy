#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"

DEFAULT_RUST_TAG="$("$SCRIPT_DIR/util/rust-image-version.sh")"
DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

usage() {
    echo "Usage: $0 [--cache=MODE] [--language-tag=TAG] [--multi-platform] [--rust-tag=TAG] [--sequential]"
}

help() {
    echo "Build Rust-based images (wrapper, proxy, watchdog)"
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
    echo "                        Language images use semver (e.g., 1.0.0) for API compatibility"
    echo "  --multi-platform      Build for both linux/amd64 and linux/arm64 (default: local platform only)"
    echo "  --rust-tag=TAG        Tag Rust images with specified tag (default: $DEFAULT_RUST_TAG)"
    echo "  --sequential          Build images sequentially (default: parallel)"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --rust-tag=0.4.8 --language-tag=1.1.0"
    echo "  $0 --multi-platform --rust-tag=0.4.8"
    echo "  $0 --cache=gha --rust-tag=0.4.8"
}

# Defaults
CACHE_MODE=none
LANGUAGE_TAG=""
MULTIARCH=false
PARALLEL=true
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
        --multi-platform)
            MULTIARCH=true
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

BUILD_CMD=()
if [ "$MULTIARCH" = true ]; then
    BUILD_CMD=("docker" "buildx" "build" "--platform" "linux/amd64,linux/arm64")
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Multi-Arch Rust Images${NC}"
    echo -e "${BLUE}  Platforms: linux/amd64, linux/arm64${NC}"
    echo -e "${BLUE}  Parallel: $PARALLEL${NC}"
    echo -e "${BLUE}  Cache: $CACHE_MODE${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
else
    BUILD_CMD=("docker" "build")
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Rust Images (Local Platform)${NC}"
    echo -e "${BLUE}  Parallel: $PARALLEL${NC}"
    echo -e "${BLUE}  Cache: $CACHE_MODE${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
fi

# Verify storage for multi-platform builds
if [ "$MULTIARCH" = true ] && [ "$(docker system info --format json | jq '.DriverStatus | any(.[]; .[0] == "driver-type" and .[1] == "io.containerd.snapshotter.v1")')" != true ]; then
    echo -e "${RED}Error: Multi-arch builds require containerd storage so Docker can load the multi-platform images${NC}"
    exit 1
fi

# Set up build arguments
BUILD_CMD+=(
    "--build-arg" "RUST_IMAGE_VERSION=$RUST_TAG"
    "--build-arg" "LANGUAGE_IMAGE_VERSION=$LANGUAGE_TAG"
)

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

    local dockerfile="dockerfiles/${name}.Dockerfile"

    if [ ! -f "$dockerfile" ]; then
        echo -e "${RED}Error: $dockerfile not found${NC}"
        return 1
    fi

    local image_tag="nuanced-lsp-${name}:${RUST_TAG}"

    echo -e "${BLUE}Building ${image_tag}...${NC}"

    local CACHE_FLAGS
    CACHE_FLAGS="$(compute_cache_flags "$name")"

    local log_file="/tmp/build-${name}.log"

    if "${BUILD_CMD[@]}" $CACHE_FLAGS -f "$dockerfile" -t "$image_tag" . > "$log_file" 2>&1; then
        if [ "$MULTIARCH" = true ]; then
            echo -e "${GREEN}✓ ${image_tag} built successfully (multi-platform)${NC}"
        else
            local size
            size=$(docker images "${image_tag}" --format "{{.Size}}")
            echo -e "${GREEN}✓ ${image_tag} built successfully ($size)${NC}"
        fi
        return 0
    else
        echo -e "${RED}✗ ${image_tag} failed to build${NC}"
        echo -e "${YELLOW}See ${log_file} for details${NC}"
        tail -20 "${log_file}" || true
        return 1
    fi
}

# Build images (wrapper, proxy, watchdog)
echo -e "${YELLOW}Building Rust images${NC}"

if [ "$PARALLEL" = true ]; then
    echo -e "${BLUE}Building in parallel (logs: /tmp/build-*.log)...${NC}"
    declare -a pids names

    (build_image "proxy") &
    pids+=($!)
    names+=("proxy")

    (build_image "watchdog") &
    pids+=($!)
    names+=("watchdog")

    (build_image "wrapper") &
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
    build_image "proxy" || exit 1
    build_image "watchdog" || exit 1
    build_image "wrapper" || exit 1
fi
echo

echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Rust Images Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}Images (Local):${NC}"
docker images | grep "nuanced-lsp-" | grep -E "(proxy|watchdog|wrapper)" | grep -F "$RUST_TAG" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
echo
echo -e "${BLUE}Total size:${NC}"
docker images | grep "nuanced-lsp-" | grep -E "(proxy|watchdog|wrapper)" | grep -F "$RUST_TAG" | awk '{size+=$7} END {print "  ~" size " (approximate)"}'
echo
echo -e "${YELLOW}To publish images to registries:${NC}"
echo -e "  $(dirname "$0")/publish-images.sh"
if [ "$MULTIARCH" = true ]; then
    echo
    echo -e "${BLUE}Multi-arch images built and cached${NC}"
    echo
    echo -e "${YELLOW}To verify multi-platform builds:${NC}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-proxy:${RUST_TAG}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-watchdog:${RUST_TAG}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-wrapper:${RUST_TAG}"
fi

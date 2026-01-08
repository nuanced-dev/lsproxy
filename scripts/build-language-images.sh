#!/usr/bin/env bash
# Build language server containers (Python, TypeScript, Rust, Go, Java, C++, C#, PHP, Ruby variants)

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/supported-languages.sh"
source "$SCRIPT_DIR/include/supported-ruby-versions.sh"

DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

usage() {
    echo "Usage: $0 [--cache=MODE] [--jobs=N] [--language-tag=TAG] [--languages=LANG...] [--multi-platform] [--registry=REGISTRY] [--sequential]"
}

help() {
    echo "Build language server images (Python, TypeScript, Rust, Go, Java, C++, C#, PHP, Ruby variants)"
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
    echo "  --jobs=N, -j=N        Max parallel builds (default: 4, prevents Docker daemon overload)"
    echo "  --language-tag=TAG    Tag images with specified semver tag (default: $DEFAULT_LANGUAGE_TAG)"
    echo "                        Language images use semver (e.g., 1.0.0) for API compatibility"
    echo "  --languages=LANG...   Build specific language(s) - comma-separated (default: all languages)"
    echo "                        Use empty value (--languages=) for no languages"
    echo "                        Supports versioned Ruby: ruby-3.2.2, ruby-sorbet-3.2.2"
    echo "  --multi-platform      Build for both amd64 and arm64 (default: local platform only)"
    echo "  --registry=REGISTRY   Push to registry: ghcr, dockerhub, local (default: no push)"
    echo "  --sequential          Build sequentially instead of parallel (default: parallel)"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Versioning: Language images use semver (MAJOR.MINOR.PATCH)"
    echo "  MAJOR = API/protocol compatibility version"
    echo "  MINOR = New LSP features, language server updates"
    echo "  PATCH = Bug fixes, dependency updates"
    echo ""
    echo "Note: Multi-arch Sorbet builds require --registry because Ruby Sorbet images depend"
    echo "on Ruby base images which must be available in a registry for multi-platform builds."
    echo ""
    echo "Note: Language images use binary injection at runtime via --volumes-from. They only"
    echo "need to be rebuilt when language server versions change or when base dependencies change."
    echo ""
    echo "Available languages: python, typescript, rust, golang, java, clangd, csharp, php, ruby, ruby-sorbet"
    echo ""
    echo "Examples:"
    echo "  $0 --language-tag=1.0.0"
    echo "  $0 --multi-platform --language-tag=1.0.0 --registry=ghcr"
    echo "  $0 --jobs=8"
    echo "  $0 --languages=python --multi-platform --language-tag=1.0.0 --registry=ghcr"
    echo "  $0 --languages=ruby,ruby-sorbet --language-tag=1.0.0"
    echo "  $0 --languages=ruby-3.2.2,ruby-sorbet-3.2.2 --language-tag=1.0.0"
}

# Defaults
CACHE_MODE=none
LANGUAGE_TAG=""
LANGUAGES=("${SUPPORTED_LANGUAGES[@]}")
MAX_JOBS=4
MULTIARCH=false
PARALLEL=true
REGISTRY=""          # Options: ghcr, dockerhub, local, or empty for no push

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
        --jobs=*|-j=*)
            MAX_JOBS="${arg#*=}"
            if ! [[ "$MAX_JOBS" =~ ^[0-9]+$ ]] || [ "$MAX_JOBS" -lt 1 ]; then
                echo -e "${RED}Invalid jobs value: $MAX_JOBS. Must be a positive integer${NC}"
                exit 1
            fi
            ;;
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --languages=*)
            IFS=',' read -ra LANGUAGES <<< "${arg#*=}"
            ;;
        --multi-platform)
            MULTIARCH=true
            ;;
        --registry=*)
            REGISTRY="${arg#*=}"
            if [[ ! "$REGISTRY" =~ ^(ghcr|dockerhub|local)$ ]]; then
                echo -e "${RED}Invalid registry: $REGISTRY. Must be ghcr, dockerhub, or local${NC}"
                exit 1
            fi
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
LANGUAGE_TAG="${LANGUAGE_TAG:-$DEFAULT_LANGUAGE_TAG}"

echo -e "${YELLOW}Building languages: ${LANGUAGES[*]}${NC}"

BUILD_CMD=()
if [ "$MULTIARCH" = true ]; then
    BUILD_CMD=("docker" "buildx" "build" "--platform" "linux/amd64,linux/arm64")
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Multi-Arch Language Images${NC}"
    echo -e "${BLUE}  Platforms: linux/amd64, linux/arm64${NC}"
    echo -e "${BLUE}  Parallel: $PARALLEL (max $MAX_JOBS jobs)${NC}"
    echo -e "${BLUE}  Cache: $CACHE_MODE${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
else
    BUILD_CMD=("docker" "build")
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Language Server images (Local Platform)${NC}"
    echo -e "${BLUE}  Parallel: $PARALLEL (max $MAX_JOBS jobs)${NC}"
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
    "--build-arg" "LANGUAGE_IMAGE_VERSION=$LANGUAGE_TAG"
)

# Set up registry configuration and authentication
REGISTRY_PREFIX=""
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
    BUILD_CMD+=("--push")
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
    local subdir="$1"  # Optional subdirectory (ruby or ruby-sorbet)
    local lang="$2"

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

    # For Ruby images, the image name includes the Ruby version
    # Format: nuanced-lsp-ruby-3.4.4 or nuanced-lsp-ruby-sorbet-3.4.4
    local image_name
    if [ -n "$subdir" ]; then
        image_name="nuanced-lsp-${subdir}-${lang}"
    else
        image_name="nuanced-lsp-${lang}"
    fi

    # Determine the full image version (with or without registry prefix)
    local full_image_tag="${REGISTRY_PREFIX}${image_name}:${LANGUAGE_TAG}"

    echo -e "${BLUE}Building ${full_image_tag}...${NC}"

    # Compute cache flags based on image name
    local cache_name
    if [ -n "$subdir" ]; then
        cache_name="${subdir}-${lang}"
    else
        cache_name="${lang}"
    fi
    local CACHE_FLAGS
    CACHE_FLAGS="$(compute_cache_flags "$cache_name")"

    local log_file="/tmp/build-${cache_name}.log"

    if "${BUILD_CMD[@]}" $CACHE_FLAGS -f "$dockerfile" -t "$full_image_tag" . > "$log_file" 2>&1; then
        if [ "$MULTIARCH" = true ]; then
            echo -e "${GREEN}✓ ${full_image_tag} built successfully (multi-platform)${NC}"
        else
            local size
            size=$(docker images "${image_name}:${LANGUAGE_TAG}" --format "{{.Size}}")
            echo -e "${GREEN}✓ ${full_image_tag} built successfully ($size)${NC}"
        fi
        return 0
    else
        echo -e "${RED}✗ ${image_name} failed to build${NC}"
        echo -e "${YELLOW}See ${log_file} for details${NC}"
        tail -20 "$log_file" || true
        return 1
    fi
}

# Throttled parallel build function
# Runs builds in parallel but limits concurrency to MAX_JOBS
# Arguments:
#   $1 - subdir (empty string, "ruby", or "ruby-sorbet")
#   $2... - items to build
# If subdir is empty, the items are language names. Otherwise,
# the subdir is the base and the items are language versions.
build_parallel_throttled() {
    local subdir="$1"
    local items=("${@:2}")

    local pids=()
    local failed=0
    local running=0

    for item in "${items[@]}"; do
        # Wait if we've hit the max concurrent jobs
        while [ $running -ge "$MAX_JOBS" ]; do
            # Wait for any job to finish
            for i in "${!pids[@]}"; do
                if ! kill -0 "${pids[$i]}" 2>/dev/null; then
                    # Job finished, check its exit status
                    if ! wait "${pids[$i]}"; then
                        failed=$((failed + 1))
                    fi
                    unset 'pids[i]'
                    running=$((running - 1))
                    break
                fi
            done
            # Small sleep to avoid busy waiting
            sleep 0.1
        done

        # Start new build
        build_image "$subdir" "$item" &
        pids+=($!)
        running=$((running + 1))
    done

    # Wait for remaining jobs
    for pid in "${pids[@]}"; do
        if [ -n "$pid" ]; then
            if ! wait "$pid"; then
                failed=$((failed + 1))
            fi
        fi
    done

    return $failed
}

# Separate languages into categories (ruby-sorbet must be built after ruby)
REGULAR_LANGUAGES=()
RUBY_VERSIONS=()
RUBY_SORBET_VERSIONS=()

for lang in "${LANGUAGES[@]}"; do
    if [[ "$lang" == "ruby" ]]; then
        RUBY_VERSIONS+=("${SUPPORTED_RUBY_VERSIONS[@]}")
    elif [[ "$lang" =~ ^ruby-[0-9] ]]; then
        version="${lang#ruby-}"
        RUBY_VERSIONS+=("$version")
    elif [[ "$lang" == "ruby-sorbet" ]]; then
        RUBY_SORBET_VERSIONS+=("${SUPPORTED_RUBY_VERSIONS[@]}")
    elif [[ "$lang" =~ ^ruby-sorbet-[0-9] ]]; then
        version="${lang#ruby-sorbet-}"
        RUBY_SORBET_VERSIONS+=("$version")
    else
        REGULAR_LANGUAGES+=("$lang")
    fi
done

# Build regular (non-Ruby) languages
if [ ${#REGULAR_LANGUAGES[@]} -eq 0 ]; then
    echo -e "${YELLOW}No regular language images to build${NC}"
else
    echo -e "${YELLOW}Building ${#REGULAR_LANGUAGES[@]} language images${NC}"

    if [ "$PARALLEL" = true ]; then
        echo -e "${BLUE}Building in parallel (max $MAX_JOBS concurrent, see /tmp/build-*.log for progress)${NC}"

        if ! build_parallel_throttled "" "${REGULAR_LANGUAGES[@]}"; then
            failed=$?
        else
            failed=0
        fi

        if [ $failed -gt 0 ]; then
            echo -e "${RED}$failed language images failed to build${NC}"
            exit 1
        fi
    else
        for lang in "${REGULAR_LANGUAGES[@]}"; do
            build_image "" "$lang" || exit 1
        done
    fi

    echo
fi

# Build Ruby images (must complete before Sorbet images)
if [ ${#RUBY_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby images to build${NC}"
else
    echo -e "${YELLOW}Building Ruby images (${#RUBY_VERSIONS[@]} versions)${NC}"

    if [ "$PARALLEL" = true ]; then
        echo -e "${BLUE}Building in parallel (max $MAX_JOBS concurrent, see /tmp/build-ruby-*.log for progress)${NC}"

        if ! build_parallel_throttled "ruby" "${RUBY_VERSIONS[@]}"; then
            failed=$?
        else
            failed=0
        fi

        if [ $failed -gt 0 ]; then
            echo -e "${RED}$failed Ruby images failed to build${NC}"
            exit 1
        fi
    else
        for version in "${RUBY_VERSIONS[@]}"; do
            build_image "ruby" "$version" || exit 1
        done
    fi

    echo
fi

# Build Ruby Sorbet variants (depends on Ruby base images)
if [ ${#RUBY_SORBET_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby Sorbet images to build${NC}"
else
    echo -e "${YELLOW}Building Ruby Sorbet images (${#RUBY_SORBET_VERSIONS[@]} versions)${NC}"

    if [ "$PARALLEL" = true ]; then
        echo -e "${BLUE}Building in parallel (max $MAX_JOBS concurrent, see /tmp/build-ruby-sorbet-*.log for progress)${NC}"

        if ! build_parallel_throttled "ruby-sorbet" "${RUBY_SORBET_VERSIONS[@]}"; then
            failed=$?
        else
            failed=0
        fi

        if [ $failed -gt 0 ]; then
            echo -e "${RED}$failed Ruby Sorbet images failed to build${NC}"
            exit 1
        fi
    else
        for version in "${RUBY_SORBET_VERSIONS[@]}"; do
            build_image "ruby-sorbet" "$version" || exit 1
        done
    fi

    echo
fi

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  All Language Images Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}Images (Local):${NC}"
docker images | grep "nuanced-lsp-" | grep -v -E "(proxy|watchdog|wrapper)" | grep -F "$LANGUAGE_TAG" | awk '{printf "  %-40s %10s\n", $1":"$2, $7}'
echo
echo -e "${BLUE}Total size:${NC}"
docker images | grep "nuanced-lsp-" | grep -v -E "(proxy|watchdog|wrapper)" | grep -F "$LANGUAGE_TAG" | awk '{size+=$7} END {print "  ~" size " (approximate)"}'
echo
if [ -n "$REGISTRY" ]; then
    # Images were pushed to registry
    echo -e "${GREEN}Images pushed to ${REGISTRY}:${NC}"
    echo -e "  • ${#REGULAR_LANGUAGES[@]} language images"
    echo -e "  • ${#RUBY_VERSIONS[@]} Ruby base images"
    echo -e "  • ${#RUBY_SORBET_VERSIONS[@]} Ruby Sorbet images"
    echo
    echo -e "${YELLOW}To verify pushed images:${NC}"
    echo -e "  docker pull ${REGISTRY_PREFIX}nuanced-lsp-<language>:${LANGUAGE_TAG}"
    if [ "$REGISTRY" = "ghcr" ]; then
        echo
        echo -e "${YELLOW}To list all packages in ${REGISTRY}:${NC}"
        echo -e "  ./scripts/ghcr-utils.sh list-packages"
    fi
    echo
fi
if [ "$MULTIARCH" = true ]; then
    echo -e "${BLUE}Multi-arch images built and cached${NC}"
    echo
    echo -e "${YELLOW}To verify multi-platform builds:${NC}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-<language>:${LANGUAGE_TAG}"
    echo
    echo -e "${YELLOW}To publish to registry:${NC}"
    echo -e "  $(dirname "$0")/publish-images.sh --registry=ghcr"
fi

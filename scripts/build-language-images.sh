#!/usr/bin/env bash

set -e

# Build language server containers (Python, TypeScript, Rust, Go, Java, C++, C#, PHP, Ruby variants)
# Usage: ./scripts/build-language-images.sh [--use-cache] [--sequential] [--all-ruby-versions] [--multiarch] [--load] [--tag=TAG] [--registry=REGISTRY] [--language=LANG] [--jobs=N]
#
# By default:
#   - Builds WITHOUT cache (use --use-cache to enable caching)
#   - Builds in PARALLEL with max 4 concurrent jobs (use --jobs=N to adjust)
#   - Builds all 12 Ruby versions (5 minor + 7 core patch versions)
#   - Builds for local platform only (use --multiarch for amd64+arm64)
#   - Tags images as :1.0.0 (use --tag=1.1.0 for custom version)
#   - Does NOT push (use --registry to push to ghcr/dockerhub/local)
#   - Builds ALL languages (use --language to build specific languages)
#
# Versioning:
#   Language containers use semantic versioning (MAJOR.MINOR.PATCH):
#   - MAJOR: API/protocol compatibility version (increment for breaking changes)
#   - MINOR: New LSP features, language server version updates
#   - PATCH: Bug fixes, dependency updates
#
# Options:
#   --use-cache           Enable Docker build cache (default: disabled)
#   --sequential          Build sequentially instead of parallel
#   --parallel            Build in parallel (default)
#   --jobs=N, -j=N        Max parallel builds (default: 4, prevents Docker daemon overload)
#   --all-ruby-versions   Build all Ruby versions from dockerfiles (currently 12 versions)
#   --multiarch           Build for both amd64 and arm64 (default: local platform only)
#   --load                Also build and load local platform into Docker (use with --multiarch)
#   --tag=TAG             Tag images with specified semver tag (default: 1.0.0)
#   --registry=REGISTRY   Push to registry: ghcr, dockerhub, or local (requires authentication)
#   --language=LANG       Build specific language(s) (comma-separated: python,typescript,ruby,ruby-sorbet)
#   --help, -h            Show help message
#
# Examples:
#   ./scripts/build-language-images.sh --tag=1.0.0
#   ./scripts/build-language-images.sh --multiarch --tag=1.0.0 --registry=ghcr
#   ./scripts/build-language-images.sh --all-ruby-versions --jobs=8
#   ./scripts/build-language-images.sh --language=python --multiarch --tag=1.0.0 --registry=ghcr
#   ./scripts/build-language-images.sh --language=ruby,ruby-sorbet --tag=1.0.0
#
# Note: Multi-arch Sorbet builds require --registry because Ruby Sorbet images depend on
# Ruby base images which must be available in a registry for multi-platform builds.
#
# Note: Language containers use binary injection at runtime via --volumes-from
# They only need to be rebuilt when language server versions change or when base dependencies change

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default: parallel builds without cache, build main Ruby versions by default, single-arch, no load, v1.0.0 tag
# Language containers use semver where major version = API compatibility version
PARALLEL=true
USE_CACHE=false
ALL_RUBY_VERSIONS=false
MULTIARCH=false
LOAD_LOCAL=false
TAG="1.0.0"
DEFAULT_RUBY_VERSION="3.4.7"
REGISTRY=""  # Options: ghcr, dockerhub, local, or empty for no push
FILTER_LANGUAGES=""  # Empty = build all, otherwise comma-separated list: python,typescript,ruby,ruby-sorbet
# Default max parallel jobs (4 is safe for most systems, prevents Docker daemon overload)
MAX_JOBS=4
# Ruby versions to build (last 1 year of releases, Nov 2024 - Nov 2025)
# 3.3.x: 3.3.6 (Nov 2024) through 3.3.10 (Oct 2025)
# 3.4.x: 3.4.0 (Dec 2024) through 3.4.7 (Oct 2025)
COMMON_RUBY_VERSIONS=("3.3.6" "3.3.7" "3.3.8" "3.3.9" "3.3.10" "3.4.0" "3.4.1" "3.4.2" "3.4.3" "3.4.4" "3.4.5" "3.4.6" "3.4.7")

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            echo "Usage: $0 [--use-cache] [--sequential] [--all-ruby-versions] [--multiarch] [--load] [--tag=TAG] [--registry=REGISTRY] [--language=LANG] [--jobs=N]"
            echo ""
            echo "Options:"
            echo "  --use-cache           Enable Docker build cache (default: disabled)"
            echo "  --sequential          Build sequentially instead of parallel"
            echo "  --parallel            Build in parallel (default)"
            echo "  --jobs=N, -j=N        Max parallel builds (default: 4, prevents Docker daemon overload)"
            echo "  --all-ruby-versions   Build all Ruby versions from dockerfiles (currently 12 versions)"
            echo "  --multiarch           Build for both amd64 and arm64 (default: local platform only)"
            echo "  --load                Also build and load local platform into Docker (use with --multiarch)"
            echo "  --tag=TAG             Tag images with specified semver tag (default: 1.0.0)"
            echo "  --registry=REGISTRY   Push to registry: ghcr, dockerhub, local (required for multi-arch Sorbet)"
            echo "  --language=LANG       Build specific language(s) - comma-separated (python,typescript,ruby,ruby-sorbet)"
            echo "  --help, -h            Show this help message"
            echo ""
            echo "Versioning: Language containers use semver (MAJOR.MINOR.PATCH)"
            echo "  MAJOR = API/protocol compatibility version"
            echo "  MINOR = New LSP features, language server updates"
            echo "  PATCH = Bug fixes, dependency updates"
            echo ""
            echo "Multi-arch Sorbet builds require --registry because Ruby Sorbet images depend on"
            echo "Ruby base images which must be available in a registry for multi-platform builds."
            echo ""
            echo "Ruby versions built (last 1 year): 3.3.6-3.3.10, 3.4.0-3.4.7 (13 versions)"
            echo ""
            echo "Available languages: python, typescript, rust, golang, java, clangd, csharp, php, ruby, ruby-sorbet"
            echo ""
            echo "Examples:"
            echo "  $0 --language=python --multiarch --tag=1.0.0 --registry=ghcr"
            echo "  $0 --language=ruby,ruby-sorbet --tag=1.0.0"
            echo "  $0 --all-ruby-versions --jobs=8"
            exit 0
            ;;
        --sequential)
            PARALLEL=false
            ;;
        --use-cache)
            USE_CACHE=true
            ;;
        --parallel)
            PARALLEL=true
            ;;
        --all-ruby-versions)
            ALL_RUBY_VERSIONS=true
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
        --language=*|--languages=*)
            FILTER_LANGUAGES="${arg#*=}"
            ;;
        --jobs=*|-j=*)
            MAX_JOBS="${arg#*=}"
            if ! [[ "$MAX_JOBS" =~ ^[0-9]+$ ]] || [ "$MAX_JOBS" -lt 1 ]; then
                echo -e "${RED}Invalid jobs value: $MAX_JOBS. Must be a positive integer${NC}"
                exit 1
            fi
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 [--use-cache] [--sequential] [--all-ruby-versions] [--multiarch] [--load] [--tag=TAG] [--registry=REGISTRY] [--language=LANG] [--jobs=N]"
            echo ""
            echo "Options:"
            echo "  --use-cache           Enable Docker build cache (default: disabled)"
            echo "  --sequential          Build sequentially instead of parallel"
            echo "  --parallel            Build in parallel (default)"
            echo "  --jobs=N, -j=N        Max parallel builds (default: 4, prevents Docker daemon overload)"
            echo "  --all-ruby-versions   Build all Ruby versions from dockerfiles (currently 12 versions)"
            echo "  --multiarch           Build for both amd64 and arm64 (default: local platform only)"
            echo "  --load                Also build and load local platform into Docker (use with --multiarch)"
            echo "  --tag=TAG             Tag images with specified semver tag (default: 1.0.0)"
            echo "  --registry=REGISTRY   Push to registry: ghcr, dockerhub, local"
            echo "  --language=LANG       Build specific language(s) - comma-separated"
            echo "  --help, -h            Show this help message"
            echo ""
            echo "Available languages: python, typescript, rust, golang, java, clangd, csharp, php, ruby, ruby-sorbet"
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
PUSH_FLAG=""
if [ "$MULTIARCH" = true ]; then
    BUILD_CMD="docker buildx build"
    PLATFORM_FLAG="--platform linux/amd64,linux/arm64"
fi

# Set up registry configuration and authentication
REGISTRY_PREFIX=""
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

    # Validate registry requirements for multi-arch
    if [ "$MULTIARCH" = true ]; then
        echo -e "${YELLOW}Multi-arch build with registry push enabled${NC}"
        echo -e "${YELLOW}Ruby base images will be pushed to ${REGISTRY_PREFIX}${NC}"
        echo -e "${YELLOW}This allows Ruby Sorbet images to build as multi-arch${NC}"
        echo
    fi
fi

# Validate multi-arch Sorbet requirements
if [ "$MULTIARCH" = true ] && [ -z "$REGISTRY" ]; then
    echo -e "${RED}Error: Multi-arch builds require --registry for Ruby Sorbet images${NC}"
    echo -e "${YELLOW}Ruby Sorbet depends on Ruby base images which must be in a registry for multi-platform builds${NC}"
    echo -e "${YELLOW}Use: --registry=ghcr or --registry=dockerhub or --registry=local${NC}"
    exit 1
fi

# Language Dockerfiles (non-Ruby, non-base)
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
RUBY_VERSIONS=()
if [ "$ALL_RUBY_VERSIONS" = true ]; then
    # Build all Ruby versions - dynamically discover from dockerfiles/ruby/ directory
    echo -e "${YELLOW}Building ALL Ruby versions (this will take a long time)${NC}"
    if [ -d "dockerfiles/ruby" ]; then
        for dockerfile in dockerfiles/ruby/*.Dockerfile; do
            if [ -f "$dockerfile" ]; then
                # Extract version from filename (e.g., 3.4.4 from 3.4.4.Dockerfile)
                version=$(basename "$dockerfile" .Dockerfile)
                RUBY_VERSIONS+=("$version")
            fi
        done
    fi
else
    # Build commonly used Ruby versions by default
    echo -e "${YELLOW}Building main Ruby versions: ${COMMON_RUBY_VERSIONS[*]}${NC}"
    RUBY_VERSIONS=("${COMMON_RUBY_VERSIONS[@]}")
fi

# Ruby Sorbet variants (depend on ruby base images)
RUBY_SORBET_VERSIONS=()
if [ "$ALL_RUBY_VERSIONS" = true ]; then
    # Build all Sorbet versions - same versions as Ruby, from ruby-sorbet directory
    if [ -d "dockerfiles/ruby-sorbet" ]; then
        for dockerfile in dockerfiles/ruby-sorbet/*.Dockerfile; do
            if [ -f "$dockerfile" ]; then
                # Extract version from filename (e.g., 3.4.4 from 3.4.4.Dockerfile)
                version=$(basename "$dockerfile" .Dockerfile)
                RUBY_SORBET_VERSIONS+=("$version")
            fi
        done
    fi
else
    # Build commonly used Sorbet versions by default (same as main Ruby versions)
    RUBY_SORBET_VERSIONS=("${COMMON_RUBY_VERSIONS[@]}")
fi

# Filter languages if --language flag was provided
if [ -n "$FILTER_LANGUAGES" ]; then
    # Convert comma-separated list to array
    IFS=',' read -ra FILTER_ARRAY <<< "$FILTER_LANGUAGES"

    # Check which language categories to build
    BUILD_REGULAR_LANGUAGES=false
    BUILD_RUBY=false
    BUILD_RUBY_SORBET=false

    for filter in "${FILTER_ARRAY[@]}"; do
        filter=$(echo "$filter" | xargs)  # Trim whitespace
        case "$filter" in
            ruby)
                BUILD_RUBY=true
                ;;
            ruby-sorbet)
                BUILD_RUBY_SORBET=true
                ;;
            python|typescript|rust|golang|java|clangd|csharp|php)
                BUILD_REGULAR_LANGUAGES=true
                ;;
            *)
                echo -e "${RED}Invalid language: $filter${NC}"
                echo -e "${YELLOW}Available languages: python, typescript, rust, golang, java, clangd, csharp, php, ruby, ruby-sorbet${NC}"
                exit 1
                ;;
        esac
    done

    # Filter the LANGUAGES array if building specific regular languages
    if [ "$BUILD_REGULAR_LANGUAGES" = true ]; then
        FILTERED_LANGUAGES=()
        for lang in "${LANGUAGES[@]}"; do
            for filter in "${FILTER_ARRAY[@]}"; do
                filter=$(echo "$filter" | xargs)
                if [ "$lang" = "$filter" ]; then
                    FILTERED_LANGUAGES+=("$lang")
                    break
                fi
            done
        done
        LANGUAGES=("${FILTERED_LANGUAGES[@]}")
    else
        # Not building any regular languages, clear the array
        LANGUAGES=()
    fi

    # Clear Ruby arrays if not requested
    if [ "$BUILD_RUBY" = false ]; then
        RUBY_VERSIONS=()
    fi

    if [ "$BUILD_RUBY_SORBET" = false ]; then
        RUBY_SORBET_VERSIONS=()
    fi
fi

echo -e "${BLUE}=========================================${NC}"
if [ "$MULTIARCH" = true ]; then
    echo -e "${BLUE}  Building Multi-Arch Language Containers${NC}"
    echo -e "${BLUE}  Platforms: linux/amd64, linux/arm64${NC}"
else
    echo -e "${BLUE}  Building Language Server Containers (Local Platform)${NC}"
fi
echo -e "${BLUE}  Parallel: $PARALLEL (max $MAX_JOBS jobs)${NC}"
echo -e "${BLUE}  Cache: $USE_CACHE${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

build_container() {
    local lang="$1"
    local subdir="$2"  # Optional subdirectory (ruby or ruby-sorbet)
    local use_registry="${3:-false}"  # Whether to push to registry
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
    if [ -n "$subdir" ]; then
        local image_name="nuanced-lsp-${subdir}-${lang}"
    else
        local image_name="nuanced-lsp-${lang}"
    fi

    # Determine the full image tag (with or without registry prefix)
    local full_image_tag
    if [ "$use_registry" = "true" ] && [ -n "$REGISTRY_PREFIX" ]; then
        full_image_tag="${REGISTRY_PREFIX}${image_name}:${TAG}"
    else
        full_image_tag="${image_name}:${TAG}"
    fi

    echo -e "${BLUE}Building ${full_image_tag}...${NC}"

    # Build with or without push depending on use_registry flag
    local build_flags="$PLATFORM_FLAG $CACHE_FLAG"
    if [ "$use_registry" = "true" ] && [ -n "$PUSH_FLAG" ]; then
        build_flags="$build_flags $PUSH_FLAG"
    fi

    # For Sorbet builds, pass Ruby base image from registry as build arg
    local build_args=""
    if [ "$subdir" = "ruby-sorbet" ] && [ "$use_registry" = "true" ] && [ -n "$REGISTRY_PREFIX" ]; then
        local ruby_base_image="${REGISTRY_PREFIX}nuanced-lsp-ruby-${lang}:${TAG}"
        build_args="--build-arg RUBY_BASE_IMAGE=${ruby_base_image}"
    fi

    if $BUILD_CMD $build_flags $build_args -f "$dockerfile" -t "${full_image_tag}" . > "/tmp/build-${subdir}-${lang}.log" 2>&1; then
        if [ "$use_registry" = "true" ] && [ -n "$PUSH_FLAG" ]; then
            echo -e "${GREEN}✓ ${full_image_tag} built and pushed successfully${NC}"
        elif [ "$MULTIARCH" = true ]; then
            echo -e "${GREEN}✓ ${full_image_tag} built successfully (multi-arch)${NC}"
        else
            local size=$(docker images "${image_name}:${TAG}" --format "{{.Size}}")
            echo -e "${GREEN}✓ ${full_image_tag} built successfully ($size)${NC}"
        fi
        return 0
    else
        echo -e "${RED}✗ ${image_name} failed to build${NC}"
        echo -e "${YELLOW}See /tmp/build-${subdir}-${lang}.log for details${NC}"
        tail -20 "/tmp/build-${subdir}-${lang}.log"
        return 1
    fi
}

# Throttled parallel build function
# Runs builds in parallel but limits concurrency to MAX_JOBS
# Arguments:
#   $1 - subdir (empty string, "ruby", or "ruby-sorbet")
#   $2 - push flag ("true" or "false")
#   $3... - items to build
build_parallel_throttled() {
    local subdir="$1"
    local push_flag="$2"
    shift 2
    local items=("$@")

    local pids=()
    local failed=0
    local running=0

    for item in "${items[@]}"; do
        # Wait if we've hit the max concurrent jobs
        while [ $running -ge $MAX_JOBS ]; do
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
        build_container "$item" "$subdir" "$push_flag" &
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

# Build non-Ruby language images
echo -e "${YELLOW}Step 1: Building non-Ruby language containers${NC}"

# Determine if we should push language images to registry
push_languages="false"
if [ -n "$REGISTRY" ]; then
    push_languages="true"
fi

if [ "$PARALLEL" = true ]; then
    echo -e "${BLUE}Building in parallel (max $MAX_JOBS concurrent, see /tmp/build-*.log for progress)${NC}"

    if ! build_parallel_throttled "" "$push_languages" "${LANGUAGES[@]}"; then
        failed=$?
    else
        failed=0
    fi

    if [ $failed -gt 0 ]; then
        echo -e "${RED}$failed language containers failed to build${NC}"
        exit 1
    fi
else
    # Build sequentially
    for lang in "${LANGUAGES[@]}"; do
        build_container "$lang" "" "$push_languages" || exit 1
    done
fi

echo

# Build Ruby base images (must complete before Sorbet variants)
echo -e "${YELLOW}Step 2: Building Ruby base images (${#RUBY_VERSIONS[@]} versions)${NC}"

if [ ${#RUBY_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby versions found in dockerfiles/ruby/, skipping${NC}"
else
    # Determine if we should push Ruby images to registry
    push_ruby="false"
    if [ -n "$REGISTRY" ]; then
        push_ruby="true"
    fi

    if [ "$PARALLEL" = true ]; then
        echo -e "${BLUE}Building in parallel (max $MAX_JOBS concurrent, see /tmp/build-ruby-*.log for progress)${NC}"

        if ! build_parallel_throttled "ruby" "$push_ruby" "${RUBY_VERSIONS[@]}"; then
            failed=$?
        else
            failed=0
        fi

        if [ $failed -gt 0 ]; then
            echo -e "${RED}$failed Ruby containers failed to build${NC}"
            exit 1
        fi
    else
        # Build sequentially
        for version in "${RUBY_VERSIONS[@]}"; do
            build_container "$version" "ruby" "$push_ruby" || exit 1
        done
    fi

fi

echo

# Build Ruby Sorbet variants (depends on Ruby base images)
echo -e "${YELLOW}Step 3: Building Ruby Sorbet variants (${#RUBY_SORBET_VERSIONS[@]} versions)${NC}"

if [ ${#RUBY_SORBET_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby Sorbet versions found in dockerfiles/ruby-sorbet/, skipping${NC}"
else
    # Determine if we should push Sorbet images to registry
    push_sorbet="false"
    if [ -n "$REGISTRY" ]; then
        push_sorbet="true"
    fi

    if [ "$PARALLEL" = true ]; then
        echo -e "${BLUE}Building in parallel (max $MAX_JOBS concurrent, see /tmp/build-ruby-sorbet-*.log for progress)${NC}"

        if ! build_parallel_throttled "ruby-sorbet" "$push_sorbet" "${RUBY_SORBET_VERSIONS[@]}"; then
            failed=$?
        else
            failed=0
        fi

        if [ $failed -gt 0 ]; then
            echo -e "${RED}$failed Ruby Sorbet containers failed to build${NC}"
            exit 1
        fi
    else
        # Build sequentially
        for version in "${RUBY_SORBET_VERSIONS[@]}"; do
            build_container "$version" "ruby-sorbet" "$push_sorbet" || exit 1
        done
    fi
fi

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  All Language Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo

if [ -n "$PUSH_FLAG" ]; then
    # Images were pushed to registry
    echo -e "${GREEN}Images pushed to ${REGISTRY} (${REGISTRY_PREFIX}):${NC}"
    echo -e "  • $(echo "${LANGUAGES[@]}" | wc -w) language containers"
    echo -e "  • ${#RUBY_VERSIONS[@]} Ruby base images"
    echo -e "  • ${#RUBY_SORBET_VERSIONS[@]} Ruby Sorbet images"
    echo
    echo -e "${YELLOW}To verify pushed images:${NC}"
    echo -e "  docker pull ${REGISTRY_PREFIX}nuanced-lsp-python:${TAG}"
    echo -e "  docker pull ${REGISTRY_PREFIX}nuanced-lsp-ruby-3.4.4:${TAG}"
    echo -e "  docker pull ${REGISTRY_PREFIX}nuanced-lsp-ruby-sorbet-3.4.4:${TAG}"
    echo
    echo -e "${YELLOW}To list all packages in ${REGISTRY}:${NC}"
    if [ "$REGISTRY" = "ghcr" ]; then
        echo -e "  ./scripts/ghcr-utils.sh list-packages"
    fi
elif [ "$MULTIARCH" = true ]; then
    echo -e "${BLUE}Multi-arch images built and cached (not loaded into local Docker)${NC}"
    echo

    # If --load was specified, also build local platform and load it
    if [ "$LOAD_LOCAL" = true ]; then
        echo -e "${YELLOW}Also building and loading local platform images...${NC}"
        echo

        # Load non-Ruby languages
        for lang in "${LANGUAGES[@]}"; do
            dockerfile="dockerfiles/${lang}.Dockerfile"
            if [ -f "$dockerfile" ]; then
                echo -e "${BLUE}Loading nuanced-lsp-${lang}:${TAG} (local platform)...${NC}"
                docker buildx build --load $CACHE_FLAG -f "$dockerfile" -t "nuanced-lsp-${lang}:${TAG}" . > /tmp/build-${lang}-local.log 2>&1
            fi
        done

        # Load Ruby base images
        for version in "${RUBY_VERSIONS[@]}"; do
            dockerfile="dockerfiles/ruby/${version}.Dockerfile"
            if [ -f "$dockerfile" ]; then
                echo -e "${BLUE}Loading nuanced-lsp-ruby-${version}:${TAG} (local platform)...${NC}"
                docker buildx build --load $CACHE_FLAG -f "$dockerfile" -t "nuanced-lsp-ruby-${version}:${TAG}" . > /tmp/build-ruby-${version}-local.log 2>&1
            fi
        done

        # Load Ruby Sorbet variants
        for version in "${RUBY_SORBET_VERSIONS[@]}"; do
            dockerfile="dockerfiles/ruby-sorbet/${version}.Dockerfile"
            if [ -f "$dockerfile" ]; then
                echo -e "${BLUE}Loading nuanced-lsp-ruby-sorbet-${version}:${TAG} (local platform)...${NC}"
                docker buildx build --load $CACHE_FLAG -f "$dockerfile" -t "nuanced-lsp-ruby-sorbet-${version}:${TAG}" . > /tmp/build-ruby-sorbet-${version}-local.log 2>&1
            fi
        done

        echo -e "${GREEN}✓ Local platform images loaded into Docker${NC}"
        echo
        echo -e "${BLUE}Container Images (Local):${NC}"
        docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | awk '{printf "  %-40s %10s\n", $1":"$2, $7}'
        echo
    fi

    echo -e "${YELLOW}To verify multi-arch builds, use:${NC}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-<language>:${TAG}"
    echo
    echo -e "${YELLOW}To publish to registry:${NC}"
    echo -e "  $0 --multiarch --tag=${TAG} --registry=ghcr"
    echo -e "  $0 --multiarch --tag=${TAG} --registry=dockerhub"
else
    echo -e "${BLUE}Container Images (Local):${NC}"
    docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | awk '{printf "  %-40s %10s\n", $1":"$2, $7}'
    echo
    echo -e "${BLUE}Total size:${NC}"
    docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | awk '{size+=$7} END {print "  ~" size " (approximate)"}'
fi

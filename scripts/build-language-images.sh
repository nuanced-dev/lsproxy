#!/usr/bin/env bash
# Build language server containers (Python, TypeScript, Rust, Go, Java, C++, C#, PHP, Ruby variants)

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/supported-ruby-versions.sh"

DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

usage() {
    echo "Usage: $0 [--use-cache] [--sequential] [--multiarch] [--load] [--tag=TAG] [--registry=REGISTRY] [--language=LANG] [--jobs=N]"
}

help() {
    echo "Build language server containers (Python, TypeScript, Rust, Go, Java, C++, C#, PHP, Ruby variants)"
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --use-cache           Enable Docker build cache (default: disabled)"
    echo "  --sequential          Build sequentially instead of parallel (default: parallel)"
    echo "  --jobs=N, -j=N        Max parallel builds (default: 4, prevents Docker daemon overload)"
    echo "  --multiarch           Build for both amd64 and arm64 (default: local platform only)"
    echo "  --load                Also build and load local platform into Docker (use with --multiarch)"
    echo "  --language-tag=TAG    Tag images with specified semver tag (default: $DEFAULT_LANGUAGE_TAG)"
    echo "                        Language containers use semver (e.g., 1.0.0) for API compatibility"
    echo "  --registry=REGISTRY   Push to registry: ghcr, dockerhub, local (required for multi-arch Sorbet) (default: no push)"
    echo "  --language=LANG       Build specific language(s) - comma-separated (python,typescript,ruby,ruby-sorbet)"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Versioning: Language containers use semver (MAJOR.MINOR.PATCH)"
    echo "  MAJOR = API/protocol compatibility version"
    echo "  MINOR = New LSP features, language server updates"
    echo "  PATCH = Bug fixes, dependency updates"
    echo ""
    echo "Note: Multi-arch Sorbet builds require --registry because Ruby Sorbet images depend"
    echo "on Ruby base images which must be available in a registry for multi-platform builds."
    echo ""
    echo "Note: Language containers use binary injection at runtime via --volumes-from. They only"
    echo "need to be rebuilt when language server versions change or when base dependencies change."
    echo ""
    echo "Available languages: python, typescript, rust, golang, java, clangd, csharp, php, ruby, ruby-sorbet"
    echo ""
    echo "Examples:"
    echo "  $0 --language-tag=1.0.0"
    echo "  $0 --multiarch --language-tag=1.0.0 --registry=ghcr"
    echo "  $0 --jobs=8"
    echo "  $0 --language=python --multiarch --language-tag=1.0.0 --registry=ghcr"
    echo "  $0 --language=ruby,ruby-sorbet --language-tag=1.0.0"
}

# Defaults
PARALLEL=true
USE_CACHE=false
MULTIARCH=false
LOAD_LOCAL=false
LANGUAGE_TAG=""
REGISTRY=""         # Options: ghcr, dockerhub, local, or empty for no push
FILTER_LANGUAGES=   # Empty = build all, otherwise comma-separated list: python,typescript,ruby,ruby-sorbet
MAX_JOBS=4

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            help
            exit 0
            ;;
        --sequential)
            PARALLEL=false
            ;;
        --use-cache)
            USE_CACHE=true
            ;;
        --multiarch)
            MULTIARCH=true
            ;;
        --load)
            LOAD_LOCAL=true
            ;;
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
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
            usage
            exit 1
            ;;
    esac
done

# Fall back to default tags
LANGUAGE_TAG="${LANGUAGE_TAG:-$DEFAULT_LANGUAGE_TAG}"

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
            if [ -z "$DOCKER_HUB_USERNAME" ] || [ -z "$DOCKER_HUB_TOKEN" ]; then
                echo -e "${RED}Error: DOCKER_HUB_USERNAME and DOCKER_HUB_TOKEN required for Docker Hub${NC}"
                echo "Please set:"
                echo "  export DOCKER_HUB_USERNAME=your_username"
                echo "  export DOCKER_HUB_TOKEN=your_token_or_password"
                exit 1
            fi
            # Authenticate to Docker Hub
            echo -e "${BLUE}Authenticating to Docker Hub...${NC}"
            if echo "$DOCKER_HUB_TOKEN" | docker login -u "$DOCKER_HUB_USERNAME" --password-stdin > /dev/null 2>&1; then
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

# Build commonly used Ruby versions by default
RUBY_VERSIONS=("${SUPPORTED_RUBY_VERSIONS[@]}")
RUBY_SORBET_VERSIONS=("${SUPPORTED_RUBY_VERSIONS[@]}")

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

echo -e "${YELLOW}Building languages: ${LANGUAGES[*]}${NC}"
echo -e "${YELLOW}Building Ruby versions: ${RUBY_VERSIONS[*]}${NC}"
echo -e "${YELLOW}Building Sorbet versions: ${RUBY_SORBET_VERSIONS[*]}${NC}"

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

    # Determine the full image version (with or without registry prefix)
    local full_image_tag
    if [ "$use_registry" = "true" ] && [ -n "$REGISTRY_PREFIX" ]; then
        full_image_tag="${REGISTRY_PREFIX}${image_name}:${LANGUAGE_TAG}"
    else
        full_image_tag="${image_name}:${LANGUAGE_TAG}"
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
        local ruby_base_image="${REGISTRY_PREFIX}nuanced-lsp-ruby-${lang}:${LANGUAGE_TAG}"
        build_args="--build-arg RUBY_BASE_IMAGE=${ruby_base_image}"
    fi

    if $BUILD_CMD $build_flags $build_args -f "$dockerfile" -t "${full_image_tag}" . > "/tmp/build-${subdir}-${lang}.log" 2>&1; then
        if [ "$use_registry" = "true" ] && [ -n "$PUSH_FLAG" ]; then
            echo -e "${GREEN}✓ ${full_image_tag} built and pushed successfully${NC}"
        elif [ "$MULTIARCH" = true ]; then
            echo -e "${GREEN}✓ ${full_image_tag} built successfully (multi-arch)${NC}"
        else
            local size
            size=$(docker images "${image_name}:${LANGUAGE_TAG}" --format "{{.Size}}")
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
    echo -e "  docker pull ${REGISTRY_PREFIX}nuanced-lsp-python:${LANGUAGE_TAG}"
    echo -e "  docker pull ${REGISTRY_PREFIX}nuanced-lsp-ruby-3.4.4:${LANGUAGE_TAG}"
    echo -e "  docker pull ${REGISTRY_PREFIX}nuanced-lsp-ruby-sorbet-3.4.4:${LANGUAGE_TAG}"
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
                echo -e "${BLUE}Loading nuanced-lsp-${lang}:${LANGUAGE_TAG} (local platform)...${NC}"
                docker buildx build --load $CACHE_FLAG -f "$dockerfile" -t "nuanced-lsp-${lang}:${LANGUAGE_TAG}" . > "/tmp/build-${lang}-local.log" 2>&1
            fi
        done

        # Load Ruby base images
        for version in "${RUBY_VERSIONS[@]}"; do
            dockerfile="dockerfiles/ruby/${version}.Dockerfile"
            if [ -f "$dockerfile" ]; then
                echo -e "${BLUE}Loading nuanced-lsp-ruby-${version}:${LANGUAGE_TAG} (local platform)...${NC}"
                docker buildx build --load $CACHE_FLAG -f "$dockerfile" -t "nuanced-lsp-ruby-${version}:${LANGUAGE_TAG}" . > "/tmp/build-ruby-${version}-local.log" 2>&1
            fi
        done

        # Load Ruby Sorbet variants
        for version in "${RUBY_SORBET_VERSIONS[@]}"; do
            dockerfile="dockerfiles/ruby-sorbet/${version}.Dockerfile"
            if [ -f "$dockerfile" ]; then
                echo -e "${BLUE}Loading nuanced-lsp-ruby-sorbet-${version}:${LANGUAGE_TAG} (local platform)...${NC}"
                docker buildx build --load $CACHE_FLAG -f "$dockerfile" -t "nuanced-lsp-ruby-sorbet-${version}:${LANGUAGE_TAG}" . > "/tmp/build-ruby-sorbet-${version}-local.log" 2>&1
            fi
        done

        echo -e "${GREEN}✓ Local platform images loaded into Docker${NC}"
        echo
        echo -e "${BLUE}Container Images (Local):${NC}"
        docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | grep -F "$LANGUAGE_TAG" | awk '{printf "  %-40s %10s\n", $1":"$2, $7}'
        echo
    fi

    echo -e "${YELLOW}To verify multi-arch builds, use:${NC}"
    echo -e "  docker buildx imagetools inspect nuanced-lsp-<language>:${LANGUAGE_TAG}"
    echo
    echo -e "${YELLOW}To publish to registry:${NC}"
    echo -e "  $0 --multiarch --tag=${LANGUAGE_TAG} --registry=ghcr"
    echo -e "  $0 --multiarch --tag=${LANGUAGE_TAG} --registry=dockerhub"
else
    echo -e "${BLUE}Container Images (Local):${NC}"
    docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | grep -F "$LANGUAGE_TAG" | awk '{printf "  %-40s %10s\n", $1":"$2, $7}'
    echo
    echo -e "${BLUE}Total size:${NC}"
    docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | grep -F "$LANGUAGE_TAG" | awk '{size+=$7} END {print "  ~" size " (approximate)"}'
fi

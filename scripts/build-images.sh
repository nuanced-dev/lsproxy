#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"

usage() {
    echo "Usage: $0 [--all-languages] [--all-services] [--cache=MODE] [--jobs=N] [--language-tag=TAG] [--languages=LANG...] [--multi-platform] [--registry=REG] [--sequential] [--service-tag=TAG] [--services=SVC...]"
}

help() {
    echo "Build Docker images for services and language servers"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --all-languages       Build all language images (shorthand for --languages=<all>)"
    echo "  --all-services        Build all service images (shorthand for --services=<all>)"
    echo "  --cache=MODE          Docker build cache mode: none, docker, gha (default: none)"
    echo "                        - none: disable all caching"
    echo "                        - docker: use default Docker layer caching"
    echo "                        - gha: build without Docker cache to ensure fresh Docker layers,"
    echo "                               but use GitHub Actions cache backend for BuildKit cache"
    echo "                               mounts (Cargo registry and build artifacts). This gives us"
    echo "                               reproducible builds while still caching Rust compilation."
    echo "  --jobs=N, -j=N        Max parallel builds (default: 4, prevents Docker daemon overload)"
    echo "  --language-tag=TAG    Tag language images with specified semver tag (default: $DEFAULT_LANGUAGE_TAG)"
    echo "                        Language images use semver (e.g., 1.0.0) for API compatibility"
    echo "  --languages=LANG...   Build specific language(s) - comma-separated (default: none)"
    echo "                        Supports versioned Ruby: ruby-3.2.2, ruby-sorbet-3.2.2"
    echo "  --multi-platform      Build for both linux/amd64 and linux/arm64 (default: local platform only)"
    echo "  --registry=REG        Container registry where proxy expects missing service images (default: $DEFAULT_REGISTRY)"
    echo "  --sequential          Build images sequentially (shorthand for --jobs=1)"
    echo "  --service-tag=TAG     Tag service images with specified tag (default: $DEFAULT_SERVICE_TAG)"
    echo "  --services=SVC...     Build specific service(s) - comma-separated (default: none)"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "At least one of --services, --all-services, --languages, or --all-languages must be specified."
    echo ""
    echo "Versioning:"
    echo "  Service images (wrapper, proxy, watchdog) use release version tags (e.g. 0.4.0)"
    echo "  Language images use independent semver for API/protocol compatibility (e.g. 1.0.0)"
    echo "  MAJOR = API/protocol compatibility version"
    echo "  MINOR = New LSP features, language server updates"
    echo "  PATCH = Bug fixes, dependency updates"
    echo ""
    echo "Note: Language images use binary injection at runtime via --volumes-from. They only"
    echo "need to be rebuilt when language server versions change or when base dependencies change."
    echo ""
    echo "Available services: ${ALL_SERVICES[*]}"
    echo "Available languages: ${ALL_LANGUAGES[*]}"
    echo ""
    echo "Examples:"
    echo "  $0 --all-services --all-languages"
    echo "  $0 --services=proxy,watchdog --service-tag=0.4.8"
    echo "  $0 --languages=python,typescript --language-tag=1.0.0"
    echo "  $0 --multi-platform --all-services --service-tag=0.4.8"
    echo "  $0 --cache=gha --all-services --service-tag=0.4.8"
    echo "  $0 --languages=ruby-3.2.2,ruby-sorbet-3.2.2 --language-tag=1.0.0"
}

# Defaults
CACHE_MODE=none
JOBS=4
LANGUAGE_TAG=""
LANGUAGES=()
MULTIPLATFORM=false
REGISTRY=""
SERVICE_TAG=""
SERVICES=()

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            help
            exit 0
            ;;
        --all-languages)
            LANGUAGES=("${ALL_LANGUAGES[@]}")
            ;;
        --all-services)
            SERVICES=("${ALL_SERVICES[@]}")
            ;;
        --cache=*)
            CACHE_MODE="${arg#*=}"
            if [[ ! "$CACHE_MODE" =~ ^(none|docker|gha)$ ]]; then
                echo -e "${RED}Invalid cache mode: $CACHE_MODE. Must be none, docker, or gha${NC}"
                exit 1
            fi
            ;;
        --jobs=*|-j=*)
            JOBS="${arg#*=}"
            if ! [[ "$JOBS" =~ ^[0-9]+$ ]] || [ "$JOBS" -lt 1 ]; then
                echo -e "${RED}Invalid jobs value: $JOBS. Must be a positive integer${NC}"
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
            MULTIPLATFORM=true
            ;;
        --registry=*)
            REGISTRY="${arg#*=}"
            ;;
        --sequential)
            JOBS=1
            ;;
        --service-tag=*)
            SERVICE_TAG="${arg#*=}"
            ;;
        --services=*)
            IFS=',' read -ra SERVICES <<< "${arg#*=}"
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            usage
            exit 1
            ;;
    esac
done

# Check that at least one of services or languages is specified
if [ ${#SERVICES[@]} -eq 0 ] && [ ${#LANGUAGES[@]} -eq 0 ]; then
    echo -e "${RED}Error: At least one of --services, --all-services, --languages, or --all-languages must be specified${NC}"
    echo ""
    usage
    exit 1
fi

# Fall back to default tags
SERVICE_TAG="${SERVICE_TAG:-$DEFAULT_SERVICE_TAG}"
LANGUAGE_TAG="${LANGUAGE_TAG:-$DEFAULT_LANGUAGE_TAG}"
REGISTRY="${REGISTRY:-$DEFAULT_REGISTRY}"

# ---------------------------------------
# Build Commands
# ---------------------------------------

BUILD_CMD=()
if [ "$MULTIPLATFORM" = true ]; then
    BUILD_CMD=("docker" "buildx" "build" "--platform" "linux/amd64,linux/arm64")
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Multi-Arch Images${NC}"
    echo -e "${BLUE}  Platforms: linux/amd64, linux/arm64${NC}"
    if [ ${#SERVICES[@]} -gt 0 ]; then
        echo -e "${BLUE}  Service images: $SERVICE_TAG${NC}"
    fi
    if [ ${#LANGUAGES[@]} -gt 0 ]; then
        echo -e "${BLUE}  Language images: $LANGUAGE_TAG${NC}"
    fi
    echo -e "${BLUE}  Jobs: $JOBS${NC}"
    echo -e "${BLUE}  Cache: $CACHE_MODE${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
else
    BUILD_CMD=("docker" "build")
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}  Building Images (Local Platform)${NC}"
    if [ ${#SERVICES[@]} -gt 0 ]; then
        echo -e "${BLUE}  Service images: $SERVICE_TAG${NC}"
    fi
    if [ ${#LANGUAGES[@]} -gt 0 ]; then
        echo -e "${BLUE}  Language images: $LANGUAGE_TAG${NC}"
    fi
    echo -e "${BLUE}  Jobs: $JOBS${NC}"
    echo -e "${BLUE}  Cache: $CACHE_MODE${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo
fi

BUILD_CMD+=(
    "--build-arg" "SERVICE_IMAGE_VERSION=$SERVICE_TAG"
    "--build-arg" "LANGUAGE_IMAGE_VERSION=$LANGUAGE_TAG"
    "--build-arg" "CONTAINER_REGISTRY=$REGISTRY"
)

# Verify storage for multi-platform builds
if [ "$MULTIPLATFORM" = true ] && [ "$(docker system info --format json | jq '.DriverStatus | any(.[]; .[0] == "driver-type" and .[1] == "io.containerd.snapshotter.v1")')" != true ]; then
    echo -e "${RED}Error: Multi-arch builds require containerd storage so Docker can load the multi-platform images${NC}"
    exit 1
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
    local dockerfile="$1"
    local image_tag="$2"
    local cache_name="$3"

    echo -e "${BLUE}Building ${image_tag}...${NC}"

    if [ ! -f "$dockerfile" ]; then
        echo -e "${RED}Error: $dockerfile not found${NC}"
        return 1
    fi

    local CACHE_FLAGS
    CACHE_FLAGS="$(compute_cache_flags "$cache_name")"

    local log_file="/tmp/build-${cache_name}.log"

    if "${BUILD_CMD[@]}" $CACHE_FLAGS -f "$dockerfile" -t "$image_tag" . > "$log_file" 2>&1; then
        if [ "$MULTIPLATFORM" = true ]; then
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

# ---------------------------------------
# Service Images
# ---------------------------------------

build_service_image() {
    local name="$1"

    local docker_file="dockerfiles/${name}.Dockerfile"
    local image_tag="nuanced-lsp-${name}:${SERVICE_TAG}"
    local cache_name="$name"

    build_image "$docker_file" "$image_tag" "$cache_name"
}

# Build service images (proxy, watchdog, wrapper)
if [ ${#SERVICES[@]} -eq 0 ]; then
    echo -e "${YELLOW}No service images to build${NC}"
else
    echo -e "${YELLOW}Building service images: ${SERVICES[*]}${NC}"

    if [ "$JOBS" -gt 1 ]; then
        echo -e "${BLUE}Building in parallel (logs: /tmp/build-*.log)...${NC}"
        declare -a pids names

        for service in "${SERVICES[@]}"; do
            (build_service_image "$service") &
            pids+=($!)
            names+=("$service")
        done

        failed=0
        for i in "${!pids[@]}"; do
            if ! wait "${pids[$i]}"; then
                failed=1
            fi
        done

        if [ $failed -ne 0 ]; then
            echo -e "${RED}One or more service images failed to build${NC}"
            exit 1
        fi
    else
        for service in "${SERVICES[@]}"; do
            build_service_image "$service" || exit 1
        done
    fi
    echo
fi

# ---------------------------------------
# Language Images
# ---------------------------------------

build_language_image() {
    local subdir="$1"  # Optional subdirectory (ruby or ruby-sorbet)
    local lang="$2"

    local dockerfile
    if [ -n "$subdir" ]; then
        dockerfile="dockerfiles/${subdir}/${lang}.Dockerfile"
    else
        dockerfile="dockerfiles/${lang}.Dockerfile"
    fi

    # For Ruby images, the image name includes the Ruby version
    # Format: nuanced-lsp-ruby-3.4.4 or nuanced-lsp-ruby-sorbet-3.4.4
    local image_name
    if [ -n "$subdir" ]; then
        image_name="nuanced-lsp-${subdir}-${lang}"
    else
        image_name="nuanced-lsp-${lang}"
    fi

    local image_tag="${image_name}:${LANGUAGE_TAG}"

    local cache_name
    if [ -n "$subdir" ]; then
        cache_name="${subdir}-${lang}"
    else
        cache_name="${lang}"
    fi

    build_image "$dockerfile" "$image_tag" "$cache_name"
}

# Throttled parallel build function
# Runs builds in parallel but limits concurrency to JOBS
# Arguments:
#   $1 - subdir (empty string, "ruby", or "ruby-sorbet")
#   $2... - items to build
# If subdir is empty, the items are language names. Otherwise,
# the subdir is the base and the items are language versions.
build_language_images_parallel() {
    local subdir="$1"
    local items=("${@:2}")

    local pids=()
    local failed=0
    local running=0

    for item in "${items[@]}"; do
        # Wait if we've hit the max concurrent jobs
        while [ $running -ge "$JOBS" ]; do
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
        build_language_image "$subdir" "$item" &
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
UNVERSIONED_LANGUAGES=()
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
        UNVERSIONED_LANGUAGES+=("$lang")
    fi
done

# Build unversioned languages
if [ ${#UNVERSIONED_LANGUAGES[@]} -eq 0 ]; then
    echo -e "${YELLOW}No unversioned language images to build${NC}"
else
    echo -e "${YELLOW}Building ${#UNVERSIONED_LANGUAGES[@]} unversioned language images${NC}"

    if [ "$JOBS" -gt 1 ]; then
        echo -e "${BLUE}Building in parallel (max $JOBS concurrent, see /tmp/build-*.log for progress)${NC}"

        if ! build_language_images_parallel "" "${UNVERSIONED_LANGUAGES[@]}"; then
            failed=$?
        else
            failed=0
        fi

        if [ $failed -gt 0 ]; then
            echo -e "${RED}$failed unversioned language images failed to build${NC}"
            exit 1
        fi
    else
        for lang in "${UNVERSIONED_LANGUAGES[@]}"; do
            build_language_image "" "$lang" || exit 1
        done
    fi

    echo
fi

# Build Ruby images (must complete before Sorbet images)
if [ ${#RUBY_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby images to build${NC}"
else
    echo -e "${YELLOW}Building Ruby images (${#RUBY_VERSIONS[@]} versions)${NC}"

    if [ "$JOBS" -gt 1 ]; then
        echo -e "${BLUE}Building in parallel (max $JOBS concurrent, see /tmp/build-ruby-*.log for progress)${NC}"

        if ! build_language_images_parallel "ruby" "${RUBY_VERSIONS[@]}"; then
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
            build_language_image "ruby" "$version" || exit 1
        done
    fi

    echo
fi

# Build Ruby Sorbet variants (depends on Ruby base images)
if [ ${#RUBY_SORBET_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby Sorbet images to build${NC}"
else
    echo -e "${YELLOW}Building Ruby Sorbet images (${#RUBY_SORBET_VERSIONS[@]} versions)${NC}"

    if [ "$JOBS" -gt 1 ]; then
        echo -e "${BLUE}Building in parallel (max $JOBS concurrent, see /tmp/build-ruby-sorbet-*.log for progress)${NC}"

        if ! build_language_images_parallel "ruby-sorbet" "${RUBY_SORBET_VERSIONS[@]}"; then
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
            build_language_image "ruby-sorbet" "$version" || exit 1
        done
    fi

    echo
fi

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  All Images Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo

if [ ${#SERVICES[@]} -gt 0 ]; then
    echo -e "${BLUE}Service Images (Local):${NC}"
    docker images | grep "nuanced-lsp-" | grep -E "(proxy|watchdog|wrapper)" | grep -F "$SERVICE_TAG" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
    echo
fi

if [ ${#LANGUAGES[@]} -gt 0 ]; then
    echo -e "${BLUE}Language Images (Local):${NC}"
    docker images | grep "nuanced-lsp-" | grep -v -E "(proxy|watchdog|wrapper)" | grep -F "$LANGUAGE_TAG" | awk '{printf "  %-40s %10s\n", $1":"$2, $7}'
    echo
fi

if [ ${#SERVICES[@]} -gt 0 ] || [ ${#LANGUAGES[@]} -gt 0 ]; then
    echo -e "${BLUE}Total size:${NC}"
    (
        if [ ${#SERVICES[@]} -gt 0 ]; then
            docker images | grep "nuanced-lsp-" | grep -E "(proxy|watchdog|wrapper)" | grep -F "$SERVICE_TAG"
        fi
        if [ ${#LANGUAGES[@]} -gt 0 ]; then
            docker images | grep "nuanced-lsp-" | grep -v -E "(proxy|watchdog|wrapper)" | grep -F "$LANGUAGE_TAG"
        fi
    ) | awk '{size+=$7} END {print "  ~" size " (approximate)"}'
    echo
fi

if [ "$MULTIPLATFORM" = true ]; then
    echo
    echo -e "${BLUE}Multi-platform images built and cached${NC}"
    echo
    echo -e "${YELLOW}To verify multi-platform builds:${NC}"
    if [ ${#SERVICES[@]} -gt 0 ]; then
        for service in "${SERVICES[@]}"; do
            echo -e "  docker buildx imagetools inspect nuanced-lsp-${service}:${SERVICE_TAG}"
        done
    fi
    if [ ${#LANGUAGES[@]} -gt 0 ]; then
        echo -e "  docker buildx imagetools inspect nuanced-lsp-<language>:${LANGUAGE_TAG}"
    fi
    echo
    echo -e "${YELLOW}To publish multi-platform images to registries:${NC}"
    echo -e "  $(dirname "$0")/publish-images.sh"
fi

#!/usr/bin/env bash

set -e

# Build language server containers (Python, TypeScript, Rust, Go, Java, C++, C#, PHP, Ruby variants)
# Usage: ./scripts/build-language-containers.sh [--use-cache] [--sequential] [--all-ruby-versions] [--multiarch] [--load] [--tag=TAG]
#
# By default:
#   - Builds WITHOUT cache (use --use-cache to enable caching)
#   - Builds in PARALLEL (use --sequential for sequential builds)
#   - Builds ONLY main Ruby versions (use --all-ruby-versions to build all 110 versions)
#   - Builds for local platform only (use --multiarch for amd64+arm64)
#   - Tags images as :1.0.0 (use --tag=1.1.0 for custom version)
#
# Versioning:
#   Language containers use semantic versioning (MAJOR.MINOR.PATCH):
#   - MAJOR: API/protocol compatibility version (increment for breaking changes)
#   - MINOR: New LSP features, language server version updates
#   - PATCH: Bug fixes, dependency updates
#
# Options:
#   --multiarch           Build for both linux/amd64 and linux/arm64
#   --load                Also build and load local platform into Docker (use with --multiarch)
#   --use-cache           Enable Docker build cache
#   --tag=TAG             Tag images with specified semver tag (default: 1.0.0)
#   --all-ruby-versions   Build all 110+ Ruby versions
#   --sequential          Build sequentially instead of parallel
#
# Note: Language containers use binary injection at runtime via --volumes-from lsproxy-wrapper
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
DEFAULT_RUBY_VERSION="3.4.4"
# Main Ruby versions that are commonly used (built by default)
COMMON_RUBY_VERSIONS=("3.2.2" "3.2.6" "3.3.5" "3.3.6" "3.4.1" "3.4.2" "3.4.4")

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            echo "Usage: $0 [--use-cache] [--sequential] [--all-ruby-versions] [--multiarch] [--load] [--tag=TAG]"
            echo ""
            echo "Options:"
            echo "  --use-cache           Enable Docker build cache (default: disabled)"
            echo "  --sequential          Build sequentially instead of parallel"
            echo "  --parallel            Build in parallel (default)"
            echo "  --all-ruby-versions   Build all 114 Ruby versions (default: main versions only)"
            echo "  --multiarch           Build for both amd64 and arm64 (default: local platform only)"
            echo "  --load                Also build and load local platform into Docker (use with --multiarch)"
            echo "  --tag=TAG             Tag images with specified semver tag (default: 1.0.0)"
            echo "  --help, -h            Show this help message"
            echo ""
            echo "Versioning: Language containers use semver (MAJOR.MINOR.PATCH)"
            echo "  MAJOR = API/protocol compatibility version"
            echo "  MINOR = New LSP features, language server updates"
            echo "  PATCH = Bug fixes, dependency updates"
            echo ""
            echo "Default Ruby versions built: 3.2.2, 3.2.6, 3.3.5, 3.3.6, 3.4.1, 3.4.2, 3.4.4"
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
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 [--use-cache] [--sequential] [--all-ruby-versions] [--multiarch] [--load] [--tag=TAG]"
            echo ""
            echo "Options:"
            echo "  --use-cache           Enable Docker build cache (default: disabled)"
            echo "  --sequential          Build sequentially instead of parallel"
            echo "  --parallel            Build in parallel (default)"
            echo "  --all-ruby-versions   Build all 114 Ruby versions (default: main versions only)"
            echo "  --multiarch           Build for both amd64 and arm64 (default: local platform only)"
            echo "  --load                Also build and load local platform into Docker (use with --multiarch)"
            echo "  --tag=TAG             Tag images with specified semver tag (default: 1.0.0)"
            echo "  --help, -h            Show this help message"
            echo ""
            echo "Versioning: Language containers use semver (MAJOR.MINOR.PATCH)"
            echo "  MAJOR = API/protocol compatibility version"
            echo "  MINOR = New LSP features, language server updates"
            echo "  PATCH = Bug fixes, dependency updates"
            echo ""
            echo "Default Ruby versions built: 3.2.2, 3.2.6, 3.3.5, 3.3.6, 3.4.1, 3.4.2, 3.4.4"
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
if [ "$MULTIARCH" = true ]; then
    BUILD_CMD="docker buildx build"
    PLATFORM_FLAG="--platform linux/amd64,linux/arm64"
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

echo -e "${BLUE}=========================================${NC}"
if [ "$MULTIARCH" = true ]; then
    echo -e "${BLUE}  Building Multi-Arch Language Containers${NC}"
    echo -e "${BLUE}  Platforms: linux/amd64, linux/arm64${NC}"
else
    echo -e "${BLUE}  Building Language Server Containers (Local Platform)${NC}"
fi
echo -e "${BLUE}  Parallel: $PARALLEL${NC}"
echo -e "${BLUE}  Cache: $USE_CACHE${NC}"
echo -e "${BLUE}=========================================${NC}"
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

    # For Ruby images, the image name includes the Ruby version
    # Format: nuanced-lsp-ruby-3.4.4 or nuanced-lsp-ruby-sorbet-3.4.4
    if [ -n "$subdir" ]; then
        local image_name="nuanced-lsp-${subdir}-${lang}"
    else
        local image_name="nuanced-lsp-${lang}"
    fi

    echo -e "${BLUE}Building ${image_name}:${TAG}...${NC}"

    if $BUILD_CMD $PLATFORM_FLAG $CACHE_FLAG -f "$dockerfile" -t "${image_name}:${TAG}" . > "/tmp/build-${subdir}-${lang}.log" 2>&1; then
        if [ "$MULTIARCH" = true ]; then
            echo -e "${GREEN}✓ ${image_name}:${TAG} built successfully (multi-arch)${NC}"
        else
            local size=$(docker images "${image_name}:${TAG}" --format "{{.Size}}")
            echo -e "${GREEN}✓ ${image_name}:${TAG} built successfully ($size)${NC}"
        fi
        return 0
    else
        echo -e "${RED}✗ ${image_name} failed to build${NC}"
        echo -e "${YELLOW}See /tmp/build-${subdir}-${lang}.log for details${NC}"
        tail -20 "/tmp/build-${subdir}-${lang}.log"
        return 1
    fi
}

# Build non-Ruby language images
echo -e "${YELLOW}Step 1: Building non-Ruby language containers${NC}"

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
echo -e "${YELLOW}Step 2: Building Ruby base images (${#RUBY_VERSIONS[@]} versions)${NC}"

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

    # For multi-arch builds, we need to load Ruby base images so Sorbet variants can use them
    if [ "$MULTIARCH" = true ]; then
        echo
        echo -e "${YELLOW}Loading Ruby base images for local buildx access...${NC}"
        for version in "${RUBY_VERSIONS[@]}"; do
            echo -e "${BLUE}Loading nuanced-lsp-ruby-${version}:${TAG}...${NC}"
            docker buildx build --load $CACHE_FLAG -f "dockerfiles/ruby/${version}.Dockerfile" -t "nuanced-lsp-ruby-${version}:${TAG}" . > /tmp/build-ruby-${version}-load.log 2>&1 || true
        done
        echo -e "${GREEN}✓ Ruby base images loaded${NC}"
    fi
fi

echo

# Build Ruby Sorbet variants (depends on Ruby base images)
echo -e "${YELLOW}Step 3: Building Ruby Sorbet variants (${#RUBY_SORBET_VERSIONS[@]} versions)${NC}"

if [ ${#RUBY_SORBET_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby Sorbet versions found in dockerfiles/ruby-sorbet/, skipping${NC}"
else
    # Save original build settings
    ORIGINAL_BUILD_CMD="$BUILD_CMD"
    ORIGINAL_PLATFORM_FLAG="$PLATFORM_FLAG"

    # For Sorbet variants, always build for local platform only (even if --multiarch was specified)
    # This is because they depend on Ruby base images which must be available locally
    if [ "$MULTIARCH" = true ]; then
        echo -e "${YELLOW}Note: Building Sorbet variants for local platform only (base image dependency)${NC}"
        BUILD_CMD="docker build"
        PLATFORM_FLAG=""
    fi

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

    # Restore original build settings
    BUILD_CMD="$ORIGINAL_BUILD_CMD"
    PLATFORM_FLAG="$ORIGINAL_PLATFORM_FLAG"
fi

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  All Language Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo

if [ "$MULTIARCH" = true ]; then
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
    echo -e "${YELLOW}To publish:${NC}"
    echo -e "  ./scripts/publish-images.sh <version> [options]"
else
    echo -e "${BLUE}Container Images (Local):${NC}"
    docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | awk '{printf "  %-40s %10s\n", $1":"$2, $7}'
    echo
    echo -e "${BLUE}Total size:${NC}"
    docker images | grep "nuanced-lsp-" | grep -v -E "(wrapper|proxy|watchdog)" | awk '{size+=$7} END {print "  ~" size " (approximate)"}'
fi

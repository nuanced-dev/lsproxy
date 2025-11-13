#!/bin/bash

set -e

# Build language server containers (Python, TypeScript, Rust, Go, Java, C++, C#, PHP, Ruby variants)
# Usage: ./scripts/build-language-containers.sh [--use-cache] [--sequential] [--all-ruby-versions]
#
# By default:
#   - Builds WITHOUT cache (use --use-cache to enable caching)
#   - Builds in PARALLEL (use --sequential for sequential builds)
#   - Builds ONLY Ruby 3.4.4 (use --all-ruby-versions to build all 110 versions)
#
# Note: Language containers use binary injection at runtime via --volumes-from lsproxy-wrapper
# They only need to be rebuilt when language server versions change or when base dependencies change

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default: parallel builds without cache, only default Ruby version
PARALLEL=true
USE_CACHE=false
ALL_RUBY_VERSIONS=false
DEFAULT_RUBY_VERSION="3.4.4"

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            echo "Usage: $0 [--use-cache] [--sequential] [--all-ruby-versions]"
            echo ""
            echo "Options:"
            echo "  --use-cache          Enable Docker build cache (default: disabled)"
            echo "  --sequential         Build sequentially instead of parallel"
            echo "  --parallel           Build in parallel (default)"
            echo "  --all-ruby-versions  Build all 110 Ruby versions (default: only 3.4.4)"
            echo "  --help, -h           Show this help message"
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
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 [--use-cache] [--sequential] [--all-ruby-versions]"
            echo ""
            echo "Options:"
            echo "  --use-cache          Enable Docker build cache (default: disabled)"
            echo "  --sequential         Build sequentially instead of parallel"
            echo "  --parallel           Build in parallel (default)"
            echo "  --all-ruby-versions  Build all 110 Ruby versions (default: only 3.4.4)"
            echo "  --help, -h           Show this help message"
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
    # Build only default Ruby version
    RUBY_VERSIONS=("$DEFAULT_RUBY_VERSION")
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
    # Build only default Sorbet version
    RUBY_SORBET_VERSIONS=("$DEFAULT_RUBY_VERSION")
fi

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Building Language Server Containers${NC}"
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
fi

echo

# Build Ruby Sorbet variants (depends on Ruby base images)
echo -e "${YELLOW}Step 3: Building Ruby Sorbet variants (${#RUBY_SORBET_VERSIONS[@]} versions)${NC}"

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
echo -e "${GREEN}  All Language Containers Built Successfully${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}Container Images:${NC}"
docker images | grep "lsproxy-" | grep -v -E "(wrapper|service|watchdog)" | awk '{printf "  %-30s %10s\n", $1":"$2, $7}'
echo
echo -e "${BLUE}Total size:${NC}"
docker images | grep "lsproxy-" | grep -v -E "(wrapper|service|watchdog)" | awk '{size+=$7} END {print "  ~" size " (approximate)"}'

#!/usr/bin/env bash

set -e

# Publish Docker images to container registries (ghcr.io and/or Docker Hub)
# Usage: ./scripts/publish-images.sh <rust-version> [--language-tag=TAG] [--dry-run] [--all-ruby-versions] [--registry=REGISTRY]
#
# Example: ./scripts/publish-images.sh 0.4.0
# Example: ./scripts/publish-images.sh 0.4.0 --language-tag=1.0.0
# Example: ./scripts/publish-images.sh 0.4.0 --language-tag=1.0.0 --registry=both
#
# Versioning:
#   Rust containers (wrapper, proxy, watchdog) use release version tags (e.g., 0.4.0)
#   Language containers use independent semver tags (e.g., 1.0.0)
#   This allows language containers to guarantee API compatibility with Rust containers
#
# Requirements:
#   - GITHUB_TOKEN environment variable must be set with ghcr.io push permissions (if publishing to ghcr)
#   - DOCKER_HUB_TOKEN environment variable must be set (if publishing to dockerhub)
#   - Images must already be built (use scripts/build-rust-images.sh and scripts/build-language-images.sh)
#   - Images must be multi-arch builds (built with --multiarch flag)

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default settings
DRY_RUN=false
ALL_RUBY_VERSIONS=false
REGISTRY_TARGET="both"  # Options: ghcr, dockerhub, both
LANGUAGE_TAG=""  # If not specified, defaults to same as RUST_VERSION
COMMON_RUBY_VERSIONS=("3.2.2" "3.2.6" "3.3.5" "3.3.6" "3.4.1" "3.4.2" "3.4.4")

# Parse arguments
RUST_VERSION=""
for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            ;;
        --all-ruby-versions)
            ALL_RUBY_VERSIONS=true
            ;;
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --registry=*)
            REGISTRY_TARGET="${arg#*=}"
            if [[ ! "$REGISTRY_TARGET" =~ ^(ghcr|dockerhub|both)$ ]]; then
                echo -e "${YELLOW}Invalid registry: $REGISTRY_TARGET. Must be ghcr, dockerhub, or both${NC}"
                exit 1
            fi
            ;;
        --help|-h)
            echo "Usage: $0 <rust-version> [--language-tag=TAG] [--dry-run] [--all-ruby-versions] [--registry=REGISTRY]"
            echo ""
            echo "Arguments:"
            echo "  <rust-version>        Version tag for Rust containers (e.g., 0.4.0)"
            echo ""
            echo "Options:"
            echo "  --language-tag=TAG    Version tag for language containers (default: same as rust-version)"
            echo "                        Language containers use semver (e.g., 1.0.0) for API compatibility"
            echo "  --dry-run             Show what would be pushed without actually pushing"
            echo "  --all-ruby-versions   Publish all Ruby versions (default: main versions only)"
            echo "  --registry=REGISTRY   Target registry: ghcr, dockerhub, or both (default: both)"
            echo "  --help, -h            Show this help message"
            echo ""
            echo "Versioning:"
            echo "  Rust containers (wrapper, proxy, watchdog) version with release tags"
            echo "  Language containers use independent semver for API/protocol compatibility"
            echo ""
            echo "Environment:"
            echo "  GITHUB_TOKEN          Required for authentication to ghcr.io (if using ghcr or both)"
            echo "  DOCKER_HUB_TOKEN      Required for authentication to Docker Hub (if using dockerhub or both)"
            exit 0
            ;;
        -*)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            echo "Usage: $0 <rust-version> [--language-tag=TAG] [--dry-run] [--all-ruby-versions] [--registry=REGISTRY]"
            exit 1
            ;;
        *)
            if [ -z "$RUST_VERSION" ]; then
                RUST_VERSION="$arg"
            else
                echo -e "${YELLOW}Unexpected argument: $arg${NC}"
                echo "Usage: $0 <rust-version> [--language-tag=TAG] [--dry-run] [--all-ruby-versions] [--registry=REGISTRY]"
                exit 1
            fi
            ;;
    esac
done

# Validate rust version argument
if [ -z "$RUST_VERSION" ]; then
    echo -e "${RED}Error: Rust version argument is required${NC}"
    echo "Usage: $0 <rust-version> [--language-tag=TAG] [--dry-run] [--all-ruby-versions] [--registry=REGISTRY]"
    exit 1
fi

# If language tag not specified, use same as Rust version
if [ -z "$LANGUAGE_TAG" ]; then
    LANGUAGE_TAG="$RUST_VERSION"
    echo -e "${YELLOW}Note: Using same version tag for language containers: $LANGUAGE_TAG${NC}"
    echo -e "${YELLOW}      Consider using --language-tag=1.0.0 for independent semver${NC}"
    echo
fi

# Validate rust version format (should be semver: X.Y.Z)
if ! [[ "$RUST_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${YELLOW}Warning: Rust version '$RUST_VERSION' does not follow semver format (X.Y.Z)${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check for required tokens
if [ "$DRY_RUN" = false ]; then
    if [[ "$REGISTRY_TARGET" =~ ^(ghcr|both)$ ]] && [ -z "$GITHUB_TOKEN" ]; then
        echo -e "${RED}Error: GITHUB_TOKEN environment variable is not set${NC}"
        echo "Please set GITHUB_TOKEN with ghcr.io push permissions"
        exit 1
    fi
    if [[ "$REGISTRY_TARGET" =~ ^(dockerhub|both)$ ]] && [ -z "$DOCKER_HUB_TOKEN" ]; then
        echo -e "${RED}Error: DOCKER_HUB_TOKEN environment variable is not set${NC}"
        echo "Please set DOCKER_HUB_TOKEN for Docker Hub authentication"
        exit 1
    fi
fi

# Registry configuration
GHCR_REGISTRY="ghcr.io/nuanced-dev"
DOCKERHUB_REGISTRY="nuanced"

# Determine which registries to publish to
PUBLISH_TO_GHCR=false
PUBLISH_TO_DOCKERHUB=false
if [[ "$REGISTRY_TARGET" =~ ^(ghcr|both)$ ]]; then
    PUBLISH_TO_GHCR=true
fi
if [[ "$REGISTRY_TARGET" =~ ^(dockerhub|both)$ ]]; then
    PUBLISH_TO_DOCKERHUB=true
fi

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Publishing Images${NC}"
echo -e "${BLUE}  Rust containers: $RUST_VERSION${NC}"
echo -e "${BLUE}  Language containers: $LANGUAGE_TAG${NC}"
if [ "$PUBLISH_TO_GHCR" = true ]; then
    echo -e "${BLUE}  GHCR: $GHCR_REGISTRY${NC}"
fi
if [ "$PUBLISH_TO_DOCKERHUB" = true ]; then
    echo -e "${BLUE}  Docker Hub: $DOCKERHUB_REGISTRY${NC}"
fi
echo -e "${BLUE}  Dry Run: $DRY_RUN${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Authenticate with registries
if [ "$DRY_RUN" = false ]; then
    if [ "$PUBLISH_TO_GHCR" = true ]; then
        echo -e "${YELLOW}Authenticating with GHCR...${NC}"
        echo "$GITHUB_TOKEN" | docker login ghcr.io -u USERNAME --password-stdin
        echo -e "${GREEN}✓ Authenticated with GHCR${NC}"
    fi
    if [ "$PUBLISH_TO_DOCKERHUB" = true ]; then
        echo -e "${YELLOW}Authenticating with Docker Hub...${NC}"
        echo "$DOCKER_HUB_TOKEN" | docker login -u nuanced --password-stdin
        echo -e "${GREEN}✓ Authenticated with Docker Hub${NC}"
    fi
    echo
fi

# Function to tag and push an image
publish_image() {
    local local_image="$1"
    local remote_base="$2"
    local version="$3"

    echo -e "${BLUE}Publishing ${local_image}...${NC}"

    # Check if local image exists with the version tag first, then try :latest
    local source_tag=""
    if docker image inspect "${local_image}:${version}" > /dev/null 2>&1; then
        source_tag="${local_image}:${version}"
    elif docker image inspect "${local_image}:latest" > /dev/null 2>&1; then
        source_tag="${local_image}:latest"
    else
        echo -e "${RED}✗ Local image ${local_image} not found with :${version} or :latest tag. Please build it first.${NC}"
        return 1
    fi

    # Publish to GHCR if enabled
    if [ "$PUBLISH_TO_GHCR" = true ]; then
        local ghcr_version_tag="${GHCR_REGISTRY}/${remote_base}:${version}"
        local ghcr_latest_tag="${GHCR_REGISTRY}/${remote_base}:latest"

        if [ "$DRY_RUN" = true ]; then
            echo -e "${YELLOW}[DRY RUN] Would tag: ${source_tag} → ${ghcr_version_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would tag: ${source_tag} → ${ghcr_latest_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${ghcr_version_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${ghcr_latest_tag}${NC}"
        else
            docker tag "$source_tag" "$ghcr_version_tag"
            docker tag "$source_tag" "$ghcr_latest_tag"

            docker push "$ghcr_version_tag"
            docker push "$ghcr_latest_tag"

            echo -e "${GREEN}✓ Published to GHCR: ${remote_base}:${version} and :latest${NC}"
        fi
    fi

    # Publish to Docker Hub if enabled
    if [ "$PUBLISH_TO_DOCKERHUB" = true ]; then
        local dockerhub_version_tag="${DOCKERHUB_REGISTRY}/${remote_base}:${version}"
        local dockerhub_latest_tag="${DOCKERHUB_REGISTRY}/${remote_base}:latest"

        if [ "$DRY_RUN" = true ]; then
            echo -e "${YELLOW}[DRY RUN] Would tag: ${source_tag} → ${dockerhub_version_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would tag: ${source_tag} → ${dockerhub_latest_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${dockerhub_version_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${dockerhub_latest_tag}${NC}"
        else
            docker tag "$source_tag" "$dockerhub_version_tag"
            docker tag "$source_tag" "$dockerhub_latest_tag"

            docker push "$dockerhub_version_tag"
            docker push "$dockerhub_latest_tag"

            echo -e "${GREEN}✓ Published to Docker Hub: ${remote_base}:${version} and :latest${NC}"
        fi
    fi

    return 0
}

# Core Rust containers (use RUST_VERSION)
echo -e "${YELLOW}Step 1: Publishing Rust containers (version: $RUST_VERSION)${NC}"
echo

RUST_CONTAINERS=(
    "nuanced-lsp-wrapper"
    "nuanced-lsp-proxy"
    "nuanced-lsp-watchdog"
)

failed=0
for container in "${RUST_CONTAINERS[@]}"; do
    publish_image "$container" "$container" "$RUST_VERSION" || failed=$((failed + 1))
done

if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed Rust containers failed to publish${NC}"
    exit 1
fi

echo

# Language containers (non-Ruby) - use LANGUAGE_TAG
echo -e "${YELLOW}Step 2: Publishing language containers (version: $LANGUAGE_TAG)${NC}"
echo

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

failed=0
for lang in "${LANGUAGES[@]}"; do
    publish_image "nuanced-lsp-${lang}" "nuanced-lsp-${lang}" "$LANGUAGE_TAG" || failed=$((failed + 1))
done

if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed language containers failed to publish${NC}"
    exit 1
fi

echo

# Determine Ruby versions to publish (use LANGUAGE_TAG)
RUBY_VERSIONS=()
if [ "$ALL_RUBY_VERSIONS" = true ]; then
    echo -e "${YELLOW}Step 3: Publishing ALL Ruby versions (version: $LANGUAGE_TAG)${NC}"
    if [ -d "dockerfiles/ruby" ]; then
        for dockerfile in dockerfiles/ruby/*.Dockerfile; do
            if [ -f "$dockerfile" ]; then
                version=$(basename "$dockerfile" .Dockerfile)
                RUBY_VERSIONS+=("$version")
            fi
        done
    fi
else
    echo -e "${YELLOW}Step 3: Publishing main Ruby versions (version: $LANGUAGE_TAG)${NC}"
    RUBY_VERSIONS=("${COMMON_RUBY_VERSIONS[@]}")
fi
echo

failed=0
for ruby_version in "${RUBY_VERSIONS[@]}"; do
    publish_image "nuanced-lsp-ruby-${ruby_version}" "nuanced-lsp-ruby-${ruby_version}" "$LANGUAGE_TAG" || failed=$((failed + 1))
done

if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed Ruby containers failed to publish${NC}"
    exit 1
fi

echo

# Ruby Sorbet variants (use LANGUAGE_TAG)
RUBY_SORBET_VERSIONS=()
if [ "$ALL_RUBY_VERSIONS" = true ]; then
    echo -e "${YELLOW}Step 4: Publishing ALL Ruby Sorbet versions (version: $LANGUAGE_TAG)${NC}"
    if [ -d "dockerfiles/ruby-sorbet" ]; then
        for dockerfile in dockerfiles/ruby-sorbet/*.Dockerfile; do
            if [ -f "$dockerfile" ]; then
                version=$(basename "$dockerfile" .Dockerfile)
                RUBY_SORBET_VERSIONS+=("$version")
            fi
        done
    fi
else
    echo -e "${YELLOW}Step 4: Publishing main Ruby Sorbet versions (version: $LANGUAGE_TAG)${NC}"
    RUBY_SORBET_VERSIONS=("${COMMON_RUBY_VERSIONS[@]}")
fi
echo

failed=0
for ruby_version in "${RUBY_SORBET_VERSIONS[@]}"; do
    publish_image "nuanced-lsp-ruby-sorbet-${ruby_version}" "nuanced-lsp-ruby-sorbet-${ruby_version}" "$LANGUAGE_TAG" || failed=$((failed + 1))
done

if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed Ruby Sorbet containers failed to publish${NC}"
    exit 1
fi

echo
echo -e "${GREEN}=========================================${NC}"
if [ "$DRY_RUN" = true ]; then
    echo -e "${GREEN}  Dry Run Complete${NC}"
    echo -e "${GREEN}  No images were actually pushed${NC}"
else
    echo -e "${GREEN}  All Images Published Successfully${NC}"
    echo -e "${GREEN}  Rust containers: $RUST_VERSION${NC}"
    echo -e "${GREEN}  Language containers: $LANGUAGE_TAG${NC}"
fi
echo -e "${GREEN}=========================================${NC}"
echo

if [ "$DRY_RUN" = false ]; then
    echo -e "${BLUE}Published images:${NC}"

    # List all images for each enabled registry
    for registry in $([ "$PUBLISH_TO_GHCR" = true ] && echo "$GHCR_REGISTRY") $([ "$PUBLISH_TO_DOCKERHUB" = true ] && echo "$DOCKERHUB_REGISTRY"); do
        if [ -n "$registry" ]; then
            echo -e "${YELLOW}$registry:${NC}"
            echo -e "  ${registry}/nuanced-lsp-wrapper:${RUST_VERSION}"
            echo -e "  ${registry}/nuanced-lsp-proxy:${RUST_VERSION}"
            echo -e "  ${registry}/nuanced-lsp-watchdog:${RUST_VERSION}"
            for lang in "${LANGUAGES[@]}"; do
                echo -e "  ${registry}/nuanced-lsp-${lang}:${LANGUAGE_TAG}"
            done
            for ruby_version in "${RUBY_VERSIONS[@]}"; do
                echo -e "  ${registry}/nuanced-lsp-ruby-${ruby_version}:${LANGUAGE_TAG}"
            done
            for ruby_version in "${RUBY_SORBET_VERSIONS[@]}"; do
                echo -e "  ${registry}/nuanced-lsp-ruby-sorbet-${ruby_version}:${LANGUAGE_TAG}"
            done
            echo
        fi
    done
fi

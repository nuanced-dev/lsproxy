#!/usr/bin/env bash

set -e

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

# Publish Docker images to container registries (ghcr.io and/or Docker Hub)
# Usage: ./scripts/publish-images.sh <rust-version> [--language-tag=TAG] [--dry-run] [--registry=REGISTRY]
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

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/supported-ruby-versions.sh"

DEFAULT_RUST_TAG="$("$SCRIPT_DIR/util/rust-image-version.sh")"
DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

usage() {
    echo "Usage: $0 <rust-version> [--language-tag=TAG] [--dry-run] [--registry=REGISTRY]"
}

help() {
    usage
    echo ""
    echo "Arguments:"
    echo "  <rust-version>        Version tag for Rust containers (e.g., 0.4.0)"
    echo ""
    echo "Options:"
    echo "  --tag=TAG             Tag images with specified tag (default: $DEFAULT_RUST_TAG)"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "                        Language containers use semver (e.g., 1.0.0) for API compatibility"
    echo "  --dry-run             Show what would be pushed without actually pushing"
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
}

# Default settings
DRY_RUN=false
REGISTRY_TARGET="both"  # Options: ghcr, dockerhub, both
RUST_TAG="$DEFAULT_RUST_TAG"
LANGUAGE_TAG="$DEFAULT_LANGUAGE_TAG"

# Parse arguments
for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            ;;
        --tag=*)
            RUST_TAG="${arg#*=}"
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
            help
            exit 0
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            usage
            exit 1
            ;;
    esac
done

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
echo -e "${BLUE}  Rust containers: $RUST_TAG${NC}"
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

    local source_tag=""
    if docker image inspect "${local_image}:${version}" > /dev/null 2>&1; then
        source_tag="${local_image}:${version}"
    else
        echo -e "${RED}✗ Local image ${local_image} not found with :${version} tag. Please build it first.${NC}"
        return 1
    fi

    # Publish to GHCR if enabled
    if [ "$PUBLISH_TO_GHCR" = true ]; then
        local ghcr_version_tag="${GHCR_REGISTRY}/${remote_base}:${version}"

        if [ "$DRY_RUN" = true ]; then
            echo -e "${YELLOW}[DRY RUN] Would tag: ${source_tag} → ${ghcr_version_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${ghcr_version_tag}${NC}"
        else
            docker tag "$source_tag" "$ghcr_version_tag"

            docker push "$ghcr_version_tag"

            echo -e "${GREEN}✓ Published to GHCR: ${remote_base}:${version}${NC}"
        fi
    fi

    # Publish to Docker Hub if enabled
    if [ "$PUBLISH_TO_DOCKERHUB" = true ]; then
        local dockerhub_version_tag="${DOCKERHUB_REGISTRY}/${remote_base}:${version}"

        if [ "$DRY_RUN" = true ]; then
            echo -e "${YELLOW}[DRY RUN] Would tag: ${source_tag} → ${dockerhub_version_tag}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${dockerhub_version_tag}${NC}"
        else
            docker tag "$source_tag" "$dockerhub_version_tag"

            docker push "$dockerhub_version_tag"

            echo -e "${GREEN}✓ Published to Docker Hub: ${remote_base}:${version}${NC}"
        fi
    fi

    return 0
}

# Core Rust containers (use RUST_TAG)
echo -e "${YELLOW}Step 1: Publishing Rust containers (version: $RUST_TAG)${NC}"
echo

RUST_CONTAINERS=(
    "nuanced-lsp-wrapper"
    "nuanced-lsp-proxy"
    "nuanced-lsp-watchdog"
)

failed=0
for container in "${RUST_CONTAINERS[@]}"; do
    publish_image "$container" "$container" "$RUST_TAG" || failed=$((failed + 1))
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
echo -e "${YELLOW}Step 3: Publishing supported Ruby versions (version: $LANGUAGE_TAG)${NC}"
RUBY_VERSIONS=("${SUPPORTED_RUBY_VERSIONS[@]}")
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
echo -e "${YELLOW}Step 4: Publishing supported Ruby Sorbet versions (version: $LANGUAGE_TAG)${NC}"
RUBY_SORBET_VERSIONS=("${SUPPORTED_RUBY_VERSIONS[@]}")
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
    echo -e "${GREEN}  Rust containers: $RUST_TAG${NC}"
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
            echo -e "  ${registry}/nuanced-lsp-wrapper:${RUST_TAG}"
            echo -e "  ${registry}/nuanced-lsp-proxy:${RUST_TAG}"
            echo -e "  ${registry}/nuanced-lsp-watchdog:${RUST_TAG}"
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

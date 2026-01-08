#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/supported-languages.sh"
source "$SCRIPT_DIR/include/supported-ruby-versions.sh"

DEFAULT_RUST_TAG="$("$SCRIPT_DIR/util/rust-image-version.sh")"
DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

usage() {
    echo "Usage: $0 [--dry-run] [--language-tag=TAG] [--languages=LANG...] [--registry=REGISTRY] [--rust-tag=TAG]"
}

help() {
    echo "Publish Docker images to container registries (ghcr.io and/or Docker Hub)"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --dry-run, -N         Show what would be pushed without actually pushing"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "  --languages=LANG...   Comma-separated list of languages (default: all languages)"
    echo "                        Use empty value (--languages=) for no languages"
    echo "                        Supports versioned Ruby: ruby-3.2.2, ruby-sorbet-3.2.2"
    echo "  --registry=REGISTRY   Target registry: ghcr, dockerhub, or both (default: both)"
    echo "  --rust-tag=TAG        Tag of Rust images to use (default: $DEFAULT_RUST_TAG)"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Images:"
    echo "  - Images must already be built (use scripts/build-rust-images.sh and scripts/build-language-images.sh)"
    echo "  - Images must be multi-platform builds (built with --multi-platform flag)"
    echo ""
    echo "Versioning:"
    echo "  Rust images (wrapper, proxy, watchdog) use release version tags (e.g. 0.4.0)"
    echo "  Language images use independent semver for API/protocol compatibility (e.g. 1.0.0)"
    echo "  This allows language images to guarantee API compatibility with Rust images"
    echo ""
    echo "Environment:"
    echo "  GITHUB_TOKEN          Required for authentication to ghcr.io (if using ghcr or both)"
    echo "  DOCKER_HUB_TOKEN      Required for authentication to Docker Hub (if using dockerhub or both)"
    echo ""
    echo "Examples:"
    echo "  $0"
    echo "  $0 --languages=python,typescript,ruby"
    echo "  $0 --languages=ruby-3.2.2,ruby-sorbet-3.2.2"
    echo "  $0 --language-tag=1.0.0 --registry=both"
}

# Default settings
DRY_RUN=false
LANGUAGE_TAG=""
LANGUAGES=("${SUPPORTED_LANGUAGES[@]}")
REGISTRY_TARGET="both"  # Options: ghcr, dockerhub, both
RUST_TAG=""

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            help
            exit 0
            ;;
        --dry-run|-N)
            DRY_RUN=true
            ;;
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --languages=*)
            IFS=',' read -ra LANGUAGES <<< "${arg#*=}"
            ;;
        --registry=*)
            REGISTRY_TARGET="${arg#*=}"
            if [[ ! "$REGISTRY_TARGET" =~ ^(ghcr|dockerhub|both)$ ]]; then
                echo -e "${YELLOW}Invalid registry: $REGISTRY_TARGET. Must be ghcr, dockerhub, or both${NC}"
                exit 1
            fi
            ;;
        --rust-tag=*)
            RUST_TAG="${arg#*=}"
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
echo -e "${BLUE}  Rust images: $RUST_TAG${NC}"
echo -e "${BLUE}  Language images: $LANGUAGE_TAG${NC}"
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
        if [ -z "${GITHUB_TOKEN:+x}" ]; then
            echo -e "${RED}Error: GITHUB_TOKEN environment variable is not set${NC}"
            echo "Please set GITHUB_TOKEN with ghcr.io push permissions"
            exit 1
        fi
        if ! echo "$GITHUB_TOKEN" | docker login ghcr.io -u nuanced-dev --password-stdin; then
            echo -e "${RED}Error: Failed to authenticate with GHCR${NC}"
            exit 1
        fi
        echo -e "${GREEN}✓ Authenticated with GHCR${NC}"
    fi
    if [ "$PUBLISH_TO_DOCKERHUB" = true ]; then
        echo -e "${YELLOW}Authenticating with Docker Hub...${NC}"
        if [ -z "${DOCKER_HUB_TOKEN:+x}" ]; then
            echo -e "${RED}Error: DOCKER_HUB_TOKEN environment variable is not set${NC}"
            echo "Please set DOCKER_HUB_TOKEN for Docker Hub authentication"
            exit 1
        fi
        if ! echo "$DOCKER_HUB_TOKEN" | docker login -u nuanced --password-stdin; then
            echo -e "${RED}Error: Failed to authenticate with Docker Hub${NC}"
            exit 1
        fi
        echo -e "${GREEN}✓ Authenticated with Docker Hub${NC}"
    fi
    echo
fi

# Array to track published images
PUBLISHED_IMAGES=()

# Function to tag and push an image
publish_image() {
    local image="$1"

    echo -e "${BLUE}Publishing ${image}...${NC}"

    if ! docker image inspect "${image}" > /dev/null 2>&1; then
        echo -e "${RED}✗ Local image ${image} not found. Please build it first.${NC}"
        return 1
    fi

    # Publish to GHCR if enabled
    if [ "$PUBLISH_TO_GHCR" = true ]; then
        local ghcr_image="${GHCR_REGISTRY}/${image}"

        if [ "$DRY_RUN" = true ]; then
            echo -e "${YELLOW}[DRY RUN] Would tag: ${image} → ${ghcr_image}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${ghcr_image}${NC}"
        else
            docker tag "$image" "$ghcr_image"
            docker push "$ghcr_image"
            PUBLISHED_IMAGES+=("${ghcr_image}")
            echo -e "${GREEN}✓ Published to GHCR: ${ghcr_image}${NC}"
        fi
    fi

    # Publish to Docker Hub if enabled
    if [ "$PUBLISH_TO_DOCKERHUB" = true ]; then
        local dockerhub_image="${DOCKERHUB_REGISTRY}/${image}"

        if [ "$DRY_RUN" = true ]; then
            echo -e "${YELLOW}[DRY RUN] Would tag: ${image} → ${dockerhub_image}${NC}"
            echo -e "${YELLOW}[DRY RUN] Would push: ${dockerhub_image}${NC}"
        else
            docker tag "$image" "$dockerhub_image"
            docker push "$dockerhub_image"
            PUBLISHED_IMAGES+=("${dockerhub_image}")
            echo -e "${GREEN}✓ Published to Docker Hub: ${dockerhub_image}${NC}"
        fi
    fi

    return 0
}

# Core Rust images
echo -e "${YELLOW}Step 1: Publishing Rust images (version: $RUST_TAG)${NC}"
echo

RUST_IMAGE_NAMES=(
    "nuanced-lsp-wrapper"
    "nuanced-lsp-proxy"
    "nuanced-lsp-watchdog"
)

failed=0
for image_name in "${RUST_IMAGE_NAMES[@]}"; do
    publish_image "$image_name:$RUST_TAG" || failed=$((failed + 1))
done

if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed Rust images failed to publish${NC}"
    exit 1
fi

echo

# Language images
echo -e "${YELLOW}Step 2: Publishing images for ${#LANGUAGES[@]} languages (version: $LANGUAGE_TAG)${NC}"
echo

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

failed=0
if [ ${#REGULAR_LANGUAGES[@]} -eq 0 ]; then
    echo -e "${YELLOW}No regular language images to publish${NC}"
else
    echo -e "${YELLOW}Publishing ${#REGULAR_LANGUAGES[@]} language images${NC}"
    for lang in "${REGULAR_LANGUAGES[@]}"; do
        publish_image "nuanced-lsp-${lang}:$LANGUAGE_TAG" || failed=$((failed + 1))
    done
fi

if [ ${#RUBY_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby images to publish${NC}"
else
    echo -e "${YELLOW}Publishing Ruby images (${#RUBY_VERSIONS[@]} versions)${NC}"
    for ruby_version in "${RUBY_VERSIONS[@]}"; do
        publish_image "nuanced-lsp-ruby-${ruby_version}:$LANGUAGE_TAG" || failed=$((failed + 1))
    done
fi

if [ ${#RUBY_SORBET_VERSIONS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No Ruby Sorbet images to publish${NC}"
else
    echo -e "${YELLOW}Publishing Ruby Sorbet images (${#RUBY_SORBET_VERSIONS[@]} versions)${NC}"
    for ruby_version in "${RUBY_SORBET_VERSIONS[@]}"; do
        publish_image "nuanced-lsp-ruby-sorbet-${ruby_version}:$LANGUAGE_TAG" || failed=$((failed + 1))
    done
fi

if [ $failed -gt 0 ]; then
    echo -e "${RED}$failed language images failed to publish${NC}"
    exit 1
fi

echo
echo -e "${GREEN}=========================================${NC}"
if [ "$DRY_RUN" = true ]; then
    echo -e "${GREEN}  Dry Run Complete${NC}"
    echo -e "${GREEN}  No images were actually pushed${NC}"
else
    echo -e "${GREEN}  All Images Published Successfully${NC}"
    echo -e "${GREEN}  Rust images: $RUST_TAG${NC}"
    echo -e "${GREEN}  Language images: $LANGUAGE_TAG${NC}"
fi
echo -e "${GREEN}=========================================${NC}"
echo

if [ "$DRY_RUN" = false ]; then
    echo -e "${BLUE}Published images:${NC}"
    for image in "${PUBLISHED_IMAGES[@]}"; do
        echo -e "  ${image}"
    done
    echo
fi

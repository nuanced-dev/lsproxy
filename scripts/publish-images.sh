#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"

DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"
DEFAULT_SERVICE_TAG="$("$SCRIPT_DIR/util/service-image-version.sh")"

DEFAULT_REGISTRY="ghcr.io/nuanced-dev"

usage() {
    echo "Usage: $0 [--dry-run] [--language-tag=TAG] [--languages=LANG...] [--registry=REG,...] [--services=SVC...] [--service-tag=TAG]"
}

help() {
    echo "Publish Docker images to container registry"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --all-languages       Publish all language images (shortcut for --languages=<all>)"
    echo "  --all-services        Publish all service images (shortcut for --services=proxy,watchdog,wrapper)"
    echo "  --dry-run, -N         Show what would be pushed without actually pushing"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "  --languages=LANG...   Comma-separated list of languages (default: none)"
    echo "                        Supports versioned Ruby: ruby-3.2.2, ruby-sorbet-3.2.2"
    echo "  --registry=REG        Target registry (default: $DEFAULT_REGISTRY)"
    echo "  --services=SVC...     Comma-separated list of services: proxy, watchdog, wrapper (default: none)"
    echo "  --service-tag=TAG     Tag of service images to use (default: $DEFAULT_SERVICE_TAG)"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Images:"
    echo "  - Images must already be built (use scripts/build-images.sh)"
    echo "  - Images must be multi-platform builds (built with --multi-platform flag)"
    echo ""
    echo "Versioning:"
    echo "  Service images (wrapper, proxy, watchdog) use release version tags (e.g. 0.4.0)"
    echo "  Language images use independent semver for API/protocol compatibility (e.g. 1.0.0)"
    echo "  This allows language images to guarantee API compatibility with service images"
    echo ""
    echo "Authentication:"
    echo "  You must be logged in to Docker registry before running this script."
    echo "  Use 'docker login <registry>' to authenticate."
    echo "  Examples:"
    echo "    docker login ghcr.io -u <username>"
    echo "    docker login -u <username>  # for Docker Hub"
    echo ""
    echo "Examples:"
    echo "  $0 --registry=nuanced,ghcr.io/nuanced-dev --all-services --all-languages"
    echo "  $0 --registry=ghcr.io/nuanced-dev --languages=python,typescript,ruby"
    echo "  $0 --registry=nuanced --languages=ruby-3.2.2,ruby-sorbet-3.2.2"
    echo "  $0 --registry=nuanced --language-tag=1.0.0"
    echo "  $0 --registry=ghcr.io/nuanced-dev --services=proxy,watchdog"
}

# Default settings
DRY_RUN=false
LANGUAGE_TAG=""
LANGUAGES=()
REGISTRY=""
SERVICES=()
SERVICE_TAG=""

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            help
            exit 0
            ;;
        --all-languages)
            LANGUAGES=("${SUPPORTED_LANGUAGES[@]}")
            ;;
        --all-services)
            SERVICES=(wrapper proxy watchdog)
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
            REGISTRY="${arg#*=}"
            ;;
        --services=*)
            IFS=',' read -ra SERVICES <<< "${arg#*=}"
            ;;
        --service-tag=*)
            SERVICE_TAG="${arg#*=}"
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            usage
            exit 1
            ;;
    esac
done

# Fall back to defaults
LANGUAGE_TAG="${LANGUAGE_TAG:-$DEFAULT_LANGUAGE_TAG}"
REGISTRY="${REGISTRY:-$DEFAULT_REGISTRY}"
SERVICE_TAG="${SERVICE_TAG:-$DEFAULT_SERVICE_TAG}"

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Publishing Images${NC}"
if [ ${#SERVICES[@]} -gt 0 ]; then
    echo -e "${BLUE}  Service images: $SERVICE_TAG${NC}"
fi
if [ ${#LANGUAGES[@]} -gt 0 ]; then
    echo -e "${BLUE}  Language images: $LANGUAGE_TAG${NC}"
fi
echo -e "${BLUE}  Registry: ${REGISTRY}${NC}"
echo -e "${BLUE}  Dry Run: $DRY_RUN${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Check authentication with registry
check_registry_auth() {
    local registry="$1"
    local registry_key="$registry"

    # Normalize registry key for Docker config lookup
    # If registry contains /, check for the part until the first /
    if [[ "$registry" == */* ]]; then
        registry_key="${registry%%/*}"
    else
        # No slash means Docker Hub
        registry_key="https://index.docker.io/v1/"
    fi

    if ! jq -e ".auths | has(\"$registry_key\")" "$HOME/.docker/config.json" > /dev/null 2>&1; then
        return 1
    fi
    return 0
}

get_login_command() {
    local registry="$1"

    if [[ "$registry" == */* ]]; then
        # Registry with / (e.g., ghcr.io/nuanced-dev)
        local registry_host="${registry%%/*}"
        echo "docker login $registry_host -u <username>"
    else
        # No slash means Docker Hub
        echo "docker login -u <username>"
    fi
}

if [ "$DRY_RUN" = false ]; then
    echo -e "${YELLOW}Checking registry authentication...${NC}"
    if ! check_registry_auth "$REGISTRY"; then
        echo -e "${RED}Error: Not logged in to registry: $REGISTRY${NC}"
        echo -e "${YELLOW}Please authenticate using:${NC}"
        echo -e "  $(get_login_command "$REGISTRY")"
        exit 1
    fi
    echo -e "${GREEN}✓ Authenticated with $registry${NC}"
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

    # Publish to specified registry
    local registry_image="${REGISTRY}/${image}"

    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY RUN] Would tag: ${image} → ${registry_image}${NC}"
        echo -e "${YELLOW}[DRY RUN] Would push: ${registry_image}${NC}"
    else
        docker tag "$image" "$registry_image"
        docker push "$registry_image"
        PUBLISHED_IMAGES+=("${registry_image}")
        echo -e "${GREEN}✓ Published to ${REGISTRY}: ${registry_image}${NC}"
    fi

    return 0
}

# Service images
if [ ${#SERVICES[@]} -eq 0 ]; then
    echo -e "${YELLOW}No service images to publish${NC}"
else
    echo -e "${YELLOW}Publishing ${#SERVICES[@]} service images (version: $SERVICE_TAG)${NC}"
    echo

    failed=0
    for image_name in "${SERVICES[@]}"; do
        publish_image "nuanced-lsp-${image_name}:$SERVICE_TAG" || failed=$((failed + 1))
    done

    if [ $failed -gt 0 ]; then
        echo -e "${RED}$failed service images failed to publish${NC}"
        exit 1
    fi

    echo
fi

# Language images
echo -e "${YELLOW}Publishing images for ${#LANGUAGES[@]} languages (version: $LANGUAGE_TAG)${NC}"

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

failed=0
if [ ${#UNVERSIONED_LANGUAGES[@]} -eq 0 ]; then
    echo -e "${YELLOW}No unversioned language images to publish${NC}"
else
    echo -e "${YELLOW}Publishing ${#UNVERSIONED_LANGUAGES[@]} language images${NC}"
    for lang in "${UNVERSIONED_LANGUAGES[@]}"; do
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
    if [ ${#SERVICES[@]} -gt 0 ]; then
        echo -e "${GREEN}  Service images: $SERVICE_TAG${NC}"
    fi
    if [ ${#LANGUAGES[@]} -gt 0 ]; then
        echo -e "${GREEN}  Language images: $LANGUAGE_TAG${NC}"
    fi
    echo -e "${BLUE}Published images:${NC}"
    for image in "${PUBLISHED_IMAGES[@]}"; do
        echo -e "  ${image}"
    done
    echo
fi
echo -e "${GREEN}=========================================${NC}"
echo

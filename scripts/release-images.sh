#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"
source "$SCRIPT_DIR/include/lib.sh"

usage() {
    echo "Usage: $0 [--all-languages] [--all-services]"
}

help() {
    echo "Release Docker images by pushing git tags"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --all-languages       Push tag for language images: languages-v${DEFAULT_LANGUAGE_TAG}"
    echo "  --all-services        Push tag for service images: service-v${DEFAULT_SERVICE_TAG}"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Pre-release checks:"
    echo "  - Git working directory must be clean"
    echo "  - Changelog entry must exist for the version being released"
    echo "  - Git tag must not already exist"
    echo ""
    echo "At least one of --all-services or --all-languages must be specified."
    echo ""
    echo "Examples:"
    echo "  $0 --all-services"
    echo "  $0 --all-languages"
    echo "  $0 --all-services --all-languages"
}

# Defaults
RELEASE_SERVICES=false
RELEASE_LANGUAGES=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            help
            exit 0
            ;;
        --all-services)
            RELEASE_SERVICES=true
            ;;
        --all-languages)
            RELEASE_LANGUAGES=true
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            usage
            exit 1
            ;;
    esac
done

# Check that at least one release type is specified
if [ "$RELEASE_SERVICES" = false ] && [ "$RELEASE_LANGUAGES" = false ]; then
    echo -e "${RED}Error: At least one of --all-services or --all-languages must be specified${NC}"
    usage
    exit 1
fi

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Release Images${NC}"
if [ "$RELEASE_SERVICES" = true ]; then
    echo -e "${BLUE}  Services: v$DEFAULT_SERVICE_TAG${NC}"
fi
if [ "$RELEASE_LANGUAGES" = true ]; then
    echo -e "${BLUE}  Languages: v$DEFAULT_LANGUAGE_TAG${NC}"
fi
echo -e "${BLUE}=========================================${NC}"
echo

# Pre-release checks
echo -e "${YELLOW}Running pre-release checks...${NC}"

# Check working directory is clean
if ! is_git_working_directory_clean "$ROOT_DIR"; then
    echo -e "${RED}Error: Git working directory is not clean${NC}"
    echo -e "${YELLOW}Please commit or stash your changes before releasing${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Git working directory is clean${NC}"

# Check changelog entries
if [ "$RELEASE_SERVICES" = true ]; then
    if ! has_changelog_entry "$DEFAULT_SERVICE_TAG" "$ROOT_DIR/CHANGELOG.md"; then
        echo -e "${RED}Error: Changelog entry missing for service version $DEFAULT_SERVICE_TAG${NC}"
        echo -e "${YELLOW}Please add a changelog entry in CHANGELOG.md with format:${NC}"
        echo -e "${YELLOW}  ## [$DEFAULT_SERVICE_TAG] - YYYY-MM-DD${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Changelog entry exists for service version $DEFAULT_SERVICE_TAG${NC}"
fi

# Check tags don't exist
if [ "$RELEASE_SERVICES" = true ]; then
    SERVICE_TAG="service-images-v$DEFAULT_SERVICE_TAG"
    if git_tag_exists "$SERVICE_TAG"; then
        echo -e "${RED}Error: Git tag $SERVICE_TAG already exists${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Git tag $SERVICE_TAG does not exist${NC}"
fi

if [ "$RELEASE_LANGUAGES" = true ]; then
    LANGUAGES_TAG="language-images-v$DEFAULT_LANGUAGE_TAG"
    if git_tag_exists "$LANGUAGES_TAG"; then
        echo -e "${RED}Error: Git tag $LANGUAGES_TAG already exists${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ Git tag $LANGUAGES_TAG does not exist${NC}"
fi

echo -e "${GREEN}All pre-release checks passed${NC}"
echo

# Push tags
echo -e "${YELLOW}Pushing git tags...${NC}"

if [ "$RELEASE_SERVICES" = true ]; then
    echo -e "${BLUE}Creating and pushing tag: $SERVICE_TAG${NC}"
    git tag "$SERVICE_TAG"
    git push origin "$SERVICE_TAG"
    echo -e "${GREEN}✓ Tag $SERVICE_TAG pushed${NC}"
fi

if [ "$RELEASE_LANGUAGES" = true ]; then
    echo -e "${BLUE}Creating and pushing tag: $LANGUAGES_TAG${NC}"
    git tag "$LANGUAGES_TAG"
    git push origin "$LANGUAGES_TAG"
    echo -e "${GREEN}✓ Tag $LANGUAGES_TAG pushed${NC}"
fi

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Release Complete${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}GitHub Actions workflows will now:${NC}"
if [ "$RELEASE_SERVICES" = true ]; then
    echo -e "${BLUE}  1. Build multi-platform service images${NC}"
    echo -e "${BLUE}  2. Publish service images to registry${NC}"
    echo -e "${BLUE}  3. Create GitHub release for $SERVICE_TAG${NC}"
fi
if [ "$RELEASE_LANGUAGES" = true ]; then
    echo -e "${BLUE}  1. Build multi-platform language images${NC}"
    echo -e "${BLUE}  2. Publish language images to registry${NC}"
    echo -e "${BLUE}  3. Create GitHub release for $LANGUAGES_TAG${NC}"
fi
echo

#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"
source "$SCRIPT_DIR/include/lib.sh"

usage() {
    echo "Usage: $0 [--dry-run|-N]"
}

help() {
    echo "Release service Docker images"
    echo ""
    echo "Options:"
    echo "  --dry-run, -N         Run all checks but only print the tag that would be pushed"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Pre-release checks:"
    echo "  - Git working directory must be clean"
    echo "  - Changelog entry must exist for v$DEFAULT_SERVICE_TAG"
    echo "  - Git tag must not already exist"
    echo ""
    echo "Creates and pushes tag: service-images-v$DEFAULT_SERVICE_TAG"
}

# Defaults
DRY_RUN=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --help|-h)
            help
            exit 0
            ;;
        --dry-run|-N)
            DRY_RUN=true
            shift
            ;;
        *)
            echo -e "${YELLOW}Unknown argument: $arg${NC}"
            usage
            exit 1
            ;;
    esac
done

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Release Service Images${NC}"
echo -e "${BLUE}  Version: v$DEFAULT_SERVICE_TAG${NC}"
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

# Check changelog entry
CHANGELOG_FILE="$ROOT_DIR/CHANGELOG.services.md"
if ! has_changelog_entry "$DEFAULT_SERVICE_TAG" "$CHANGELOG_FILE"; then
    echo -e "${RED}Error: Missing changelog entry for $DEFAULT_SERVICE_TAG${NC}"
    echo -e "${YELLOW}Please add a changelog entry in CHANGELOG.services.md with format:${NC}"
    echo -e "${YELLOW}  ## [$DEFAULT_SERVICE_TAG] - $(date -I)${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Changelog entry exists for $DEFAULT_SERVICE_TAG${NC}"

# Check tag doesn't exist
SERVICE_TAG="service-images-v$DEFAULT_SERVICE_TAG"
if git_tag_exists "$SERVICE_TAG"; then
    echo -e "${RED}Error: Git tag $SERVICE_TAG already exists${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Git tag $SERVICE_TAG does not exist${NC}"

echo -e "${GREEN}All pre-release checks passed${NC}"
echo

# Push tag
if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}DRY RUN: Would create and push tag: $SERVICE_TAG${NC}"
    echo
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  Dry Run Complete${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo
    echo -e "${BLUE}Would push tag: $SERVICE_TAG${NC}"
else
    echo -e "${YELLOW}Pushing git tag...${NC}"
    echo -e "${BLUE}Creating and pushing tag: $SERVICE_TAG${NC}"
    git tag "$SERVICE_TAG"
    git push origin "$SERVICE_TAG"
    echo -e "${GREEN}✓ Tag $SERVICE_TAG pushed${NC}"

    echo
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  Release Complete${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo
    echo -e "${BLUE}GitHub Actions workflow will now:${NC}"
    echo -e "${BLUE}  1. Build multi-platform service images${NC}"
    echo -e "${BLUE}  2. Publish service images to registry${NC}"
    echo -e "${BLUE}  3. Create GitHub release for $SERVICE_TAG${NC}"
    echo
fi

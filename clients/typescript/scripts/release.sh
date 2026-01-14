#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"
CLIENT_DIR="$(dirname "$SCRIPT_DIR")"

source "$CLIENT_DIR/../../scripts/include/colors.sh"
source "$CLIENT_DIR/../../scripts/include/lib.sh"

PACKAGE_VERSION=$(node -p "require('$CLIENT_DIR/package.json').version")
export PACKAGE_VERSION

usage() {
    echo "Usage: $0"
}

help() {
    echo "Release TypeScript client by pushing git tag"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Pre-release checks:"
    echo "  - Git working directory must be clean"
    echo "  - Changelog entry must exist for the version being released"
    echo "  - Git tag must not already exist"
    echo ""
    echo "This will push tag: typescript-client-v${PACKAGE_VERSION}"
}

# Parse arguments
for arg in "$@"; do
    case $arg in
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

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Release TypeScript Client${NC}"
echo -e "${BLUE}  Version: v$PACKAGE_VERSION${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Pre-release checks
echo -e "${YELLOW}Running pre-release checks...${NC}"

# Check working directory is clean
if ! is_git_working_directory_clean "$CLIENT_DIR"; then
    echo -e "${RED}Error: Git working directory is not clean${NC}"
    echo -e "${YELLOW}Please commit or stash your changes before releasing${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Git working directory is clean${NC}"

# Check version is semver
if ! is_semver "$PACKAGE_VERSION"; then
    echo -e "${RED}Error: Package version must be in MAJOR.MINOR.PATCH format${NC}"
    echo -e "${YELLOW}Current version: $PACKAGE_VERSION${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Package version is valid semver: $PACKAGE_VERSION${NC}"

# Check changelog entry
if ! has_changelog_entry "$PACKAGE_VERSION" "$CLIENT_DIR/CHANGELOG.md"; then
    echo -e "${RED}Error: Changelog entry missing for version $PACKAGE_VERSION${NC}"
    echo -e "${YELLOW}Please add a changelog entry in clients/typescript/CHANGELOG.md with format:${NC}"
    echo -e "${YELLOW}  ## [$PACKAGE_VERSION] - YYYY-MM-DD${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Changelog entry exists for version $PACKAGE_VERSION${NC}"

# Check tag doesn't exist
TAG="typescript-client-v$PACKAGE_VERSION"
if git_tag_exists "$TAG"; then
    echo -e "${RED}Error: Git tag $TAG already exists${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Git tag $TAG does not exist${NC}"

echo -e "${GREEN}All pre-release checks passed${NC}"
echo

# Push tag
echo -e "${YELLOW}Pushing git tag...${NC}"
echo -e "${BLUE}Creating and pushing tag: $TAG${NC}"
git tag "$TAG"
git push origin "$TAG"
echo -e "${GREEN}✓ Tag $TAG pushed${NC}"

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Release Complete${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}GitHub Actions workflow will now:${NC}"
echo -e "${BLUE}  1. Run tests${NC}"
echo -e "${BLUE}  2. Build the package${NC}"
echo -e "${BLUE}  3. Publish to npm${NC}"
echo -e "${BLUE}  4. Create GitHub release for $TAG${NC}"
echo

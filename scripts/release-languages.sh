#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"
source "$SCRIPT_DIR/include/lib.sh"

usage() {
    echo "Usage: $0 [--all-languages] [--languages=LANG...] LANGUAGE_TAG"
}

help() {
    echo "Release language Docker images"
    echo ""
    echo "Usage: $0 [OPTIONS...] LANGUAGE_TAG"
    echo ""
    echo "Options:"
    echo "  --all-languages       Release all language images (shorthand for --languages=<all>)"
    echo "  --languages=LANG...   Release specific language(s) - comma-separated"
    echo "                        Supports versioned Ruby: ruby-3.2.2, ruby-sorbet-3.2.2"
    echo "  --help, -h            Show this help message"
    echo ""
    echo "Arguments:"
    echo "  LANGUAGE_TAG          Semver version (X.Y.Z) to tag language images with"
    echo ""
    echo "Pre-release checks:"
    echo "  - Git working directory must be clean"
    echo "  - Version must be valid semver"
    echo "  - Git tags must not already exist"
    echo ""
    echo "Tag format: language-images-LANGUAGE-vX.Y.Z"
    echo "Examples: language-images-python-v1.0.1"
    echo "          language-images-ruby-3.2.2-v1.0.1"
    echo ""
    echo "Available languages: ${ALL_LANGUAGES[*]}"
    echo ""
    echo "Examples:"
    echo "  $0 --languages=python,typescript 1.0.1"
    echo "  $0 --all-languages 1.0.1"
    echo "  $0 --languages=ruby,ruby-sorbet 1.0.1"
    echo "  $0 --languages=ruby-3.2.2,ruby-sorbet-3.2.2 1.0.1"
}

# Defaults
LANGUAGES=()
LANGUAGE_TAG=""

# Parse options
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            help
            exit 0
            ;;
        --all-languages)
            LANGUAGES=("${ALL_LANGUAGES[@]}")
            shift
            ;;
        --languages=*)
            IFS=',' read -ra LANGUAGES <<< "${1#*=}"
            shift
            ;;
        -*)
            echo -e "${YELLOW}Unknown option: $1${NC}"
            usage
            exit 1
            ;;
        *)
            # Positional argument - should be LANGUAGE_TAG
            if [ -z "$LANGUAGE_TAG" ]; then
                LANGUAGE_TAG="$1"
                shift
            else
                echo -e "${YELLOW}Unexpected argument: $1${NC}"
                usage
                exit 1
            fi
            ;;
    esac
done

# Validate arguments
if [ ${#LANGUAGES[@]} -eq 0 ]; then
    echo -e "${RED}Error: At least one of --languages or --all-languages must be specified${NC}"
    usage
    exit 1
fi

if [ -z "$LANGUAGE_TAG" ]; then
    echo -e "${RED}Error: LANGUAGE_TAG is required${NC}"
    usage
    exit 1
fi

# Expand ruby and ruby-sorbet, and validate all languages
EXPANDED_LANGUAGES=()
for lang in "${LANGUAGES[@]}"; do
    if [[ "$lang" == "ruby" ]]; then
        # Expand to all supported Ruby versions
        for version in "${SUPPORTED_RUBY_VERSIONS[@]}"; do
            EXPANDED_LANGUAGES+=("ruby-$version")
        done
    elif [[ "$lang" =~ ^ruby-[0-9] ]]; then
        # Already versioned Ruby - keep as-is
        EXPANDED_LANGUAGES+=("$lang")
    elif [[ "$lang" == "ruby-sorbet" ]]; then
        # Expand to all supported Ruby Sorbet versions
        for version in "${SUPPORTED_RUBY_VERSIONS[@]}"; do
            EXPANDED_LANGUAGES+=("ruby-sorbet-$version")
        done
    elif [[ "$lang" =~ ^ruby-sorbet-[0-9] ]]; then
        # Already versioned Ruby Sorbet - keep as-is
        EXPANDED_LANGUAGES+=("$lang")
    else
        # Check if language is in supported list
        VALID_LANGUAGE=false
        for valid_lang in "${ALL_LANGUAGES[@]}"; do
            if [ "$lang" = "$valid_lang" ]; then
                VALID_LANGUAGE=true
                break
            fi
        done

        if [ "$VALID_LANGUAGE" = false ]; then
            echo -e "${RED}Error: Unknown language: $lang${NC}"
            echo -e "${YELLOW}Available languages: ${ALL_LANGUAGES[*]}${NC}"
            exit 1
        fi

        EXPANDED_LANGUAGES+=("$lang")
    fi
done

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Release Language Images${NC}"
echo -e "${BLUE}  Languages: ${EXPANDED_LANGUAGES[*]}${NC}"
echo -e "${BLUE}  Version: v$LANGUAGE_TAG${NC}"
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

# Check tags don't exist for any language
TAGS_TO_CREATE=()
for lang in "${EXPANDED_LANGUAGES[@]}"; do
    local_tag="language-images-${lang}-v${LANGUAGE_TAG}"
    if git_tag_exists "$local_tag"; then
        echo -e "${RED}Error: Git tag $local_tag already exists${NC}"
        exit 1
    fi
    TAGS_TO_CREATE+=("$local_tag")
done
echo -e "${GREEN}✓ Git tags do not exist for any language${NC}"

echo -e "${GREEN}All pre-release checks passed${NC}"
echo

# Push tags
echo -e "${YELLOW}Creating and pushing git tags...${NC}"
for tag in "${TAGS_TO_CREATE[@]}"; do
    echo -e "${BLUE}Creating and pushing tag: $tag${NC}"
    git tag "$tag"
    git push origin "$tag"
    echo -e "${GREEN}✓ Tag $tag pushed${NC}"
done

echo
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Release Complete${NC}"
echo -e "${GREEN}=========================================${NC}"
echo
echo -e "${BLUE}GitHub Actions workflow will now:${NC}"
for lang in "${EXPANDED_LANGUAGES[@]}"; do
    echo -e "${BLUE}  - Build and publish $lang images${NC}"
done
echo -e "${BLUE}  - Tag with: $LANGUAGE_TAG${NC}"
echo -e "${BLUE}  - Create GitHub releases${NC}"
echo

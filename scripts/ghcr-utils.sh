#!/usr/bin/env bash

# GitHub Container Registry (GHCR) utility script
# Provides functions for managing container images in ghcr.io
#
# Usage:
#   ./scripts/ghcr-utils.sh list-packages
#   ./scripts/ghcr-utils.sh list-versions <package-name>
#   ./scripts/ghcr-utils.sh delete-version <package-name> <version-id>
#   ./scripts/ghcr-utils.sh delete-package <package-name>
#   ./scripts/ghcr-utils.sh delete-all-ruby-images [--confirm]
#
# Environment:
#   GITHUB_TOKEN - Required for authentication to ghcr.io

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
ORG_NAME="nuanced-dev"
API_BASE="https://api.github.com/orgs/${ORG_NAME}"

# Check for required environment variables
check_token() {
    if [ -z "$GITHUB_TOKEN" ]; then
        echo -e "${RED}Error: GITHUB_TOKEN environment variable is not set${NC}"
        echo "Please set GITHUB_TOKEN with appropriate permissions"
        exit 1
    fi
}

# List all container packages in the organization
list_packages() {
    check_token
    echo -e "${BLUE}Fetching packages from ghcr.io/${ORG_NAME}...${NC}"

    response=$(curl -s \
        -H "Authorization: Bearer $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github.v3+json" \
        "${API_BASE}/packages?package_type=container")

    # Check if response is valid JSON array
    if ! echo "$response" | jq -e 'type == "array"' > /dev/null 2>&1; then
        echo -e "${RED}Error: Unexpected API response${NC}"
        echo -e "${YELLOW}Response:${NC}"
        echo "$response" | jq . 2>/dev/null || echo "$response"
        return 1
    fi

    # Check if array is empty
    if [ "$(echo "$response" | jq 'length')" = "0" ]; then
        echo -e "${YELLOW}No packages found in ghcr.io/${ORG_NAME}${NC}"
        return 0
    fi

    echo "$response" | jq -r '.[] | "\(.name) - \(.package_type) - \(.visibility)"'
}

# List all versions of a specific package
list_versions() {
    local package_name="$1"

    if [ -z "$package_name" ]; then
        echo -e "${RED}Error: Package name is required${NC}"
        echo "Usage: $0 list-versions <package-name>"
        exit 1
    fi

    check_token
    echo -e "${BLUE}Fetching versions for ${package_name}...${NC}"

    curl -s \
        -H "Authorization: Bearer $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github.v3+json" \
        "${API_BASE}/packages/container/${package_name}/versions" \
        | jq -r '.[] | "ID: \(.id) - Tags: \(.metadata.container.tags | join(", ")) - Created: \(.created_at)"'
}

# Get version ID by tag
get_version_id() {
    local package_name="$1"
    local tag="$2"

    if [ -z "$package_name" ] || [ -z "$tag" ]; then
        echo -e "${RED}Error: Package name and tag are required${NC}"
        return 1
    fi

    check_token

    curl -s \
        -H "Authorization: Bearer $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github.v3+json" \
        "${API_BASE}/packages/container/${package_name}/versions" \
        | jq -r --arg tag "$tag" '.[] | select(.metadata.container.tags | index($tag)) | .id' \
        | head -1
}

# Delete a specific version of a package
delete_version() {
    local package_name="$1"
    local version_id="$2"
    local skip_confirm="${3:-false}"

    if [ -z "$package_name" ] || [ -z "$version_id" ]; then
        echo -e "${RED}Error: Package name and version ID are required${NC}"
        echo "Usage: $0 delete-version <package-name> <version-id>"
        exit 1
    fi

    check_token

    # Ask for confirmation unless skipped (for batch operations)
    if [ "$skip_confirm" != "true" ]; then
        echo -e "${YELLOW}WARNING: This will delete version ${version_id} of ${package_name}${NC}"
        read -p "Are you sure? (yes/no): " confirm

        if [ "$confirm" != "yes" ]; then
            echo -e "${BLUE}Cancelled${NC}"
            exit 0
        fi
    fi

    echo -e "${YELLOW}Deleting version ${version_id} of ${package_name}...${NC}"

    response=$(curl -s -w "\n%{http_code}" \
        -X DELETE \
        -H "Authorization: Bearer $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github.v3+json" \
        "${API_BASE}/packages/container/${package_name}/versions/${version_id}")

    http_code=$(echo "$response" | tail -1)

    if [ "$http_code" = "204" ]; then
        echo -e "${GREEN}✓ Successfully deleted version ${version_id}${NC}"
    else
        echo -e "${RED}✗ Failed to delete version (HTTP $http_code)${NC}"
        echo "$response" | head -n -1
        exit 1
    fi
}

# Delete a specific version by tag name
delete_version_by_tag() {
    local package_name="$1"
    local tag="$2"

    if [ -z "$package_name" ] || [ -z "$tag" ]; then
        echo -e "${RED}Error: Package name and tag are required${NC}"
        echo "Usage: $0 delete-version-by-tag <package-name> <tag>"
        exit 1
    fi

    # Ask for confirmation first
    echo -e "${YELLOW}WARNING: This will delete ${package_name}:${tag}${NC}"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        echo -e "${BLUE}Cancelled${NC}"
        exit 0
    fi

    echo -e "${BLUE}Finding version ID for tag ${tag}...${NC}"
    version_id=$(get_version_id "$package_name" "$tag")

    if [ -z "$version_id" ]; then
        echo -e "${RED}✗ No version found with tag ${tag}${NC}"
        exit 1
    fi

    echo -e "${BLUE}Found version ID: ${version_id}${NC}"
    # Skip confirmation in delete_version since we already confirmed
    delete_version "$package_name" "$version_id" "true"
}

# Delete entire package (all versions)
delete_package() {
    local package_name="$1"

    if [ -z "$package_name" ]; then
        echo -e "${RED}Error: Package name is required${NC}"
        echo "Usage: $0 delete-package <package-name>"
        exit 1
    fi

    check_token
    echo -e "${YELLOW}WARNING: This will delete ALL versions of ${package_name}${NC}"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        echo -e "${BLUE}Cancelled${NC}"
        exit 0
    fi

    echo -e "${YELLOW}Deleting package ${package_name}...${NC}"

    response=$(curl -s -w "\n%{http_code}" \
        -X DELETE \
        -H "Authorization: Bearer $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github.v3+json" \
        "${API_BASE}/packages/container/${package_name}")

    http_code=$(echo "$response" | tail -1)

    if [ "$http_code" = "204" ]; then
        echo -e "${GREEN}✓ Successfully deleted package ${package_name}${NC}"
    else
        echo -e "${RED}✗ Failed to delete package (HTTP $http_code)${NC}"
        echo "$response" | head -n -1
        exit 1
    fi
}

# Delete all Ruby and Ruby Sorbet images
delete_all_ruby_images() {
    local confirm_flag="$1"

    check_token

    if [ "$confirm_flag" != "--confirm" ]; then
        echo -e "${YELLOW}WARNING: This will delete ALL Ruby and Ruby Sorbet packages from GHCR${NC}"
        echo -e "${YELLOW}This includes:${NC}"
        echo -e "${YELLOW}  - All nuanced-lsp-ruby-* packages${NC}"
        echo -e "${YELLOW}  - All nuanced-lsp-ruby-sorbet-* packages${NC}"
        echo ""
        read -p "Are you sure? Type 'DELETE ALL RUBY' to confirm: " confirm

        if [ "$confirm" != "DELETE ALL RUBY" ]; then
            echo -e "${BLUE}Cancelled${NC}"
            exit 0
        fi
    fi

    echo -e "${BLUE}Fetching Ruby packages...${NC}"
    packages=$(curl -s \
        -H "Authorization: Bearer $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github.v3+json" \
        "${API_BASE}/packages?package_type=container" \
        | jq -r '.[] | select(.name | startswith("nuanced-lsp-ruby")) | .name')

    if [ -z "$packages" ]; then
        echo -e "${YELLOW}No Ruby packages found${NC}"
        exit 0
    fi

    echo -e "${BLUE}Found packages:${NC}"
    echo "$packages"
    echo ""

    count=0
    while IFS= read -r package; do
        echo -e "${YELLOW}Deleting ${package}...${NC}"

        response=$(curl -s -w "\n%{http_code}" \
            -X DELETE \
            -H "Authorization: Bearer $GITHUB_TOKEN" \
            -H "Accept: application/vnd.github.v3+json" \
            "${API_BASE}/packages/container/${package}")

        http_code=$(echo "$response" | tail -1)

        if [ "$http_code" = "204" ]; then
            echo -e "${GREEN}✓ Deleted ${package}${NC}"
            ((count++))
        else
            echo -e "${RED}✗ Failed to delete ${package} (HTTP $http_code)${NC}"
        fi
    done <<< "$packages"

    echo ""
    echo -e "${GREEN}Deleted ${count} packages${NC}"
}

# Show help
show_help() {
    cat << EOF
GitHub Container Registry (GHCR) Utility Script

Usage:
  $0 <command> [arguments]

Commands:
  list-packages                           List all container packages in ghcr.io/${ORG_NAME}
  list-versions <package>                 List all versions of a specific package
  get-version-id <package> <tag>          Get version ID for a specific tag
  delete-version <package> <version-id>   Delete a specific version by ID
  delete-version-by-tag <package> <tag>   Delete a specific version by tag name
  delete-package <package>                Delete entire package (all versions)
  delete-all-ruby-images [--confirm]      Delete all Ruby and Ruby Sorbet packages
  help                                    Show this help message

Environment:
  GITHUB_TOKEN    Required - GitHub token with package:delete permissions

Examples:
  # List all packages
  $0 list-packages

  # List versions of a specific package
  $0 list-versions nuanced-lsp-ruby-3.4.4

  # Delete a specific version by tag
  $0 delete-version-by-tag nuanced-lsp-ruby-3.4.4 1.0.0

  # Delete entire package
  $0 delete-package nuanced-lsp-ruby-3.4.4

  # Delete all Ruby images (with confirmation prompt)
  $0 delete-all-ruby-images

  # Delete all Ruby images (skip confirmation - BE CAREFUL!)
  $0 delete-all-ruby-images --confirm

Organization: ${ORG_NAME}
API Base: ${API_BASE}
EOF
}

# Main command dispatcher
case "${1:-help}" in
    list-packages)
        list_packages
        ;;
    list-versions)
        list_versions "$2"
        ;;
    get-version-id)
        get_version_id "$2" "$3"
        ;;
    delete-version)
        delete_version "$2" "$3"
        ;;
    delete-version-by-tag)
        delete_version_by_tag "$2" "$3"
        ;;
    delete-package)
        delete_package "$2"
        ;;
    delete-all-ruby-images)
        delete_all_ruby_images "$2"
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        echo ""
        show_help
        exit 1
        ;;
esac

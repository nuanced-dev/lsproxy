## Utility functions

# Check if all given commands are present
# Usage: has_commands COMMAND...
has_commands() {
    result=0
    for cmd in "$@"; do
        if ! command -v "$cmd" &> /dev/null; then
            echo "missing command: $cmd" >&2
            result=1
        fi
    done
    return $result
}

# Ensure there's a line `[version] - YYYY-MM-DD` in CHANGELOG.md
# Usage: has_changelog_entry VERSION FILE
has_changelog_entry() {
    local version="$1"
    local file="$2"
    if [ ! -f "$file" ]; then
        return 0
    fi
    if ! cat "$file" | grep -F "## [$version]" | grep -qE '- [0-9]{4}-[0-9]{2}-[0-9]{2}'; then
        return 1
    fi
    return 0
}

# Verify that the git working directory is clean
# Usage: is_git_working_directory_clean WORKING_DIR
is_git_working_directory_clean() {
    local dir="$1"
    if ! status="$(git status --porcelain)" || [ -n "$status" ]; then
        return 1
    fi
    return 0
}

# Check MAJOR.MINOR.PATCH version format
# Usage: is_semver VERSION
is_semver() {
    local version="$1"
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
}

# Extract major version from semver tag or fail if not in semver format
# Usage: extract_major_version VERSION
extract_major_version() {
    local version="$1"
    if is_semver "$version"; then
        echo "${version%%.*}"
        exit 0
    else
        exit 1
    fi
}

# Check if git tag exists
# Usage: git_tag_exists TAG
git_tag_exists() {
    local tag="$1"
    git rev-parse "$tag" >/dev/null 2>&1
}

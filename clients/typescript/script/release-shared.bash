## Shared functions for lsp release scripts.

die() {
    echo "❌ $*" >&2
    exit 1
}

is_semver() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
}

check_required_commands() {
    echo "Checking required commands..."
    for cmd in gh git node npm; do
        if ! command -v "$cmd" &> /dev/null; then
            die "$cmd is not installed or not in PATH"
        fi
    done
}

read_package_metadata() {
    echo "Reading package metadata..."
    package_name="$(node -p "require('./package.json').name")"
    package_version="$(node -p "require('./package.json').version")"
    echo "Package: $package_name@$package_version"
}

check_changelog_entry() {
    # Ensure there's a line `[version] - YYYY-MM-DD` in CHANGELOG.md
    if [ -f "CHANGELOG.md" ]; then
        echo "Checking changelog entry..."
        # Match any date format (YYYY-MM-DD) for the version
        changelog_pattern="^## \[$package_version\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$"
        if ! grep -q -E "$changelog_pattern" CHANGELOG.md; then
            die "missing CHANGELOG.md entry for version $package_version with date format: ## [$package_version] - YYYY-MM-DD"
        fi
    fi
}

check_clean_working_directory() {
    # Verify that the git working directory is clean after build
    echo "Checking git working directory is clean..."
    if ! status="$(git status --porcelain)" || [ -n "$status" ]; then
        die "git working directory is dirty. Please commit or stash your changes."
    fi
}

# Reads a JSON field from config/version.json
read_version_field() {
  local field="$1"
  node -e 'const fs=require("fs");const p=process.argv[1];const f=process.argv[2];process.stdout.write(JSON.parse(fs.readFileSync(p,"utf8"))[f]||"");' "$CONFIG_JSON" "$field"
}

read_lsp_version() {
  read_version_field "lsp-version"
}

read_proxy_image_version() {
  read_version_field "proxy-image-version"
}

read_wrapper_image_version() {
  read_version_field "wrapper-image-version"
}

read_watchdog_image_version() {
  read_version_field "watchdog-image-version"
}

read_language_image_version() {
  read_version_field "language-image-version"
}

# Validates all 5 required versions are present and valid semver
validate_all_versions() {
  local missing=()

  LSP_VERSION="$(read_lsp_version)"
  PROXY_IMAGE_VERSION="$(read_proxy_image_version)"
  WRAPPER_IMAGE_VERSION="$(read_wrapper_image_version)"
  WATCHDOG_IMAGE_VERSION="$(read_watchdog_image_version)"
  LANGUAGE_IMAGE_VERSION="$(read_language_image_version)"

  if [[ -z "$LSP_VERSION" ]]; then
    missing+=("lsp-version")
  elif ! is_semver "$LSP_VERSION"; then
    die "lsp-version must be in x.y.z form (got: '$LSP_VERSION')"
  fi

  if [[ -z "$PROXY_IMAGE_VERSION" ]]; then
    missing+=("proxy-image-version")
  elif ! is_semver "$PROXY_IMAGE_VERSION"; then
    die "proxy-image-version must be in x.y.z form (got: '$PROXY_IMAGE_VERSION')"
  fi

  if [[ -z "$WRAPPER_IMAGE_VERSION" ]]; then
    missing+=("wrapper-image-version")
  elif ! is_semver "$WRAPPER_IMAGE_VERSION"; then
    die "wrapper-image-version must be in x.y.z form (got: '$WRAPPER_IMAGE_VERSION')"
  fi

  if [[ -z "$WATCHDOG_IMAGE_VERSION" ]]; then
    missing+=("watchdog-image-version")
  elif ! is_semver "$WATCHDOG_IMAGE_VERSION"; then
    die "watchdog-image-version must be in x.y.z form (got: '$WATCHDOG_IMAGE_VERSION')"
  fi

  if [[ -z "$LANGUAGE_IMAGE_VERSION" ]]; then
    missing+=("language-image-version")
  elif ! is_semver "$LANGUAGE_IMAGE_VERSION"; then
    die "language-image-version must be in x.y.z form (got: '$LANGUAGE_IMAGE_VERSION')"
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    die "Missing required versions in $CONFIG_JSON: ${missing[*]}"
  fi
}

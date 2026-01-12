CONSTANTS_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

if [ -n "${LANGUAGE_IMAGE_VERSION:+x}" ]; then
    DEFAULT_LANGUAGE_TAG="$LANGUAGE_IMAGE_VERSION"
else
    DEFAULT_LANGUAGE_TAG="$(cat "$CONSTANTS_DIR/../../language-image-version")"
fi
export DEFAULT_LANGUAGE_TAG

if [ -n "${SERVICE_IMAGE_VERSION:+x}" ]; then
    DEFAULT_SERVICE_TAG="$SERVICE_IMAGE_VERSION"
else
    DEFAULT_SERVICE_TAG="$(cargo metadata --no-deps --format-version 1 --manifest-path "$CONSTANTS_DIR/../../Cargo.toml" | jq -r '.packages[] | select(.name == "proxy") | .version')"
fi
export DEFAULT_SERVICE_TAG

export DEFAULT_REGISTRY="ghcr.io/nuanced-dev"

export ALL_SERVICES=( \
    "proxy"
    "watchdog"
    "wrapper"
)

export ALL_LANGUAGES=( \
    "clangd"
    "csharp"
    "golang"
    "java"
    "php"
    "python"
    "ruby"
    "ruby-sorbet"
    "rust"
    "typescript"
)

# Supported Ruby versions:
# - Core versions from original support (3.2.2, 3.2.6, 3.3.5)
# - Last 1 year of releases (Nov 2024 - Nov 2025): 3.3.6-3.3.10, 3.4.0-3.4.7
export SUPPORTED_RUBY_VERSIONS=(
    "3.2.2" "3.2.6"
    "3.3.5" "3.3.6" "3.3.7" "3.3.8" "3.3.9" "3.3.10"
    "3.4.0" "3.4.1" "3.4.2" "3.4.3" "3.4.4" "3.4.5" "3.4.6" "3.4.7"
)

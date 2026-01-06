#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"

DEFAULT_RUST_TAG="$("$SCRIPT_DIR/util/rust-image-version.sh")"
DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

help() {
    echo "Comprehensive test suite for Nuanced LSP"
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --tag=TAG             Tag of Rust images to use (default: $DEFAULT_RUST_TAG)"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "  --help, -h            Show this help"
    echo ""
    echo "Runs all test suites: Rust unit/integration tests and shell-based endpoint tests."
}

# Default values
RUST_TAG=""
LANGUAGE_TAG=""

# Parse options
for arg in "$@"; do
    case $arg in
        --tag=*)
            RUST_TAG="${arg#*=}"
            ;;
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --help|-h)
            help
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
    esac
done

# Flags to pass on to other tests
TAG_FLAGS=()
TAG_ENV=()
if [ -n "$RUST_TAG" ]; then
    echo "Using Rust image version: ${RUST_TAG}"
    TAG_FLAGS+=("--tag=$RUST_TAG")
    TAG_ENV+=("RUST_IMAGE_VERSION=$RUST_TAG")
fi
if [ -n "$LANGUAGE_TAG" ]; then
    echo "Using language image version: ${LANGUAGE_TAG}"
    TAG_FLAGS+=("--language-tag=$LANGUAGE_TAG")
    TAG_ENV+=("LANGUAGE_IMAGE_VERSION=$LANGUAGE_TAG")
fi

# Fall back to default tags
RUST_TAG="${RUST_TAG:-$DEFAULT_RUST_TAG}"

echo "========================================"
echo "  Nuanced LSP Test Suite                "
echo "========================================"
echo

# Check if Docker is running
if ! docker ps >/dev/null 2>&1; then
    echo "Error: Docker is not running"
    exit 1
fi

# 1. Run Rust unit and integration tests
echo "1. Running Rust unit and integration tests..."
echo "----------------------------------------"
# Run with --test-threads=1 to ensure serial execution of integration tests
# The container orchestration tests use #[serial] and a shared fixture
env "${TAG_ENV[@]}" cargo test --workspace --all-targets --all-features -- --test-threads=1
echo "✓ Rust tests passed"
echo

# 2. Build all containers (if not already built)
echo "2. Checking Docker images..."
echo "----------------------------------------"
if ! docker images | grep -F "nuanced-lsp-proxy" | grep -qF "$RUST_TAG"; then
    echo "✗ Service image not found. Build first with scripts/build-rust-images.sh and scripts/build-language-images.sh"
    exit 1
else
    echo "✓ Docker images found"
fi
echo

# 3. Run container lifecycle tests
echo "3. Running container lifecycle tests..."
echo "----------------------------------------"
./scripts/test-container-lifecycle.sh "${TAG_FLAGS[@]}"
echo

# 4. Run watchdog tests
echo "4. Running watchdog tests..."
echo "----------------------------------------"
./scripts/test-watchdog.sh "${TAG_FLAGS[@]}"
echo

# 5. Run endpoint tests
echo "5. Running endpoint tests..."
echo "----------------------------------------"
./scripts/test-all-endpoints.sh "${TAG_FLAGS[@]}"
echo

echo "========================================"
echo "  All tests passed! ✓"
echo "========================================"

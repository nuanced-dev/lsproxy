#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"

DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"
DEFAULT_SERVICE_TAG="$("$SCRIPT_DIR/util/service-image-version.sh")"

help() {
    echo "Comprehensive test suite for Nuanced LSP"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "  --service-tag=TAG     Tag of service images to use (default: $DEFAULT_SERVICE_TAG)"
    echo "  --help, -h            Show this help"
    echo ""
    echo "Runs all test suites: Rust unit/integration tests and shell-based endpoint tests."
}

# Default values
LANGUAGE_TAG=""
SERVICE_TAG=""

# Parse options
for arg in "$@"; do
    case $arg in
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --service-tag=*)
            SERVICE_TAG="${arg#*=}"
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
CARGO_ENV=()
if [ -n "$SERVICE_TAG" ]; then
    echo "Using service image version: ${SERVICE_TAG}"
    TAG_FLAGS+=("--service-tag=$SERVICE_TAG")
    CARGO_ENV+=("SERVICE_IMAGE_VERSION=$SERVICE_TAG")
fi
if [ -n "$LANGUAGE_TAG" ]; then
    echo "Using language image version: ${LANGUAGE_TAG}"
    TAG_FLAGS+=("--language-tag=$LANGUAGE_TAG")
    CARGO_ENV+=("LANGUAGE_IMAGE_VERSION=$LANGUAGE_TAG")
fi

# Fall back to default tags
SERVICE_TAG="${SERVICE_TAG:-$DEFAULT_SERVICE_TAG}"

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
env "${CARGO_ENV[@]}" cargo test --workspace --all-targets --all-features -- --test-threads=1
echo "✓ Rust tests passed"
echo

# 2. Build all containers (if not already built)
echo "2. Checking Docker images..."
echo "----------------------------------------"
if ! docker images | grep -F "nuanced-lsp-proxy" | grep -qF "$SERVICE_TAG"; then
    echo "✗ Service image not found. Build first with scripts/build-images.sh --all-services"
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

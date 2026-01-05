#!/usr/bin/env bash
# Comprehensive test suite for Nuanced LSP.
# Runs all test suites: Rust unit/integration tests and shell-based endpoint tests.

set -eu  # Exit immediately if a command exits with a non-zero status

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"

DEFAULT_RUST_TAG="$("$SCRIPT_DIR/util/rust-image-version.sh")"
DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"

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
# Set Docker image versions for tests
# Use "latest" for development, or override with specific versions for production testing
export RUST_IMAGE_VERSION="$DEFAULT_RUST_TAG"
export LANGUAGE_IMAGE_VERSION="$DEFAULT_LANGUAGE_TAG"
echo "Using image versions: Rust=${RUST_IMAGE_VERSION}, Language=${LANGUAGE_IMAGE_VERSION}"
# Run with --test-threads=1 to ensure serial execution of integration tests
# The container orchestration tests use #[serial] and a shared fixture
cargo test --workspace --all-targets --all-features -- --test-threads=1 $@
echo "✓ Rust tests passed"
echo

# 2. Build all containers (if not already built)
echo "2. Checking Docker images..."
echo "----------------------------------------"
if ! docker images | grep -q "nuanced-lsp-proxy.*$RUST_IMAGE_VERSION"; then
    echo "✗ Service image not found. Build first with $(dirname $0)/build-rust-images.sh and $(dirname $0)/build-language-images.sh"
    exit 1
else
    echo "✓ Docker images found"
fi
echo

# 3. Run container lifecycle tests
echo "3. Running container lifecycle tests..."
echo "----------------------------------------"
./scripts/test-container-lifecycle.sh
echo

# 4. Run watchdog tests
echo "4. Running watchdog tests..."
echo "----------------------------------------"
./scripts/test-watchdog.sh
echo

# 5. Run endpoint tests
echo "5. Running endpoint tests..."
echo "----------------------------------------"
./scripts/test-all-endpoints.sh
echo

echo "========================================"
echo "  All tests passed! ✓"
echo "========================================"

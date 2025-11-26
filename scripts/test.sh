#!/usr/bin/env bash
# Comprehensive test suite for Nuanced LSP.
# Runs all test suites: Rust unit/integration tests and shell-based endpoint tests.

set -e  # Exit immediately if a command exits with a non-zero status

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
# RUST_IMAGE_VERSION: "latest" for locally-built images via build-rust-images.sh
# LANGUAGE_IMAGE_VERSION: "1.0.0" to match published images on GHCR
export RUST_IMAGE_VERSION="${RUST_IMAGE_VERSION:-latest}"
export LANGUAGE_IMAGE_VERSION="${LANGUAGE_IMAGE_VERSION:-1.0.0}"
echo "Using image versions: Rust=${RUST_IMAGE_VERSION}, Language=${LANGUAGE_IMAGE_VERSION}"
# Run with --test-threads=1 to ensure serial execution of integration tests
# The container orchestration tests use #[serial] and a shared fixture
cargo test --workspace -- --test-threads=1 $@
echo "✓ Rust tests passed"
echo

# 2. Build all containers (if not already built)
echo "2. Checking Docker images..."
echo "----------------------------------------"
if ! docker images | grep -q "nuanced-lsp-proxy.*latest"; then
    echo "Service image not found. Building Rust images..."
    ./scripts/build-rust-images.sh
    echo "Building language images..."
    ./scripts/build-language-images.sh
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

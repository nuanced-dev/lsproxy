#!/bin/bash
# Default Rust image version equals the crate version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

if [ -n "${RUST_IMAGE_VERSION:+x}" ]; then
    echo "$RUST_IMAGE_VERSION"
else
    cargo metadata --no-deps --format-version 1 --manifest-path "$SCRIPT_DIR/../../Cargo.toml" | jq -r '.packages[] | select(.name == "proxy") | .version'
fi

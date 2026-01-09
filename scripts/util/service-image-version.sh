#!/bin/bash
# Default service image version equals the Rust crate version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

if [ -n "${SERVICE_IMAGE_VERSION:+x}" ]; then
    echo "$SERVICE_IMAGE_VERSION"
else
    cargo metadata --no-deps --format-version 1 --manifest-path "$SCRIPT_DIR/../../Cargo.toml" | jq -r '.packages[] | select(.name == "proxy") | .version'
fi

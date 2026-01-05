#!/bin/bash
# Default Rust image version equals the crate version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

cargo metadata --no-deps --format-version 1 --manifest-path "$SCRIPT_DIR/../../Cargo.toml" | jq -r '.packages[] | select(.name == "proxy") | .version'

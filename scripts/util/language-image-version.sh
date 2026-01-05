#!/bin/bash
# Default language image version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

cat "$SCRIPT_DIR/../../language-image-version"

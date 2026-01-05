#!/bin/bash
# Default language image version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

if [ -n "${LANGUAGE_IMAGE_VERSION:+x}" ]; then
    echo "$LANGUAGE_IMAGE_VERSION"
else
    cat "$SCRIPT_DIR/../../language-image-version"
fi

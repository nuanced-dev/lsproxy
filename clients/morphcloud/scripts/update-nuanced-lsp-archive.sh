#!/usr/bin/env bash

set -eu

ARCHIVE_DIR="$(realpath "$(dirname "$0")/..")"

if [ $# -ne 1 ]; then
    echo "Usage: $0 REPO_DIR"
    exit 1
fi

exec git -C "$1" archive -o "$ARCHIVE_DIR/nuanced-lsp-src.zip" HEAD

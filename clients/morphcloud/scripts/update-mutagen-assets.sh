#!/usr/bin/env bash

set -eu

if [ $# -ne 1 ]; then
    echo "Usage: $0 <mutagen-directory>" >&2
    exit 1
fi

MUTAGEN_DIR="$1"
RELEASE_DIR="$MUTAGEN_DIR/build/release"
SCRIPT_DIR="$(dirname "$0")"
BIN_DIR="$SCRIPT_DIR/../bin/mutagen"

if [ ! -d "$RELEASE_DIR" ]; then
    echo "Error: Release directory not found: $RELEASE_DIR" >&2
    exit 1
fi

rm -rf "$BIN_DIR"
mkdir -p "$BIN_DIR"

unpack () {
    source_platform="$1"
    target_platform="$2"
    if ! ls "$RELEASE_DIR/mutagen_${source_platform}_v"*".tar.gz"; then
        echo "Error: Missing release for platform $source_platform"
        exit 1
    fi
    target_dir="$BIN_DIR/$target_platform"
    if [ -e "$target_dir" ]; then
        echo "Error: Directory already exists for $target_platform"
        exit 1
    fi
    mkdir -p "$target_dir"
    tar -xzf "$RELEASE_DIR/mutagen_${source_platform}_v"*".tar.gz" -C "$target_dir"
}

unpack darwin_amd64 darwin_x64
unpack darwin_arm64 darwin_arm64

unpack linux_amd64 linux_x64
unpack linux_arm64 linux_arm64

unpack windows_amd64 windows_x64

echo "Successfully updated mutagen assets in $BIN_DIR"

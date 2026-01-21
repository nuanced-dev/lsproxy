#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin) OS_NAME="darwin" ;;
  linux) OS_NAME="linux" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_NAME="x64" ;;
  arm64|aarch64) ARCH_NAME="arm64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

MUTAGEN_BIN="$PROJECT_ROOT/bin/mutagen/${OS_NAME}_${ARCH_NAME}/mutagen"

if [[ ! -x "$MUTAGEN_BIN" ]]; then
  echo "Mutagen binary not found or not executable: $MUTAGEN_BIN" >&2
  exit 1
fi

export MUTAGEN_DATA_DIRECTORY="$HOME/.nuanced/mutagen"

exec "$MUTAGEN_BIN" "$@"

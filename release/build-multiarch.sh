#!/usr/bin/env bash
set -euo pipefail

# Default repo; override with --repo
REPO_DEFAULT="ghcr.io/nuanced-dev/nuanced-lsproxy"

usage() {
  cat <<EOF
Usage: $(basename "$0") <tag> [--repo ghcr.io/owner/name] [--no-login]

Builds and pushes a multi-platform image for release/Dockerfile with:
  platforms: linux/amd64,linux/arm64
  repo:      ${REPO_DEFAULT}

Examples:
  $(basename "$0") 0.3.1
  $(basename "$0") 1.2.3 --repo ghcr.io/nuanced-dev/nuanced-lsproxy
EOF
}

TAG="${1:-}"
if [[ -z "${TAG}" ]]; then usage; exit 1; fi
shift || true

REPO="${REPO_DEFAULT}"
DO_LOGIN=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      REPO="${2:?--repo needs a value}"; shift 2;;
    --no-login)
      DO_LOGIN=0; shift;;
    -h|--help)
      usage; exit 0;;
    *)
      echo "Unknown arg: $1"; usage; exit 1;;
  esac
done

IMAGE="${REPO}:${TAG}"

echo ">> Image: ${IMAGE}"
echo ">> Platforms: linux/amd64, linux/arm64"
echo ">> Context:   lsproxy"
echo ">> Dockerfile: release/Dockerfile"

# 1) Login to GHCR (needs a PAT with 'write:packages' scope).
if [[ ${DO_LOGIN} -eq 1 ]]; then
  if ! docker login ghcr.io >/dev/null 2>&1; then
    echo "You need to login to GHCR."
    echo "Provide a GitHub Personal Access Token (classic) with 'write:packages' scope."
    echo "Username is your GitHub username. Press Ctrl+C to abort."
    docker login ghcr.io
  fi
fi

# 2) Ensure buildx builder exists and is active.
if ! docker buildx inspect nuanced-builder >/dev/null 2>&1; then
  echo ">> Creating buildx builder 'nuanced-builder'..."
  docker buildx create --name nuanced-builder --driver docker-container --use
else
  echo ">> Using existing buildx builder 'nuanced-builder'."
  docker buildx use nuanced-builder
fi

# 3) Ensure binfmt (QEMU) is set up (Docker Desktop usually handles this).
docker buildx inspect --bootstrap >/dev/null

# 4) Build & push a multi-arch manifest
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f release/Dockerfile \
  -t "${IMAGE}" \
  --push \
  lsproxy

echo ">> Pushed ${IMAGE}"

# 5) (Optional) Show what got published
echo ">> Inspecting manifest:"
docker buildx imagetools inspect "${IMAGE}" || true

echo "✅ Done."

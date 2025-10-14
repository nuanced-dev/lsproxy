#!/bin/bash

# Rebuild Docker images for lsproxy
# Supports rebuilding base image and/or specific language images
# Usage: ./scripts/rebuild-image.sh [OPTIONS] [LANGUAGE...]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default options
NO_CACHE=""
REBUILD_LSP_WRAPPER=false
REBUILD_SERVICE=false
VERBOSE=false

# Available languages
LANGUAGES=(
    "python"
    "typescript"
    "ruby"
    "php"
    "golang"
    "rust"
    "java"
    "clangd"
    "csharp"
)

usage() {
    cat <<EOF
${BLUE}Usage:${NC} $0 [OPTIONS] [LANGUAGE...]

${BLUE}Rebuild Docker images for lsproxy${NC}

${YELLOW}Options:${NC}
  -h, --help              Show this help message
  -n, --no-cache          Build without using Docker cache
  -w, --lsp-wrapper       Also rebuild the lsp-wrapper base image
  -s, --service           Also rebuild the service image (lsproxy orchestrator)
  -a, --all               Rebuild all language images
  -v, --verbose           Show full build output

${YELLOW}Languages:${NC}
  ${LANGUAGES[@]}

${YELLOW}Examples:${NC}
  # Rebuild PHP with cache
  $0 php

  # Rebuild PHP without cache (forces fresh build)
  $0 --no-cache php

  # Rebuild lsp-wrapper base and PHP without cache
  $0 --no-cache --lsp-wrapper php

  # Rebuild service image (after modifying lsproxy/src)
  $0 --no-cache --service

  # Rebuild all language images
  $0 --all

  # Full rebuild: service, lsp-wrapper, and all languages
  $0 --no-cache --service --lsp-wrapper --all

${YELLOW}When to rebuild what:${NC}
  ${GREEN}LSP Wrapper base (--lsp-wrapper):${NC}
    - When you modify lsp-wrapper source code (lsproxy/lsp-wrapper/src)
    - This rebuilds the HTTP wrapper binary that language containers use
    - After rebuilding lsp-wrapper, you must rebuild affected language images

  ${GREEN}Service image (--service):${NC}
    - When you modify the main orchestrator (lsproxy/src)
    - This rebuilds the container that spawns/manages language containers
    - Service changes don't require rebuilding language images

  ${GREEN}Language images:${NC}
    - When you modify a language-specific Dockerfile
    - When you rebuild the lsp-wrapper base (most languages depend on it)
    - When you want to update the LSP server version

${YELLOW}Notes:${NC}
  - Use --no-cache when debugging build issues or after significant changes
  - Without --no-cache, Docker will reuse cached layers (faster but may miss changes)

EOF
    exit 0
}

log_info() {
    echo -e "${BLUE}$1${NC}"
}

log_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

log_warn() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

log_error() {
    echo -e "${RED}✗ $1${NC}"
}

rebuild_lsp_wrapper() {
    log_info "Rebuilding lsp-wrapper base image (lsproxy-base-build)..."

    cd "$PROJECT_ROOT"

    if [ "$VERBOSE" = true ]; then
        docker build $NO_CACHE --target base-build \
            -f dockerfiles/base.Dockerfile \
            -t lsproxy-base-build:latest .
    else
        docker build $NO_CACHE --target base-build \
            -f dockerfiles/base.Dockerfile \
            -t lsproxy-base-build:latest . 2>&1 | tail -10
    fi

    if [ $? -eq 0 ]; then
        log_success "LSP wrapper base image rebuilt successfully"
    else
        log_error "Failed to rebuild lsp-wrapper base image"
        exit 1
    fi
}

rebuild_service() {
    log_info "Rebuilding service image (lsproxy-service)..."

    cd "$PROJECT_ROOT"

    if [ "$VERBOSE" = true ]; then
        docker build $NO_CACHE \
            -f dockerfiles/service.Dockerfile \
            -t lsproxy-service:latest .
    else
        docker build $NO_CACHE \
            -f dockerfiles/service.Dockerfile \
            -t lsproxy-service:latest . 2>&1 | tail -10
    fi

    if [ $? -eq 0 ]; then
        log_success "Service image rebuilt successfully"
    else
        log_error "Failed to rebuild service image"
        exit 1
    fi
}

rebuild_language() {
    local lang=$1

    # Map language name to Dockerfile
    local dockerfile="dockerfiles/${lang}.Dockerfile"

    if [ ! -f "$PROJECT_ROOT/$dockerfile" ]; then
        log_error "Dockerfile not found: $dockerfile"
        return 1
    fi

    log_info "Rebuilding $lang image..."

    cd "$PROJECT_ROOT"

    if [ "$VERBOSE" = true ]; then
        docker build $NO_CACHE \
            -f "$dockerfile" \
            -t "lsproxy-${lang}:latest" .
    else
        docker build $NO_CACHE \
            -f "$dockerfile" \
            -t "lsproxy-${lang}:latest" . 2>&1 | tail -10
    fi

    if [ $? -eq 0 ]; then
        log_success "$lang image rebuilt successfully"
    else
        log_error "Failed to rebuild $lang image"
        return 1
    fi
}

# Parse arguments
SELECTED_LANGUAGES=()
REBUILD_ALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            ;;
        -n|--no-cache)
            NO_CACHE="--no-cache"
            shift
            ;;
        -w|--lsp-wrapper)
            REBUILD_LSP_WRAPPER=true
            shift
            ;;
        -s|--service)
            REBUILD_SERVICE=true
            shift
            ;;
        -a|--all)
            REBUILD_ALL=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -*)
            log_error "Unknown option: $1"
            echo ""
            usage
            ;;
        *)
            # Check if it's a valid language
            if [[ " ${LANGUAGES[@]} " =~ " $1 " ]]; then
                SELECTED_LANGUAGES+=("$1")
            else
                log_error "Unknown language: $1"
                echo ""
                log_info "Available languages: ${LANGUAGES[@]}"
                exit 1
            fi
            shift
            ;;
    esac
done

# If --all is specified, add all languages
if [ "$REBUILD_ALL" = true ]; then
    SELECTED_LANGUAGES=("${LANGUAGES[@]}")
fi

# Validate we have something to build
if [ "$REBUILD_LSP_WRAPPER" = false ] && [ "$REBUILD_SERVICE" = false ] && [ ${#SELECTED_LANGUAGES[@]} -eq 0 ]; then
    log_error "No images specified to rebuild"
    echo ""
    echo "Please specify either:"
    echo "  - One or more languages: $0 php ruby"
    echo "  - The --all flag: $0 --all"
    echo "  - The --lsp-wrapper flag: $0 --lsp-wrapper"
    echo "  - The --service flag: $0 --service"
    echo ""
    echo "Use --help for more information"
    exit 1
fi

# Print summary
echo ""
log_info "========================================="
log_info "  Docker Image Rebuild"
log_info "========================================="
echo ""
log_info "Options:"
[ -n "$NO_CACHE" ] && echo "  - No cache: enabled" || echo "  - No cache: disabled"
[ "$VERBOSE" = true ] && echo "  - Verbose: enabled" || echo "  - Verbose: disabled"
echo ""

if [ "$REBUILD_SERVICE" = true ] || [ "$REBUILD_LSP_WRAPPER" = true ] || [ ${#SELECTED_LANGUAGES[@]} -gt 0 ]; then
    log_info "Will rebuild:"
fi

if [ "$REBUILD_SERVICE" = true ]; then
    echo "  - Service image (lsproxy-service)"
fi

if [ "$REBUILD_LSP_WRAPPER" = true ]; then
    echo "  - LSP wrapper base image (lsproxy-base-build)"
fi

if [ ${#SELECTED_LANGUAGES[@]} -gt 0 ]; then
    for lang in "${SELECTED_LANGUAGES[@]}"; do
        echo "  - $lang"
    done
fi

echo ""
log_info "========================================="
echo ""

# Rebuild service if requested (typically done first, but independent of others)
if [ "$REBUILD_SERVICE" = true ]; then
    rebuild_service
    echo ""
fi

# Rebuild lsp-wrapper base if requested (must be before languages that depend on it)
if [ "$REBUILD_LSP_WRAPPER" = true ]; then
    rebuild_lsp_wrapper
    echo ""
fi

# Rebuild selected languages
FAILED_LANGUAGES=()
for lang in "${SELECTED_LANGUAGES[@]}"; do
    if ! rebuild_language "$lang"; then
        FAILED_LANGUAGES+=("$lang")
    fi
    echo ""
done

# Print summary
log_info "========================================="
log_info "  Build Summary"
log_info "========================================="
echo ""

if [ "$REBUILD_SERVICE" = true ]; then
    log_success "Service image: rebuilt"
fi

if [ "$REBUILD_LSP_WRAPPER" = true ]; then
    log_success "LSP wrapper base image: rebuilt"
fi

if [ ${#SELECTED_LANGUAGES[@]} -gt 0 ]; then
    SUCCESS_COUNT=$((${#SELECTED_LANGUAGES[@]} - ${#FAILED_LANGUAGES[@]}))
    log_success "Languages: $SUCCESS_COUNT/${#SELECTED_LANGUAGES[@]} successful"
fi

if [ ${#FAILED_LANGUAGES[@]} -gt 0 ]; then
    echo ""
    log_error "Failed languages:"
    for lang in "${FAILED_LANGUAGES[@]}"; do
        echo "  - $lang"
    done
    echo ""
    log_warn "Tip: Try running with --verbose to see full build output"
    exit 1
fi

echo ""
log_info "Next steps:"
echo "  1. Restart the service: ./scripts/stop-service.sh && ./scripts/start-service.sh"
echo "  2. Run tests: ./scripts/test-all-endpoints.sh"
echo ""

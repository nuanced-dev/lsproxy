#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"

usage() {
    echo "Usage: $0 [--auth] [--foreground] [--language-tag=TAG] [--logs] [--port=PORT] [--registry=REG] [--service-tag=TAG] WORKSPACE_DIR"
}

help() {
    echo "Start Nuanced LSP proxy with container orchestration"
    echo ""
    echo "Usage: $0 [OPTIONS...] WORKSPACE_DIR"
    echo ""
    echo "Options:"
    echo "  --auth                Enable JWT authentication"
    echo "  --foreground, -f      Run in foreground (not detached)"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "  --logs, -l            Tail logs after starting"
    echo "  --port=PORT           Use custom port (default: 4444)"
    echo "  --registry=REG        Container registry for service images (default: none)"
    echo "  --service-tag=TAG     Tag images with specified tag (default: $DEFAULT_SERVICE_TAG)"
    echo "  --help, -h            Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 sample_project/python              # Start with Python workspace"
    echo "  $0 sample_project/all --logs          # Start and tail logs"
    echo "  $0 /path/to/workspace --port=5000     # Custom port"
}

# Default values
USE_AUTH=false
DETACHED=true
LANGUAGE_TAG=""
TAIL_LOGS=false
PORT=4444
REGISTRY=""
SERVICE_TAG=""
WORKSPACE_PATH=

# Parse options
for arg in "$@"; do
    case $arg in
        --auth)
            USE_AUTH=true
            ;;
        --foreground|-f)
            DETACHED=false
            ;;
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --logs|-l)
            TAIL_LOGS=true
            ;;
        --port=*)
            PORT="${arg#*=}"
            ;;
        --registry=*)
            REGISTRY="${arg#*=}"
            ;;
        --service-tag=*)
            SERVICE_TAG="${arg#*=}"
            ;;
        --help|-h)
            help
            exit 0
            ;;
        -*)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
        *)
            WORKSPACE_PATH="$arg"
            ;;
    esac
done

# Verify workspace argument
if [ -z "$WORKSPACE_PATH" ]; then
    echo -e "${RED}Error: Workspace directory argument missing${NC}"
    usage
    exit 1
fi
if [ ! -d "$WORKSPACE_PATH" ]; then
    echo -e "${RED}Error: Workspace directory does not exist: $WORKSPACE_PATH${NC}"
    exit 1
fi

# Convert to absolute path
WORKSPACE_PATH="$(cd "$WORKSPACE_PATH" && pwd)"








echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Starting Nuanced LSP Service${NC}           "
echo -e "${BLUE}=========================================${NC}"
echo -e "Workspace: ${WORKSPACE_PATH}"
echo -e "Port:      ${PORT}"
echo -e "Auth:      ${USE_AUTH}"
echo -e "Mode:      $([ "$DETACHED" = true ] && echo "detached" || echo "foreground")"
echo -e "${BLUE}=========================================${NC}"
echo

# Check if service is already running
if docker ps --filter "name=nuanced-lsp-proxy" --format "{{.Names}}" | grep -q "nuanced-lsp-proxy"; then
    echo -e "${YELLOW}Warning: nuanced-lsp-proxy is already running${NC}"
    read -p "Stop and restart? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Stopping existing service...${NC}"
        docker rm -f nuanced-lsp-proxy
        # Also stop any orphaned language containers
        docker rm -f $(docker ps -aq --filter "name=nuanced-lsp-") 2>/dev/null || true
    else
        echo -e "${YELLOW}Exiting without changes${NC}"
        exit 0
    fi
fi

# Start the service
echo -e "${BLUE}Starting service container...${NC}"

DOCKER_ARGS=(
    "-v" "/var/run/docker.sock:/var/run/docker.sock"
    "-v" "${WORKSPACE_PATH}:/mnt/workspace"
    "-e" "RUST_LOG=info,nuanced_lsp_proxy=debug,proxy=debug,nuanced_lsp_wrapper=debug,wrapper=debug"
)
if [ "$DETACHED" = true ]; then
    DOCKER_ARGS+=("-d")
else
    DOCKER_ARGS+=("-it")
fi
if [ "$USE_AUTH" = false ]; then
    DOCKER_ARGS+=("-e" "USE_AUTH=false")
fi
# If tags were set via flags, pass them on
if [ -n "$LANGUAGE_TAG" ]; then
    DOCKER_ARGS+=("-e" "LANGUAGE_IMAGE_VERSION=${LANGUAGE_TAG}")
fi
if [ -n "$SERVICE_TAG" ]; then
    DOCKER_ARGS+=("-e" "SERVICE_IMAGE_VERSION=${SERVICE_TAG}")
fi
if [ -n "$REGISTRY" ]; then
    DOCKER_ARGS+=("-e" "CONTAINER_REGISTRY=${REGISTRY}")
fi
# If images were set in the environment, pass them on
if [ -n "${WATCHDOG_IMAGE:+x}" ]; then
    DOCKER_ARGS+=("-e" "WATCHDOG_IMAGE=${WATCHDOG_IMAGE}")
fi
if [ -n "${WRAPPER_IMAGE:+x}" ]; then
    DOCKER_ARGS+=("-e" "WRAPPER_IMAGE=${WRAPPER_IMAGE}")
fi
# If languages were set in the environment, pass them on
if [ -n "${ENABLED_LANGUAGES:+x}" ]; then
    DOCKER_ARGS+=("-e" "ENABLED_LANGUAGES=${ENABLED_LANGUAGES}")
fi

PROXY_IMAGE="${REGISTRY:+$REGISTRY/}${PROXY_IMAGE:-nuanced-lsp-proxy:${SERVICE_TAG:-$DEFAULT_SERVICE_TAG}}"

docker run \
    --name nuanced-lsp-proxy \
    -p "${PORT}:4444" \
    -e NUANCED_LSP_MAX_MEMORY=8192 \
    "${DOCKER_ARGS[@]}" \
    "$PROXY_IMAGE"

if [ "$DETACHED" = true ]; then
    echo -e "${GREEN}✓ Service container started${NC}"
    echo

    # Wait a moment for container to initialize
    echo -e "${BLUE}Waiting for service to initialize...${NC}"
    sleep 3

    # Check if container is running
    if ! docker ps --filter "name=nuanced-lsp-proxy" --format "{{.Names}}" | grep -q "nuanced-lsp-proxy"; then
        echo -e "${RED}✗ Service failed to start${NC}"
        echo -e "${YELLOW}Showing logs:${NC}"
        docker logs nuanced-lsp-proxy
        exit 1
    fi

    echo -e "${GREEN}✓ Service is running${NC}"
    echo

    # Show info
    echo -e "${BLUE}Service Information:${NC}"
    echo -e "  Container: nuanced-lsp-proxy"
    echo -e "  URL:       http://localhost:${PORT}/v1"
    echo -e "  Health:    http://localhost:${PORT}/v1/system/health"
    echo -e "  Swagger:   http://localhost:${PORT}/swagger-ui/"
    echo

    # Wait longer for language containers to spawn
    echo -e "${BLUE}Initializing workspace and spawning language containers...${NC}"
    echo -e "${YELLOW}This may take 20-30 seconds...${NC}"

    # Monitor logs for "Workspace initialization complete"
    TIMEOUT=60
    ELAPSED=0
    while [ $ELAPSED -lt $TIMEOUT ]; do
        if docker logs nuanced-lsp-proxy 2>&1 | grep -q "Workspace initialization complete"; then
            echo -e "${GREEN}✓ Workspace initialization complete${NC}"
            break
        fi
        sleep 2
        ELAPSED=$((ELAPSED + 2))
        echo -n "."
    done
    echo

    if [ $ELAPSED -ge $TIMEOUT ]; then
        echo -e "${YELLOW}Warning: Timeout waiting for initialization${NC}"
        echo -e "${YELLOW}Check logs with: docker logs nuanced-lsp-proxy${NC}"
    fi

    # Show running containers
    echo -e "${BLUE}Running containers:${NC}"
    docker ps --filter "name=nuanced-lsp-" --format "  {{.Names}}\t({{.Status}})"
    echo

    echo -e "${BLUE}Useful commands:${NC}"
    echo -e "  View logs:        docker logs -f nuanced-lsp-proxy"
    echo -e "  Stop service:     docker rm -f nuanced-lsp-proxy"
    echo -e "  Test health:      curl http://localhost:${PORT}/v1/system/health | jq"
    echo

    # Tail logs if requested
    if [ "$TAIL_LOGS" = true ]; then
        echo -e "${BLUE}Tailing logs (Ctrl+C to exit):${NC}"
        docker logs -f nuanced-lsp-proxy
    fi
else
    # Foreground mode - logs will display automatically
    echo -e "${GREEN}Service running in foreground mode${NC}"
fi

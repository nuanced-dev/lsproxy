#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"

DEFAULT_LANGUAGE_TAG="$("$SCRIPT_DIR/util/language-image-version.sh")"
DEFAULT_SERVICE_TAG="$("$SCRIPT_DIR/util/service-image-version.sh")"

help() {
    echo "Test container lifecycle: build, run, health check, cleanup"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "  --service-tag=TAG     Tag of service images to use (default: $DEFAULT_SERVICE_TAG)"
    echo "  --help, -h            Show this help"
}

# Default values
LANGUAGE_TAG=""
SERVICE_TAG=""

# Parse options
for arg in "$@"; do
    case $arg in
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --service-tag=*)
            SERVICE_TAG="${arg#*=}"
            ;;
        --help|-h)
            help
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
    esac
done

# Fall back to default tags
SERVICE_TAG="${SERVICE_TAG:-$DEFAULT_SERVICE_TAG}"

WORKSPACE_PATH="$(cd "$SCRIPT_DIR/../sample_project/python" && pwd)"
SERVICE_NAME="nuanced-lsp-proxy-$(uuidgen | tr '[:upper:]' '[:lower:]' | cut -c1-12)"

# Flag to track if we started containers
CONTAINERS_STARTED=false

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Container Lifecycle Tests${NC}"
echo -e "${BLUE}  Workspace: $WORKSPACE_PATH${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

test_step() {
    local description="$1"
    local command="$2"

    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Testing: $description... "

    if eval "$command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Cleanup function
cleanup() {
    local exit_code=$?

    if [ "$CONTAINERS_STARTED" = true ]; then
        echo
        echo -e "${YELLOW}Cleaning up...${NC}"
        docker rm -f "${SERVICE_NAME}" 2>/dev/null || true

        # Give it a moment for language containers to stop
        sleep 2

        # Clean up any remaining language containers
        ORPHANS=$(docker ps -aq --filter "label=nuanced.parent=${PARENT_LABEL_ID}" 2>/dev/null || true)
        if [ -n "$ORPHANS" ]; then
            echo "$ORPHANS" | xargs docker rm -f > /dev/null 2>&1 || true
        fi

        echo -e "${GREEN}Cleanup complete${NC}"
    fi

    exit "$exit_code"
}

# Register cleanup on exit (success, failure, or Ctrl+C)
trap cleanup EXIT INT TERM

# Test 1: Service image exists
test_step "Service image exists" \
    "docker images nuanced-lsp-proxy:${SERVICE_TAG} --format '{{.Repository}}' | grep -q nuanced-lsp-proxy"

# Test 2: Start service container
echo
echo -e "${BLUE}Starting service container (${SERVICE_NAME})...${NC}"

DOCKER_ARGS=(
    "-v" "/var/run/docker.sock:/var/run/docker.sock"
    "-v" "${WORKSPACE_PATH}:/mnt/workspace"
    "-e" "RUST_LOG=info,nuanced_lsp_proxy=debug,proxy=debug,nuanced_lsp_wrapper=debug,wrapper=debug"
    "-e" "USE_AUTH=false"
)
if [ -n "$LANGUAGE_TAG" ]; then
    DOCKER_ARGS+=("-e" "LANGUAGE_IMAGE_VERSION=${LANGUAGE_TAG}")
fi
if [ -n "${WATCHDOG_IMAGE:+x}" ]; then
    DOCKER_ARGS+=("-e" "WATCHDOG_IMAGE=${WATCHDOG_IMAGE}")
fi
if [ -n "${WRAPPER_IMAGE:+x}" ]; then
    DOCKER_ARGS+=("-e" "WRAPPER_IMAGE=${WRAPPER_IMAGE}")
fi

PROXY_IMAGE="${PROXY_IMAGE:-nuanced-lsp-proxy:${SERVICE_TAG}}"

docker run -d \
    --name "${SERVICE_NAME}" \
    -p 4444:4444 \
    "${DOCKER_ARGS[@]}" \
    "$PROXY_IMAGE"

SERVICE_ID=$(docker inspect --format '{{.Id}}' "${SERVICE_NAME}" 2>/dev/null || echo "")
# Language containers label their parent with the orchestrator instance ID, which is the
# container hostname (12-char short ID) when running inside Docker.
SERVICE_INSTANCE_ID=$(docker inspect --format '{{.Config.Hostname}}' "${SERVICE_NAME}" 2>/dev/null || echo "")
PARENT_LABEL_ID="${SERVICE_INSTANCE_ID:-$(echo "$SERVICE_ID" | cut -c1-12)}"

CONTAINERS_STARTED=true

# Wait for service to be ready (poll /system/health up to 60s)
echo -e "${YELLOW}Waiting for service to initialize (up to 60s)...${NC}"
ready=false
for i in $(seq 1 60); do
    HEALTH=$(curl -sf http://localhost:4444/v1/system/health || true)
    STATUS=$(echo "$HEALTH" | jq -r '.status' 2>/dev/null || echo "")
    LANG_FAILED=$(echo "$HEALTH" | jq -r '.languages | to_entries[]? | select(.value == false) | .key' 2>/dev/null || true)

    if [ "$STATUS" = "ok" ] && [ -n "$LANG_FAILED" ]; then
        echo -e "${RED}✗ ERROR: Service failed to start languages:${NC}"
        echo -e "${RED}${LANG_FAILED}${NC}"
        exit 1
    fi

    if [ "$STATUS" = "ok" ]; then
        echo -e "${GREEN}✓ Service and languages healthy after ${i}s${NC}"
        ready=true
        break
    fi

    if (( i % 5 == 0 )); then
        echo -e "${YELLOW}  [${i}s] Status: $STATUS...${NC}"
    else
        printf "${YELLOW}.${NC}"
    fi
    sleep 1
done
echo
if [ "$ready" = false ]; then
    echo "Service did not become healthy within timeout"
    exit 1
fi

# Test 3: Service container is running
test_step "Service container running" \
    "docker ps --filter name=${SERVICE_NAME} --format '{{.Names}}' | grep -q ${SERVICE_NAME}"

# Test 4: Service health check
test_step "Service health check responds" \
    "curl -sf http://localhost:4444/v1/system/health > /dev/null"

# Test 5: Verify language containers were spawned
echo
echo -e "${BLUE}Checking language containers...${NC}"
CONTAINER_COUNT=$(docker ps --filter "label=nuanced.parent=${PARENT_LABEL_ID}" --filter "label=nuanced.role=language-server" --format '{{.Names}}' | wc -l | tr -d ' ')
echo "Language containers running: $CONTAINER_COUNT"

if [ "$CONTAINER_COUNT" -ge 1 ]; then
    echo -e "${GREEN}✓ Language containers spawned${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    docker ps --filter "label=nuanced.parent=${PARENT_LABEL_ID}" --filter "label=nuanced.role=language-server" --format "  - {{.Names}} ({{.Status}})"
else
    echo -e "${RED}✗ No language containers found${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi
TESTS_RUN=$((TESTS_RUN + 1))

# Test 6: Test an actual API endpoint
test_step "List files endpoint works" \
    "curl -sf http://localhost:4444/v1/workspace/list-files | jq -e 'type == \"array\"' > /dev/null"

# Test 7: Test language-specific endpoint
test_step "Python language works" \
    "curl -sf -X POST http://localhost:4444/v1/workspace/read-source-code \
        -H 'Content-Type: application/json' \
        -d '{\"path\":\"main.py\"}' | jq -e '.source_code | length > 0' > /dev/null"

# Test 8: Stop service and verify cleanup
echo
echo -e "${BLUE}Testing cleanup...${NC}"
docker stop "${SERVICE_NAME}"
echo -e "${YELLOW}Waiting for watchdog to clean up language containers (up to 60s)...${NC}"
cleanup_ready=false
for i in $(seq 1 60); do
    running=$(docker ps --filter "label=nuanced.parent=$PARENT_LABEL_ID" --filter "label=nuanced.role=language-server" -q | wc -l | tr -d ' ')
    if [ "$running" -eq 0 ]; then
        echo -e "${GREEN}Cleanup complete after ${i}s${NC}"
        cleanup_ready=true
        break
    fi
    if (( i % 5 == 0 )); then
        echo -e "${YELLOW}  [${i}s] Waiting on ${running} language container(s)...${NC}"
    else
        printf "${YELLOW}.${NC}"
    fi
    sleep 1
done
echo
if [ "$cleanup_ready" = false ]; then
    echo -e "${RED}✗ Watchdog did not clean up language containers within timeout${NC}"
    exit 1
fi

test_step "Language containers stopped" \
    "[ \$(docker ps --filter \"label=nuanced.parent=$PARENT_LABEL_ID\" --filter \"label=nuanced.role=language-server\" -q | wc -l) -eq 0 ]"

# Summary
echo
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Container Lifecycle Test Summary${NC}"
echo -e "${BLUE}=========================================${NC}"
echo "Tests Run:    $TESTS_RUN"
echo -e "${GREEN}Passed:       $TESTS_PASSED${NC}"
echo -e "${RED}Failed:       $TESTS_FAILED${NC}"
echo -e "Success Rate: $(awk "BEGIN {printf \"%.1f%%\", ($TESTS_PASSED / $TESTS_RUN) * 100}")"
echo -e "${BLUE}=========================================${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All lifecycle tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some lifecycle tests failed${NC}"
    exit 1
fi

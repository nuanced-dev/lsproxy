#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"

help() {
    echo "Test watchdog container functionality"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "Options:"
    echo "  --language-tag=TAG    Tag of language images to use (default: $DEFAULT_LANGUAGE_TAG)"
    echo "  --registry=REG        Container registry for service images (default: none)"
    echo "  --service-tag=TAG     Tag of service images to use (default: $DEFAULT_SERVICE_TAG)"
    echo "  --help, -h            Show this help"
    echo ""
    echo "Tests: watchdog spawning, clean shutdown, SIGKILL cleanup, multiple instances"
}

# Default values
LANGUAGE_TAG=""
SERVICE_TAG=""
REGISTRY=""

# Parse options
for arg in "$@"; do
    case $arg in
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
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
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
    esac
done

# Configuration
WORKSPACE_PATH="$(cd "$SCRIPT_DIR/../sample_project/python" && pwd)"

DOCKER_ARGS=(
    "-v" "/var/run/docker.sock:/var/run/docker.sock"
    "-v" "${WORKSPACE_PATH}:/mnt/workspace"
    "-e" "RUST_LOG=info,nuanced_lsp_proxy=debug,proxy=debug,nuanced_lsp_wrapper=debug,wrapper=debug"
    "-e" "USE_AUTH=false"
)
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

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Watchdog Functionality Tests${NC}"
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

# Wait for a service to report healthy (all languages ready) on the given port
wait_for_service_ready() {
    local port="$1"
    local timeout="${2:-60}"

    echo -e "${YELLOW}Waiting for service to initialize (up to ${timeout}s)...${NC}"
    local ready=false
    for i in $(seq 1 "$timeout"); do
        HEALTH=$(curl -sf http://localhost:$port/v1/system/health || true)
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
        echo -e "${RED}Service did not become healthy within timeout${NC}"
        return 1
    fi
}

# Wait for containers of a given role/parent to disappear
wait_for_child_cleanup() {
    local parent_label="$1"
    local role="$2"
    local timeout="${3:-60}"
    local ready=false

    echo -e "${YELLOW}Waiting for watchdog to clean up ${role} containers (up to ${timeout}s)...${NC}"
    for i in $(seq 1 "$timeout"); do
        running=$(docker ps --filter "label=nuanced.parent=$parent_label" --filter "label=nuanced.role=${role}" -q | wc -l | tr -d ' ')
        if [ "$running" -eq 0 ]; then
            echo -e "${GREEN}Cleanup complete after ${i}s${NC}"
            ready=true
            break
        fi

        if (( i % 5 == 0 )); then
            echo -e "${YELLOW}  [${i}s] Waiting on ${running} ${role} container(s)...${NC}"
        else
            printf "${YELLOW}.${NC}"
        fi
        sleep 1
    done
    echo

    if [ "$ready" = false ]; then
        echo -e "${RED}Cleanup did not complete within timeout${NC}"
        return 1
    fi
}

# Wait for watchdog container to disappear
wait_for_watchdog_cleanup() {
    local parent_label="$1"
    local timeout="${2:-30}"
    local ready=false

    echo -e "${YELLOW}Waiting for watchdog to clean itself up (up to ${timeout}s)...${NC}"
    for i in $(seq 1 "$timeout"); do
        running=$(docker ps -a --filter "name=nuanced-lsp-watchdog-${parent_label}" -q | wc -l | tr -d ' ')
        if [ "$running" -eq 0 ]; then
            echo -e "${GREEN}Watchdog removed after ${i}s${NC}"
            ready=true
            break
        fi
        if (( i % 5 == 0 )); then
            echo -e "${YELLOW}  [${i}s] Waiting on watchdog container...${NC}"
        else
            printf "${YELLOW}.${NC}"
        fi
        sleep 1
    done
    echo

    if [ "$ready" = false ]; then
        echo -e "${RED}Watchdog did not remove itself within timeout${NC}"
        return 1
    fi
}

# Get the value used for the nuanced.parent label (hostname inside the container)
get_parent_label_id() {
    local container_name="$1"
    local hostname
    hostname=$(docker inspect --format '{{.Config.Hostname}}' "$container_name" 2>/dev/null || echo "")
    if [ -n "$hostname" ]; then
        echo "$hostname"
        return
    fi

    local id
    id=$(docker inspect --format '{{.Id}}' "$container_name" 2>/dev/null || echo "")
    if [ -n "$id" ]; then
        echo "${id:0:12}"
    fi
}

# Cleanup function
cleanup() {
    local exit_code=$?

    echo
    echo -e "${YELLOW}Cleaning up test containers...${NC}"
    docker rm -f test-watchdog-svc test-watchdog-kill test-watchdog-multi1 test-watchdog-multi2 2>/dev/null || true

    # Clean up any orphaned containers
    ORPHANS=$(docker ps -aq --filter "name=nuanced-lsp-" 2>/dev/null || true)
    if [ -n "$ORPHANS" ]; then
        echo "$ORPHANS" | xargs docker rm -f > /dev/null 2>&1 || true
    fi

    echo -e "${GREEN}Cleanup complete${NC}"
    exit "$exit_code"
}

# Register cleanup on exit
trap cleanup EXIT INT TERM

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}Test 1: Watchdog Spawning${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Start service
echo -e "${BLUE}Starting service container...${NC}"

docker run -d \
    --name test-watchdog-svc \
    -p 4455:4444 \
    "${DOCKER_ARGS[@]}" \
    "$PROXY_IMAGE" > /dev/null

wait_for_service_ready 4455

SERVICE_PARENT_LABEL=$(get_parent_label_id test-watchdog-svc)

test_step "Service container is running" \
    "docker ps --filter 'name=test-watchdog-svc' --format '{{.Names}}' | grep -q test-watchdog-svc"

test_step "Watchdog container is spawned" \
    "docker ps --filter 'name=nuanced-lsp-watchdog-$SERVICE_PARENT_LABEL' --format '{{.Names}}' | grep -q nuanced-lsp-watchdog"

test_step "Language containers are spawned" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$SERVICE_PARENT_LABEL' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l) -ge 1 ]"

test_step "Wrapper container is spawned" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$SERVICE_PARENT_LABEL' --filter 'label=nuanced.role=wrapper' --format '{{.Names}}' | wc -l | tr -d ' ') -ge 1 ]"

test_step "Language containers have parent labels" \
    "[ \$(docker ps -q --filter \"label=nuanced.parent=$SERVICE_PARENT_LABEL\" --filter 'label=nuanced.role=language-server' | wc -l) -ge 1 ] && docker inspect \$(docker ps -q --filter \"label=nuanced.parent=$SERVICE_PARENT_LABEL\" --filter 'label=nuanced.role=language-server' | head -1) --format '{{.Config.Labels}}' | grep -q \"nuanced.parent:$SERVICE_PARENT_LABEL\""

# Note: LSP wrapper readiness is thoroughly tested by the integration tests
# which perform actual LSP operations through the service. The watchdog tests
# focus on container lifecycle management.

echo
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}Test 2: Clean Shutdown${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Stop service cleanly
echo -e "${BLUE}Stopping service with SIGTERM...${NC}"
docker stop test-watchdog-svc > /dev/null
wait_for_child_cleanup "$SERVICE_PARENT_LABEL" "language-server" 60
wait_for_child_cleanup "$SERVICE_PARENT_LABEL" "wrapper" 60
wait_for_watchdog_cleanup "$SERVICE_PARENT_LABEL" 30

test_step "Service container stopped" \
    "[ \$(docker ps --filter 'name=test-watchdog-svc' --format '{{.Names}}' | wc -l) -eq 0 ]"

test_step "Language containers cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$SERVICE_PARENT_LABEL' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "Wrapper container cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$SERVICE_PARENT_LABEL' --filter 'label=nuanced.role=wrapper' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "Watchdog auto-removed after clean shutdown" \
    "[ \$(docker ps -a --filter 'name=nuanced-lsp-watchdog-$SERVICE_PARENT_LABEL' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

# Remove stopped service container
docker rm test-watchdog-svc > /dev/null 2>&1 || true

echo
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}Test 3: SIGKILL Emergency Cleanup${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Start new service
echo -e "${BLUE}Starting service container...${NC}"
docker run -d \
    --name test-watchdog-kill \
    -p 4456:4444 \
    "${DOCKER_ARGS[@]}" \
    "$PROXY_IMAGE" > /dev/null

wait_for_service_ready 4456

KILL_PARENT_LABEL=$(get_parent_label_id test-watchdog-kill)

# Count language containers before kill
BEFORE_COUNT=$(docker ps --filter "label=nuanced.parent=$KILL_PARENT_LABEL" --filter "label=nuanced.role=language-server" --format '{{.Names}}' | wc -l | tr -d ' ')

echo -e "${BLUE}Language containers before SIGKILL: $BEFORE_COUNT${NC}"

test_step "Language containers exist before SIGKILL" \
    "[ $BEFORE_COUNT -gt 0 ]"

# SIGKILL the service
echo -e "${BLUE}Sending SIGKILL to service...${NC}"
docker kill test-watchdog-kill > /dev/null

wait_for_child_cleanup "$KILL_PARENT_LABEL" "language-server" 60
wait_for_child_cleanup "$KILL_PARENT_LABEL" "wrapper" 60
wait_for_watchdog_cleanup "$KILL_PARENT_LABEL" 30

test_step "Language containers cleaned up by watchdog" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$KILL_PARENT_LABEL' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "Wrapper container cleaned up by watchdog" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$KILL_PARENT_LABEL' --filter 'label=nuanced.role=wrapper' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "Watchdog auto-removed after emergency cleanup" \
    "[ \$(docker ps -a --filter 'name=nuanced-lsp-watchdog-$KILL_PARENT_LABEL' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

# Verify watchdog logged the cleanup
docker rm test-watchdog-kill > /dev/null 2>&1 || true

echo
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}Test 4: Multiple Service Instances${NC}"
echo -e "${BLUE}=========================================${NC}"
echo

# Start two services
echo -e "${BLUE}Starting two service instances...${NC}"
docker run -d \
    --name test-watchdog-multi1 \
    -p 4457:4444 \
    "${DOCKER_ARGS[@]}" \
    "$PROXY_IMAGE" > /dev/null

docker run -d \
    --name test-watchdog-multi2 \
    -p 4458:4444 \
    "${DOCKER_ARGS[@]}" \
    "$PROXY_IMAGE" > /dev/null

wait_for_service_ready 4457 120
wait_for_service_ready 4458 120

MULTI1_PARENT=$(get_parent_label_id test-watchdog-multi1)
MULTI2_PARENT=$(get_parent_label_id test-watchdog-multi2)

test_step "Both watchdogs are running" \
    "[ \$(docker ps --filter 'name=nuanced-lsp-watchdog-' --format '{{.Names}}' | wc -l) -eq 2 ]"

test_step "Service 1 has language containers" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$MULTI1_PARENT' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l) -gt 0 ]"

test_step "Service 2 has language containers" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$MULTI2_PARENT' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l) -gt 0 ]"

test_step "Both services have wrapper containers" \
    "[ \$(docker ps --filter 'label=nuanced.role=wrapper' --filter 'label=nuanced.parent=$MULTI1_PARENT' -q | wc -l | tr -d ' ') -ge 1 ] && \
     [ \$(docker ps --filter 'label=nuanced.role=wrapper' --filter 'label=nuanced.parent=$MULTI2_PARENT' -q | wc -l | tr -d ' ') -ge 1 ]"

# Kill first service
echo -e "${BLUE}Killing first service instance...${NC}"
docker kill test-watchdog-multi1 > /dev/null
wait_for_child_cleanup "$MULTI1_PARENT" "language-server" 60
wait_for_child_cleanup "$MULTI1_PARENT" "wrapper" 60
wait_for_watchdog_cleanup "$MULTI1_PARENT" 30

test_step "Service 1 containers cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$MULTI1_PARENT' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "Service 1 wrapper cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$MULTI1_PARENT' --filter 'label=nuanced.role=wrapper' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "Service 2 containers still running" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$MULTI2_PARENT' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l) -gt 0 ]"

test_step "Service 2 watchdog still running" \
    "docker ps --filter 'name=nuanced-lsp-watchdog-$MULTI2_PARENT' --format '{{.Names}}' | grep -q nuanced-lsp-watchdog"

# Clean up second service
echo -e "${BLUE}Stopping second service instance...${NC}"
docker stop test-watchdog-multi2 > /dev/null
wait_for_child_cleanup "$MULTI2_PARENT" "language-server" 60
wait_for_child_cleanup "$MULTI2_PARENT" "wrapper" 60
wait_for_watchdog_cleanup "$MULTI2_PARENT" 30

test_step "Service 2 containers cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$MULTI2_PARENT' --filter 'label=nuanced.role=language-server' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "Service 2 wrapper cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$MULTI2_PARENT' --filter 'label=nuanced.role=wrapper' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

test_step "All watchdogs auto-removed" \
    "[ \$(docker ps -a --filter 'name=nuanced-lsp-watchdog-' --format '{{.Names}}' | wc -l | tr -d ' ') -eq 0 ]"

# Summary
echo
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Watchdog Test Summary${NC}"
echo -e "${BLUE}=========================================${NC}"
echo "Tests Run:    $TESTS_RUN"
echo -e "${GREEN}Passed:       $TESTS_PASSED${NC}"
echo -e "${RED}Failed:       $TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${GREEN}✓ All watchdog tests passed!${NC}"
    exit 0
else
    echo -e "${BLUE}=========================================${NC}"
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi

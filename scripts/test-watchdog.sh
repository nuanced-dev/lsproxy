#!/usr/bin/env bash

set -e

# Test watchdog container functionality
# Tests: watchdog spawning, clean shutdown, SIGKILL cleanup, multiple instances

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

WORKSPACE_PATH="${1:-sample_project/all}"
WORKSPACE_PATH="$(cd "$WORKSPACE_PATH" && pwd)"

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
    exit $exit_code
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
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$WORKSPACE_PATH:/mnt/workspace" \
    -e RUST_LOG=info \
    -e USE_AUTH=false \
    nuanced-lsp-proxy:latest > /dev/null

# Wait for initialization (with health checks for all language containers)
echo "Waiting for service to initialize (60s)..."
sleep 60

# Get service container ID
SERVICE_ID=$(docker ps --filter "name=test-watchdog-svc" --format "{{.ID}}")
SERVICE_SHORT_ID="${SERVICE_ID:0:12}"

test_step "Service container is running" \
    "docker ps --filter 'name=test-watchdog-svc' --format '{{.Names}}' | grep -q test-watchdog-svc"

test_step "Watchdog container is spawned" \
    "docker ps --filter 'name=nuanced-lsp-watchdog-$SERVICE_SHORT_ID' --format '{{.Names}}' | grep -q nuanced-lsp-watchdog"

test_step "Language containers are spawned" \
    "[ \$(docker ps --filter 'name=nuanced-lsp-python' --filter 'name=nuanced-lsp-rust' --filter 'name=nuanced-lsp-typescript' --format '{{.Names}}' | wc -l) -ge 3 ]"

test_step "Language containers have parent labels" \
    "docker inspect \$(docker ps -q --filter 'name=nuanced-lsp-python' | head -1) --format '{{.Config.Labels}}' | grep -q 'nuanced.parent:$SERVICE_SHORT_ID'"

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
# Cleanup can take 7-10 seconds with many language containers
sleep 10

test_step "Service container stopped" \
    "[ \$(docker ps --filter 'name=test-watchdog-svc' --format '{{.Names}}' | wc -l) -eq 0 ]"

test_step "Language containers cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$SERVICE_SHORT_ID' --format '{{.Names}}' | wc -l) -eq 0 ]"

test_step "Watchdog auto-removed after clean shutdown" \
    "[ \$(docker ps -a --filter 'name=nuanced-lsp-watchdog-$SERVICE_SHORT_ID' --format '{{.Names}}' | wc -l) -eq 0 ]"

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
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$WORKSPACE_PATH:/mnt/workspace" \
    -e RUST_LOG=info \
    -e USE_AUTH=false \
    nuanced-lsp-proxy:latest > /dev/null

echo "Waiting for service to initialize (60s)..."
sleep 60

KILL_SERVICE_ID=$(docker ps --filter "name=test-watchdog-kill" --format "{{.ID}}")
KILL_SHORT_ID="${KILL_SERVICE_ID:0:12}"

# Count language containers before kill
BEFORE_COUNT=$(docker ps --filter "label=nuanced.parent=$KILL_SHORT_ID" --format '{{.Names}}' | wc -l | tr -d ' ')

echo -e "${BLUE}Language containers before SIGKILL: $BEFORE_COUNT${NC}"

test_step "Language containers exist before SIGKILL" \
    "[ $BEFORE_COUNT -gt 0 ]"

# SIGKILL the service
echo -e "${BLUE}Sending SIGKILL to service...${NC}"
docker kill test-watchdog-kill > /dev/null

# Wait for watchdog to detect and cleanup
echo "Waiting for watchdog to detect death and cleanup (15s)..."
sleep 15

test_step "Language containers cleaned up by watchdog" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$KILL_SHORT_ID' --format '{{.Names}}' | wc -l) -eq 0 ]"

test_step "Watchdog auto-removed after emergency cleanup" \
    "[ \$(docker ps -a --filter 'name=nuanced-lsp-watchdog-$KILL_SHORT_ID' --format '{{.Names}}' | wc -l) -eq 0 ]"

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
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$WORKSPACE_PATH:/mnt/workspace" \
    -e RUST_LOG=warn \
    -e USE_AUTH=false \
    nuanced-lsp-proxy:latest > /dev/null

docker run -d \
    --name test-watchdog-multi2 \
    -p 4458:4444 \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$WORKSPACE_PATH:/mnt/workspace" \
    -e RUST_LOG=warn \
    -e USE_AUTH=false \
    nuanced-lsp-proxy:latest > /dev/null

echo "Waiting for both services to initialize (180s)..."
sleep 180

MULTI1_ID=$(docker ps --filter "name=test-watchdog-multi1" --format "{{.ID}}")
MULTI1_SHORT="${MULTI1_ID:0:12}"
MULTI2_ID=$(docker ps --filter "name=test-watchdog-multi2" --format "{{.ID}}")
MULTI2_SHORT="${MULTI2_ID:0:12}"

test_step "Both watchdogs are running" \
    "[ \$(docker ps --filter 'name=nuanced-lsp-watchdog-' --format '{{.Names}}' | wc -l) -eq 2 ]"

test_step "Service 1 has language containers" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$MULTI1_SHORT' --format '{{.Names}}' | wc -l) -gt 0 ]"

test_step "Service 2 has language containers" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$MULTI2_SHORT' --format '{{.Names}}' | wc -l) -gt 0 ]"

# Kill first service
echo -e "${BLUE}Killing first service instance...${NC}"
docker kill test-watchdog-multi1 > /dev/null
echo "Waiting for watchdog to cleanup (15s)..."
sleep 15

test_step "Service 1 containers cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$MULTI1_SHORT' --format '{{.Names}}' | wc -l) -eq 0 ]"

test_step "Service 2 containers still running" \
    "[ \$(docker ps --filter 'label=nuanced.parent=$MULTI2_SHORT' --format '{{.Names}}' | wc -l) -gt 0 ]"

test_step "Service 2 watchdog still running" \
    "docker ps --filter 'name=nuanced-lsp-watchdog-$MULTI2_SHORT' --format '{{.Names}}' | grep -q nuanced-lsp-watchdog"

# Clean up second service
echo -e "${BLUE}Stopping second service instance...${NC}"
docker stop test-watchdog-multi2 > /dev/null
# Cleanup can take 7-10 seconds with many language containers
sleep 10

test_step "Service 2 containers cleaned up" \
    "[ \$(docker ps -a --filter 'label=nuanced.parent=$MULTI2_SHORT' --format '{{.Names}}' | wc -l) -eq 0 ]"

test_step "All watchdogs auto-removed" \
    "[ \$(docker ps -a --filter 'name=nuanced-lsp-watchdog-' --format '{{.Names}}' | wc -l) -eq 0 ]"

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

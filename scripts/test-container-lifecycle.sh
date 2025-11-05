#!/bin/bash

set -e

# Test container lifecycle: build, run, health check, cleanup
# Usage: ./scripts/test-container-lifecycle.sh [workspace_path]

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

WORKSPACE_PATH="${1:-sample_project/all}"
WORKSPACE_PATH="$(cd "$WORKSPACE_PATH" && pwd)"

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
        docker rm -f lsproxy-service 2>/dev/null || true

        # Give it a moment for language containers to stop
        sleep 2

        # Clean up any remaining language containers
        ORPHANS=$(docker ps -aq --filter "name=lsproxy-" 2>/dev/null || true)
        if [ -n "$ORPHANS" ]; then
            echo "$ORPHANS" | xargs docker rm -f > /dev/null 2>&1 || true
        fi

        docker network rm lsproxy-network 2>/dev/null || true
        echo -e "${GREEN}Cleanup complete${NC}"
    fi

    exit $exit_code
}

# Register cleanup on exit (success, failure, or Ctrl+C)
trap cleanup EXIT INT TERM

# Test 1: Service image exists
test_step "Service image exists" \
    "docker images lsproxy-service:latest --format '{{.Repository}}' | grep -q lsproxy-service"

# Test 2: Start service container
echo
echo -e "${BLUE}Starting service container...${NC}"
docker run -d \
    --name lsproxy-service \
    -p 4444:4444 \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$WORKSPACE_PATH:/mnt/workspace" \
    -e HOST_WORKSPACE_PATH="$WORKSPACE_PATH" \
    -e RUST_LOG=info \
    -e USE_AUTH=false \
    lsproxy-service:latest

CONTAINERS_STARTED=true

# Wait for service to be ready
echo "Waiting for service to initialize (30s)..."
sleep 30

# Test 3: Service container is running
test_step "Service container running" \
    "docker ps --filter name=lsproxy-service --format '{{.Names}}' | grep -q lsproxy-service"

# Test 4: Service health check
test_step "Service health check responds" \
    "curl -sf http://localhost:4444/v1/system/health > /dev/null"

# Test 5: Verify language containers were spawned
echo
echo -e "${BLUE}Checking language containers...${NC}"
CONTAINER_COUNT=$(docker ps --filter "name=lsproxy-" --format '{{.Names}}' | wc -l | tr -d ' ')
echo "Language containers running: $CONTAINER_COUNT"

if [ $CONTAINER_COUNT -gt 1 ]; then
    echo -e "${GREEN}✓ Language containers spawned${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    docker ps --filter "name=lsproxy-" --format "  - {{.Names}} ({{.Status}})"
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
        -d '{\"path\":\"main.py\"}' | jq -e '.content | length > 0' > /dev/null"

# Test 8: Check Docker network
test_step "LSProxy Docker network exists" \
    "docker network ls --format '{{.Name}}' | grep -q lsproxy"

# Test 9: Stop service and verify cleanup
echo
echo -e "${BLUE}Testing cleanup...${NC}"
docker stop lsproxy-service
sleep 5

test_step "Language containers stopped" \
    "[ \$(docker ps --filter 'name=lsproxy-' --format '{{.Names}}' | wc -l) -eq 0 ]"

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

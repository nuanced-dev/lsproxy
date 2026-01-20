#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" && pwd)"

source "$SCRIPT_DIR/include/colors.sh"
source "$SCRIPT_DIR/include/constants.sh"
source "$SCRIPT_DIR/include/lib.sh"

usage() {
    echo "Usage: $0 [--language-tag=TAG] [--no-cleanup] [--service-tag=TAG]"
}

help() {
    echo "Comprehensive Nuanced LSP test script that validates all endpoints for all languages"
    echo ""
    echo "Usage: $0 [OPTIONS...]"
    echo ""
    echo "This script automatically starts Nuanced LSP if it's not already running."
    echo "If the service is already running, it uses the existing containers."
    echo ""
    echo "Options:"
    echo "  --language-tag=TAG    Tag of language images to use"
    echo "  --no-cleanup          Don't stop containers after tests (useful for debugging)"
    echo "  --service-tag=TAG     Tag images with specified tag (default: $DEFAULT_SERVICE_TAG)"
    echo "  --help, -h            Show this help"
    echo ""
    echo "Behavior:"
    echo "  - If service is NOT running: Starts containers, runs tests, stops containers"
    echo "  - If service IS running: Runs tests, leaves containers running"
    echo "  - With --no-cleanup: Runs tests, always leaves containers running"
}

# Default values
LANGUAGE_TAG=""
CLEANUP_ON_EXIT=true
SERVICE_TAG=""

# Parse options
for arg in "$@"; do
    case $arg in
        --language-tag=*)
            LANGUAGE_TAG="${arg#*=}"
            ;;
        --no-cleanup)
            CLEANUP_ON_EXIT=false
            ;;
        --service-tag=*)
            SERVICE_TAG="${arg#*=}"
            ;;
        -h|--help)
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
BASE_URL="http://localhost:4444"
WORKSPACE_PATH="$(cd "$SCRIPT_DIR/../sample_project/all" && pwd)"

# Counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Track if we started the service (to know if we should clean it up)
STARTED_SERVICE=false

# Workspace URI for testing LSP endpoint
WORKSPACE_URI="file://$(realpath "$WORKSPACE_PATH")"

# Check required commands
if ! missing=$(has_commands curl docker websocat); then
    echo -e "${RED}Missing required commands: $missing${NC}"
    exit 1
fi

# Cleanup function
cleanup() {
    local exit_code=$?

    # Only cleanup if we started the service AND cleanup is enabled
    if [ "$CLEANUP_ON_EXIT" = true ] && [ "$STARTED_SERVICE" = true ]; then
        echo
        echo -e "${BLUE}=========================================${NC}"
        echo -e "${BLUE}  Cleaning up containers...${NC}"
        echo -e "${BLUE}=========================================${NC}"

        # Stop service container (language containers stop automatically)
        if docker ps -q --filter "name=nuanced-lsp-proxy" > /dev/null 2>&1; then
            docker rm -f nuanced-lsp-proxy > /dev/null 2>&1 || true
            echo -e "${GREEN}✓ Containers stopped${NC}"
        fi

        # Clean up any orphaned language containers
        ORPHANS=$(docker ps -aq --filter "name=nuanced-lsp-" 2>/dev/null || true)
        if [ -n "$ORPHANS" ]; then
            echo "$ORPHANS" | xargs docker rm -f > /dev/null 2>&1 || true
            echo -e "${GREEN}✓ Orphaned containers cleaned${NC}"
        fi
    elif [ "$CLEANUP_ON_EXIT" = false ]; then
        echo
        echo -e "${YELLOW}Skipping cleanup (--no-cleanup specified)${NC}"
        echo -e "${YELLOW}To clean up manually, run: ./scripts/stop-proxy.sh --force${NC}"
    elif [ "$STARTED_SERVICE" = false ]; then
        echo
        echo -e "${YELLOW}Leaving existing containers running (tests used pre-existing service)${NC}"
    fi

    exit "$exit_code"
}

# Register cleanup on exit (success, failure, or Ctrl+C)
trap cleanup EXIT INT TERM

# Language configurations
# Format: language_key test_file symbol_name symbol_line symbol_char health_key
LANGUAGE_CONFIGS="
cpp|astar_search.cpp|main|2|4|cpp
csharp|Program.cs|Main|4|20|csharp
go|main.go|main|7|5|golang
java|Main.java|main|5|23|java
javascript|src/main.ts|main|5|6|typescript_javascript
php|AStar.php|findPathTo|26|20|php
python|main.py|main|14|4|python
ruby|main.rb|main|35|4|ruby_3_4_4
rust|src/main.rs|main|10|3|rust
typescript|src/main.ts|main|5|6|typescript_javascript
"

# Deep validation for find-referenced-symbols (ast-grep backed)
# Format: language|file|line|char|min_workspace_symbols|expected_symbol_names
#
# Note: Expectations are set based on actual ast-grep behavior with full_scan:false
#
# Language Support Status (find-referenced-symbols requires ast-grep reference rules):
# ✓ WORKING (4 languages) - Have ast-grep reference rules configured:
#   - Python: Excellent support with decorator and function-call rules
#   - TypeScript: Good support with component-render, function-call, decorator rules
#   - C#: Working with class-instantiation and function-call rules
#   - PHP: Working with attribute-usage and function-call rules
#
# ✗ NOT WORKING (5 languages) - Missing ast-grep reference rules:
#   - Ruby: No reference rules configured in ast_grep_configs/reference/rules/
#   - Go: No reference rules configured
#   - Rust: No reference rules configured
#   - Java: No reference rules configured
#   - C/C++ (clangd): No reference rules configured
#
# Note: All languages have identifier and symbol rules, but find-referenced-symbols
# specifically requires reference rules to find symbol usages within a method body.
FIND_REF_TESTS="
csharp|AStar.cs|23|27|1|AddNeighborsToOpenList
php|AStar.php|26|20|1|addNeighborsToOpenList
python|main.py|14|4|1|AStarGraph
typescript|src/astar.ts|60|12|2|isInBounds,isWalkable
"

# Tests that are not working (commented out - need ast-grep reference rules)
# clangd|astar_search.cpp|??|??|1|??
# golang|golang_astar/astar.go|??|??|1|??
# java|AStar.java|39|22|1|??
# ruby|search.rb|31|15|1|initialize_search
# rust|src/astar.rs|??|??|1|??
#"

test_http_endpoint() {
    local test_name="$1"
    local method="$2"
    local endpoint="$3"
    local data="$4"
    local expected_status="${5:-200}"
    local validation_check="$6"

    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    echo -n "  Testing $test_name... "

    # Build curl command with timeout
    local curl_cmd=("curl" "-s" "-w" '\n%{http_code}' "--max-time" "30" "-X" "$method")
    if [ -n "$data" ]; then
        curl_cmd+=("-H" "Content-Type: application/json" "-d" "$data")
    fi
    curl_cmd+=("$BASE_URL$endpoint")

    # Execute request (curl has built-in timeout via --max-time)
    if response=$("${curl_cmd[@]}" 2>&1); then
        # Split response body and status code
        local body status
        body=$(echo "$response" | sed '$d')
        status=$(echo "$response" | tail -n 1)

        # Validate HTTP status code
        if [ "$status" != "$expected_status" ]; then
            echo -e "${RED}✗ FAIL${NC} - Expected status $expected_status, got $status"
            echo "    Response: $body" | head -3
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Validate JSON structure
        if ! echo "$body" | jq . > /dev/null 2>&1; then
            echo -e "${RED}✗ FAIL${NC} - Invalid JSON response"
            echo "    Response: $body" | head -3
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Run custom validation if provided
        if [ -n "$validation_check" ]; then
            if ! echo "$body" | eval "$validation_check"; then
                echo -e "${RED}✗ FAIL${NC} - Validation check failed"
                echo "    Check: $validation_check"
                echo "    Response: $body" | head -5
                FAILED_TESTS=$((FAILED_TESTS + 1))
                return 1
            fi
        fi

        echo -e "${GREEN}✓ PASS${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 28 ]; then
            echo -e "${RED}✗ FAIL${NC} - Timeout (30s)"
        else
            echo -e "${RED}✗ FAIL${NC} - Request failed (exit code: $exit_code)"
            echo "    Error: $response" | head -3
        fi
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

test_ws_endpoint() {
    local test_name="$1"
    local endpoint="$2"
    local data="$3"
    local validation_check="$4"

    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    echo -n "  Testing $test_name... "

    # Build curl command with timeout
    local ws_cmd=("websocat" "-q1" "ws${BASE_URL#http}$endpoint")

    # Execute request
    if response=$(echo "$data" | timeout 30 "${ws_cmd[@]}" 2>&1); then
        local body="$response"

        # Validate JSON structure
        if ! echo "$body" | jq . > /dev/null 2>&1; then
            echo -e "${RED}✗ FAIL${NC} - Invalid JSON response"
            echo "    Response: $body" | head -3
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Run custom validation if provided
        if [ -n "$validation_check" ]; then
            if ! echo "$body" | eval "$validation_check"; then
                echo -e "${RED}✗ FAIL${NC} - Validation check failed"
                echo "    Check: $validation_check"
                echo "    Response: $body" | head -5
                FAILED_TESTS=$((FAILED_TESTS + 1))
                return 1
            fi
        fi

        echo -e "${GREEN}✓ PASS${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 28 ]; then
            echo -e "${RED}✗ FAIL${NC} - Timeout (30s)"
        else
            echo -e "${RED}✗ FAIL${NC} - Request failed (exit code: $exit_code)"
            echo "    Error: $response" | head -3
        fi
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

test_find_referenced_symbols_enhanced() {
    local lang="$1"
    local file="$2"
    local line="$3"
    local char="$4"
    local min_workspace="$5"
    local expected_names="$6"

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -n "  Testing Find Referenced Symbols ($lang) [deep validation]... "

    # Build request data
    local data="{\"identifier_position\":{\"path\":\"$file\",\"position\":{\"line\":$line,\"character\":$char}},\"full_scan\":false}"

    # Make request
    local curl_cmd="curl -s -w '\n%{http_code}' --max-time 30 -X POST -H 'Content-Type: application/json' -d '$data' '$BASE_URL/v1/symbol/find-referenced-symbols'"

    if response=$(eval "$curl_cmd" 2>&1); then
        # Split response body and status code
        local body status
        body=$(echo "$response" | sed '$d')
        status=$(echo "$response" | tail -n 1)

        # Validate HTTP status code
        if [ "$status" != "200" ]; then
            echo -e "${RED}✗ FAIL${NC} - Expected status 200, got $status"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Validate JSON structure
        if ! echo "$body" | jq . > /dev/null 2>&1; then
            echo -e "${RED}✗ FAIL${NC} - Invalid JSON"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Check for error response
        if echo "$body" | jq -e '.error' > /dev/null 2>&1; then
            echo -e "${RED}✗ FAIL${NC} - Error in response"
            echo "    $(echo "$body" | jq -r '.error' | head -1)"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Check required fields exist
        if ! echo "$body" | jq -e '.workspace_symbols' > /dev/null 2>&1; then
            echo -e "${RED}✗ FAIL${NC} - Missing workspace_symbols field"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        if ! echo "$body" | jq -e '.external_symbols' > /dev/null 2>&1; then
            echo -e "${RED}✗ FAIL${NC} - Missing external_symbols field"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Check workspace_symbols count
        local workspace_count
        workspace_count=$(echo "$body" | jq '.workspace_symbols | length')
        if [ "$workspace_count" -lt "$min_workspace" ]; then
            echo -e "${RED}✗ FAIL${NC} - Expected ≥${min_workspace} workspace symbols, got $workspace_count"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi

        # Check expected symbol names (if provided)
        if [ -n "$expected_names" ]; then
            IFS=',' read -ra EXPECTED <<< "$expected_names"
            local all_names
            all_names=$(echo "$body" | jq -r '.workspace_symbols[].reference.name' | tr '\n' ' ')

            for name in "${EXPECTED[@]}"; do
                if ! echo "$all_names" | grep -qw "$name"; then
                    echo -e "${RED}✗ FAIL${NC} - Expected symbol '$name' not found"
                    echo "    Found: $all_names"
                    FAILED_TESTS=$((FAILED_TESTS + 1))
                    return 1
                fi
            done
        fi

        # Verify workspace symbols have definitions
        local first_ws
        first_ws=$(echo "$body" | jq '.workspace_symbols[0]')
        if [ "$first_ws" != "null" ]; then
            local def_count
            def_count=$(echo "$first_ws" | jq '.definitions | length' 2>/dev/null)
            if [ "$def_count" = "null" ] || [ "$def_count" = "0" ]; then
                echo -e "${RED}✗ FAIL${NC} - Workspace symbol missing definitions"
                FAILED_TESTS=$((FAILED_TESTS + 1))
                return 1
            fi
        fi

        echo -e "${GREEN}✓ PASS${NC} (${workspace_count} workspace symbols)"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 28 ]; then
            echo -e "${RED}✗ FAIL${NC} - Timeout (30s)"
        else
            echo -e "${RED}✗ FAIL${NC} - Request failed (exit code: $exit_code)"
        fi
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

# Check if service is already running, start it if not
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Nuanced LSP Service Check${NC}"
echo -e "${BLUE}=========================================${NC}"

if docker ps --filter "name=nuanced-lsp-proxy" --format '{{.Names}}' | grep -q "nuanced-lsp-proxy"; then
    echo -e "${GREEN}✓ Service already running${NC}"
    echo -e "${YELLOW}  Using existing containers (will not clean up on exit)${NC}"
    STARTED_SERVICE=false
else
    echo -e "${YELLOW}Service not running, starting containers...${NC}"
    echo -e "${YELLOW}  Workspace: $WORKSPACE_PATH${NC}"

    # Check if workspace exists
    if [ ! -d "$WORKSPACE_PATH" ]; then
        echo -e "${RED}✗ ERROR: Workspace not found: $WORKSPACE_PATH${NC}"
        usage
        exit 1
    fi

    # Start the service using start-proxy.sh
    PROXY_ARGS=()
    if [ -n "$SERVICE_TAG" ]; then
        PROXY_ARGS+=("--service-tag=${SERVICE_TAG}")
    fi
    if [ -n "$LANGUAGE_TAG" ]; then
        PROXY_ARGS+=("--language-tag=${LANGUAGE_TAG}")
    fi
    if ! ./scripts/start-proxy.sh "${PROXY_ARGS[@]}" "$WORKSPACE_PATH" > /tmp/test-proxy-startup.log 2>&1; then
        echo -e "${RED}✗ ERROR: Failed to start service${NC}"
        echo -e "${YELLOW}  Check logs: tail -50 /tmp/test-proxy-startup.log${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ Service started successfully${NC}"
    STARTED_SERVICE=true

    # Poll /system/health for service + language readiness (timeout 60s)
    echo -e "${YELLOW}  Waiting for service and language health (up to 100s)...${NC}"
    ready=false
    for i in $(seq 1 100); do
        HEALTH=$(curl -sf "${BASE_URL}/v1/system/health" || true)
        echo "$HEALTH" | jq .
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
        echo -e "${RED}✗ Service did not become healthy within timeout${NC}"
        exit 1
    fi
fi
echo

# Main test execution
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}  Nuanced LSP Comprehensive Endpoint Tests  ${NC}"
echo -e "${BLUE}  Base URL: $BASE_URL${NC}                       "
echo -e "${BLUE}  Workspace: $WORKSPACE_PATH${NC}                "
echo -e "${BLUE}============================================${NC}"
echo

# Test 1: System Health
echo -e "${YELLOW}1. System Health Check${NC}"
test_http_endpoint "Health Check" \
    "GET" \
    "/v1/system/health" \
    "" \
    "200" \
    "jq -e '.status == \"ok\"' > /dev/null"
echo

# Test 2: List Files (language-agnostic)
echo -e "${YELLOW}2. Workspace Endpoints (Language-Agnostic)${NC}"
test_http_endpoint "List Files" \
    "GET" \
    "/v1/workspace/list-files" \
    "" \
    "200" \
    "jq -e 'type == \"array\" and length > 0' > /dev/null"
echo

# Test 3: Test each language
echo -e "${YELLOW}3. Language-Specific Tests${NC}"
echo

# Disable set -e during testing so we continue on failures
set +e

while IFS='|' read -r lang test_file symbol_name symbol_line symbol_char health_key; do
    # Skip empty lines
    [ -z "$lang" ] && continue

    test_uri="$WORKSPACE_URI/$test_file"

    echo -e "${BLUE}Testing language: $(echo "$lang" | tr '[:lower:]' '[:upper:]')${NC}"

    # Health check for this language
    test_http_endpoint "Health ($lang)" \
        "GET" \
        "/v1/system/health" \
        "" \
        "200" \
        "jq -e '.languages.$health_key == true' > /dev/null"

    # Read Source Code
    test_http_endpoint "Read Source ($lang)" \
        "POST" \
        "/v1/workspace/read-source-code" \
        "{\"path\":\"$test_file\"}" \
        "200" \
        "jq -e '.source_code | type == \"string\" and length > 0' > /dev/null"

    # Read Source Code with Range
    test_http_endpoint "Read Source with Range ($lang)" \
        "POST" \
        "/v1/workspace/read-source-code" \
        "{\"path\":\"$test_file\",\"range\":{\"start\":{\"line\":0,\"character\":0},\"end\":{\"line\":1,\"character\":0}}}" \
        "200" \
        "jq -e '.source_code | type == \"string\"' > /dev/null"

    # Find Definition (assert selected identifier and at least one definition)
    test_http_endpoint "Find Definition ($lang)" \
        "POST" \
        "/v1/symbol/find-definition" \
        "{\"position\":{\"path\":\"$test_file\",\"position\":{\"line\":$symbol_line,\"character\":$symbol_char}},\"include_source_code\":false,\"include_raw_response\":false}" \
        "200" \
        "jq -e '.selected_identifier.name == \"$symbol_name\" and (.definitions | length) >= 0 and (.selected_identifier.file_range.path == \"$test_file\")' > /dev/null"

    # Find References (assert selected identifier matches and references is an array)
    test_http_endpoint "Find References ($lang)" \
        "POST" \
        "/v1/symbol/find-references" \
        "{\"identifier_position\":{\"path\":\"$test_file\",\"position\":{\"line\":$symbol_line,\"character\":$symbol_char}},\"include_code_context_lines\":0}" \
        "200" \
        "jq -e '.selected_identifier.name == \"$symbol_name\" and .selected_identifier.file_range.path == \"$test_file\" and (.references | type == \"array\")' > /dev/null"

    # Find Referenced Symbols
    test_http_endpoint "Find Referenced Symbols ($lang)" \
        "POST" \
        "/v1/symbol/find-referenced-symbols" \
        "{\"identifier_position\":{\"path\":\"$test_file\",\"position\":{\"line\":$symbol_line,\"character\":$symbol_char}},\"full_scan\":false}" \
        "200" \
        "jq -e 'type == \"object\"' > /dev/null"

    # Definitions in File
    test_http_endpoint "Definitions in File ($lang)" \
        "GET" \
        "/v1/symbol/definitions-in-file?file_path=$test_file" \
        "" \
        "200" \
        "jq -e 'type == \"array\"' > /dev/null"

    # Find Identifier
    test_http_endpoint "Find Identifier ($lang)" \
        "POST" \
        "/v1/symbol/find-identifier" \
        "{\"path\":\"$test_file\",\"name\":\"$symbol_name\"}" \
        "200" \
        "jq -e 'type == \"object\"' > /dev/null"

    # Find Definition (assert selected identifier and at least one definition)
    test_ws_endpoint "LSP GoTo Definition ($lang)" \
        "/lsp/ws" \
        "{\"jsonrpc\":\"2.0\",\"id\":\"$TOTAL_TESTS\",\"method\":\"textDocument/definition\",\"params\":{\"textDocument\":{\"uri\":\"$test_uri\"},\"position\":{\"line\":$symbol_line,\"character\":$symbol_char}}}" \
        "jq -e '.result | if type == \"array\" then . else [.] end | length > 0' > /dev/null"

    echo
done <<< "$LANGUAGE_CONFIGS"

# Test 4: Validation of find-referenced-symbols (ast-grep)
echo -e "${YELLOW}4. Validation of find-referenced-symbols (ast-grep)${NC}"
echo

while IFS='|' read -r lang file line char min_ws expected; do
    [ -z "$lang" ] && continue
    test_find_referenced_symbols_enhanced "$lang" "$file" "$line" "$char" "$min_ws" "$expected"
done <<< "$FIND_REF_TESTS"
echo

# Re-enable set -e after testing
set -e

# Summary
echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  Test Summary${NC}"
echo -e "${BLUE}=========================================${NC}"
echo -e "Total Tests:  $TOTAL_TESTS"
echo -e "${GREEN}Passed:       $PASSED_TESTS${NC}"
echo -e "${RED}Failed:       $FAILED_TESTS${NC}"
if [ $TOTAL_TESTS -gt 0 ]; then
    SUCCESS_RATE=$(awk "BEGIN {printf \"%.1f\", ($PASSED_TESTS / $TOTAL_TESTS) * 100}")
    echo -e "Success Rate: ${SUCCESS_RATE}%"
else
    echo -e "Success Rate: N/A"
fi
echo -e "${BLUE}=========================================${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi

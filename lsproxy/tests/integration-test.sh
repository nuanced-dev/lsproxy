#!/bin/bash

set -e

# Get language to test (default: python)
LANGUAGE="${1:-python}"

BASE_URL="http://localhost:4444/v1"
PASS=0
FAIL=0

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

test_endpoint() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local data="$4"
    local expected_status="${5:-200}"
    local validation_check="$6"

    echo -n "Testing $name... "

    # Build curl command
    local curl_cmd="curl -s -w '\n%{http_code}' -X $method"
    if [ -n "$data" ]; then
        curl_cmd="$curl_cmd -H 'Content-Type: application/json' -d '$data'"
    fi
    curl_cmd="$curl_cmd '$BASE_URL$endpoint'"

    # Execute request
    if response=$(eval "$curl_cmd" 2>&1); then
        # Split response body and status code (status is last line)
        local body=$(echo "$response" | sed '$d')
        local status=$(echo "$response" | tail -n 1)

        # Validate HTTP status code
        if [ "$status" != "$expected_status" ]; then
            echo -e "${RED}✗ FAIL${NC} - Expected status $expected_status, got $status"
            echo "  Response: $body"
            FAIL=$((FAIL + 1))
            return 1
        fi

        # Validate JSON structure
        if ! echo "$body" | jq . > /dev/null 2>&1; then
            echo -e "${RED}✗ FAIL${NC} - Invalid JSON response"
            echo "  Response: $body"
            FAIL=$((FAIL + 1))
            return 1
        fi

        # Run custom validation if provided
        if [ -n "$validation_check" ]; then
            if ! echo "$body" | eval "$validation_check"; then
                echo -e "${RED}✗ FAIL${NC} - Validation check failed"
                echo "  Response: $body"
                FAIL=$((FAIL + 1))
                return 1
            fi
        fi

        echo -e "${GREEN}✓ PASS${NC}"
        PASS=$((PASS + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC} - Request failed"
        echo "  Error: $response"
        FAIL=$((FAIL + 1))
        return 1
    fi
}

# Language test configurations
# Format: language_name:test_file:symbol_name:symbol_line:symbol_char
LANGUAGE_TESTS="
python:main.py:main:14:4
typescript:test.ts:hello:0:4
rust:main.rs:main:0:3
golang:main.go:main:0:5
java:src/main/java/com/example/Main.java:main:0:16
cpp:main.cpp:main:0:5
csharp:Program.cs:Main:0:16
php:test.php:hello:0:9
ruby:test.rb:hello:0:4
"

echo "========================================="
echo "  LSProxy API Endpoint Test Suite"
echo "  Language: $(echo $LANGUAGE | tr '[:lower:]' '[:upper:]')"
echo "========================================="
echo

echo "1. System Endpoints"
echo "-------------------"
# Map language names to health check keys
case "$LANGUAGE" in
    "typescript"|"javascript") HEALTH_KEY="typescript_javascript" ;;
    *) HEALTH_KEY="$LANGUAGE" ;;
esac

test_endpoint "Health Check" \
    "GET" \
    "/system/health" \
    "" \
    "200" \
    "jq -e '.status == \"ok\" and .languages.$HEALTH_KEY == true' > /dev/null"

echo

echo "2. Workspace Endpoints (Language-Agnostic)"
echo "-------------------------------------------"
test_endpoint "List Files" \
    "GET" \
    "/workspace/list-files" \
    "" \
    "200" \
    "jq -e '.files | type == \"array\"' > /dev/null"

echo

# Test the specified language
for config in $LANGUAGE_TESTS; do
    [ -z "$config" ] && continue  # Skip empty lines
    IFS=':' read -r lang_key test_file symbol_name symbol_line symbol_char <<< "$config"

    # Skip if not the language we're testing
    [ "$lang_key" != "$LANGUAGE" ] && continue

    echo "3. Language-Specific Symbol Operations"
    echo "-------------------------------------------"

    # Read Source Code
    test_endpoint "Read Source Code ($lang_key)" \
        "POST" \
        "/workspace/read-source-code" \
        "{\"path\":\"$test_file\"}" \
        "200" \
        "jq -e '.content | type == \"string\" and length > 0' > /dev/null"

    # Read Source Code with Range
    test_endpoint "Read Source Code with Range ($lang_key)" \
        "POST" \
        "/workspace/read-source-code" \
        "{\"path\":\"$test_file\",\"range\":{\"start\":{\"line\":0,\"character\":0},\"end\":{\"line\":1,\"character\":0}}}" \
        "200" \
        "jq -e '.content | type == \"string\"' > /dev/null"

    # Find Definition
    test_endpoint "Find Definition ($lang_key)" \
        "POST" \
        "/symbol/find-definition" \
        "{\"position\":{\"path\":\"$test_file\",\"position\":{\"line\":$symbol_line,\"character\":$symbol_char}},\"include_source_code\":false,\"include_raw_response\":false}" \
        "200" \
        "jq -e 'type == \"object\"' > /dev/null"

    # Find References
    test_endpoint "Find References ($lang_key)" \
        "POST" \
        "/symbol/find-references" \
        "{\"identifier_position\":{\"path\":\"$test_file\",\"position\":{\"line\":$symbol_line,\"character\":$symbol_char}},\"context_lines\":0}" \
        "200" \
        "jq -e '.references | type == \"array\"' > /dev/null"

    # Find Referenced Symbols
    test_endpoint "Find Referenced Symbols ($lang_key)" \
        "POST" \
        "/symbol/find-referenced-symbols" \
        "{\"identifier_position\":{\"path\":\"$test_file\",\"position\":{\"line\":$symbol_line,\"character\":$symbol_char}},\"full_scan\":false}" \
        "200" \
        "jq -e 'type == \"object\"' > /dev/null"

    # Definitions in File
    test_endpoint "Definitions in File ($lang_key)" \
        "GET" \
        "/symbol/definitions-in-file?file_path=$test_file" \
        "" \
        "200" \
        "jq -e 'type == \"array\"' > /dev/null"

    # Find Identifier
    test_endpoint "Find Identifier ($lang_key)" \
        "POST" \
        "/symbol/find-identifier" \
        "{\"path\":\"$test_file\",\"name\":\"$symbol_name\"}" \
        "200" \
        "jq -e 'type == \"object\"' > /dev/null"

    echo
done

echo "========================================="
echo "  Test Results"
echo "========================================="
echo -e "${GREEN}Passed: $PASS${NC}"
echo -e "${RED}Failed: $FAIL${NC}"
echo "Total:  $((PASS + FAIL))"
echo "========================================="

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi

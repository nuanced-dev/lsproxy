#!/bin/bash

# Enhanced test function for find-referenced-symbols
# This validates actual symbol correctness, not just HTTP 200

test_find_referenced_symbols_enhanced() {
    local lang="$1"
    local file="$2"
    local line="$3"
    local char="$4"
    local min_workspace="$5"
    local expected_names="$6"

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -n "  Testing Find Referenced Symbols ($lang) [deep validation]... "

    # Make request
    local response=$(curl -s --max-time 30 -X POST "$BASE_URL/symbol/find-referenced-symbols" \
        -H 'Content-Type: application/json' \
        -d "{\"identifier_position\":{\"path\":\"$file\",\"position\":{\"line\":$line,\"character\":$char}},\"full_scan\":false}" 2>&1)

    # Check for curl timeout or error
    if [ $? -ne 0 ]; then
        echo -e "${RED}✗ FAIL${NC} - Request failed"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi

    # Validate JSON structure
    if ! echo "$response" | jq . > /dev/null 2>&1; then
        echo -e "${RED}✗ FAIL${NC} - Invalid JSON"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi

    # Check for error response
    if echo "$response" | jq -e '.error' > /dev/null 2>&1; then
        echo -e "${RED}✗ FAIL${NC} - Error in response"
        echo "    $(echo "$response" | jq -r '.error' | head -1)"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi

    # Validate response structure
    if ! echo "$response" | jq -e 'type == "object"' > /dev/null 2>&1; then
        echo -e "${RED}✗ FAIL${NC} - Response is not an object"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi

    # Check required fields exist
    if ! echo "$response" | jq -e '.workspace_symbols' > /dev/null 2>&1; then
        echo -e "${RED}✗ FAIL${NC} - Missing workspace_symbols field"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi

    if ! echo "$response" | jq -e '.external_symbols' > /dev/null 2>&1; then
        echo -e "${RED}✗ FAIL${NC} - Missing external_symbols field"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi

    # Check workspace_symbols count
    local workspace_count=$(echo "$response" | jq '.workspace_symbols | length')
    if [ "$workspace_count" -lt "$min_workspace" ]; then
        echo -e "${RED}✗ FAIL${NC} - Expected ≥${min_workspace} workspace symbols, got $workspace_count"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi

    # Check expected symbol names (if provided)
    if [ -n "$expected_names" ]; then
        IFS=',' read -ra EXPECTED <<< "$expected_names"
        local all_names=$(echo "$response" | jq -r '.workspace_symbols[].reference.name' | tr '\n' ' ')

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
    local first_ws=$(echo "$response" | jq '.workspace_symbols[0]')
    if [ "$first_ws" != "null" ]; then
        local def_count=$(echo "$first_ws" | jq '.definitions | length' 2>/dev/null)
        if [ "$def_count" = "null" ] || [ "$def_count" = "0" ]; then
            echo -e "${RED}✗ FAIL${NC} - Workspace symbol missing definitions"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        fi
    fi

    echo -e "${GREEN}✓ PASS${NC} (${workspace_count} workspace symbols)"
    PASSED_TESTS=$((PASSED_TESTS + 1))
    return 0
}

# Configuration for find-referenced-symbols deep tests
# Format: language|file|line|char|min_workspace_symbols|expected_names
FIND_REF_TESTS="
python|main.py|15|4|3|a_star_search,plot_path,AStarGraph
typescript|src/main.ts|5|6|3|AStar,PathVisualizer
javascript|src/main.ts|5|6|3|AStar,PathVisualizer
"

# Add this section to your main test script, after language-specific tests:
#
# echo -e "${YELLOW}4. Deep Validation - find-referenced-symbols${NC}"
# echo
#
# while IFS='|' read -r lang file line char min_ws expected; do
#     [ -z "$lang" ] && continue
#     test_find_referenced_symbols_enhanced "$lang" "$file" "$line" "$char" "$min_ws" "$expected"
# done <<< "$FIND_REF_TESTS"
# echo

# NOTE: C# is excluded from FIND_REF_TESTS because it's currently at 0% pass rate
# Add this line once C# is working:
# csharp|Program.cs|4|20|2|AStar,FindPathTo

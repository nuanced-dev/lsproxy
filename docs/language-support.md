# Language Support Status for find-referenced-symbols

This document tracks the status of `find-referenced-symbols` endpoint support across different programming languages.

## Overview

The `find-referenced-symbols` endpoint combines ast-grep (for finding symbol references) with LSP `textDocument/definition` and `documentSymbol` (for retrieving symbol details). For a language to work correctly, it needs:

1. **ast-grep rules** configured to find symbol references in the language syntax
2. **LSP server** that reliably reports symbol definitions and document symbols
3. **Position alignment** between ast-grep and LSP responses

## Current Status

### ✅ WORKING (4 languages)

These languages have reliable support with accurate results and ast-grep reference rules:

| Language | Status | Reference Rules | Notes |
|----------|--------|-----------------|-------|
| **Python** | ✅ Working | decorator, function-call | Excellent support. ast-grep and python-lsp-server work well together |
| **TypeScript/JavaScript** | ✅ Working | component-render, function-call, decorator | Good support. Includes fallback for arrow function properties (see below) |
| **C#** | ✅ Working | class-instantiation, function-call | Working. Finds method references and instantiations |
| **PHP** | ✅ Working | attribute-usage, function-call | Working. Finds function calls and attribute usage |

### ❌ NOT WORKING (5 languages)

These languages have identifier and symbol rules (can find functions), but cannot find referenced symbols because ast-grep reference rules are not configured:

| Language | Status | Identifiers Working | Reference Rules | Solution |
|----------|--------|--------------------|--------------------|----------|
| **Ruby** | ❌ Broken | ✓ Yes | ✗ Missing | Need to add Ruby reference rules (function-call, method-call, etc.) |
| **Go** | ❌ Broken | ✓ Yes | ✗ Missing | Need to add Go reference rules |
| **Rust** | ❌ Broken | ✓ Yes | ✗ Missing | Need to add Rust reference rules |
| **Java** | ❌ Broken | ✓ Yes | ✗ Missing | Need to add Java reference rules |
| **C/C++ (clangd)** | ❌ Broken | ✓ Yes | ✗ Missing | Need to add C/C++ reference rules |

**Note**: All these languages have working LSP servers and ast-grep can find identifiers (methods, functions) in their code. The limitation is specifically for the `find-referenced-symbols` endpoint, which requires ast-grep reference rules to identify symbol usages (function calls, method calls, etc.) within a function body.

## Known Issues & Solutions

### TypeScript Arrow Function Properties

**Problem**: TypeScript's LSP returns different character positions for arrow function properties:
```typescript
private isWalkable = (point: Point): boolean => { ... }
        ^            ^
        char 12      char 25
```
- `documentSymbol` reports the identifier at character 12
- `textDocument/definition` returns character 25 (the arrow `=>`)

**Solution**: Implemented fallback mechanism in `find_referenced_symbols.rs`:
1. Try `get_symbol_from_position` with `textDocument/definition` position
2. If that fails, use `get_file_identifiers` to find the symbol by name and line
3. Call `get_symbol_from_position` with the ast-grep identifier position

This fallback is documented in `crates/wrapper/src/handlers/find_referenced_symbols.rs` lines 143-193.

## Testing

Tests are defined in `scripts/test-all-endpoints.sh` under the `FIND_REF_TESTS` variable.

### Working Language Tests
```bash
# Python - finds 5 workspace symbols
python|main.py|14|4|1|AStarGraph

# TypeScript - finds 15 workspace symbols
typescript|src/astar.ts|60|12|2|isInBounds,isWalkable

# C# - finds 2 workspace symbols
csharp|AStar.cs|23|27|1|AddNeighborsToOpenList

# PHP - finds 5 workspace symbols
php|AStar.php|26|20|1|addNeighborsToOpenList
```

### Commented Out Tests (Not Working - Need Reference Rules)
```bash
# Ruby - identifiers work but no reference rules
# ruby|search.rb|31|15|1|initialize_search

# Go - identifiers work but no reference rules
# golang|golang_astar/astar.go|23|5|1|??

# Rust - identifiers work but no reference rules
# rust|src/astar.rs|31|7|1|??

# Java - identifiers work but no reference rules
# java|AStar.java|39|22|1|??

# C/C++ - identifiers work but no reference rules
# clangd|astar_search.cpp|2|4|1|??
```

## Investigation Steps for Broken Languages

To investigate why a language isn't working:

1. **Check ast-grep rules**: Look in `ast_grep_configs/` for language-specific rules
2. **Test ast-grep directly**: Run ast-grep commands to see if it can find symbols
3. **Check LSP integration**: Verify the LSP server is running and responding correctly
4. **Test position alignment**: Compare positions returned by ast-grep vs LSP

Example investigation command:
```bash
# Check if ast-grep finds a symbol at a position
curl -X POST http://localhost:4444/v1/symbol/find-referenced-symbols \
  -H "Content-Type: application/json" \
  -d '{"identifier_position": {"path": "file.rb", "position": {"line": 32, "character": 16}}, "full_scan": false}'
```

## Future Work

### Short Term
1. Investigate ast-grep configuration for Ruby, PHP, Go
2. Test different positions to find working symbol locations
3. Document any language-specific position requirements

### Long Term
1. Add ast-grep rules for languages that don't have them
2. Implement language-specific fallback strategies similar to TypeScript
3. Consider alternative approaches for languages where ast-grep doesn't work well

## Related Files

- `crates/wrapper/src/handlers/find_referenced_symbols.rs` - Main handler with fallback logic
- `crates/wrapper/src/manager.rs` - Manager methods for symbol operations
- `scripts/test-all-endpoints.sh` - Test configuration
- `ast_grep_configs/` - ast-grep rule configurations

## Last Updated

2025-11-08 - Verified C# and PHP support, confirmed 4 working languages (Python, TypeScript, C#, PHP) and identified root cause for 5 non-working languages (missing ast-grep reference rules)

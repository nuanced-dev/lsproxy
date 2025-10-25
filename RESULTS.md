# LSProxy Container Orchestration - Key Improvements

## Image Size Reduction

### Before: Monolithic Image
- **Single image**: 13.3 GB
- Contains all 10 language servers
- Downloaded entirely even if user only needs 1-2 languages

### After: Per-Language Images

| Language | Image Name | Size | Reduction vs Monolithic |
|----------|-----------|------|------------------------|
| **Base** | lsproxy-base-runtime | 287 MB | -97.8% |
| **Base (build)** | lsproxy-base-build | 596 MB | -95.5% |
| **Golang** | lsproxy-golang | 694 MB | -94.8% |
| **Python** | lsproxy-python | 701 MB | -94.7% |
| **PHP** | lsproxy-php | 804 MB | -94.0% |
| **TypeScript** | lsproxy-typescript | 912 MB | -93.1% |
| **Clangd (C/C++)** | lsproxy-clangd | 967 MB | -92.7% |
| **Ruby 3.4.4** | lsproxy-ruby-3.4.4 | 1.00 GB | -92.5% |
| **Ruby Sorbet 3.4.4** | lsproxy-ruby-sorbet-3.4.4 | 1.04 GB | -92.2% |
| **Java 21** | lsproxy-java | 1.17 GB | -91.2% |
| **Rust** | lsproxy-rust | 1.43 GB | -89.2% |
| **C#** | lsproxy-csharp | 2.15 GB | -83.8% |

### Real-World User Scenarios

**Single-language project** (e.g., Python web app):
- Before: 13.3 GB
- After: 287 MB (base) + 701 MB (Python) = **988 MB**
- **Savings: 92.6% (12.3 GB)**

**Multi-language project** (e.g., TypeScript + Python + Golang):
- Before: 13.3 GB
- After: 287 MB (base) + 912 MB (TS) + 701 MB (Python) + 694 MB (Go) = **2.59 GB**
- **Savings: 80.5% (10.7 GB)**

**Enterprise polyglot repo** (all 10 languages):
- Before: 13.3 GB
- After: 287 MB (base) + sum of all languages = **~10.6 GB**
- **Savings: 20% (2.7 GB)**

> **Note**: Most users don't need all 10 languages. Typical savings are 80-95%.

## Performance Improvements

### Concurrent Request Handling

**Before: Serialized Requests**
```
Manager → Arc<Mutex<LspClient>> → blocks entire request/response cycle

Request A: lock → send → wait 100ms → unlock
Request B:        (waits) → lock → send → wait 100ms → unlock
Request C:                 (waits 200ms) → lock → send → wait 100ms

Total time: 300ms for 3 concurrent requests
```

**After: Concurrent Requests**
```
lsp-wrapper → LspProcess → brief lock for stdin write only

Request A: lock write (~1μs) → release → wait 100ms (concurrent) →
Request B: lock write (~1μs) → release → wait 100ms (concurrent) →
Request C: lock write (~1μs) → release → wait 100ms (concurrent) →

Total time: ~100ms for 3 concurrent requests
```

**Key improvements**:
- Lock held only during stdin write (~microseconds) instead of entire request cycle
- All requests wait concurrently for responses
- Background task demultiplexes responses by ID to correct request
- **Expected throughput: 3x improvement** for typical LSP operations with 3 concurrent requests
- **Scalability**: Throughput continues to improve with more concurrent requests (up to LSP server limits)

### Architecture Benefits

1. **Lazy Loading**: Only download/start containers for languages actually used in workspace
2. **Parallel Startup**: Multiple language containers can initialize simultaneously
3. **Isolation**: Container crashes don't affect other languages or base service
4. **Resource Management**: Can set per-language memory/CPU limits via Docker
5. **Version Flexibility**: Support multiple versions per language (e.g., Ruby 3.4.4, 3.3.6, 2.7.8)
6. **Independent Updates**: Update one language server without rebuilding entire stack

## Architecture Comparison

### Before: Monolithic
```
┌─────────────────────────────────────────┐
│   Single 13.3GB Docker Image            │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │  Rust Service (Manager)            │ │
│  └──────────┬─────────────────────────┘ │
│             │                            │
│  ┌──────────▼───────────┐              │
│  │  10 LSP Servers      │              │
│  │  (all in one image)  │              │
│  │  - gopls             │              │
│  │  - jedi              │              │
│  │  - tsserver          │              │
│  │  - ruby-lsp          │              │
│  │  - rust-analyzer     │              │
│  │  - clangd            │              │
│  │  - jdtls             │              │
│  │  - phpactor          │              │
│  │  - csharp-ls         │              │
│  │  - sorbet            │              │
│  └──────────────────────┘              │
└─────────────────────────────────────────┘

Issues:
- Must download entire 13.3GB regardless of needs
- All languages loaded into memory
- Single point of failure
- Difficult to update individual languages
```

### After: Container Orchestration
```
┌──────────────────────────────────────────────────────────┐
│  Base Rust Service (287 MB)                              │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Container Orchestrator                            │  │
│  │  - Detects languages in workspace                  │  │
│  │  - Spawns appropriate containers on-demand         │  │
│  │  - Routes HTTP requests to containers              │  │
│  │  - Manages lifecycle and cleanup                   │  │
│  └────┬───────────┬────────────┬────────────┬─────────┘  │
└───────┼───────────┼────────────┼────────────┼────────────┘
        │           │            │            │
        ▼           ▼            ▼            ▼
    ┌───────┐   ┌───────┐   ┌───────┐   ┌───────┐
    │  Go   │   │Python │   │  TS   │   │ Ruby  │  ...
    │ 694MB │   │ 701MB │   │ 912MB │   │  1GB  │
    │       │   │       │   │       │   │       │
    │ gopls │   │ jedi  │   │ tsls  │   │ruby-  │
    │       │   │       │   │       │   │lsp    │
    └───────┘   └───────┘   └───────┘   └───────┘

Each container is self-contained:
- Runs lsp-wrapper (HTTP API)
- Manages LSP server process
- Handles ast-grep operations locally
- Independent lifecycle and resources

Benefits:
- Download only what you need
- Isolated failures
- Independent scaling
- Easy to add/update languages
```

## Workspace Operations Refactor

### Overview

Refactored workspace operations (`list-files` and `read-source-code`) to live exclusively in the base Rust service, removing unnecessary complexity from LSP wrapper containers.

### Key Changes

#### 1. List Files - Direct Filesystem Walking

**Before**: Aggregated files by calling all language containers via HTTP
```rust
// Get all containers and call each one
for container in all_containers {
    client.list_files().await
}
```

**After**: Walks workspace filesystem directly in base service
```rust
// Walk /mnt/workspace directly
for result in WalkBuilder::new(workspace_path)
    .hidden(false)
    .git_ignore(false)  // Don't filter by gitignore
    .git_exclude(false)
    .build()
{
    // Collect files
}
```

**Benefits**:
- ✅ No container HTTP calls needed
- ✅ Faster (no network overhead)
- ✅ Simpler (one implementation)
- ✅ Fixes gitignore filtering issue that was causing empty results

#### 2. Read Source Code - Direct File Reading

**Before**: Proxied to language container based on file extension
```rust
let client = container_proxy::get_client_for_file(&orchestrator, &path).await?;
client.read_source(&request).await
```

**After**: Reads file directly from filesystem in base service
```rust
let file_path = PathBuf::from(&workspace_path).join(&request.path);

// Security: path traversal check
let canonical_file = std::fs::canonicalize(&file_path)?;
if !canonical_file.starts_with(&canonical_workspace) {
    return Err("Invalid path");
}

// Read file
tokio::fs::read_to_string(&file_path).await
```

**Benefits**:
- ✅ No container routing needed
- ✅ Language-agnostic (no file extension matching)
- ✅ Path traversal protection
- ✅ Supports range requests

### Root Cause Resolution

The original issue with `list-files` returning empty was:
1. LSP wrapper used `WalkBuilder` with `.git_ignore(true)`
2. This filtered out test files in non-git workspaces
3. Base service aggregated empty results from all containers

New implementation:
1. Base service walks `/mnt/workspace` directly
2. No gitignore filtering (shows all files)
3. No container calls needed
4. ✅ Simpler, faster, and actually works!

### Endpoint Distribution

**Base Service Endpoints (Language-Agnostic)**:
- `GET /v1/system/health` - System status
- `GET /v1/workspace/list-files` - List all workspace files ✨ (refactored)
- `POST /v1/workspace/read-source-code` - Read file content ✨ (refactored)

**LSP Wrapper Endpoints (Language-Specific)**:
- `POST /v1/symbol/find-definition` - LSP textDocument/definition
- `POST /v1/symbol/find-references` - LSP textDocument/references
- `POST /v1/symbol/definitions-in-file` - LSP textDocument/documentSymbol
- `POST /v1/symbol/find-referenced-symbols` - ast-grep analysis
- `POST /v1/symbol/find-identifier` - Symbol search

## Summary

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| **Typical download size** | 13.3 GB | 1-3 GB | **80-95% smaller** |
| **Startup time** | All languages load | Only needed languages | **Faster** |
| **Concurrent request throughput** | Serialized | Parallel | **~3x faster** |
| **Memory usage** | All languages in memory | Only active languages | **Lower** |
| **Failure isolation** | Single point of failure | Per-language isolation | **More reliable** |
| **Update flexibility** | Rebuild entire image | Update individual languages | **More flexible** |
| **Workspace operations** | N containers × HTTP overhead | Direct filesystem access | **Faster & simpler** |

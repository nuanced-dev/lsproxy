# LSProxy Architecture Refactor: Workspace Operations

## Overview

Refactored workspace operations to live exclusively in the base Rust service, removing unnecessary complexity from LSP wrapper containers.

## Changes Made

### 1. Base Service `/src/handlers/list_files.rs`

**Before**: Aggregated files by calling all language containers via HTTP
```rust
// Get all containers and call each one
for container in all_containers {
    client.list_files().await
}
```

**After**: Walks workspace filesystem directly
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
- ✅ Fixes gitignore filtering issue

### 2. Base Service `/src/handlers/read_source_code.rs`

**Before**: Proxied to language container based on file extension
```rust
let client = container_proxy::get_client_for_file(&orchestrator, &path).await?;
client.read_source(&request).await
```

**After**: Reads file directly from filesystem
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

## What Still Needs To Be Done

### 3. Remove from LSP Wrapper

**Files to Delete**:
- `lsp-wrapper/src/handlers/list_files.rs`
- `lsp-wrapper/src/handlers/read_source_code.rs`

**Files to Modify**:
- `lsp-wrapper/src/handlers/mod.rs` - Remove module declarations
- `lsp-wrapper/src/main.rs` - Remove these routes (lines 112-113):
  ```rust
  .route("/workspace/list-files", web::get().to(handlers::list_files::list_files))
  .route("/workspace/read-source-code", web::post().to(handlers::read_source_code::read_source_code))
  ```
- `lsp-wrapper/src/manager.rs` - Remove `list_files()` method

### 4. Rebuild LSP Wrapper Containers

After removing workspace endpoints:
```bash
# Rebuild all LSP language images
./scripts/build-all.sh
```

## Final Architecture

### Base Service Endpoints (Language-Agnostic)
- `GET /v1/system/health` - System status
- `GET /v1/workspace/list-files` - List all workspace files
- `POST /v1/workspace/read-source-code` - Read file content

### LSP Wrapper Endpoints (Language-Specific)
- `POST /v1/symbol/find-definition` - LSP textDocument/definition
- `POST /v1/symbol/find-references` - LSP textDocument/references
- `POST /v1/symbol/definitions-in-file` - LSP textDocument/documentSymbol
- `POST /v1/symbol/find-referenced-symbols` - ast-grep analysis
- `GET /v1/symbol/find-identifier` - Symbol search

## Testing

After rebuild, test with:
```bash
# Start service
docker run --rm -p 4444:4444 \
  -e HOST_WORKSPACE_PATH=/tmp/test-workspaces/python \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /tmp/test-workspaces/python:/mnt/workspace \
  lsproxy-service:latest

# Test list-files
curl http://localhost:4444/v1/workspace/list-files | jq

# Test read-source-code
curl -X POST http://localhost:4444/v1/workspace/read-source-code \
  -H 'Content-Type: application/json' \
  -d '{"path":"test.py"}' | jq
```

Expected results:
- ✅ list-files returns actual files (not empty array)
- ✅ read-source-code returns file content
- ✅ No HTTP calls to language containers for workspace operations
- ✅ LSP operations still work via language containers

## Root Cause Resolved

The original issue with `list-files` returning empty was:
1. LSP wrapper used `WalkBuilder` with `.git_ignore(true)`
2. This filtered out test files in non-git workspaces
3. Base service aggregated empty results from all containers

New implementation:
1. Base service walks `/mnt/workspace` directly
2. No gitignore filtering (shows all files)
3. No container calls needed
4. Simpler, faster, and actually works!

## Testing Results

After rebuild with correct Dockerfile (`dockerfiles/service.Dockerfile`):

```bash
# Container starts successfully
docker run --rm --name lsproxy-service --add-host=host.docker.internal:host-gateway \
  -p 4444:4444 -e USE_AUTH=false -e RUST_LOG=info \
  -e HOST_WORKSPACE_PATH=/tmp/test-workspaces/python \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /tmp/test-workspaces/python:/mnt/workspace \
  lsproxy-service:latest
# ✅ Workspace initialization complete
# ✅ Container spawned for Python
# ✅ Service started on port 4444

# Test list-files
curl http://localhost:4444/v1/workspace/list-files
# ✅ Returns: {"files":["test.py"]}
# ✅ Previously returned: {"files":[]}

# Test read-source-code
curl -X POST http://localhost:4444/v1/workspace/read-source-code \
  -H 'Content-Type: application/json' \
  -d '{"path":"test.py"}'
# ✅ Returns file content
# ✅ No HTTP calls to language containers
```

**Results**: Both workspace operations now work correctly and bypass container HTTP calls entirely.

---

*Refactor Date: 2025-10-02 to 2025-10-03*
*Status: Base service refactored ✅ | Base service tested ✅ | LSP wrapper cleanup pending ⏳*

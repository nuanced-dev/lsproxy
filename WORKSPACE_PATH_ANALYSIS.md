# LSProxy Workspace Path Mapping Analysis

## Overview

This document analyzes the workspace path mapping architecture in the containerized LSProxy system and investigates why the `list-files` endpoint returns an empty array despite files being correctly mounted in all containers.

## Architecture: Three-Layer Path Translation

### Layer 1: Host Filesystem
```
Example: /tmp/test-workspaces/python/
├── test.py
└── [other workspace files]
```

The actual workspace files on the Docker host machine.

### Layer 2: Base Service Container
```
docker run ... \
  -v /tmp/test-workspaces/python:/mnt/workspace \
  lsproxy-service:latest
```

The base service container receives the workspace mounted at `/mnt/workspace`.

### Layer 3: Language Containers (Docker-in-Docker)
```rust
// From src/container/orchestrator.rs:54-61
let mount_source = std::env::var("HOST_WORKSPACE_PATH")
    .unwrap_or_else(|_| workspace_path.to_string());

let host_config = HostConfig {
    binds: Some(vec![format!("{}:/mnt/workspace:rw", mount_source)]),
    // ...
}
```

Language containers are spawned by the base service using Docker-in-Docker. They need the **original host path**, not the base service's mount point.

## Path Mapping Mechanism

### The HOST_WORKSPACE_PATH Solution

When the base service spawns language containers, Docker interprets mount sources from the **host's perspective**, not the container's perspective.

**Problem**: If we tried to mount `/mnt/workspace` from the base service container:
```rust
// ❌ WRONG - Docker can't find /mnt/workspace on the host
binds: vec!["/mnt/workspace:/mnt/workspace:rw"]
```

**Solution**: Pass the original host path via environment variable:
```bash
docker run ... \
  -e HOST_WORKSPACE_PATH="/tmp/test-workspaces/python" \
  -v /tmp/test-workspaces/python:/mnt/workspace \
  lsproxy-service:latest
```

Then use that for spawning language containers:
```rust
// ✅ CORRECT - Docker finds the host path
let mount_source = std::env::var("HOST_WORKSPACE_PATH")
    .unwrap_or_else(|_| workspace_path.to_string());

binds: vec![format!("{}:/mnt/workspace:rw", mount_source)]
```

## Verification: Path Mapping Works

### Evidence 1: Files Present in Base Service
```bash
$ docker exec lsproxy-service ls -la /mnt/workspace
total 4
drwxr-xr-x 3 root root  96 Oct  2 00:00 .
drwxr-xr-x 1 root root 4096 Oct  2 00:00 ..
-rw-r--r-- 1 root root   45 Oct  2 00:00 test.py
```

✅ **Base service has workspace files**

### Evidence 2: Files Present in Language Container
```bash
$ docker exec lsproxy-python-4e887aff-b7f9-4b5a-a6b4-3716372b8d76 ls -la /mnt/workspace
total 4
drwxr-xr-x 3 root root  96 Oct  2 00:00 .
drwxr-xr-x 1 root root 4096 Oct  2 00:00 ..
-rw-r--r-- 1 root root   45 Oct  2 00:00 test.py
```

✅ **Language container has workspace files**

### Evidence 3: Other Endpoints Successfully Access Workspace
From `ENDPOINT_VALIDATION_RESULTS.md`:

- ✅ **Read Source** - Successfully reads file content from workspace
- ✅ **Find Definition** - Successfully analyzes workspace files
- ✅ **Find References** - Successfully searches workspace files
- ✅ **Definitions in File** - Successfully parses workspace files
- ✅ **Find Identifier** - Successfully locates symbols in workspace

All these endpoints prove that:
1. Path mapping is correct
2. Workspace files are accessible
3. LSP servers can analyze the workspace

## The Actual Problem: list-files Returns Empty

### Symptom
```json
{
  "files": []
}
```

### What We Know

**✅ Works:**
- Workspace files mounted in all containers
- Container networking functional
- LSP operations on workspace files succeed
- Container orchestration operational

**❌ Doesn't Work:**
- File listing/discovery returns empty array

### Code Flow Analysis

#### 1. Base Service Handler
**File**: `src/handlers/list_files.rs`

```rust
pub async fn list_files(data: Data<AppState>) -> HttpResponse {
    info!("Received list files request");

    // Get all running containers and collect their files
    let all_containers = data.orchestrator.all_containers().await;

    if all_containers.is_empty() {
        // No containers running yet - return empty list
        return HttpResponse::Ok().json(ListFilesResponse { files: vec![] });
    }

    let mut all_files = Vec::new();

    for (_lang, container_info) in all_containers {
        let client = crate::container::ContainerHttpClient::new(&container_info.endpoint);
        match client.list_files().await {
            Ok(files) => all_files.extend(files),
            Err(e) => {
                error!("Failed to list files from container {}: {}",
                       container_info.container_id, e);
            }
        }
    }

    // Deduplicate files
    all_files.sort();
    all_files.dedup();

    HttpResponse::Ok().json(ListFilesResponse { files: all_files })
}
```

**Logic**:
1. Get all running containers from orchestrator
2. For each container, call its `/v1/workspace/list-files` endpoint
3. Aggregate results
4. Return deduplicated list

**Potential Issues**:
- Are containers being returned by `all_containers()`?
- Is HTTP client successfully calling container endpoints?
- Are errors being logged but silently swallowed?

#### 2. Language Container Handler
**File**: `lsp-wrapper/src/handlers/list_files.rs`

```rust
pub async fn list_files(data: Data<AppState>) -> HttpResponse {
    let files = data.manager.list_files().await;
    match files {
        Ok(files) => HttpResponse::Ok().json(files),
        Err(e) => {
            error!("Failed to get workspace files: {}", e);
            e.into_http_response()
        }
    }
}
```

**Logic**:
1. Delegate to `manager.list_files()`
2. Return files or error response

**Potential Issues**:
- What does `manager.list_files()` actually do?
- Is it searching the correct path (`/mnt/workspace`)?
- Are there permissions issues?
- Is it filtering out files incorrectly?

### Root Cause Hypotheses

#### Hypothesis 1: Container Enumeration Issue
The orchestrator's `all_containers()` might be returning an empty map, causing the handler to skip container queries entirely.

**Test**: Check if containers are registered in orchestrator after eager initialization.

#### Hypothesis 2: HTTP Client Communication Failure
The HTTP client might be failing to connect to language containers, causing all requests to error out (logged but not returned).

**Test**: Check base service logs for "Failed to list files from container" errors.

#### Hypothesis 3: Manager Implementation Issue
The `manager.list_files()` implementation might be:
- Looking at wrong path
- Using incorrect file system API
- Filtering out all files
- Encountering permissions issues

**Test**: Check language container logs for errors in list_files handler.

#### Hypothesis 4: Empty Response vs Error Response
Language containers might be successfully returning empty arrays (not errors), which the base service aggregates into an empty result.

**Test**: Directly call language container's list-files endpoint.

## Diagnostic Commands

### 1. Check Container Registration
```bash
# Get base service container ID
docker ps --filter "name=lsproxy-service" --format "{{.ID}}"

# Check logs for container registration
docker logs lsproxy-service 2>&1 | grep -i "container\|spawn\|register"
```

### 2. Check Base Service list-files Logs
```bash
# Check for list-files requests and errors
docker logs lsproxy-service 2>&1 | grep -i "list files\|Failed to list files"
```

### 3. Check Language Container Logs
```bash
# Find language container
docker ps --filter "name=lsproxy-python" --format "{{.ID}}"

# Check its logs
docker logs <container-id> 2>&1 | grep -i "list\|files\|workspace"
```

### 4. Direct Container Endpoint Test
```bash
# Get container IP from orchestrator logs or inspect
docker inspect lsproxy-python-<id> | jq '.[0].NetworkSettings.IPAddress'

# Test directly
curl -s http://<container-ip>:8080/v1/workspace/list-files | jq
```

### 5. Test Manager Implementation Directly
```bash
# Execute ls inside language container to see what it sees
docker exec lsproxy-python-<id> ls -la /mnt/workspace

# Check if the binary can see workspace
docker exec lsproxy-python-<id> find /mnt/workspace -type f
```

## Next Steps

### Immediate Actions
1. **Capture Logs**: Run test and capture all logs from base service and language containers
2. **Direct Test**: Directly call language container's list-files endpoint to isolate issue
3. **Trace Manager**: Find and examine `manager.list_files()` implementation in lsp-wrapper

### Investigation Priority
1. **First**: Verify containers are registered in orchestrator (`all_containers()` returns data)
2. **Second**: Check if HTTP client successfully reaches language containers
3. **Third**: Examine actual manager implementation of file listing logic
4. **Fourth**: Test file permissions and visibility from Rust process perspective

## Conclusion

**Path Mapping: ✅ Working**
- All three layers have correct workspace files
- HOST_WORKSPACE_PATH environment variable correctly enables Docker-in-Docker mounts
- LSP operations successfully access workspace

**File Discovery: ❌ Broken**
- `list-files` returns empty despite files being present
- Issue is in file **discovery/listing logic**, not path **mapping/mounting**
- Root cause likely in one of:
  - Container registration/enumeration
  - HTTP client communication
  - Manager file listing implementation
  - Path used for file discovery vs LSP operations

**Architecture Assessment**: The containerized architecture is fundamentally sound. This is a specific implementation issue in the file listing code path, not a flaw in the overall design.

---

*Generated: 2025-10-02*
*Service Version: 0.4.5*

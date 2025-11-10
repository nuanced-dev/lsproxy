# LSP State Persistence Architecture

## Overview

This document describes how LSP state persistence works in the lsproxy system, enabling LSP servers to avoid full reindexing when containers restart.

## The Problem

When an LSP server container restarts:
- All in-memory state is lost
- Index files stored in the container filesystem are lost
- LSP server must reindex the entire project from scratch
- For large repositories, this can take minutes and causes poor user experience

## The Solution: Persistent Cache Volumes

The binary injection architecture enables clean separation of concerns, making it straightforward to add persistent state volumes for LSP caches.

### Architecture

```
Language Container File System (with persistent state):

/
├── opt/
│   ├── lsp-wrapper/           <-- MOUNTED from wrapper container (read-only)
│   │   ├── bin/lsp-wrapper
│   │   └── ast_grep/
│   └── rbenv/                 <-- From language image
├── mnt/
│   └── workspace/             <-- MOUNTED from host (source code)
└── var/
    └── cache/
        └── lsp/               <-- MOUNTED persistent volume for LSP state
            ├── ruby-lsp/
            │   ├── index.db
            │   └── cache/
            ├── sorbet/
            │   └── .sorbet/
            ├── typescript/
            │   └── tsserver/
            └── rust-analyzer/
                └── index/
```

## Implementation

### Orchestrator Container Start

When the orchestrator starts a language container, it mounts three types of volumes:

```rust
docker.run(Container {
    name: format!("lsproxy-{}-{}", language, instance_id),
    image: format!("lsproxy-{}:latest", language),

    // 1. Binary injection (wrapper binary and configs)
    volumes_from: ["lsproxy-wrapper"],

    volumes: [
        // 2. Source code (read/write)
        "/path/to/workspace:/mnt/workspace",

        // 3. Persistent LSP state (keyed by project hash)
        format!("lsp-state-{}:/var/cache/lsp", project_hash),
    ],
});
```

### Volume Types

1. **Wrapper Binary Volume** (`--volumes-from lsproxy-wrapper`)
   - Read-only
   - Contains lsp-wrapper binary and ast-grep configs
   - Shared across all language containers
   - Lives in wrapper container

2. **Workspace Volume** (`/path/to/workspace:/mnt/workspace`)
   - Read/write
   - Contains source code being analyzed
   - Mounted from host filesystem
   - Different for each project

3. **LSP State Volume** (`lsp-state-{hash}:/var/cache/lsp`)
   - Read/write
   - Contains LSP server index and cache data
   - Named volume persists across container restarts
   - Keyed by project hash (unique per project)

### Project Hash Generation

The project hash uniquely identifies a workspace to ensure cache isolation:

```rust
fn generate_project_hash(workspace_path: &Path) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();

    // Hash the absolute path to workspace
    hasher.update(workspace_path.canonicalize().unwrap().to_string_lossy().as_bytes());

    // Return first 16 chars of hex digest
    format!("{:x}", hasher.finalize())[..16].to_string()
}
```

## Container Lifecycle

### First Start (Cold Start)

1. Orchestrator creates named volume: `lsp-state-abc123def456`
2. Container starts with empty `/var/cache/lsp/` directory
3. LSP server initializes and indexes the project
4. LSP server writes index to `/var/cache/lsp/{language}/`
5. Data persists in Docker named volume

### Restart (Warm Start)

1. Container stops/crashes
2. Orchestrator starts new container with same volume: `lsp-state-abc123def456`
3. Container mounts existing `/var/cache/lsp/` with cached data
4. LSP server reads existing index from cache
5. LSP server only reindexes changed files (incremental update)

### Performance Impact

- **Cold Start**: Full indexing (e.g., 2-5 minutes for large Ruby repo)
- **Warm Start**: Incremental indexing (e.g., 5-15 seconds)
- **Improvement**: 10-30x faster restart for large projects

## LSP Server Support

Most modern LSP servers support persistent caching via configurable cache directories:

### ruby-lsp

```ruby
# Stores index in configurable cache directory
# Configure via lsp-wrapper environment variable
ENV LSP_CACHE_DIR=/var/cache/lsp/ruby-lsp
```

### sorbet

```yaml
# Uses .sorbet/ directory by default
# Can be redirected via --cache-dir flag
CMD ["--lsp-command", "sorbet", "--lsp-arg=--cache-dir=/var/cache/lsp/sorbet"]
```

### typescript-language-server

```json
{
  "typescript.tsserver.pluginPaths": ["/var/cache/lsp/typescript"]
}
```

### rust-analyzer

```json
{
  "rust-analyzer.files.watcherExclude": ["/var/cache/lsp/rust-analyzer"]
}
```

## Volume Management

### Volume Listing

```bash
# List all LSP state volumes
docker volume ls | grep lsp-state-

# Example output:
# lsp-state-abc123def456    # Project A
# lsp-state-789ghi012jkl    # Project B
```

### Volume Cleanup

```bash
# Remove unused volumes (safe - only removes unreferenced volumes)
docker volume prune

# Remove specific project cache
docker volume rm lsp-state-abc123def456

# Remove all LSP state volumes
docker volume ls -q | grep lsp-state- | xargs docker volume rm
```

### Disk Usage

```bash
# Check volume sizes
docker system df -v | grep lsp-state-

# Example:
# lsp-state-abc123def456    250MB    # Large Ruby project
# lsp-state-789ghi012jkl     45MB    # Small TypeScript project
```

## Trade-offs

### Pros

- **Dramatically faster restarts** for large repositories (10-30x improvement)
- **State survives container crashes** and updates
- **Shared state across container recreations** for same project
- **Better user experience** with near-instant LSP availability after restart
- **Clean separation of concerns** (code, binary, state all separate volumes)

### Cons

- **Volume management complexity** - need to clean up old project states
- **Disk usage** - each project consumes disk space for cached state
- **Cache invalidation needed** if language version changes
- **Project hash collision** (extremely unlikely but possible with SHA-256)
- **Debugging difficulty** - state persists between runs, harder to reproduce "clean slate" issues

## Cache Invalidation Strategy

### When to Invalidate

Cache should be invalidated when:
1. Language version changes (e.g., Ruby 3.3 → 3.4)
2. LSP server version changes
3. Project structure changes significantly (e.g., monorepo split)
4. User explicitly requests cache clear

### Implementation

```rust
fn should_invalidate_cache(
    volume_name: &str,
    current_language_version: &str,
) -> bool {
    // Check volume metadata for stored language version
    let stored_version = get_volume_label(volume_name, "language_version");

    stored_version != current_language_version
}

fn start_container_with_cache_check(
    project_hash: &str,
    language: &str,
    version: &str,
) -> Result<Container> {
    let volume_name = format!("lsp-state-{}", project_hash);

    if should_invalidate_cache(&volume_name, version) {
        // Remove old cache
        docker.volumes().remove(&volume_name)?;
    }

    // Create/reuse volume with current version label
    docker.volumes().create(&VolumeCreateOptions {
        name: volume_name.clone(),
        labels: HashMap::from([
            ("language", language),
            ("language_version", version),
            ("created", Utc::now().to_rfc3339()),
        ]),
    })?;

    // Start container with volume
    start_container(project_hash, language, version)
}
```

## Future Enhancements

### Volume Compression

For infrequently used projects, compress cache volumes:

```bash
# Export volume to compressed archive
docker run --rm -v lsp-state-abc123:/data -v $(pwd):/backup \
  alpine tar czf /backup/lsp-state-abc123.tar.gz /data

# Remove volume to free space
docker volume rm lsp-state-abc123

# Later: restore from archive
docker volume create lsp-state-abc123
docker run --rm -v lsp-state-abc123:/data -v $(pwd):/backup \
  alpine tar xzf /backup/lsp-state-abc123.tar.gz -C /
```

### Smart Cache Preloading

For common projects, pre-populate cache volumes:

```bash
# Build cache for popular Ruby version + common gems
docker run --rm \
  -v lsp-state-template-ruby-3.4:/var/cache/lsp \
  -v /path/to/template/project:/mnt/workspace \
  lsproxy-ruby-3.4.4:latest \
  --preindex

# Clone template cache for new projects
docker volume create lsp-state-new-project
docker run --rm \
  -v lsp-state-template-ruby-3.4:/src:ro \
  -v lsp-state-new-project:/dest \
  alpine cp -a /src/. /dest/
```

### Cache Sharing Between Similar Projects

For monorepos or projects with shared dependencies, consider shared cache volumes:

```bash
# Shared gems cache for Ruby projects
-v lsp-gems-cache-ruby-3.4:/var/cache/lsp/shared/gems:ro

# Project-specific index
-v lsp-state-project-abc:/var/cache/lsp/project
```

## Monitoring and Metrics

### Cache Hit Rate

Track how often warm starts occur vs cold starts:

```rust
struct CacheMetrics {
    cold_starts: u64,  // No cache found
    warm_starts: u64,  // Cache found and valid
    invalidations: u64, // Cache found but invalidated
}
```

### Cache Performance

Measure LSP initialization time:

```rust
// Log initialization time
let start = Instant::now();
lsp_server.initialize().await?;
let init_time = start.elapsed();

info!(
    cache_status = if cache_exists { "warm" } else { "cold" },
    init_time_ms = init_time.as_millis(),
    "LSP server initialized"
);
```

## Related Documentation

- [Binary Injection Architecture](./BINARY_INJECTION_ARCHITECTURE.md) - How wrapper binary is mounted
- [Docker Volume Management](https://docs.docker.com/storage/volumes/) - Official Docker docs
- [LSP Protocol](https://microsoft.github.io/language-server-protocol/) - LSP specification

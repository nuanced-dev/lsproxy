# Binary Injection Architecture Migration Guide

## Overview

This document describes how to migrate language Dockerfiles from the coupled `lsproxy-base` architecture to the binary injection architecture.

## Current Status

- **Completed**:
  - `dockerfiles/wrapper.Dockerfile` - Standalone wrapper image created
  - `dockerfiles/golang.Dockerfile` - Migrated as reference implementation

- **Pending**:
  - 231 other Dockerfiles need migration
  - Orchestrator code needs to mount wrapper binary
  - Build scripts need to build wrapper image

## Migration Pattern

### Before (Old Architecture)

```dockerfile
# Runtime stage: Use lsproxy-base and copy only what's needed
FROM lsproxy-base:latest          # <-- DEPENDENCY ON BASE IMAGE

ENV DEBIAN_FRONTEND=noninteractive

# Language-specific setup...
COPY --from=builder /path/to/language /usr/local/language

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="language-name"

# Set workspace path
WORKDIR /mnt/workspace

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "language-lsp", "--lsp-arg=..."]
```

### After (Binary Injection Architecture)

```dockerfile
# Runtime stage: Pure Debian base (no dependency on lsproxy-base)
# Wrapper binary will be mounted at runtime via --volumes-from
FROM debian:bookworm-slim          # <-- PURE DEBIAN, NO BASE DEPENDENCY

ENV DEBIAN_FRONTEND=noninteractive
ENV HOME=/home/user

# Install minimal runtime dependencies
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Language-specific setup...
COPY --from=builder /path/to/language /usr/local/language

# Set language for lsp-wrapper configuration
ENV LSP_LANGUAGE="language-name"

# Add wrapper binary location to PATH (will be mounted from wrapper container)
ENV PATH="/opt/lsp-wrapper/bin:${PATH}"

# Create workspace directory
RUN mkdir -p /mnt/workspace && chmod 755 /mnt/workspace

# Set workspace path
WORKDIR /mnt/workspace

# ENTRYPOINT expects wrapper at /opt/lsp-wrapper/bin/lsp-wrapper (mounted at runtime)
ENTRYPOINT ["/opt/lsp-wrapper/bin/lsp-wrapper"]

# CMD provides the language-specific command to lsp-wrapper ENTRYPOINT
CMD ["--lsp-command", "language-lsp", "--lsp-arg=..."]
```

## Key Changes

1. **FROM directive**: `lsproxy-base:latest` → `debian:bookworm-slim`

2. **Runtime dependencies**: Explicitly install `ca-certificates` and `git`
   - For build variant (Ruby, Python): Also add `pkg-config`, `libssl3`, `build-essential`

3. **PATH environment**: Add `/opt/lsp-wrapper/bin` to PATH

4. **Workspace directory**: Explicitly create `/mnt/workspace`

5. **ENTRYPOINT**: Add explicit entrypoint pointing to mounted wrapper binary

## For Languages Needing Build Tools

Some languages (Ruby, Python, PHP) need build tools for compiling native extensions. For these, use:

```dockerfile
# Install runtime and build dependencies for native extensions
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    ca-certificates \
    git \
    pkg-config \
    libssl3 \
    build-essential \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
```

Instead of just the minimal runtime dependencies.

## Reference Implementation

See `dockerfiles/golang.Dockerfile` for a complete working example of the migrated architecture.

## Files to Migrate

### Top-Level Language Dockerfiles (7 files)
- `dockerfiles/clangd.Dockerfile`
- `dockerfiles/csharp.Dockerfile`
- `dockerfiles/java.Dockerfile`
- `dockerfiles/php.Dockerfile`
- `dockerfiles/python.Dockerfile`
- `dockerfiles/rust.Dockerfile`
- `dockerfiles/typescript.Dockerfile`

### Ruby Versions (110+ files)
- `dockerfiles/ruby/*.Dockerfile` (110+ versions from 2.0.0 to 3.4.4)

### Ruby Sorbet Variants (110+ files)
- `dockerfiles/ruby-sorbet/*.Dockerfile` (110+ versions matching Ruby versions)

### Files to Skip
- `dockerfiles/base.Dockerfile` - Infrastructure (will eventually be removed)
- `dockerfiles/wrapper.Dockerfile` - Already created
- `dockerfiles/service.Dockerfile` - Orchestrator image
- `dockerfiles/watchdog.Dockerfile` - Monitoring image

## Migration Options

### Option A: Manual Migration

Manually update each dockerfile following the pattern above.

**Pros**:
- Full control over each file
- Can verify each change
- Lower risk

**Cons**:
- Time-consuming (232 files)
- Error-prone for repetitive changes

### Option B: Automated Migration Script

Use the provided migration script:

```bash
# Test on a single file first
python3 scripts/migrate-dockerfiles.py

# Review changes
git diff dockerfiles/

# If satisfied, keep changes, otherwise restore
git checkout dockerfiles/
```

**Pros**:
- Fast (minutes instead of hours)
- Consistent changes across all files

**Cons**:
- Need to verify script correctness
- May need manual fixes for edge cases

### Option C: Incremental Migration

Migrate languages incrementally as needed:

1. Start with actively used languages (golang, ruby-3.4, typescript)
2. Migrate others as they're requested
3. Keep old base images around temporarily

**Pros**:
- Lower risk
- Can test each language thoroughly
- Gradual rollout

**Cons**:
- Longer migration period
- Need to maintain both architectures temporarily

## Testing After Migration

For each migrated language:

1. Build the wrapper image:
   ```bash
   docker build -f dockerfiles/wrapper.Dockerfile -t lsproxy-wrapper:latest .
   ```

2. Build the language image:
   ```bash
   docker build -f dockerfiles/golang.Dockerfile -t lsproxy-golang:latest .
   ```

3. Test manually:
   ```bash
   # Start wrapper container
   docker run -d --name lsproxy-wrapper lsproxy-wrapper:latest

   # Start language container with wrapper mounted
   docker run --rm -it \
     --volumes-from lsproxy-wrapper \
     -v $(pwd):/mnt/workspace \
     lsproxy-golang:latest \
     --lsp-command gopls --lsp-arg=-mode=stdio
   ```

4. Verify wrapper binary is accessible:
   ```bash
   docker run --rm \
     --volumes-from lsproxy-wrapper \
     lsproxy-golang:latest \
     which lsp-wrapper
   # Should output: /opt/lsp-wrapper/bin/lsp-wrapper
   ```

## Next Steps

After Dockerfile migration:

1. **Update orchestrator** to:
   - Create/maintain wrapper container on startup
   - Mount wrapper binary when starting language containers

2. **Update build scripts** to:
   - Build wrapper image separately
   - Remove base image from build order

3. **Test end-to-end** with orchestrator

4. **Clean up**:
   - Remove `dockerfiles/base.Dockerfile` (no longer needed)
   - Remove backup files
   - Update documentation

## Benefits After Migration

1. **Independent builds**: Changing wrapper doesn't require rebuilding 232 language images
2. **Faster CI**: Only rebuild what changed
3. **Smaller images**: Language images don't include wrapper build artifacts
4. **Clearer architecture**: Separation of concerns between wrapper and language runtime
5. **Easier LSP state persistence**: Clean mount points for persistent volumes

## Rollback Plan

If migration causes issues:

1. Restore from backup files:
   ```bash
   find dockerfiles -name '*.backup' | while read backup; do
     mv "$backup" "${backup%.backup}"
   done
   ```

2. Rebuild images from restored dockerfiles

3. Restart services with old architecture

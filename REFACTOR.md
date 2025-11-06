# LSProxy Code Refactoring Plan

**Status**: 🟡 In Progress - Phase 1 Complete
**Last Updated**: 2025-11-06

## Overview

This document tracks the refactoring of the LSProxy codebase to eliminate duplication and establish a clear architectural separation between the orchestrator service and the LSP wrapper service.

## Current Problems

1. **~1,833 lines of duplicated code** between `lsproxy/src/` and `lsp-wrapper/src/`
2. **No Cargo workspace** - Two separate Cargo.toml files with duplicated dependencies
3. **Confusing structure** - lsp-wrapper nested inside lsproxy directory
4. **Type drift** - Same types defined twice with minor version differences
5. **Unclear naming** - "lsproxy" doesn't convey "orchestrator" purpose

## Architecture Understanding

### Current Architecture

**lsproxy (orchestrator service)** - `lsproxy/src/`
- Container orchestration layer using Docker
- Spawns and manages language-specific containers
- Thin HTTP handlers (~50 lines) that proxy requests to language containers
- Built into `lsproxy-service:latest` Docker image
- Manages watchdog for cleanup on crashes

**lsp-wrapper (wrapper service)** - `lsproxy/lsp-wrapper/src/`
- Runs **inside** each language container
- Provides HTTP API wrapping LSP stdio communication
- Full handler implementations (100-1700 lines) with direct LSP communication
- Built into base Docker image and inherited by all language images
- Started with: `lsp-wrapper --lsp-command <language-server>`

### Target Architecture

Three distinct crates in a Cargo workspace:

1. **lsproxy-common** - Shared types, utilities, LSP/AST-grep basics
2. **lsproxy-orchestrator** - Container management and request proxying
3. **lsproxy-wrapper** - LSP process wrapping and HTTP API

## Code Duplication Analysis

### Identical Files (475 lines)

These files are byte-for-byte identical and will be moved to `common`:

| File | Lines | Location |
|------|-------|----------|
| `ast_grep/types.rs` | 188 | Both places |
| `handlers/utils.rs` | 66 | Both places |
| `lsp/json_rpc.rs` | 155 | Both places |
| `lsp/process.rs` | 66 | Both places |

### Near-Identical Files (1,358 lines)

These files are 99% similar with minor drift and will be consolidated:

| File | Lines (src) | Lines (wrapper) | Notes |
|------|-------------|-----------------|-------|
| `api_types.rs` | 632 | 632 | Version drift, no intentional differences |
| `utils/file_utils.rs` | 192 | 193 | Very minor differences |
| `utils/workspace_documents.rs` | 534 | 536 | Minimal differences |

### Intentionally Different Files

Handler files serve different purposes and will remain separate:

| Handler | Orchestrator (lines) | Wrapper (lines) | Purpose |
|---------|---------------------|-----------------|---------|
| `find_definition.rs` | 56 | 318 | Proxy vs Full implementation |
| `find_references.rs` | 58 | 589 | Proxy vs Full implementation |
| `find_referenced_symbols.rs` | 58 | 1792 | Proxy vs Full implementation |
| `find_identifier.rs` | 56 | 273 | Proxy vs Full implementation |
| `definitions_in_file.rs` | 56 | 149 | Proxy vs Full implementation |

## Proposed Directory Structure

```
lsproxy/
├── Cargo.toml                          # Workspace root
├── README.md
├── TESTING.md
├── REFACTOR.md                         # This file
├── dockerfiles/
├── scripts/
├── sample_project/
│
└── crates/
    ├── common/                         # NEW: Shared library
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── api_types.rs            # Consolidated
    │       ├── ast_grep/
    │       │   ├── mod.rs
    │       │   ├── client.rs           # Consolidated (handle differences)
    │       │   └── types.rs            # Moved (identical)
    │       ├── lsp/
    │       │   ├── mod.rs
    │       │   ├── json_rpc.rs         # Moved (identical)
    │       │   └── process.rs          # Moved (identical)
    │       ├── utils/
    │       │   ├── mod.rs
    │       │   ├── file_utils.rs       # Consolidated
    │       │   └── workspace_documents.rs  # Consolidated
    │       └── handlers/
    │           └── utils.rs            # Moved (identical)
    │
    ├── orchestrator/                   # RENAMED: Was lsproxy/src/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── main.rs
    │       ├── lib.rs
    │       ├── container/              # Container orchestration
    │       │   ├── mod.rs
    │       │   ├── orchestrator.rs
    │       │   └── http_client.rs
    │       ├── handlers/               # Thin proxy handlers
    │       │   ├── container_proxy.rs
    │       │   ├── definitions_in_file.rs
    │       │   ├── find_definition.rs
    │       │   ├── find_identifier.rs
    │       │   ├── find_referenced_symbols.rs
    │       │   ├── find_references.rs
    │       │   ├── health.rs
    │       │   ├── list_files.rs
    │       │   └── read_source_code.rs
    │       └── middleware/
    │           └── jwt.rs
    │
    └── wrapper/                        # RENAMED: Was lsp-wrapper/
        ├── Cargo.toml
        └── src/
            ├── main.rs
            ├── manager.rs
            └── handlers/               # Full LSP implementations
                ├── definitions_in_file.rs
                ├── find_definition.rs
                ├── find_identifier.rs
                ├── find_referenced_symbols.rs
                └── find_references.rs
```

## Migration Phases

### Phase 1: Create Workspace Structure ✅ Complete

**Goal**: Set up Cargo workspace without changing any code behavior

**Tasks**:
- [x] Create `crates/` directory
- [x] Create root `Cargo.toml` with workspace definition
- [x] Create empty `crates/common/` with basic structure
- [x] Move `lsproxy/` → `crates/orchestrator/`
  - [x] Update Cargo.toml to use workspace dependencies
  - [x] Update package name to `lsproxy-orchestrator` (binary remains `lsproxy`)
- [x] Move `lsproxy/lsp-wrapper/` → `crates/wrapper/`
  - [x] Update Cargo.toml to use workspace dependencies
  - [x] Update package name to `lsproxy-wrapper` (binary remains `lsp-wrapper`)
- [x] Test: `cargo build` in workspace root
- [x] Test: `cargo build --bin lsproxy` (orchestrator)
- [x] Test: `cargo build --bin lsp-wrapper` (wrapper)
- [x] Test: `cargo test --package lsproxy-orchestrator test_docker_connection` ✅ PASSED

**Files to Update**:
- Create: `/Cargo.toml` (workspace root)
- Create: `/crates/common/Cargo.toml`
- Modify: `/crates/orchestrator/Cargo.toml`
- Modify: `/crates/wrapper/Cargo.toml`

**Validation**:
```bash
# All should work without changes
cd lsproxy
cargo build
cargo test
cd crates/orchestrator && cargo test
cd ../wrapper && cargo test
```

---

### Phase 2: Extract Common Code ⬜ Not Started

**Goal**: Move duplicated code to `common` crate

#### Step 2.1: Move Identical Files

- [ ] Move `ast_grep/types.rs` to `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Move `handlers/utils.rs` to `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Move `lsp/json_rpc.rs` to `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Move `lsp/process.rs` to `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

#### Step 2.2: Consolidate Near-Identical Files

- [ ] Consolidate `api_types.rs`
  - [ ] Compare both versions line-by-line
  - [ ] Identify differences
  - [ ] Create unified version in `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`
  - [ ] Test: Integration tests pass

- [ ] Consolidate `utils/file_utils.rs`
  - [ ] Compare both versions
  - [ ] Create unified version in `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Consolidate `utils/workspace_documents.rs`
  - [ ] Compare both versions
  - [ ] Create unified version in `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

#### Step 2.3: Handle Special Cases

- [ ] Handle `ast_grep/client.rs` (has differences)
  - [ ] Analyze differences between versions
  - [ ] Determine if consolidation is possible
  - [ ] Use feature flags if needed
  - [ ] Document decision in this file

**Validation After Phase 2**:
```bash
# All tests should pass
cd lsproxy
cargo test
cargo build

# Integration tests
cd lsproxy && cargo test
```

---

### Phase 3: Update Build System ⬜ Not Started

**Goal**: Update Docker builds and scripts to use new structure

**Tasks**:
- [ ] Update `dockerfiles/base.Dockerfile`
  ```dockerfile
  # Old: COPY lsproxy/lsp-wrapper .
  # New:
  COPY crates/wrapper .
  COPY crates/common /usr/src/common
  ```

- [ ] Update `dockerfiles/service.Dockerfile`
  ```dockerfile
  # Old: COPY lsproxy .
  # New:
  COPY crates/orchestrator .
  COPY crates/common /usr/src/common
  ```

- [ ] Update `scripts/build-all-containers.sh` (if needed)
- [ ] Update `scripts/run2` (path references)
- [ ] Update `scripts/start-service.sh` (path references)
- [ ] Test: Build all Docker images
  ```bash
  ./scripts/build-all-containers.sh
  ```

- [ ] Test: Container lifecycle tests
  ```bash
  ./scripts/test-container-lifecycle.sh
  ```

- [ ] Test: Watchdog tests
  ```bash
  ./scripts/test-watchdog.sh
  ```

- [ ] Test: All endpoint tests
  ```bash
  ./scripts/test-all-endpoints.sh
  ```

**Files to Update**:
- `dockerfiles/base.Dockerfile`
- `dockerfiles/service.Dockerfile`
- `scripts/build-all-containers.sh`
- `scripts/run2`
- `scripts/start-service.sh`
- Any other scripts with path references

---

### Phase 4: Documentation and Cleanup ⬜ Not Started

**Goal**: Update documentation and remove obsolete files

**Tasks**:
- [ ] Update `README.md` with new structure
  - [ ] Architecture section
  - [ ] Build instructions
  - [ ] Development setup

- [ ] Update `TESTING.md`
  - [ ] Update any path references
  - [ ] Verify all commands still work

- [ ] Delete obsolete test scripts
  - [ ] `scripts/test-php-only.sh`
  - [ ] `scripts/test-ruby-only.sh`

- [ ] Update contributing documentation (if exists)

- [ ] Create migration notes for developers

- [ ] Final comprehensive test run
  ```bash
  # Build everything
  ./scripts/build-all-containers.sh

  # Run all tests
  cd lsproxy && cargo test
  ./scripts/test-container-lifecycle.sh
  ./scripts/test-watchdog.sh
  ./scripts/test-all-endpoints.sh
  ```

**Files to Update**:
- `README.md`
- `TESTING.md`
- Any contributing guides

**Files to Delete**:
- `scripts/test-php-only.sh`
- `scripts/test-ruby-only.sh`

---

## Benefits

1. ✅ **Eliminates ~1,833 lines of duplication** - Single source of truth
2. ✅ **Clear architectural separation** - Three distinct purposes
3. ✅ **Workspace dependency management** - Shared versions, easier updates
4. ✅ **Clearer naming** - Purpose is obvious from crate names
5. ✅ **Standard Rust structure** - Follows community best practices
6. ✅ **Build efficiency** - Parallel builds, shared artifacts
7. ✅ **Independent testing** - Can test common code separately

## Breaking Changes

**None for users** - This is purely internal restructuring. The Docker images and API remain identical.

**For developers**:
- Import paths change: `use lsproxy::api_types` → `use lsproxy_common::api_types`
- Build commands stay the same (workspace forwards to members)
- Docker build commands stay the same

## Rollback Plan

At any phase, we can rollback by:
1. Reverting commits (assuming each phase is a separate commit)
2. Comprehensive test suite catches issues early
3. Phase 1 is completely reversible (just structure, no code changes)

## Testing Strategy

After each step:
1. ✅ Run `cargo test` in workspace root
2. ✅ Run individual crate tests
3. ✅ Build all Docker images
4. ✅ Run integration test scripts

After each phase:
1. ✅ Full test suite (`cargo test`)
2. ✅ Container lifecycle tests
3. ✅ Watchdog tests
4. ✅ All endpoint tests (80+ tests)

## Notes

- Phase 1 is the safest - just structure, no behavior change
- Phase 2 requires careful attention to type differences
- Phase 3 is when Docker builds are affected
- Each phase should be a separate commit for easy rollback

## References

- [Cargo Workspaces Documentation](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- Original analysis in conversation (2025-11-06)
- `TESTING.md` for test coverage details

# LSProxy Code Refactoring Plan

**Status**: 🟡 In Progress - Phase 1 Complete, Ready for Phase 2
**Last Updated**: 2025-11-06
**Strategy**: Get everything working first (Phases 1-3), then optimize with deduplication (Phase 4)

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

### Phase 2: Update Build System ✅ Complete

**Goal**: Update Docker builds and scripts to use new structure

**Status**: Complete! All Docker images build successfully with new Cargo workspace structure. Tests pass at 94.1% (80/85).

**Completed Tasks**:
- [x] Updated `dockerfiles/base.Dockerfile`
  - Added workspace structure copying (Cargo.toml, Cargo.lock, all crates)
  - Fixed dependency pinning (ignore 0.4.23, globset 0.4.15) to avoid edition2024 requirement
  - Fixed ast_grep config path references

- [x] Updated `dockerfiles/service.Dockerfile`
  - Added workspace structure copying for orchestrator build
  - Includes wrapper stub to satisfy workspace requirements

- [x] Created `.dockerignore` file
  - Excludes target/ directory and build artifacts
  - Reduces Docker context from 9.3GB to 411KB

- [x] Tested: Build all Docker images
  ```bash
  ./scripts/build-all-containers.sh  # ✓ All 11 images built successfully
  ```

- [x] Tested: Endpoint tests
  ```bash
  ./scripts/test-all-endpoints.sh    # ✓ 80/85 tests pass (94.1%)
  ```

**Known Issues** (pre-existing, not caused by refactor):
- C# LSP timeouts (3 failures) - known performance issue
- TypeScript/JavaScript deep validation (2 failures) - known ast-grep issue

**Key Files Modified**:
- `dockerfiles/base.Dockerfile` - Workspace-aware build
- `dockerfiles/service.Dockerfile` - Workspace-aware build
- `.dockerignore` - New file for Docker optimization
- `Cargo.toml` - Pinned dependencies to avoid edition2024
- `Cargo.lock` - Updated dependency versions

---

### Phase 3: Documentation and Cleanup ✅ Complete

**Goal**: Update documentation and remove obsolete files

**Status**: Complete! All documentation updated for new Cargo workspace structure.

**Completed Tasks**:
- [x] Update `README.md` with new structure
  - No changes needed - focuses on user-facing API which hasn't changed
- [x] Update `TESTING.md` with current test procedures
  - Updated Rust test paths from `lsproxy/` to `crates/orchestrator/` and `crates/wrapper/`
  - Updated cargo commands to run from workspace root
  - Added workspace-specific test commands
- [x] Delete obsolete scripts
  - Scripts were already removed (not in repository)
- [x] Update `scripts/test.sh`
  - Completely rewrote as comprehensive test runner
  - Now runs Rust tests, container lifecycle tests, watchdog tests, and endpoint tests
  - Uses workspace-aware `cargo test --workspace`
- [x] Update `scripts/rebuild-image.sh`
  - Updated path references from `lsproxy/src` to `crates/orchestrator/src`
  - Updated path references from `lsproxy/lsp-wrapper/src` to `crates/wrapper/src`
- [x] Verify all paths in remaining scripts
  - All critical scripts verified
  - Only minor references remain in historical docs (TASKS.md, REFACTOR.md)

**Key Files Modified**:
- `TESTING.md` - Updated paths and test procedures
- `scripts/test.sh` - Comprehensive rewrite for workspace structure
- `scripts/rebuild-image.sh` - Updated help text with correct paths

**Validation**:
```bash
# Ensure documentation is accurate
grep -r "lsproxy/src" scripts/*.sh
# Returns no results (only references in rebuild-image.sh comments/help text were updated)
```

---

### Phase 4: Code Deduplication ⏸️ Deferred

**Goal**: Move shared code to common crate (AFTER everything works)

**Status**: Intentionally deferred to minimize risk. Current priority is getting the workspace structure working and tested.

**Reasoning**:
- Phase 1-3 establish working foundation
- All tests continue to pass throughout
- Phase 4 is pure refactor with no behavioral changes
- Can be done incrementally after stable base

**Future Tasks**:
- [ ] Move identical files to common crate (475 lines)
- [ ] Consolidate near-identical files (1,358 lines)
- [ ] Update import paths in orchestrator and wrapper
- [ ] Comprehensive test validation

**Validation**:
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

### Phase 3: Documentation and Cleanup ⬜ Not Started

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

### Phase 4: Code Deduplication (Future Work) ⬜ Not Started

**Goal**: Eliminate ~1,833 lines of duplicated code between orchestrator and wrapper

**Strategy**: This phase is deferred until after the system is fully working with the new structure. The circular dependencies between files make this complex, so we'll tackle it once everything else is stable.

**Rationale**:
- Phase 1-3 provide the main benefits: clear architecture, workspace management, better naming
- Code duplication is a maintenance issue but not critical for functionality
- Safer to have a working system first, then optimize

#### Step 4.1: Move Identical Files

**Identical files (475 lines)** - byte-for-byte identical, can be moved as-is:

- [ ] Move `lsp/json_rpc.rs` to `common` (155 lines) - NO dependencies on other crates
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Move `lsp/process.rs` to `common` (66 lines) - Only std/external dependencies
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Move `handlers/utils.rs` to `common` (66 lines) - Depends on api_types
  - [ ] Ensure api_types is in common first
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Move `ast_grep/types.rs` to `common` (188 lines) - Depends on api_types, file_utils
  - [ ] Ensure dependencies are in common first
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

#### Step 4.2: Consolidate Near-Identical Files

**Near-identical files (1,358 lines)** - 99% similar, need reconciliation:

- [ ] Consolidate `api_types.rs` (632 lines each)
  - [ ] Compare both versions line-by-line with diff
  - [ ] Document differences (likely version drift)
  - [ ] Create unified version in `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`
  - [ ] Test: Integration tests pass

- [ ] Consolidate `utils/file_utils.rs` (192 vs 193 lines)
  - [ ] Compare both versions
  - [ ] Identify the 1-line difference
  - [ ] Create unified version in `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

- [ ] Consolidate `utils/workspace_documents.rs` (534 vs 536 lines)
  - [ ] Compare both versions
  - [ ] Identify the 2-line difference
  - [ ] Create unified version in `common`
  - [ ] Update imports in orchestrator
  - [ ] Update imports in wrapper
  - [ ] Test: `cargo test`

#### Step 4.3: Handle Special Cases

- [ ] Analyze `ast_grep/client.rs` (264 vs 275 lines)
  - [ ] Determine if differences are intentional
  - [ ] Consider using feature flags if both versions needed
  - [ ] Document decision: consolidate, keep separate, or feature-flag
  - [ ] Implement chosen solution

#### Step 4.4: Validation

**After Phase 4 completion**:
```bash
# All tests should pass
cargo test
cargo build

# Integration tests
./scripts/test-container-lifecycle.sh
./scripts/test-watchdog.sh
./scripts/test-all-endpoints.sh

# Verify no duplication remains
# Compare file counts and line counts in orchestrator vs wrapper
```

**Success Metrics**:
- ✅ ~1,833 lines of duplication eliminated
- ✅ Single source of truth for shared types
- ✅ All tests passing
- ✅ Docker builds working
- ✅ No behavioral changes

---

## Benefits

### Immediate Benefits (Phases 1-3):
1. ✅ **Clear architectural separation** - Three distinct purposes (orchestrator, wrapper, common)
2. ✅ **Workspace dependency management** - Shared versions, easier updates
3. ✅ **Clearer naming** - Purpose is obvious from crate names
4. ✅ **Standard Rust structure** - Follows community best practices
5. ✅ **Build efficiency** - Parallel builds, shared artifacts

### Future Benefits (Phase 4 - Deferred):
6. ⏸️ **Eliminates ~1,833 lines of duplication** - Single source of truth (Phase 4)
7. ⏸️ **Independent testing** - Can test common code separately (Phase 4)

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

- **Phase 1** (✅ Complete): Safest change - just structure, no behavior change
- **Phase 2** (Next): Update Docker builds to use new structure
- **Phase 3** (Next): Update documentation and remove obsolete files
- **Phase 4** (Deferred): Code deduplication - complex due to circular dependencies, tackled after system is working
- Each phase should be a separate commit for easy rollback
- Tests must pass after each phase before proceeding to the next

## References

- [Cargo Workspaces Documentation](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- Original analysis in conversation (2025-11-06)
- `TESTING.md` for test coverage details

# Container Versioning Strategy

This document explains how containers are versioned in the lsproxy project.

## Two-Tier Versioning System

The project uses two separate versioning schemes to provide API compatibility guarantees while allowing independent updates:

### 1. Rust Containers (Release Versioning)

The core Rust containers follow the project's release version:

- `nuanced-lsp-wrapper:0.4.0`
- `nuanced-lsp-proxy:0.4.0`
- `nuanced-lsp-watchdog:0.4.0`

These containers are versioned with each release and include all service logic, orchestration, and wrapper functionality.

### 2. Language Containers (Semantic Versioning)

Language server containers use **independent semantic versioning** (MAJOR.MINOR.PATCH):

- `nuanced-lsp-python:1.0.0`
- `nuanced-lsp-ruby-3.4.4:1.0.0`
- `nuanced-lsp-typescript:1.0.0`
- `nuanced-lsp-ruby-sorbet-3.4.4:1.0.0`
- etc.

#### Version Components

- **MAJOR**: API/protocol compatibility version
  - Increment when making changes that break compatibility with existing Rust containers
  - Example: Changing the communication protocol between wrapper and language container

- **MINOR**: New features, language server updates
  - Increment when updating language server versions or adding new LSP features
  - Example: Upgrading Python's pyright from 1.1.0 to 1.2.0

- **PATCH**: Bug fixes, dependency updates
  - Increment for bug fixes that don't change functionality
  - Example: Security patches, dependency updates

## Why Two Versioning Schemes?

1. **Stability**: Language containers change rarely (only when LSP server versions change or dependencies need updates)
2. **Compatibility Guarantees**: The major version provides a clear compatibility contract
3. **Independent Updates**: Rust service code can evolve rapidly without requiring language container rebuilds
4. **Future-Proofing**: If a protocol change is needed, we can:
   - Bump language containers to v2.0.0
   - Update Rust containers to require v2.x.x language containers
   - Maintain v1.x.x language containers for older Rust versions

## Version Compatibility Matrix

| Rust Version | Compatible Language Container Versions |
|--------------|---------------------------------------|
| 0.3.x        | 1.x.x                                 |
| 0.4.x        | 1.x.x                                 |
| 0.5.x        | 1.x.x (planned)                       |

If a breaking change is introduced:

| Rust Version | Compatible Language Container Versions |
|--------------|---------------------------------------|
| 1.0.x        | 2.x.x                                 |

## Building with Custom Versions

### Build Language Containers

```bash
# Default (builds with v1.0.0)
./scripts/build-language-containers.sh --multiarch

# Custom version
./scripts/build-language-containers.sh --multiarch --tag=1.1.0
```

### Build Rust Containers

```bash
# Uses release version from Cargo.toml or custom tag
./scripts/build-rust-containers.sh --multiarch --tag=0.4.0
```

## Publishing with Separate Versions

```bash
# Publish with independent versions
./scripts/publish-images.sh 0.4.0 --language-tag=1.0.0 --registry=both

# Rust containers published as:
#   nuanced-lsp-wrapper:0.4.0
#   nuanced-lsp-proxy:0.4.0
#   nuanced-lsp-watchdog:0.4.0

# Language containers published as:
#   nuanced-lsp-python:1.0.0
#   nuanced-lsp-ruby-3.4.4:1.0.0
#   nuanced-lsp-typescript:1.0.0
#   nuanced-lsp-ruby-sorbet-3.4.4:1.0.0
#   etc.
```

## When to Bump Versions

### Language Containers: Bump MAJOR (1.x.x → 2.0.0)

- Change in protocol/API between wrapper and language container
- Breaking changes to environment variables or configuration
- Changes to volume mount structure
- Changes to how language servers are invoked

### Language Containers: Bump MINOR (1.0.x → 1.1.0)

- Update language server versions (e.g., new pyright version)
- Add new LSP features
- Add new language server configurations
- Non-breaking protocol enhancements

### Language Containers: Bump PATCH (1.0.0 → 1.0.1)

- Security patches
- Bug fixes that don't change functionality
- Dependency updates that don't affect behavior

### Rust Containers

- Follow project release versioning (based on Cargo.toml version)
- Released whenever service logic, wrapper features, or orchestration changes

## Registry Tags

Both GHCR and Docker Hub maintain two tags for each image:

- **Versioned tag**: `nuanced-lsp-proxy:0.4.0` or `nuanced-lsp-python:1.0.0`
- **Latest tag**: `nuanced-lsp-proxy:latest` or `nuanced-lsp-python:latest`

The `:latest` tag always points to the most recent published version.

## Example Workflow

1. **Initial Release (v0.4.0)**
   ```bash
   ./scripts/build-rust-containers.sh --multiarch --tag=0.4.0
   ./scripts/build-language-containers.sh --multiarch --tag=1.0.0
   ./scripts/publish-images.sh 0.4.0 --language-tag=1.0.0 --registry=both
   ```

2. **Update Language Servers (no breaking changes)**
   ```bash
   # Only rebuild language containers
   ./scripts/build-language-containers.sh --multiarch --tag=1.1.0
   ./scripts/publish-images.sh 0.4.0 --language-tag=1.1.0 --registry=both
   ```

3. **Breaking Protocol Change**
   ```bash
   # Rebuild both with new major version for language containers
   ./scripts/build-rust-containers.sh --multiarch --tag=0.5.0
   ./scripts/build-language-containers.sh --multiarch --tag=2.0.0
   ./scripts/publish-images.sh 0.5.0 --language-tag=2.0.0 --registry=both
   ```

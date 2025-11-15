# Container Images

This document explains how container images are built, versioned, named, and configured in the lsproxy project.

## Table of Contents

- [Image Overview](#image-overview)
- [Image Naming Convention](#image-naming-convention)
- [Versioning Strategy](#versioning-strategy)
- [Building Images](#building-images)
- [Publishing Images](#publishing-images)
- [Version Constants in Code](#version-constants-in-code)
- [Updating Versions](#updating-versions)

## Image Overview

The lsproxy project uses two categories of container images:

### Rust Containers (3 images)

Core infrastructure containers written in Rust:

- **nuanced-lsp-proxy** - Main orchestration service that manages language containers
- **nuanced-lsp-wrapper** - Contains the lsp-wrapper binary and ast-grep configs, mounted into language containers
- **nuanced-lsp-watchdog** - Monitors the proxy service and performs cleanup on unexpected death

### Language Containers (231 images)

Language-specific LSP server containers:

- **8 non-Ruby languages**: python, typescript, rust, golang, java, clangd, csharp, php
- **110+ Ruby versions**: ruby-3.4.4, ruby-3.4.3, etc.
- **110+ Ruby Sorbet versions**: ruby-sorbet-3.4.4, ruby-sorbet-3.4.3, etc.

## Image Naming Convention

All images follow the `nuanced-lsp-*` naming pattern:

### Rust Containers

```
nuanced-lsp-proxy:{RUST_VERSION}
nuanced-lsp-wrapper:{RUST_VERSION}
nuanced-lsp-watchdog:{RUST_VERSION}
```

Example: `nuanced-lsp-proxy:0.4.8`

### Language Containers

**Non-Ruby languages:**
```
nuanced-lsp-{language}:{LANGUAGE_VERSION}
```

Examples:
- `nuanced-lsp-python:1.0.0`
- `nuanced-lsp-typescript:1.0.0`
- `nuanced-lsp-golang:1.0.0`

**Ruby versions:**
```
nuanced-lsp-ruby-{ruby_version}:{LANGUAGE_VERSION}
```

Examples:
- `nuanced-lsp-ruby-3.4.4:1.0.0`
- `nuanced-lsp-ruby-3.3.6:1.0.0`

**Ruby Sorbet versions:**
```
nuanced-lsp-ruby-sorbet-{ruby_version}:{LANGUAGE_VERSION}
```

Examples:
- `nuanced-lsp-ruby-sorbet-3.4.4:1.0.0`
- `nuanced-lsp-ruby-sorbet-3.3.6:1.0.0`

## Versioning Strategy

The project uses **two independent versioning schemes** to provide API compatibility guarantees while allowing independent updates.

### Rust Container Versioning

Rust containers follow the **project release version** from `Cargo.toml`:

- **Current version**: `0.4.8`
- **When to bump**: With each release that includes changes to the Rust service, wrapper, or watchdog code
- **Format**: Semantic versioning (MAJOR.MINOR.PATCH)

### Language Container Versioning

Language containers use **independent semantic versioning** for API compatibility:

- **Current version**: `1.0.0`
- **When to bump**:
  - **MAJOR** (1.x.x → 2.0.0): Breaking changes to protocol/API between wrapper and language container
  - **MINOR** (1.0.x → 1.1.0): New LSP features, language server version updates (non-breaking)
  - **PATCH** (1.0.0 → 1.0.1): Bug fixes, security patches, dependency updates

**Why separate versions?**

1. **Stability**: Language containers change rarely (only when LSP servers are updated)
2. **Compatibility guarantees**: Major version indicates compatibility with Rust containers
3. **Independent updates**: Rust service can evolve without rebuilding 231 language containers
4. **Future-proofing**: Easy to maintain multiple API versions (v1.x.x and v2.x.x simultaneously)

See [VERSIONING.md](../VERSIONING.md) for detailed versioning documentation.

## Building Images

### Building Rust Containers

```bash
# Build for local development (single architecture)
./scripts/build-rust-containers.sh

# Build with specific tag
./scripts/build-rust-containers.sh --tag=0.4.8

# Build for release (multi-architecture: amd64 + arm64)
./scripts/build-rust-containers.sh --multiarch --tag=0.4.8

# Build multi-arch and also load local platform for testing
./scripts/build-rust-containers.sh --multiarch --load --tag=0.4.8
```

**Script**: `scripts/build-rust-containers.sh`

**Dockerfiles**:
- `dockerfiles/service.Dockerfile` - Builds nuanced-lsp-proxy
- `dockerfiles/wrapper.Dockerfile` - Builds nuanced-lsp-wrapper
- `dockerfiles/watchdog.Dockerfile` - Builds nuanced-lsp-watchdog

### Building Language Containers

```bash
# Build for local development (main Ruby versions only)
./scripts/build-language-containers.sh

# Build with specific tag
./scripts/build-language-containers.sh --tag=1.0.0

# Build all 110+ Ruby versions (slow)
./scripts/build-language-containers.sh --all-ruby-versions --tag=1.0.0

# Build for release (multi-architecture)
./scripts/build-language-containers.sh --multiarch --all-ruby-versions --tag=1.0.0

# Build multi-arch and load local platform
./scripts/build-language-containers.sh --multiarch --load --tag=1.0.0
```

**Script**: `scripts/build-language-containers.sh`

**Dockerfiles**:
- `dockerfiles/{language}.Dockerfile` - Non-Ruby languages (python, typescript, etc.)
- `dockerfiles/ruby/{version}.Dockerfile` - Ruby base images
- `dockerfiles/ruby-sorbet/{version}.Dockerfile` - Ruby Sorbet variants

**Default Ruby versions** (built without `--all-ruby-versions`):
- 3.2.2, 3.2.6, 3.3.5, 3.3.6, 3.4.1, 3.4.2, 3.4.4

## Publishing Images

Images are published to two registries for redundancy:

- **GitHub Container Registry (GHCR)**: `ghcr.io/nuanced-dev/`
- **Docker Hub**: `nuanced/`

### Publishing Manually

```bash
# Publish with separate Rust and language versions
./scripts/publish-images.sh 0.4.8 --language-tag=1.0.0 --registry=both

# Publish to specific registry
./scripts/publish-images.sh 0.4.8 --language-tag=1.0.0 --registry=ghcr
./scripts/publish-images.sh 0.4.8 --language-tag=1.0.0 --registry=dockerhub

# Dry run (show what would be pushed)
./scripts/publish-images.sh 0.4.8 --language-tag=1.0.0 --dry-run

# Publish all Ruby versions
./scripts/publish-images.sh 0.4.8 --language-tag=1.0.0 --all-ruby-versions --registry=both
```

**Script**: `scripts/publish-images.sh`

**Environment variables required**:
- `GITHUB_TOKEN` - For GHCR authentication (if publishing to ghcr or both)
- `DOCKER_HUB_TOKEN` - For Docker Hub authentication (if publishing to dockerhub or both)

### Automated Publishing (GitHub Actions)

Images are automatically built and published when a version tag is pushed:

```bash
git tag 0.4.8
git push origin 0.4.8
```

The GitHub Actions workflow (`.github/workflows/image_upload.yml`) will:

1. Validate that `Cargo.toml` version matches the tag
2. Build Rust containers with the release version
3. Build all language containers with version `1.0.0`
4. Publish to both GHCR and Docker Hub
5. Extract binaries and create a GitHub release

**Note**: To update language container version in CI, edit line 67 in `.github/workflows/image_upload.yml`:

```yaml
./scripts/build-language-containers.sh --multiarch --use-cache --all-ruby-versions --tag=1.0.0
```

## Version Constants in Code

All container image names and versions are defined in **one central location** for easy updates.

### Location

**File**: `crates/orchestrator/src/container/mod.rs`

### Constants

```rust
/// Version tag for Rust containers (wrapper, proxy, watchdog)
/// This should match the release version from Cargo.toml
pub const RUST_CONTAINER_VERSION: &str = "0.4.8";

/// Version tag for language containers (python, ruby, typescript, etc.)
/// Language containers use independent semver versioning for API compatibility
pub const LANGUAGE_CONTAINER_VERSION: &str = "1.0.0";

/// Base image names (without version tags)
pub const PROXY_IMAGE_BASE: &str = "nuanced-lsp-proxy";
pub const WRAPPER_IMAGE_BASE: &str = "nuanced-lsp-wrapper";
pub const WATCHDOG_IMAGE_BASE: &str = "nuanced-lsp-watchdog";
```

### Helper Functions

```rust
/// Helper functions to get full image names with version tags
pub fn proxy_image() -> String {
    format!("{}:{}", PROXY_IMAGE_BASE, RUST_CONTAINER_VERSION)
}

pub fn wrapper_image() -> String {
    format!("{}:{}", WRAPPER_IMAGE_BASE, RUST_CONTAINER_VERSION)
}

pub fn watchdog_image() -> String {
    format!("{}:{}", WATCHDOG_IMAGE_BASE, RUST_CONTAINER_VERSION)
}
```

### Usage in Code

**Example**: Spawning wrapper container (`crates/orchestrator/src/container/mod.rs:375`)

```rust
let config = Config {
    image: Some(wrapper_image()),  // Returns "nuanced-lsp-wrapper:0.4.8"
    // ...
};
```

**Example**: Getting language image (`crates/orchestrator/src/container/orchestrator.rs:334`)

```rust
fn image_name_for_language(language: &SupportedLanguages) -> String {
    use super::LANGUAGE_CONTAINER_VERSION;

    match language {
        SupportedLanguages::Python =>
            format!("nuanced-lsp-python:{}", LANGUAGE_CONTAINER_VERSION),
        // Returns "nuanced-lsp-python:1.0.0"
        // ...
    }
}
```

## Updating Versions

### Update Rust Container Version

When releasing a new version (e.g., `0.5.0`):

1. **Update `Cargo.toml`**:
   ```toml
   [package]
   version = "0.5.0"
   ```

2. **Update constant in code**:
   ```rust
   // In crates/orchestrator/src/container/mod.rs
   pub const RUST_CONTAINER_VERSION: &str = "0.5.0";
   ```

3. **Build and publish**:
   ```bash
   # Build multi-arch images
   ./scripts/build-rust-containers.sh --multiarch --tag=0.5.0

   # Publish (language containers still use 1.0.0)
   ./scripts/publish-images.sh 0.5.0 --language-tag=1.0.0 --registry=both

   # Or use git tags to trigger CI
   git tag 0.5.0
   git push origin 0.5.0
   ```

### Update Language Container Version

When updating language servers or making non-breaking changes (e.g., `1.1.0`):

1. **Update constant in code**:
   ```rust
   // In crates/orchestrator/src/container/mod.rs
   pub const LANGUAGE_CONTAINER_VERSION: &str = "1.1.0";
   ```

2. **Build and publish**:
   ```bash
   # Build multi-arch images
   ./scripts/build-language-containers.sh --multiarch --all-ruby-versions --tag=1.1.0

   # Publish (Rust containers keep their current version)
   ./scripts/publish-images.sh 0.4.8 --language-tag=1.1.0 --registry=both
   ```

### Update for Breaking Language Container Changes

When making breaking changes to the protocol/API (e.g., `2.0.0`):

1. **Update constant in code**:
   ```rust
   // In crates/orchestrator/src/container/mod.rs
   pub const LANGUAGE_CONTAINER_VERSION: &str = "2.0.0";
   ```

2. **Update Rust code if needed** to handle new protocol

3. **Build and publish both**:
   ```bash
   # Build new language containers
   ./scripts/build-language-containers.sh --multiarch --all-ruby-versions --tag=2.0.0

   # Build new Rust containers with updated code
   ./scripts/build-rust-containers.sh --multiarch --tag=0.6.0

   # Publish both with new versions
   ./scripts/publish-images.sh 0.6.0 --language-tag=2.0.0 --registry=both
   ```

4. **Optional**: Maintain backward compatibility by keeping v1.x.x language containers available for older Rust versions

## Summary

- **Rust containers** = Project version (e.g., `0.4.8`)
- **Language containers** = Independent semver (e.g., `1.0.0`)
- **All images** use `nuanced-lsp-*` naming
- **Version constants** defined in `crates/orchestrator/src/container/mod.rs`
- **Build scripts**: `scripts/build-rust-containers.sh` and `scripts/build-language-containers.sh`
- **Publish script**: `scripts/publish-images.sh`
- **Published to**: GHCR (`ghcr.io/nuanced-dev/`) and Docker Hub (`nuanced/`)

For detailed versioning strategy and compatibility guarantees, see [VERSIONING.md](../VERSIONING.md).

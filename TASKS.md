# Dynamic Container Orchestration Implementation Plan

## Overview

Transform LSProxy from a single 12.5GB monolithic Docker image to a dynamic container orchestration system with language-specific images.

## Current Architecture

- Single Docker image (12.5GB) containing all 10 languages and LSP servers
- Rust service runs inside the container
- Manager detects languages and starts LSP clients as local processes
- Process communication via stdin/stdout using LSP protocol

## Target Architecture

- **Base Image**: Minimal Rust service + Docker client (~500MB)
- **Language Images**: Separate images per language (e.g., `lsproxy-golang`, `lsproxy-ruby`)
- **Container Orchestration**: Rust service manages language containers dynamically
- **Communication**: HTTP-based inter-container communication
- **Lifecycle**: Containers spawn on workspace initialization, cleanup on shutdown

## Detailed Task Breakdown

### Phase 1: Foundation Setup

#### Task 1.1: Add Docker SDK Dependencies
**File**: `lsproxy/Cargo.toml`

**Actions**:
- Add `bollard = "0.16"` dependency for Docker API communication
- Add `hyper = "0.14"` for HTTP client communication with LSP containers
- Add `tower = "0.4"` for service utilities

**Verification**:
- Run `cargo check` in `lsproxy` directory
- Ensure no dependency conflicts
- Verify compilation succeeds

**Command**:
```bash
cd lsproxy && cargo check
```

---

#### Task 1.2: Create Container Orchestrator Module
**File**: `lsproxy/src/container/mod.rs` (new)

**Actions**:
- Create `lsproxy/src/container/` directory
- Create `mod.rs` with basic module structure
- Define `ContainerOrchestrator` struct with:
  ```rust
  pub struct ContainerOrchestrator {
      docker: Arc<Docker>,
      containers: Arc<Mutex<HashMap<SupportedLanguages, ContainerInfo>>>,
      network_id: String,
  }

  struct ContainerInfo {
      container_id: String,
      image_name: String,
      port: u16,
      endpoint: String,
  }
  ```

**Verification**:
- Module compiles without errors
- Struct definitions are valid

**Command**:
```bash
cd lsproxy && cargo check --lib
```

---

#### Task 1.3: Implement Docker Connection
**File**: `lsproxy/src/container/mod.rs`

**Actions**:
- Implement `ContainerOrchestrator::new()`:
  - Connect to Docker daemon (Unix socket or TCP)
  - Verify Docker daemon is accessible
  - Create dedicated Docker network for LSP containers
  - Return error if Docker is unavailable

**Verification**:
- Write unit test `test_docker_connection()`
- Test should verify Docker connectivity
- Test should create and remove a test network

**Command**:
```bash
cd lsproxy && cargo test test_docker_connection -- --nocapture
```

---

### Phase 2: Container Lifecycle Management

#### Task 2.1: Implement Container Spawning
**File**: `lsproxy/src/container/orchestrator.rs` (new)

**Actions**:
- Create `orchestrator.rs` with container management functions
- Implement `spawn_container()` method:
  ```rust
  pub async fn spawn_container(
      &mut self,
      language: SupportedLanguages,
      workspace_path: &str,
  ) -> Result<ContainerInfo, OrchestratorError>
  ```
- Container configuration:
  - Mount workspace as read-only volume
  - Connect to LSP network
  - Set environment variables (WORKSPACE_PATH, LOG_LEVEL)
  - Expose dynamic port for HTTP communication
  - Set resource limits (memory, CPU)
- Wait for container to be healthy before returning

**Verification**:
- Write test `test_spawn_single_container()`
- Verify container is created and running
- Verify network connectivity
- Verify workspace volume is mounted
- Clean up container after test

**Command**:
```bash
cd lsproxy && cargo test test_spawn_single_container -- --nocapture
```

---

#### Task 2.2: Implement Container Health Checks
**File**: `lsproxy/src/container/orchestrator.rs`

**Actions**:
- Implement `check_container_health()` method
- Poll container's `/health` endpoint with retry logic
- Timeout after 30 seconds if container doesn't become healthy
- Return error with container logs if health check fails

**Verification**:
- Write test `test_container_health_check()`
- Test successful health check
- Test health check timeout scenario
- Verify error messages include container logs

**Command**:
```bash
cd lsproxy && cargo test test_container_health_check -- --nocapture
```

---

#### Task 2.3: Implement Container Cleanup
**File**: `lsproxy/src/container/orchestrator.rs`

**Actions**:
- Implement `stop_container()` method:
  - Gracefully stop container (SIGTERM)
  - Wait up to 10 seconds for graceful shutdown
  - Force kill if graceful shutdown times out
  - Remove container
- Implement `cleanup_all()` method:
  - Stop all tracked containers
  - Remove LSP network
  - Clear internal state

**Verification**:
- Write test `test_container_cleanup()`
- Spawn test container, then stop it
- Verify container is removed from Docker
- Write test `test_cleanup_all()`
- Spawn multiple containers, cleanup all
- Verify all containers and network are removed

**Commands**:
```bash
cd lsproxy && cargo test test_container_cleanup -- --nocapture
cd lsproxy && cargo test test_cleanup_all -- --nocapture
```

---

### Phase 3: Language-Specific Dockerfiles

#### Task 3.1: Create Dockerfile for Golang
**File**: `dockerfiles/golang.Dockerfile`

**Actions**:
- Create minimal Alpine-based image
- Install Go 1.23.5 and gopls
- Copy LSP HTTP wrapper script/binary
- Expose port 8080
- Set entrypoint to start HTTP LSP wrapper

**Size Target**: < 800MB

**Verification**:
- Build image: `docker build -f dockerfiles/golang.Dockerfile -t lsproxy-golang:latest .`
- Check image size: `docker images lsproxy-golang:latest`
- Run container with test workspace
- Verify gopls is functional via HTTP endpoint

**Commands**:
```bash
docker build -f dockerfiles/golang.Dockerfile -t lsproxy-golang:latest .
docker images lsproxy-golang:latest
docker run --rm -v $(pwd)/sample_project/golang:/workspace lsproxy-golang:latest
```

---

#### Task 3.2: Create Dockerfile for Python
**File**: `dockerfiles/python.Dockerfile`

**Actions**:
- Create minimal Alpine-based image
- Install Python 3.11 and jedi-language-server
- Copy LSP HTTP wrapper
- Expose port 8080
- Set entrypoint

**Size Target**: < 500MB

**Verification**:
- Build image
- Check size
- Test with Python workspace
- Verify jedi-language-server responds correctly

**Commands**:
```bash
docker build -f dockerfiles/python.Dockerfile -t lsproxy-python:latest .
docker images lsproxy-python:latest
```

---

#### Task 3.3: Create Dockerfile for TypeScript/JavaScript
**File**: `dockerfiles/typescript.Dockerfile`

**Actions**:
- Create Alpine-based image
- Install Node.js 20 and typescript-language-server
- Copy LSP HTTP wrapper
- Expose port 8080

**Size Target**: < 600MB

**Verification**:
- Build and test with TypeScript workspace
- Verify LSP responses for .ts and .js files

**Commands**:
```bash
docker build -f dockerfiles/typescript.Dockerfile -t lsproxy-typescript:latest .
```

---

#### Task 3.4: Create Dockerfile for Ruby (ruby-lsp)
**File**: `dockerfiles/ruby.Dockerfile`

**Actions**:
- Create image with rbenv
- Install Ruby 3.4.4, 3.3.6, 3.2.6
- Install ruby-lsp gem for each version
- Copy LSP HTTP wrapper with Ruby version detection
- Expose port 8080

**Size Target**: < 1.5GB (multiple Ruby versions)

**Verification**:
- Build and test with Ruby workspace
- Verify correct Ruby version is detected
- Test with different .ruby-version files

**Commands**:
```bash
docker build -f dockerfiles/ruby.Dockerfile -t lsproxy-ruby:latest .
```

---

#### Task 3.5: Create Dockerfile for Ruby Sorbet
**File**: `dockerfiles/ruby-sorbet.Dockerfile`

**Actions**:
- Similar to ruby.Dockerfile
- Install sorbet gem in addition to ruby-lsp
- Copy LSP HTTP wrapper configured for sorbet

**Size Target**: < 1.5GB

**Verification**:
- Build image
- Test with Ruby workspace containing `typed:` annotations
- Verify sorbet type checking works

---

#### Task 3.6: Create Dockerfile for Rust
**File**: `dockerfiles/rust.Dockerfile`

**Actions**:
- Create image with Rust 1.82.0
- Install rust-analyzer and rustfmt
- Copy LSP HTTP wrapper
- Expose port 8080

**Size Target**: < 2GB

**Verification**:
- Build and test with Rust workspace
- Verify rust-analyzer responds correctly

---

#### Task 3.7: Create Dockerfile for C/C++ (Clangd)
**File**: `dockerfiles/clangd.Dockerfile`

**Actions**:
- Create image with clangd
- Install build essentials (gcc, g++)
- Copy LSP HTTP wrapper
- Expose port 8080

**Size Target**: < 800MB

**Verification**:
- Build and test with C/C++ workspace

---

#### Task 3.8: Create Dockerfile for Java (JDTLS)
**File**: `dockerfiles/java.Dockerfile`

**Actions**:
- Create image with JDK 21
- Install Eclipse JDT Language Server
- Install Maven and Gradle
- Copy LSP HTTP wrapper

**Size Target**: < 1.2GB

**Verification**:
- Build and test with Java workspace
- Test with both Maven and Gradle projects

---

#### Task 3.9: Create Dockerfile for PHP (Phpactor)
**File**: `dockerfiles/php.Dockerfile`

**Actions**:
- Create image with PHP 8.x and Composer
- Install Phpactor
- Copy LSP HTTP wrapper

**Size Target**: < 700MB

**Verification**:
- Build and test with PHP workspace

---

#### Task 3.10: Create Dockerfile for C# (csharp-ls)
**File**: `dockerfiles/csharp.Dockerfile`

**Actions**:
- Create image with .NET SDK 8.0 and 9.0
- Install csharp-ls
- Copy LSP HTTP wrapper

**Size Target**: < 1.5GB

**Verification**:
- Build and test with C# workspace

---

### Phase 4: LSP HTTP Wrapper

#### Task 4.1: Create HTTP Wrapper Binary
**File**: `lsp-wrapper/src/main.rs` (new crate)

**Actions**:
- Create new Rust binary crate `lsp-wrapper`
- Implement HTTP server (actix-web) that:
  - Accepts LSP JSON-RPC requests via HTTP POST
  - Forwards to local LSP process via stdin/stdout
  - Returns LSP responses as HTTP responses
- Endpoints:
  - `POST /lsp` - LSP JSON-RPC requests
  - `GET /health` - Health check
- Environment variables:
  - `WORKSPACE_PATH` - Path to mounted workspace
  - `LSP_COMMAND` - Command to start LSP server
  - `PORT` - HTTP port (default 8080)

**Verification**:
- Build wrapper binary
- Test with gopls locally
- Verify request/response flow
- Test health endpoint

**Commands**:
```bash
cd lsp-wrapper && cargo build --release
cargo run -- --lsp-command="gopls" --workspace="/tmp/test" --port=8080
curl http://localhost:8080/health
```

---

#### Task 4.2: Add Wrapper to Language Dockerfiles
**File**: All `dockerfiles/*.Dockerfile`

**Actions**:
- Build lsp-wrapper binary in each language Dockerfile
- Configure appropriate LSP_COMMAND for each language
- Set up proper entrypoint

**Verification**:
- Rebuild all language images
- Verify wrapper is present and executable
- Test health endpoint for each image

---

### Phase 5: Remote LSP Client Implementation

#### Task 5.1: Create HTTP LSP Client
**File**: `lsproxy/src/lsp/remote_client.rs` (new)

**Actions**:
- Create `RemoteLspClient` struct:
  ```rust
  pub struct RemoteLspClient {
      endpoint: String,
      http_client: Client,
      workspace_documents: Arc<Mutex<WorkspaceDocuments>>,
      language: SupportedLanguages,
  }
  ```
- Implement `LspClient` trait for `RemoteLspClient`
- All trait methods send HTTP requests to container endpoint
- Parse HTTP responses back to LSP types

**Verification**:
- Write unit tests with mock HTTP server
- Test each LspClient trait method
- Verify error handling for network failures

**Commands**:
```bash
cd lsproxy && cargo test remote_client -- --nocapture
```

---

#### Task 5.2: Update Language Clients to Use Remote
**File**: `lsproxy/src/lsp/languages/*.rs`

**Actions**:
- Modify each language client (GoplsClient, JediClient, etc.) to:
  - Accept optional `container_endpoint` parameter
  - Use `RemoteLspClient` internally when endpoint is provided
  - Fall back to local process spawning if no endpoint (backward compatibility)

**Verification**:
- Write integration tests for each language
- Test both remote and local modes
- Verify same behavior in both modes

**Commands**:
```bash
cd lsproxy && cargo test languages::golang -- --nocapture
cd lsproxy && cargo test languages::python -- --nocapture
```

---

### Phase 6: Version Detection and Container Integration

#### Task 6.1: Create Version Detection Module
**File**: `lsproxy/src/container/version_detector.rs` (new)

**Actions**:
- Create `version_detector.rs` with version detection functions
- Implement version detection for each language:
  - **Ruby**: `.ruby-version`, `Gemfile` (ruby directive), `.tool-versions` (asdf)
  - **Python**: `.python-version`, `pyproject.toml` (requires-python), `Pipfile` (python_version), `runtime.txt` (Heroku)
  - **Node.js**: `.nvmrc`, `.node-version`, `package.json` (engines.node), `.tool-versions`
  - **Java**: `pom.xml` (maven.compiler.source/target), `build.gradle` (sourceCompatibility), `.java-version`, `.tool-versions`
  - **Go**: `go.mod` (first line: `go 1.23`), `.go-version`, `.tool-versions`
  - **PHP**: `composer.json` (require.php), `.php-version`, `.tool-versions`
- Return `Option<String>` with detected version (e.g., "3.4.4", "3.11", "20", "21", "1.23", "8.3")

**Verification**:
- Write unit tests for each language's detection
- Test with various config file formats
- Test fallback when no version specified (return None)

**Commands**:
```bash
cd lsproxy && cargo test version_detector -- --nocapture
```

---

#### Task 6.2: Implement Image Name Resolution
**File**: `lsproxy/src/container/version_detector.rs`

**Actions**:
- Add `resolve_image_name()` function:
  ```rust
  pub fn resolve_image_name(
      language: SupportedLanguages,
      detected_version: Option<&str>
  ) -> String
  ```
- Map detected versions to Docker image tags:
  - Exact match: Ruby 3.4.4 → `lsproxy-ruby-3.4.4:latest`
  - Fallback: Ruby 3.4.5 → `lsproxy-ruby-3.4.4:latest` (closest available)
  - No version: Ruby (no version file) → `lsproxy-ruby-3.4.4:latest` (latest)
- For now, use hardcoded available versions list per language
- TODO: Later fetch available images from Docker registry dynamically

**Verification**:
- Write unit tests for version resolution
- Test exact matches, fallbacks, and defaults

**Commands**:
```bash
cd lsproxy && cargo test resolve_image_name -- --nocapture
```

---

#### Task 6.3: Create HTTP LSP Client Wrapper
**File**: `lsproxy/src/lsp/http_client.rs` (new)

**Actions**:
- Create `HttpLspClient` struct:
  ```rust
  pub struct HttpLspClient {
      endpoint: String,
      http_client: reqwest::Client,
      workspace_documents: Arc<Mutex<WorkspaceDocuments>>,
      language: SupportedLanguages,
  }
  ```
- Implement `LspClient` trait for `HttpLspClient`
- All trait methods send HTTP POST requests to `{endpoint}/lsp` with LSP JSON-RPC
- Parse HTTP responses back to LSP types
- Handle errors (network failures, timeouts, LSP errors)

**Verification**:
- Write unit tests with mock HTTP server
- Test each LspClient trait method
- Verify error handling

**Commands**:
```bash
cd lsproxy && cargo test http_client -- --nocapture
```

---

#### Task 6.4: Update Manager to Use Container Orchestrator
**File**: `lsproxy/src/lsp/manager/manager.rs`

**Actions**:
- Add `ContainerOrchestrator` field to `Manager` struct
- Modify `Manager::new()` to initialize orchestrator
- Update `start_langservers()` to:
  - For each detected language:
    - Call version detector to get language version (placeholder returns None for now)
    - Resolve Docker image name based on detected version
    - Spawn container via orchestrator with resolved image
    - Create `HttpLspClient` with container endpoint
    - Store client in `lsp_clients` HashMap
  - Special Ruby logic:
    - Detect if Sorbet needed (check for `typed:` annotations or sorbet/ directory)
    - Spawn ruby-lsp container (always)
    - Spawn ruby-sorbet container (conditionally)
- TODO: Wire up actual version detection in Task 6.11

**Verification**:
- Write integration test `test_manager_with_containers()`
- Test workspace with single language
- Test workspace with multiple languages
- Verify containers are spawned and clients work
- Test Ruby workspace with and without Sorbet

**Commands**:
```bash
cd lsproxy && cargo test test_manager_with_containers -- --nocapture
```

---

#### Task 6.5: Add Graceful Shutdown
**File**: `lsproxy/src/lsp/manager/manager.rs`

**Actions**:
- Implement `Drop` trait for `Manager`:
  ```rust
  impl Drop for Manager {
      fn drop(&mut self) {
          // Cleanup all containers via orchestrator
      }
  }
  ```
- Ensure orchestrator cleanup is called on Manager drop
- Add signal handlers in main.rs for SIGTERM/SIGINT

**Verification**:
- Test that containers are cleaned up when service stops
- Test with `docker ps` before and after shutdown
- Test cleanup on error/panic scenarios

**Commands**:
```bash
# Start service, verify containers running
docker ps | grep lsproxy-
# Stop service
kill -TERM <pid>
# Verify containers cleaned up
docker ps | grep lsproxy-  # Should be empty
```

---

#### Task 6.6: Implement Version Detection (Ruby)
**File**: `lsproxy/src/container/version_detector.rs`

**Actions**:
- Implement `detect_ruby_version(workspace_path: &str) -> Option<String>`
- Check in order:
  1. `.ruby-version` file (exact version like "3.4.4")
  2. `Gemfile` ruby directive (e.g., `ruby "3.4.4"`)
  3. `.tool-versions` file (asdf format: `ruby 3.4.4`)
- Parse and return version string

**Verification**:
- Write tests with sample workspaces containing each config type
- Test version parsing edge cases

---

#### Task 6.7: Implement Version Detection (Python)
**File**: `lsproxy/src/container/version_detector.rs`

**Actions**:
- Implement `detect_python_version(workspace_path: &str) -> Option<String>`
- Check in order:
  1. `.python-version` file
  2. `pyproject.toml` `requires-python` field (e.g., `>=3.11`)
  3. `Pipfile` `python_version` field
  4. `runtime.txt` (Heroku format: `python-3.11.0`)
- Parse version constraints and extract base version

---

#### Task 6.8: Implement Version Detection (Node.js)
**File**: `lsproxy/src/container/version_detector.rs`

**Actions**:
- Implement `detect_nodejs_version(workspace_path: &str) -> Option<String>`
- Check in order:
  1. `.nvmrc` file
  2. `.node-version` file
  3. `package.json` `engines.node` field (e.g., `>=18.0.0`)
  4. `.tool-versions` file
- Parse version constraints and extract base version

---

#### Task 6.9: Implement Version Detection (Java)
**File**: `lsproxy/src/container/version_detector.rs`

**Actions**:
- Implement `detect_java_version(workspace_path: &str) -> Option<String>`
- Check in order:
  1. `pom.xml` maven.compiler.source/target (e.g., `<source>21</source>`)
  2. `build.gradle` sourceCompatibility/targetCompatibility
  3. `.java-version` file
  4. `.tool-versions` file
- Parse XML/Groovy and extract version

---

#### Task 6.10: Implement Version Detection (Go)
**File**: `lsproxy/src/container/version_detector.rs`

**Actions**:
- Implement `detect_go_version(workspace_path: &str) -> Option<String>`
- Check in order:
  1. `go.mod` file first line (e.g., `go 1.23`)
  2. `.go-version` file
  3. `.tool-versions` file
- Parse and return version string

---

#### Task 6.11: Implement Version Detection (PHP)
**File**: `lsproxy/src/container/version_detector.rs`

**Actions**:
- Implement `detect_php_version(workspace_path: &str) -> Option<String>`
- Check in order:
  1. `composer.json` `require.php` field (e.g., `"php": "^8.2"`)
  2. `.php-version` file
  3. `.tool-versions` file
- Parse version constraints and extract base version

---

#### Task 6.12: Wire Up Version Detection in Manager
**File**: `lsproxy/src/lsp/manager/manager.rs`

**Actions**:
- Update `start_langservers()` to call appropriate version detection functions
- Remove TODO placeholders from Task 6.4
- Use detected versions to resolve image names
- Test end-to-end version detection → image selection → container spawn

**Verification**:
- Test with workspaces containing version files for each language
- Verify correct image tags are used
- Test fallback behavior when version not available
- Integration test with multi-language workspace with mixed version files

**Commands**:
```bash
cd lsproxy && cargo test manager_version_detection -- --nocapture
```

---

### Phase 7: Base Image Creation

#### Task 7.1: Create Minimal Base Dockerfile
**File**: `dockerfiles/base.Dockerfile`

**Actions**:
- Start from Debian slim or Alpine
- Install Docker CLI (to communicate with host Docker daemon)
- Copy lsproxy Rust binary
- Expose port 4444
- Set entrypoint to lsproxy binary

**Size Target**: < 500MB

**Verification**:
- Build base image
- Check size
- Run with Docker socket mounted
- Verify can spawn language containers

**Commands**:
```bash
docker build -f dockerfiles/base.Dockerfile -t lsproxy-base:latest .
docker images lsproxy-base:latest
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock -v $(pwd)/sample_project:/mnt/workspace lsproxy-base:latest
```

---

#### Task 7.2: Update Docker Compose Configuration
**File**: `docker-compose.yml` (new/update)

**Actions**:
- Create docker-compose.yml for easy development:
  ```yaml
  services:
    lsproxy:
      image: lsproxy-base:latest
      volumes:
        - /var/run/docker.sock:/var/run/docker.sock
        - ./sample_project:/mnt/workspace
      ports:
        - "4444:4444"
      environment:
        - RUST_LOG=info
  ```

**Verification**:
- Run `docker-compose up`
- Verify service starts and spawns containers
- Test API endpoints
- Run `docker-compose down`
- Verify cleanup

**Commands**:
```bash
docker-compose up -d
docker-compose ps
docker-compose logs
docker-compose down
```

---

### Phase 8: Integration Testing

#### Task 8.1: End-to-End Test - Golang Workspace
**File**: `lsproxy/tests/e2e_golang.rs` (new)

**Actions**:
- Create test that:
  - Starts lsproxy with Golang workspace
  - Verifies golang container is spawned
  - Makes API calls: list_files, find_definition, find_references
  - Verifies responses match expected values
  - Cleans up containers

**Verification**:
```bash
cd lsproxy && cargo test e2e_golang -- --nocapture
```

---

#### Task 8.2: End-to-End Test - Multi-Language Workspace
**File**: `lsproxy/tests/e2e_multilang.rs` (new)

**Actions**:
- Create test workspace with Python, TypeScript, and Go files
- Verify all three containers are spawned
- Test cross-language operations
- Verify correct language detection per file

**Verification**:
```bash
cd lsproxy && cargo test e2e_multilang -- --nocapture
```

---

#### Task 8.3: End-to-End Test - Ruby with Sorbet
**File**: `lsproxy/tests/e2e_ruby_sorbet.rs` (new)

**Actions**:
- Create Ruby workspace with some files having `typed:` annotation
- Verify both ruby-lsp and ruby-sorbet containers spawn
- Test that Sorbet provides type information
- Test workspace without Sorbet annotations (only ruby-lsp should spawn)

**Verification**:
```bash
cd lsproxy && cargo test e2e_ruby_sorbet -- --nocapture
```

---

#### Task 8.4: Resource Cleanup Test
**File**: `lsproxy/tests/cleanup_test.rs` (new)

**Actions**:
- Start lsproxy multiple times with different workspaces
- Verify containers are cleaned up each time
- Check for container/network leaks
- Test cleanup on abnormal termination (kill -9)

**Verification**:
```bash
cd lsproxy && cargo test cleanup_test -- --nocapture
# Manual verification
docker ps -a | grep lsproxy
docker network ls | grep lsproxy
```

---

#### Task 8.5: Performance Benchmark
**File**: `lsproxy/benches/container_startup.rs` (new)

**Actions**:
- Create benchmark measuring:
  - Container spawn time per language
  - API response time (local vs container)
  - Memory usage comparison
- Document results in PERFORMANCE.md

**Verification**:
```bash
cd lsproxy && cargo bench
```

---

### Phase 9: Build and Release

#### Task 9.1: Create Multi-Arch Build Script
**File**: `scripts/build-images.sh`

**Actions**:
- Create script to build all images for amd64 and arm64:
  ```bash
  #!/bin/bash
  for dockerfile in dockerfiles/*.Dockerfile; do
    name=$(basename $dockerfile .Dockerfile)
    docker buildx build --platform linux/amd64,linux/arm64 \
      -t lsproxy-${name}:latest \
      -f $dockerfile .
  done
  ```

**Verification**:
- Run script on both architectures
- Verify all images build successfully
- Check image sizes meet targets

**Commands**:
```bash
./scripts/build-images.sh
docker images | grep lsproxy-
```

---

#### Task 9.2: Update Documentation
**File**: `README.md`, `ARCHITECTURE.md` (new)

**Actions**:
- Update README with new architecture
- Document image sizes (before: 12.5GB, after: ~500MB base + language images on demand)
- Create ARCHITECTURE.md explaining container orchestration
- Document environment variables
- Add troubleshooting section

**Verification**:
- Review documentation for completeness
- Test all example commands

---

#### Task 9.3: Create Release GitHub Actions
**File**: `.github/workflows/release.yml`

**Actions**:
- Create workflow to:
  - Build all Docker images on release tag
  - Push to container registry
  - Create GitHub release with notes
  - Generate size comparison report

**Verification**:
- Create test release
- Verify all images are published
- Verify release notes are correct

---

### Phase 10: Migration and Backward Compatibility

#### Task 10.1: Add Feature Flag
**File**: `lsproxy/Cargo.toml`, `lsproxy/src/lib.rs`

**Actions**:
- Add feature flag `container-orchestration`:
  ```toml
  [features]
  default = ["local-processes"]
  local-processes = []
  container-orchestration = ["bollard", "hyper"]
  ```
- Make container code conditional on feature
- Allow users to opt-in to new architecture

**Verification**:
- Build with default features (local)
- Build with container-orchestration feature
- Verify both work correctly

**Commands**:
```bash
cargo build
cargo build --features container-orchestration
```

---

#### Task 10.2: Performance Comparison Testing
**File**: `docs/MIGRATION.md`

**Actions**:
- Test both architectures side-by-side
- Document performance differences
- Create migration guide
- Document when to use which mode

**Verification**:
- Run benchmarks on both modes
- Compare results
- Document findings

---

## Success Criteria

1. **Size Reduction**: Base image < 500MB (from 12.5GB)
2. **Functionality**: All 10 languages work via containers
3. **Performance**: API latency increase < 50ms for container mode
4. **Reliability**: 100% container cleanup on normal and abnormal termination
5. **Testing**: All integration tests pass
6. **Documentation**: Complete architecture and migration documentation

## Risk Mitigation

- **Network Latency**: Use Unix sockets if HTTP performance is insufficient
- **Container Startup Time**: Pre-warm containers or implement lazy initialization
- **Docker Dependency**: Maintain backward compatibility with local process mode
- **Volume Permissions**: Ensure proper file permissions in mounted volumes

## Timeline Estimate

- Phase 1-2: 2-3 days (Foundation)
- Phase 3: 3-4 days (Dockerfiles)
- Phase 4: 1-2 days (HTTP Wrapper)
- Phase 5-6: 2-3 days (Integration)
- Phase 7: 1 day (Base Image)
- Phase 8: 2-3 days (Testing)
- Phase 9: 1-2 days (Release)
- Phase 10: 1 day (Migration)

**Total**: ~14-20 days of focused development
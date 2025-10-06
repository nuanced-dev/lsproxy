# Testing Infrastructure

## Overview

LSProxy has multiple layers of testing to validate functionality across all supported languages.

## Test Coverage

### 1. Rust Integration Tests
**Location**: `tests/{python,java}_test.rs`

**What they test**: Full E2E tests that:
- Start the LSProxy server directly in Rust
- Test workspace endpoints (list-files, read-source-code)
- Test symbol endpoints (definitions-in-file)
- Validate response structures and content

**How they run**: Via `scripts/test.sh`
- Builds a Docker image with all language servers
- Runs `cargo test` inside the container
- Mounts `sample_project` at `/mnt/lsproxy_root/sample_project`

**Current Status**: ❌ **BROKEN** - Needs updating for new DinD architecture

**Issue**: Tests call `initialize_app_state()` which tries to spawn Docker containers via `ContainerOrchestrator`, but the test container doesn't have access to `/var/run/docker.sock`.

**Fix Required**: Update `scripts/test.sh` to mount Docker socket:
```bash
docker run --rm \
  -v "$(pwd)/lsproxy":/usr/src/app \
  -v "$(pwd)":/mnt/lsproxy_root \
  -v /var/run/docker.sock:/var/run/docker.sock \  # ADD THIS
  lsproxy-dev cargo test --target-dir /tmp/target $@
```

### 2. Unit Tests
**Location**: `src/lsp/manager/language_tests/*.rs`

**What they test**: Per-language LSP operations at the unit level

**Current Status**: ✅ Should work (they don't require Docker orchestration)

### 3. Shell Integration Tests
**Location**: `tests/integration-test.sh`

**What they test**: HTTP endpoint validation via curl
- Health check with all language availability
- Workspace endpoints (list-files, read-source-code)
- Symbol endpoints for each language (find-definition, find-references, etc.)

**Current Status**: ✅ **READY TO USE**

**How to run**:
```bash
# 1. Start the service with sample_project mounted
docker run --rm --name lsproxy-service \
  --add-host=host.docker.internal:host-gateway \
  -p 4444:4444 \
  -e USE_AUTH=false \
  -e HOST_WORKSPACE_PATH=/path/to/sample_project/python \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /path/to/sample_project/python:/mnt/workspace \
  lsproxy-service:latest

# 2. Run the integration test
./tests/integration-test.sh
```

## Test Data

### Sample Project Workspaces
**Location**: `/Users/rewinfrey/code/nuanced/lsproxy/sample_project/`

**Languages covered**:
- Python: `sample_project/python/`
- Java: `sample_project/java/`
- TypeScript: `sample_project/typescript/`
- JavaScript: `sample_project/js/`
- Rust: `sample_project/rust/`
- Go: `sample_project/go/`
- C++: `sample_project/cpp/`
- C: `sample_project/c/`
- C#: `sample_project/csharp/`
- PHP: `sample_project/php/`
- Ruby: `sample_project/ruby/`
- Bash: `sample_project/bash/`
- Perl: `sample_project/perl/`

**Usage**: These are the official test workspaces used by Rust integration tests. They contain real code with LSP operations that can be tested.

## Architecture Changes Impact

### Old Architecture
- LSP clients spawned as **processes** inside the same container
- Tests could run entirely inside one Docker container
- No Docker-in-Docker required

### New Architecture (Current)
- LSP clients run in **separate Docker containers**
- Base service spawns containers via Docker API
- Tests need Docker socket access (Docker-in-Docker)

## Testing Strategy

### Phase 1: Validate with Shell Script ✅ CURRENT
1. Use `tests/integration-test.sh` to quickly validate endpoints
2. Test against real workspaces (sample_project)
3. Verify all languages work correctly
4. Fast feedback loop without rebuilding Docker images

### Phase 2: Fix Rust Integration Tests
1. Update `scripts/test.sh` to mount Docker socket
2. Ensure LSP container images are available in test environment
3. May need to adjust test expectations for DinD environment
4. Validate backward compatibility

## Running Tests

### Building the Service Image

**Important**: Build the service using `dockerfiles/service.Dockerfile`, NOT `lsproxy/Dockerfile`:

```bash
# From repository root
docker build -f dockerfiles/service.Dockerfile -t lsproxy-service:latest .
```

### Quick Validation (Recommended for development)
```bash
# Terminal 1: Start service
docker run --rm --name lsproxy-service \
  --add-host=host.docker.internal:host-gateway \
  -p 4444:4444 \
  -e USE_AUTH=false \
  -e HOST_WORKSPACE_PATH=$(pwd)/sample_project/python \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v $(pwd)/sample_project/python:/mnt/workspace \
  lsproxy-service:latest

# Terminal 2: Run integration tests
cd lsproxy
./tests/integration-test.sh
```

### Full Test Suite (Once DinD support is added)
```bash
cd /path/to/lsproxy
./scripts/test.sh --test python_test
./scripts/test.sh --test java_test
```

## Test Maintenance

### When Adding New Endpoints
1. Add test case to `tests/integration-test.sh`
2. Add Rust integration test if complex behavior
3. Update this document

### When Adding New Languages
1. Add workspace to `sample_project/{language}/`
2. Add language configuration to `tests/integration-test.sh`
3. Add Rust integration test file if needed
4. Update this document

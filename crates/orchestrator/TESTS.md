# Testing Infrastructure

## Overview

LSProxy has multiple layers of testing to validate functionality across all supported languages.

## Test Coverage

### 1. Shell Integration Tests ✅ **RECOMMENDED**
**Location**: `lsproxy/tests/integration-test.sh`

**What they test**: HTTP endpoint validation for **9 languages**:
- Python, TypeScript, Rust, Golang, Java, C++, C#, PHP, Ruby

**Endpoints tested**:
- Health check with language availability
- Workspace endpoints (list-files, read-source-code)
- Symbol endpoints (find-definition, find-references, find-referenced-symbols, definitions-in-file, find-identifier)

**How to run**:
```bash
# Terminal 1: Start the service (using service.Dockerfile)
docker run --rm --name lsproxy-service \
  --add-host=host.docker.internal:host-gateway \
  -p 4444:4444 \
  -e USE_AUTH=false \
  -e HOST_WORKSPACE_PATH=$(pwd)/sample_project/python \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v $(pwd)/sample_project/python:/mnt/workspace \
  lsproxy-service:latest

# Terminal 2: Run the integration test for a specific language
cd lsproxy
./tests/integration-test.sh python
./tests/integration-test.sh typescript
./tests/integration-test.sh golang
# ... etc for other languages
```

**Current Status**: ✅ **READY TO USE** - Works with new service.Dockerfile architecture

---

### 2. Rust Integration Tests
**Location**: `lsproxy/tests/{python,java}_test.rs`

**What they test**: Full E2E tests that:
- Start the LSProxy server directly in Rust
- Test workspace endpoints (list-files, read-source-code)
- Test symbol endpoints (definitions-in-file)
- Validate response structures and content

**How they run**: Via `scripts/test.sh`
- Builds a Docker image with all language servers (lsproxy/Dockerfile)
- Runs `cargo test` inside the container
- Mounts `sample_project` at `/mnt/lsproxy_root/sample_project`

**Current Status**: ⚠️ **NEEDS VERIFICATION** - May need updates for DinD architecture

**What was fixed**: Updated `scripts/test.sh` to mount Docker socket:
```bash
docker run --rm \
  -v "$(pwd)/lsproxy":/usr/src/app \
  -v "$(pwd)":/mnt/lsproxy_root \
  -v /var/run/docker.sock:/var/run/docker.sock \         # ✅ Added
  --add-host=host.docker.internal:host-gateway \        # ✅ Added
  -e HOST_WORKSPACE_PATH=/mnt/lsproxy_root/sample_project \  # ✅ Added
  lsproxy-dev cargo test --target-dir /tmp/target $@
```

**Potential issues**:
- Tests may need language container images to be pre-built
- Container spawning in test environment may have networking challenges
- Tests spawn server in-process which may conflict with container orchestration

---

### 3. Unit Tests
**Location**: `lsproxy/src/lsp/manager/language_tests/*.rs`

**What they test**: Per-language LSP operations at the unit level

**Current Status**: ✅ Should work (they don't require Docker orchestration)

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

---

## Architecture Changes Impact

### Old Architecture (Process-based)
- LSP clients spawned as **processes** inside the same container
- Tests could run entirely inside one Docker container
- No Docker-in-Docker required
- Single 13.3GB monolithic image

### New Architecture (Container-based) ✅ CURRENT
- Base service (287 MB) built with `dockerfiles/service.Dockerfile`
- LSP clients run in **separate Docker containers** per language
- Base service spawns containers via Docker API
- Tests need Docker socket access (Docker-in-Docker)
- Language images built with `dockerfiles/{language}.Dockerfile`

---

## Testing Strategy

### Recommended Approach ✅

**Use the shell integration test** (`lsproxy/tests/integration-test.sh`):

1. Build service image: `docker build -f dockerfiles/service.Dockerfile -t lsproxy-service:latest .`
2. Build language images you want to test
3. Start the service with appropriate workspace mounted
4. Run `./tests/integration-test.sh <language>` for each language

**Advantages**:
- Fast feedback loop
- Tests real HTTP API endpoints
- Works with new container architecture
- Covers 9 languages comprehensively
- No complex Rust test setup required

### Alternative: Rust Integration Tests (Needs Verification)

Use `scripts/test.sh` for cargo test-based integration tests:
- May need language container images pre-built
- Requires Docker-in-Docker setup
- More complex to debug
- Currently unverified with new architecture

---

## Running Tests

### Building Images

**Service Image**:
```bash
# From repository root
docker build -f dockerfiles/service.Dockerfile -t lsproxy-service:latest .
```

**Language Images** (build as needed):
```bash
docker build -f dockerfiles/python.Dockerfile -t lsproxy-python:latest .
docker build -f dockerfiles/typescript.Dockerfile -t lsproxy-typescript:latest .
docker build -f dockerfiles/golang.Dockerfile -t lsproxy-golang:latest .
# ... etc for other languages
```

### Quick Validation (Recommended)
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

# Terminal 2: Run integration test
cd lsproxy
./tests/integration-test.sh python
```

### Testing All Languages
```bash
# In separate terminals or sequentially:
cd lsproxy
for lang in python typescript golang java rust cpp csharp php ruby; do
  # Start service with appropriate workspace
  docker rm -f lsproxy-service 2>/dev/null
  docker run --rm --name lsproxy-service \
    --add-host=host.docker.internal:host-gateway \
    -p 4444:4444 \
    -e USE_AUTH=false \
    -e HOST_WORKSPACE_PATH=$(pwd)/../sample_project/$lang \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v $(pwd)/../sample_project/$lang:/mnt/workspace \
    lsproxy-service:latest &

  sleep 5  # Wait for service to start
  ./tests/integration-test.sh $lang
  docker rm -f lsproxy-service
done
```

### Rust Integration Tests (Advanced)
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

# LSProxy Testing Guide

This document describes how to test the LSProxy container orchestration system.

## Overview

LSProxy uses a containerized architecture where:
- **Service Container** (`lsproxy-service`) orchestrates language-specific containers
- **Watchdog Container** (`lsproxy-watchdog`) monitors the service and ensures cleanup on crashes
- **Language Containers** (`lsproxy-python`, `lsproxy-golang`, etc.) run LSP servers
- The service spawns language containers dynamically based on workspace content
- The watchdog provides defense-in-depth cleanup for catastrophic failures (SIGKILL, crashes)

## Quick Start

### 1. Build All Containers

```bash
# Build sequentially (safer, easier to debug)
./scripts/build-all-containers.sh

# Or build in parallel (faster)
./scripts/build-all-containers.sh --parallel
```

This builds:
- Service image (lsproxy-service)
- Watchdog image (lsproxy-watchdog) - monitors service and cleans up on crashes
- Base images (lsproxy-base, lsproxy-base-runtime, lsproxy-base-build)
- 10 language images (python, typescript, rust, golang, java, clangd, csharp, php, ruby-3.4.4, ruby-sorbet-3.4.4)

### 2. Start the Service

```bash
# Using the run2 script (recommended)
./scripts/run2 sample_project/all

# Or manually with docker run
docker run -d \
    --name lsproxy-service \
    -p 4444:4444 \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v $(pwd)/sample_project/all:/mnt/workspace \
    -e HOST_WORKSPACE_PATH=$(pwd)/sample_project/all \
    -e USE_AUTH=false \
    lsproxy-service:latest
```

### 3. Run Tests

Wait ~30 seconds for the service to initialize and spawn language containers, then:

```bash
# Test container lifecycle
./scripts/test-container-lifecycle.sh

# Test all endpoints for all languages
./scripts/test-all-endpoints.sh

# Test watchdog cleanup functionality
./scripts/test-watchdog.sh

# Rust integration and unit tests (requires images built)
cd lsproxy && cargo test
```

## Test Suites

### 1. Container Lifecycle Tests

**Script:** `scripts/test-container-lifecycle.sh`

Tests:
- ✓ Service image exists
- ✓ Service container starts
- ✓ Service health check responds
- ✓ Language containers are spawned automatically
- ✓ Docker network is created
- ✓ API endpoints work
- ✓ Containers stop and cleanup properly

**Usage:**
```bash
./scripts/test-container-lifecycle.sh [workspace_path]
```

### 2. Comprehensive Endpoint Tests

**Script:** `scripts/test-all-endpoints.sh`

Tests all 8 endpoints × 10 languages = 80+ tests:

**Endpoints tested:**
1. `GET /v1/system/health` - System health check
2. `GET /v1/workspace/list-files` - List all files
3. `POST /v1/workspace/read-source-code` - Read file content
4. `POST /v1/workspace/read-source-code` (with range) - Read file range
5. `POST /v1/symbol/find-definition` - Find symbol definitions
6. `POST /v1/symbol/find-references` - Find symbol references
7. `POST /v1/symbol/find-referenced-symbols` - Find referenced symbols
8. `GET /v1/symbol/definitions-in-file` - Get all definitions in file
9. `POST /v1/symbol/find-identifier` - Find identifier by name

**Languages tested:**
- Python (jedi-language-server)
- TypeScript/JavaScript (typescript-language-server)
- Rust (rust-analyzer)
- Golang (gopls)
- Java (jdtls)
- C/C++ (clangd)
- C# (csharp-ls)
- PHP (phpactor)
- Ruby (ruby-lsp)
- Ruby Sorbet (sorbet)

**Usage:**
```bash
./scripts/test-all-endpoints.sh [workspace_path]
```

### 3. Watchdog Cleanup Tests

**Script:** `scripts/test-watchdog.sh`

Tests the watchdog container functionality and automatic cleanup:

**Tests:**
- ✓ Watchdog container spawns automatically
- ✓ Language containers are labeled with parent ID
- ✓ Clean shutdown (SIGTERM) removes all containers
- ✓ Watchdog auto-removes after clean shutdown
- ✓ SIGKILL emergency cleanup works
- ✓ Watchdog auto-removes after emergency cleanup
- ✓ Multiple service instances don't conflict
- ✓ Per-instance isolation via container labels

**Usage:**
```bash
./scripts/test-watchdog.sh [workspace_path]
```

**What it tests:**

1. **Watchdog Spawning**: Verifies that when the service starts, it automatically spawns a watchdog container
2. **Clean Shutdown**: Tests that `docker stop` properly cleans up language containers and removes the watchdog
3. **SIGKILL Emergency Cleanup**: Tests that `docker kill` triggers the watchdog to cleanup orphaned containers
4. **Multiple Instances**: Verifies that multiple service instances can run simultaneously without interfering with each other's cleanup


### 4. Rust Integration & Unit Tests

**Location:** `lsproxy/tests/*.rs` and inline in `lsproxy/src/`

Tests the Rust codebase at multiple levels:

**Integration tests** (`lsproxy/tests/`):
- `container_orchestration_test.rs` - Full Docker container lifecycle testing
  - Service health checks
  - Dynamic container spawning
  - Request forwarding to language containers
  - Container reuse across requests
  - LSP functionality (find references, etc.)
- `python_test.rs` - Python LSP end-to-end integration
- `java_test.rs` - Java LSP end-to-end integration

**Unit tests** (inline in source files):
- Application initialization tests
- OpenAPI specification tests
- Server startup tests
- Docker connection tests

**Usage:**
```bash
cd lsproxy

# Run all tests (unit + integration)
cargo test

# Run specific test
cargo test test_container_spawn_on_request

# Run tests with output
cargo test -- --nocapture

# List all tests
cargo test -- --list
```

**Note:** Rust integration tests require Docker images to be built first.

## Supported Languages

| Language | LSP Server | Container Image | Status |
|----------|-----------|----------------|--------|
| Python | jedi-language-server | lsproxy-python | ✅ |
| TypeScript/JavaScript | typescript-language-server | lsproxy-typescript | ✅ |
| Rust | rust-analyzer | lsproxy-rust | ✅ |
| Golang | gopls | lsproxy-golang | ✅ |
| Java | jdtls | lsproxy-java | ✅ |
| C/C++ | clangd | lsproxy-clangd | ✅ |
| C# | csharp-ls | lsproxy-csharp | ✅ |
| PHP | phpactor | lsproxy-php | ✅ |
| Ruby | ruby-lsp | lsproxy-ruby-3.4.4 | ✅ |
| Ruby Sorbet | sorbet | lsproxy-ruby-sorbet-3.4.4 | ✅ |

## Test Workspaces

### Multi-Language Workspace
**Path:** `sample_project/all`

Contains sample code for all 10 supported languages. Use this for comprehensive testing.

### Language-Specific Workspaces
Each language has its own sample project:
- `sample_project/python/`
- `sample_project/typescript/`
- `sample_project/rust/`
- `sample_project/go/`
- `sample_project/java/`
- `sample_project/cpp/`
- `sample_project/csharp/`
- `sample_project/php/`
- `sample_project/ruby/`

## Test Summary

### Shell Test Scripts (Integration/System)
1. **`test-container-lifecycle.sh`** - 8 tests covering service/container lifecycle
2. **`test-watchdog.sh`** - 18 tests covering watchdog functionality and cleanup
3. **`test-all-endpoints.sh`** - 80+ tests (9 endpoints × 10 languages)

### Rust Tests
- **12 tests** covering unit and integration testing
- Container orchestration, LSP functionality, API handlers
- Requires Docker images built first

### Total Test Coverage
- **100+ automated tests** across all layers
- Tests run in ~2-5 minutes (depending on system)

## Debugging

### View Service Logs
```bash
docker logs lsproxy-service
```

### View Language Container Logs
```bash
# List all containers (including watchdog)
docker ps --filter "name=lsproxy-"

# View specific container logs
docker logs lsproxy-python
docker logs lsproxy-golang

# View watchdog logs (get container ID first)
WATCHDOG=$(docker ps --filter "name=lsproxy-watchdog" --format "{{.Names}}")
docker logs $WATCHDOG
```

### Check Container Network
```bash
# Containers use the default bridge network
docker network inspect bridge

# Check which containers are on the network
docker network inspect bridge --format '{{range .Containers}}{{.Name}} {{end}}'
```

### Interactive Service Container
```bash
docker exec -it lsproxy-service /bin/bash
```

### Manual API Testing
```bash
# Health check
curl http://localhost:4444/v1/system/health | jq

# List files
curl http://localhost:4444/v1/workspace/list-files | jq

# Read source code
curl -X POST http://localhost:4444/v1/workspace/read-source-code \
    -H 'Content-Type: application/json' \
    -d '{"path":"main.py"}' | jq

# Find definition
curl -X POST http://localhost:4444/v1/symbol/find-definition \
    -H 'Content-Type: application/json' \
    -d '{
        "position": {
            "path": "main.py",
            "position": {"line": 15, "character": 4}
        },
        "include_source_code": false
    }' | jq
```

## Container Architecture

```
┌─────────────────────────────────────────────────┐
│         Host Machine                            │
│                                                 │
│  ┌──────────────────────────────────────────┐  │
│  │   lsproxy-service                        │  │
│  │   (Rust service + orchestrator)          │  │
│  │                                          │  │
│  │   Spawns:                                │  │
│  │   ┌────────────────────────────────────┐│  │
│  │   │ lsproxy-watchdog (monitoring)      ││  │
│  │   └────────────────────────────────────┘│  │
│  │   ┌────────────────────────────────────┐│  │
│  │   │ lsproxy-python                     ││  │
│  │   │ lsproxy-golang                     ││  │
│  │   │ lsproxy-rust                       ││  │
│  │   │ lsproxy-typescript                 ││  │
│  │   │ ...                                ││  │
│  │   │ (labeled with parent service ID)   ││  │
│  │   └────────────────────────────────────┘│  │
│  │                                          │  │
│  │   Connected via Docker bridge network   │  │
│  └──────────────────────────────────────────┘  │
│                                                 │
│  Workspace mounted at:                          │
│  /mnt/workspace (service)                       │
│  /workspace (language containers)               │
│                                                 │
│  Docker socket mounted for container spawning   │
└─────────────────────────────────────────────────┘
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Test LSProxy

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v2

    - name: Build containers
      run: ./scripts/build-all-containers.sh

    - name: Test container lifecycle
      run: ./scripts/test-container-lifecycle.sh

    - name: Test watchdog cleanup
      run: ./scripts/test-watchdog.sh

    - name: Test all endpoints
      run: ./scripts/test-all-endpoints.sh

    - name: Run Rust tests
      run: cd lsproxy && cargo test

    - name: Cleanup
      if: always()
      run: |
        docker rm -f $(docker ps -aq --filter "name=lsproxy-") || true
        docker network rm lsproxy-network || true
```

## Performance Benchmarks

To benchmark container startup time and API latency:

```bash
# Measure service startup
time ./scripts/run2 sample_project/all

# Measure endpoint latency
time curl http://localhost:4444/v1/workspace/list-files

# Measure container spawn time
docker logs lsproxy-service | grep "Container spawned"
```

Expected performance:
- Service startup: ~5-10 seconds
- Language container spawn: ~2-5 seconds each
- API endpoint latency: ~10-100ms

## Troubleshooting

### Containers won't start
1. Check Docker is running: `docker ps`
2. Check for port conflicts: `lsof -i :4444`
3. Check disk space: `df -h`
4. View logs: `docker logs lsproxy-service`

### Tests failing
1. Ensure service is fully initialized (wait 30s after start)
2. Check container status: `docker ps --filter "name=lsproxy-"`
3. Verify workspace mount: `docker exec lsproxy-service ls -la /mnt/workspace`
4. Check network: `docker network inspect bridge`
5. Verify watchdog is running: `docker ps --filter "name=lsproxy-watchdog"`

### Language container not spawning
1. Check language detection: `docker logs lsproxy-service | grep "Detected languages"`
2. Verify language image exists: `docker images | grep lsproxy-<language>`
3. Check workspace contains files for that language

## Contributing

When adding a new language:

1. Create Dockerfile in `dockerfiles/<language>.Dockerfile`
2. Add language config to `scripts/test-all-endpoints.sh`
3. Add sample project in `sample_project/all/`
4. Update this document
5. Run full test suite

## Resources

- [API Documentation](https://docs.lsproxy.dev/api-reference)
- [GitHub Issues](https://github.com/agentic-labs/lsproxy/issues)
- [Discord Community](https://discord.gg/EUFGjSawyk)

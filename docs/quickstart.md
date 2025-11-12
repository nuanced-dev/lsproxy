# LSProxy Quick Start Guide

Get LSProxy running in 3 minutes with container orchestration.

## Prerequisites

- Docker installed and running
- Git (to clone the repo)
- jq (for testing, optional)

## Step 1: Build Service Container (One-time setup)

```bash
# Build the Rust-based service container (~2-5 minutes first time)
# This builds: orchestrator, lsp-wrapper, and watchdog
./scripts/build-rust-containers.sh
```

**Note**: Language containers are pulled from GitHub Container Registry (ghcr.io) automatically when needed. You don't need to build them manually unless you're developing language-specific changes.

## Step 2: Start the Service

```bash
# Start with multi-language test workspace
./scripts/start-service.sh sample_project/all

# Or use your own workspace
./scripts/start-service.sh path/to/your/workspace

# With logs
./scripts/start-service.sh sample_project/all --logs

# Custom port
./scripts/start-service.sh sample_project/all --port 5000
```

**What happens:**
1. Service container starts
2. Detects languages in workspace
3. Spawns language-specific containers automatically (pulls images from ghcr.io if needed)
4. Takes ~20-30 seconds to fully initialize

## Step 3: Test It Works

```bash
# Quick health check
curl http://localhost:4444/v1/system/health | jq

# List files in workspace
curl http://localhost:4444/v1/workspace/list-files | jq

# Find definition in Python file
curl -X POST http://localhost:4444/v1/python/find-definition \
  -H 'Content-Type: application/json' \
  -d '{
    "path": "main.py",
    "position": {"line": 15, "character": 4}
  }' | jq
```

## Step 4: Run Tests

```bash
# Run all tests (unit, integration, container lifecycle, watchdog, and endpoint tests)
./scripts/test.sh

# Or run specific test suites:
./scripts/test-container-lifecycle.sh  # Container orchestration tests
./scripts/test-watchdog.sh             # Watchdog cleanup tests
./scripts/test-all-endpoints.sh        # API endpoint validation
```

## Common Commands

### View Logs
```bash
# Service logs
docker logs -f lsproxy-service

# Specific language container
docker logs lsproxy-python
```

### Check Running Containers
```bash
# All LSProxy containers
docker ps --filter "name=lsproxy-"

# Just the service
docker ps --filter "name=lsproxy-service"
```

### Stop Service
```bash
# Graceful stop (recommended)
./scripts/stop-service.sh

# Force stop without confirmation
./scripts/stop-service.sh --force

# Emergency cleanup (if something is stuck)
./scripts/cleanup-all.sh
```

**Note:** Test scripts automatically clean up when they finish!

### Restart Service
```bash
# Stop and start again
./scripts/stop-service.sh
./scripts/start-service.sh sample_project/all
```

### Verify Clean State
```bash
# Check for running containers
docker ps --filter "name=lsproxy-"

# Should be empty - if not, run cleanup
./scripts/stop-service.sh --force
```

## Troubleshooting

### Service won't start
```bash
# Check if port is already in use
lsof -i :4444

# Check Docker is running
docker ps

# View startup logs
docker logs lsproxy-service
```

### Language containers not spawning
```bash
# Check workspace has files for that language
ls sample_project/all/*.py  # Python
ls sample_project/all/*.go  # Golang

# Check logs for detection
docker logs lsproxy-service | grep "Detected languages"

# Verify language image exists
docker images | grep lsproxy-python
```

### Tests failing
```bash
# Make sure service is fully initialized (wait 30s after start)
sleep 30

# Check container status
docker ps --filter "name=lsproxy-"

# Run individual language test
./lsproxy/tests/integration-test.sh python
```

### Permission errors
```bash
# Make sure Docker socket is accessible
ls -la /var/run/docker.sock

# On macOS/Linux, you may need to add your user to docker group
sudo usermod -aG docker $USER
```

## Next Steps

- **Explore the API**: Visit http://localhost:4444/swagger-ui/
- **Run Full Tests**: `./scripts/test-all-endpoints.sh`
- **Check Test Results**: See `ENDPOINT_VALIDATION_RESULTS.md`
- **Read Full Docs**: See `TESTING.md` and `README.md`

## Architecture at a Glance

```
lsproxy-wrapper (minimal - provides binaries/configs via volume)
    ↓ (--volumes-from)
lsproxy-service (187MB)
    ├─ Spawns → lsproxy-python (791MB)
    ├─ Spawns → lsproxy-golang (1GB)
    ├─ Spawns → lsproxy-rust (1.74GB)
    ├─ Spawns → lsproxy-typescript (1GB)
    └─ Spawns → ... (7 more languages)
```

- **Binary Injection**: lsproxy-wrapper provides lsp-wrapper binary + ast-grep via volume mount
- **Workspace** mounted at `/mnt/workspace` in all containers
- **Communication** service ↔ language containers via HTTP on container-specific ports

## Quick Reference Card

| Task | Command |
|------|---------|
| Build service | `./scripts/build-rust-containers.sh` |
| Start service | `./scripts/start-service.sh sample_project/all` |
| Stop service | `./scripts/stop-service.sh` |
| View logs | `docker logs -f lsproxy-service` |
| Health check | `curl localhost:4444/v1/system/health \| jq` |
| Run all tests | `./scripts/test.sh` |
| Run endpoint tests | `./scripts/test-all-endpoints.sh` |
| List containers | `docker ps --filter "name=lsproxy-"` |
| Cleanup all | `./scripts/cleanup-all.sh` |

## Getting Help

- **Documentation**: See `docs/architecture.md`, `TESTING.md`, `README.md`
- **Issues**: Check `ENDPOINT_VALIDATION_RESULTS.md` for known issues
- **GitHub**: https://github.com/nuanced-dev/lsproxy/issues

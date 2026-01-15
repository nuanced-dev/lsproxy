# Development

Nuanced LSP is a containerized code navigation service based on LSP servers.

## Architecture

Nuanced LSP consists of the following service and language components.

- **Proxy:** Manages the other service containers and the language containers, serves the API, and forwards API requests to the right language containers.

- **Watchdog:** Cleans up other containers if the proxy unexpectedly fails.

- **Wrapper:** Contains the binary that manages LSP server processes and serves the internal API that is called by the proxy. The wrapper container does not run LSP servers directly. Instead the binary is injected into the language containers, which run it. This allows for updating the wrapper logic without having to rebuild every language image.

- **Languages:** Contains the LSP server for a specific language. It does not contain the wrapper binary it runs to serve the internal API, but relies on the wrapper being injected at run time.

See [architecture](docs/architecture.md) documentation for more details.

## Repo layout

| Path              | Description                                                       |
|-------------------|-------------------------------------------------------------------|
| `crates/common`   | Shared logic between the proxy and wrapper                        |
| `crates/proxy`    | Proxy service code                                                |
| `crates/wrapper`  | Wrapper service code                                              |
| `dockerfiles`     | Docker build files for service and language images                |
| `sample_project`  | Sample projects for different languages for testing and debugging |
| `scripts`         | Development scripts                                               |

## Requirements

- Rust and Cargo installed
- Docker installed and running

Additionally for building mulit-platform images:

- Docker buildx
- QEMU

## Local development

### Rust code

**Building:**

Build the code:

```bash
cargo build
```

Format the code:

```bash
cargo fmt
```

**Testing:**

Test the code:

```bash
cargo test
```

_Note that this excludes tests that require Docker images to be available. This is to ensure basic tests can always run even if the iamges haven't been build yet. See the section on service image development for a more comprehensive test method._

### Service images

**Building:**

Build the service images:

```bash
scripts/build-images.sh --all-services
```

The build script accepts flags to customize the build:

```bash
scripts/build-images.sh --help
```

Common custom build workflows:

- **Only build specific services:**

  ```bash
  scripts/build-images.sh --services=wrapper
  ```

- **Build with a custom image tag:** use a custom tag to create a development version

  ```bash
  scripts/build-images.sh --all-services --service-tag=dev
  ```

- **Use specific language image versions:** to depend on development versions of languages

  ```bash
  scripts/build-images.sh --all-services --language-tag=dev
  ```

**Running:**

Start the service for a workspace:

```bash
scripts/start-proxy.sh sample_project/all
```

Stop the service:

```bash
scripts/stop-proxy.sh
```

_These two scripts start and stop the service without relying on the TypeScript client, which is useful for developing and debugging the Rust code independently._

**Testing:**

Test the images:

```bash
scripts/test.sh
```

This will test the Rust code including the tests that depend on availabel images, as well as container lifecycle, watchdog behavior, and API endpoints.

It is also possible to run these tests individually:

```bash
scripts/test-container-lifecycle.sh
scripts/test-watchdog.sh
scripts/test-all-endpoints.sh
```

Flags control the tests and can be used for example to test against a custom tag:

```bash
scripts/test.sh --help
```

**Publishing:**

_Publishing images is done through release automation. For manual publishing during development:_

Use `--publish` to build multi-platform images and push to the registry:

```bash
scripts/build-images.sh --all-services --publish
```

To publish to a custom registry:

```bash
scripts/build-images.sh --all-services --publish --registry=nuanced
```

### Language images

**Building:**

Build the language images:

```bash
scripts/build-images.sh --all-languages
```

_Note that building all language images will take a long time! Typically you'd only want to rebuild the image for one or a few languages._

The build script accepts flags to customize the build:

```bash
scripts/build-images.sh --help
```

Common custom build workflows:

- **Only build specific languages:**

  ```bash
  scripts/build-images.sh --languages=python,ruby-3.2.2
  ```

- **Build with a custom language image tag:** use a custom tag to create a development version

  ```bash
  scripts/build-images.sh --all-languages --language-tag=dev
  ```

**Publishing:**

_Publishing images is done through release automation. For manual publishing during development:_

Use `--publish` to build multi-platform images and push to the registry:

```bash
scripts/build-images.sh --all-languages --language-tag=1.0.0 --publish
```

To publish to a custom registry:

```bash
scripts/build-images.sh --all-languages --language-tag=1.0.0 --publish --registry=nuanced
```

**Adding a language:**

Follow these steps to add a new language:

1. Create Dockerfile in `dockerfiles/<language>.Dockerfile`
1. Add language to the various lists of supported languages
1. Add language config to `scripts/test-all-endpoints.sh`
1. Add sample project in `sample_project/all/`
1. Run full test suite

## Debugging

**View service logs:**

```bash
docker logs nuanced-lsp-proxy
```

**View language container logs:**

```bash
# List all containers (including watchdog)
docker ps --filter "name=nuanced-lsp-"

# View specific container logs
docker logs nuanced-lsp-python
docker logs nuanced-lsp-golang

# View watchdog logs (get container ID first)
WATCHDOG=$(docker ps --filter "name=nuanced-lsp-watchdog" --format "{{.Names}}")
docker logs $WATCHDOG
```

**Check container network:**

```bash
# Containers use the default bridge network
docker network inspect bridge

# Check which containers are on the network
docker network inspect bridge --format '{{range .Containers}}{{.Name}} {{end}}'
```

**Interactive service container:**

```bash
docker exec -it nuanced-lsp-proxy /bin/bash
```

**Manual API testing:**

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

## Troubleshooting

**Containers won't start:**

- Check Docker is running: `docker ps`
- Check for port conflicts: `lsof -i :4444`
- Check disk space: `df -h`
- View logs: `docker logs nuanced-lsp-proxy`

**Tests failing:**

- Ensure service is fully initialized (wait 30s after start)
- Check container status: `docker ps --filter "name=nuanced-lsp-"`
- Verify workspace mount: `docker exec nuanced-lsp-proxy ls -la /mnt/workspace`
- Check network: `docker network inspect bridge`
- Verify watchdog is running: `docker ps --filter "name=nuanced-lsp-watchdog"`

**Language container not spawning:**

- Check language detection: `docker logs nuanced-lsp-proxy | grep "Detected languages"`
- Verify language image exists: `docker images | grep nuanced-lsp-<language>`
- Check workspace contains files for that language

## Performance

To benchmark container startup time and API latency:

```bash
# Measure service startup
time scripts/start-proxy.sh sample_project/all

# Measure endpoint latency
time curl http://localhost:4444/v1/workspace/list-files

# Measure container spawn time
docker logs nuanced-lsp-proxy | grep "Container spawned"
```

Expected performance:
- Service startup: ~5-10 seconds
- Language container spawn: ~2-5 seconds each
- API endpoint latency: ~10-100ms

## Versioning

Service and language images use semantic versioning, where versions have the form `MAJOR.MINOR.PATCH`.

Service image versioning:

- Major version increases for breaking changes.

  For example, environment variables that are passed to teh service images have been renamed, or an API endpoint's result format changed.

- Minor version increases for added functionality that is backward compatible.

  For example, a new API endpoint was added.

- Patch version increases for small changes and bug fixes.

For `0.x.y` versions, the minor version is treated like the major version.

Language image versioning:

- Major version increases when the interface with the service images changes.

  For example, the wrapper flags have been renamed.

- Minor version increases when the language server got a significant update.

  For example, the language server was updated to a new major version, or to a minor version which added noticable new functionality.

- Patch version increases with bug fixes or insignificant language server updates.

  For example, the language server was updated to a new patch version that fixed some bugs.

_Language images are also published under their major version, and by default the service depends on the major version only. That way, new language container updates are picked up automatically._

## Release

**Service images:**

Follow these steps to release a new version of the service images:

1. Open a branch for the new release.

1. Update the crate version in `Cargo.toml` to the desired new version.

1. Update the [changelog](CHANGELOG.services.md) to include an entry for the new version.

1. Run the release script:

   ```bash
   scripts/release-service.sh --all-services
   ```

   The release script pushes a tag to GitHub that will trigger the release workflow. The release workflow builds the multi-platform images, publishes them to GHCR, and creates a GitHub release for the new version.

1. If releases are successful, merge the release branch.

**Language images:**

Follow these steps to release a new version of a language image:

1. Open a branch for the new release.

1. Determine the MAJOR.MINOR.PATCH language version you want to release.

1. Update the [changelog](CHANGELOG.languages.md) to include entries for each language being released.

1. Run the release script:

   ```bash
   scripts/release-languages.sh --languages=LANGUAGE MAJOR.MINOR.PATCH
   ```

   The release script pushes a tag to GitHub that will trigger the release workflow. The release workflow builds the multi-platform images, publishes them to GHCR under their version and major version, and creates a GitHub release for the new version.

   The release script supports multiple languages, and even `--all-languages`, but releasing all languages is probably only necessary if the interface between the wrapper and the language containers changes (and thus the major version).

1. If releases are successful, merge the release branch.

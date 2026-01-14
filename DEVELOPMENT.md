# Development

## Repo layout

| Path              | Description                                  |
|-------------------|----------------------------------------------|
| `crates/common`   |
| `crates/proxy`    |
| `crates/wrapper`  |
| `dockerfiles`     |
| `scripts`         |

## Requirements

- Rust and Cargo installed
- Docker installed and running

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

- **Build multi-platform images:**

  ```bash
  scripts/build-images.sh --all-services --multi-platform
  ```

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

_Note that publishing images through the release automation is preferred, especially for service images. Manual publishing can be useful for debugging. Be careful to only publish when you've built multi-platform images!_

Publish services images:

```bash
scripts/publish-images.sh --all-services
```

Flags can be used to control the container registry or image version to publish:

```bash
scripts/publish-images.sh --help
```

Common custom publish workflows:

- **Change registry:** publish to Dockerhub registry

  ```bash
  scripts/publish-images.sh --all-services --registry=nuanced
  ```

### Language images

**Building:**

Build the langauge images:

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

- **Build multi-platform images:**

  ```bash
  scripts/build-images.sh --all-languages --multi-platform
  ```

**Publishing:**

_Note that publishing images through the release automation is preferred. Manual publishing can be useful for debugging. Be careful to only publish when you've built multi-platform images!_

Publish services images:

```bash
scripts/publish-images.sh --all-languages
```

Flags can be used to control the container registry or image version to publish:

```bash
scripts/publish-images.sh --help
```

Common custom publish workflows:

- **Change registry:** publish to Dockerhub registry

  ```bash
  scripts/publish-images.sh --all-languages --registry=nuanced
  ```

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

## Release

**Service images:**

Follow these steps to release a new version of the service images:

1. Open a branch for the new release.

1. Update the crate version in `Cargo.toml` to the desired new version.

1. Update the [changelog](CHANGELOG.md) to include an entry for the new version.

1. Run the release script:

   ```bash
   scripts/release.sh --all-services
   ```

   The release script pushes a tag to GitHub that will trigger the release workflow. The release workflow builds the mutli-platform images, publishes them to GHCR, and creates a GitHub release for the new version.

1. If releases are successful, merge the release branch.

**Language images:**

TBD

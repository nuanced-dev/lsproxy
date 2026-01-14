# Developing Nuanced LSP TypeScript client

## Repo layout

| Path              | Description                                  |
|-------------------|----------------------------------------------|
| `src/`            | TypeScript client (CLI + library)            |
| `scripts/`        | Project-level scripts                        |
| `tests/`          | Unified test suite                           |

## Local development

**Building:**

Build the project:

```bash
npm run build
```

Run the CLI from source:

```bash
npm run dev
```

**Testing:**

Running tests with default settings:

```bash
npm run test
```

Running tests with custom settings:

```bash
./scripts/test.sh --help
```

_Note that the full test suite takes a long time to run and requires pulling several languages images. Run time or disk usage can be limited by running the tests only for specific workspaces. See instructions below._

Common `scripts/test.sh` usage:

- **Fail fast:** stop on first failure

  ```bash
  scripts/test.sh --fail-fast
  ```

- **Concurrency:** run with multiple Vitest workers

  ```bash
  scripts/test.sh --workers 8
  scripts/test.sh --workers auto
  ```

  By default tests execute sequentially (`--workers auto`) to avoid Docker port
  collisions. Provide an explicit worker count to run tests in parallel.

- **Run specific tests:** pass Vitest filters/paths

  ```bash
  # a single spec file
  scripts/test.sh -- tests/specs/workspace.spec.ts

  # pattern match by test name
  scripts/test.sh -- --testNamePattern "find definitions"
  ```

- **Limit workspaces included in test:** Provide a comma-separated list to expand coverage.

  ```bash
  scripts/test.sh --workspaces=php
  scripts/test.sh --workspaces=php,ts
  ```

  This can be useful to reduce test time when still working on changes or when debugging issues with a specific language.

**Test fixtures:**

All API tests are based on fixtures. These can be re-recorded easily using the `scripts/test.sh --record-fixtures` flag.

## Updating image versions

The TypeScript client depends on **published** versions of the service and language images. The versions used by the client are defined in `src/defaults.ts`.

It is intentional that the client does not automatically depend the image versions in the repository. This allows developing the service before moving the client to this new version.

_Note that if images and the client are updated in the same pull request, the images need to be released before the client tests can succeed in CI._

## Release

Follow these steps to release a new version:

1. Open a branch for the new release.

1. Update the package version to the desired new version.

1. Update the [changelog](CHANGELOG.md) to include an entry for the new version.

1. Run the release script:

   ```bash
   scripts/release.sh
   ```

   The release script pushes a tag to GitHub that will trigger the release workflow. The release workflow publishes the NPM package and creates a GitHub release for the new version.

1. If releases are successful, merge the release branch.

# Development

## Repo layout

| Path              | Description                                  |
|-------------------|----------------------------------------------|
| `src/`            | TypeScript client (CLI + library)            |
| `scripts/`        | Project-level scripts                        |
| `tests/`          | Unified test suite                           |

## Requirements

- Recent Node.js version installed
- Docker installed and running

## Local development

**Building:**

Build the source:

```bash
npm run build
```

Lint and format the source:

```bash
npm run lint:fix
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
scripts/test.sh --help
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

## Versioning

The TypeScript client uses semantic versioning, where versions have the form `MAJOR.MINOR.PATCH`.

**Client versions:**

- Major version increases for breaking changes.

  For example, flags have been removed, or renamed.

- Minor version increases for added functionality that is backward compatible.

  For example, a new command has been added to the CLI.

- Patch version increases for small changes and bug fixes.

For `0.x.y` versions, the minor version is treated like the major version.

**Image versions:**

The TypeScript client depend on the image versions defined in `src/defaults.ts`.

A local test run will use locally built images if available. The testing workflow will have service images built from the repo available.

The release workflow will **not** build any images and verify that all images can be pulled from the registry before publishign the release.

## Release

Follow these steps to release a new version:

1. Open a branch for the new release.

1. Update the package version in `package.json` to the desired new version.

1. Update the [changelog](CHANGELOG.md) to include an entry for the new version.

1. Run the release script:

   ```bash
   scripts/release.sh
   ```

   The release script pushes a tag to GitHub that will trigger the release workflow. The release workflow publishes the NPM package and creates a GitHub release for the new version.

1. If releases are successful, merge the release branch.

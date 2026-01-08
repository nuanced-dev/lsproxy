# Developing Nuanced LSP

Key points of difference between Nuanced LSP and the upstream LSProxy:

* Uses `sorbet` and `ruby-lsp` LSP servers in place of `solargraph` for Ruby language support.
* Nuanced's [fork of `lsproxy`](https://github.com/nuanced-dev/lsproxy) was originally based on [Tusk's fork](https://github.com/sohankshirsagar/lsproxy).

---

## Repo layout

| Path              | Description                                  |
|-------------------|----------------------------------------------|
| `bin/nuanced-lsp` | Built CLI binary (output of `npm run build`) |
| `src/`            | TypeScript client (CLI + library)            |
| `scripts/`        | Project-level scripts                        |
| `tests/`          | Unified test suite                           |

---

## Nuanced LSProxy image

We publish multi-platform `nuanced-lsproxy` images to ghcr.io. These images are public and do not require authorization.

[Available images](https://github.com/orgs/nuanced-dev/packages/container/package/nuanced-lsproxy).

For releasing new images, please see [nuanced-dev/lsproxy](https://github.com/nuanced-dev/lsproxy)'s [`release/build-multiarch.sh`](https://github.com/nuanced-dev/lsproxy/blob/main/release/build-multiarch.sh) script.

## Local development

### Building

Build the project with
```bash
npm run build
```

### Testing

The tests can be run in two ways
- Run with default settings
  ```bash
  npm run test
  ```
- Run with possible custom falgs using `scripts/test.sh`
  ```bash
  scripts/test.sh [...flags]
  ```

#### Common `scripts/test.sh` usage

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
  scripts/test.sh tests/specs/workspace.spec.ts

  # pattern match by test name
  scripts/test.sh --testNamePattern "find definitions"
  ```

- **Limit languages under test:** Provide a comma-separated list to expand coverage.

  ```bash
  scripts/test.sh --languages php
  scripts/test.sh --languages php,ts
  ```

For additional options (fixtures, image overrides, timeouts, etc.), see [`scripts/test.sh`](scripts/test.sh).

## Fixtures

All API tests are based on fixtures. These can be re-recorded easily using the `scripts/test.sh --record-fixtures` flag.

---

## Releases

The release process is automated and initiated via:

```bash
scripts/release.sh
```

- You may pass a single version (`X.Y.Z`). If passed, this overwrites the `lsp-version` in `config/version.json`. The `lsp-version` value in `config/version.json` is the package version set in `package.json`.
- You may also pass a second version (`X.Y.Z`). If passed, this overwrites the `lsproxy-version` in `config/version.json`. This version is used to determined the default `nuanced-lsproxy` image tag version.

After tagging, both the **branch and tag** are pushed to GitHub. The workflow [`.github/release.yml`](.github/release.yml) performs a checkout of the project, builds the TypeScript package, and invokes `scripts/ci-release.sh` to complete the release automation and publish the bundle to npmjs.

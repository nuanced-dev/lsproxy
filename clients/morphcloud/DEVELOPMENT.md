# Development

## Repo layout

| Path              | Description                                  |
|-------------------|----------------------------------------------|
| `src/`            | TypeScript client (CLI + library)            |
| `scripts/`        | Project-level scripts                        |
| `tests/`          | Unified test suite                           |

## Requirements

- Recent Node.js version installed
- A [Morph Cloud](https://cloud.morph.so) account, and an API token in the `MORPH_API_KEY` environment variable.

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

## Mutagen

This project uses Mutagen for file syncing. It uses an isolated configuration so it does not interfere with other uses of Mutagen on the system.

The CLI has a hidden `mutagen` command that allows you to run Mutagen with the right configuration. For example:

```bash
npm run dev mutagen sync list
```

We use a [modified](https://github.com/nuanced-dev/mutagen/) Mutagen version which is published as [@nuanced-dev/mutagen](https://www.npmjs.com/package/@nuanced-dev/mutagen).

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

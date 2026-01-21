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

To run `mutagen` with the right configuration to see the file syncs created by this project, run:

```bash
scripts/mutagen.sh
```

We use a [modified](https://github.com/nuanced-dev/mutagen/tree/hendrikvanantwerpen/custom-ssh-config) Mutagen version. The binaries are shipped as part of the NPM package.

If these need changing:

- Run `go run scripts/build.go --mode=release-slim` in a Mutagen checkout
- Run `scripts/updarte-mutagen-assets.sh /path/to/mutagen/checkout`

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

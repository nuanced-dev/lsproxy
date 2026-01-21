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

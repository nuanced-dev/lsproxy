# Coding Agent Instructions for nuanced-dev/lsp

This repo contains a containerized LSP implementation.

Main code directories:

- `clients`: clients for different languages
- `crates`: the main Rust code for the proxy (front-end) and the wrapper (per-language back-end)
- `dockerfiles`: the Dockerfiles for the Rust containers and language-specific containers

## Rust crates

The code is managed with Cargo. A typical workflow:

- Verify code with `cargo build`
- Once done, format code with `cargo fmt`

Do not run tests unless explicitly asked!

## TypeScript client

The code is managed with NPM. A typical workflow:

- Verify code with `npm run build`
- Once done, format and lint with `npm run lint:fix`

Do not run tests unless explicitly asked!

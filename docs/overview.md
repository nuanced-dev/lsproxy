# Nuanced LSP

Nuanced LSP exposes extended LSP functionality to provide precise and detailed code context for a variety of agentic tasks.

Nuanced LSP builds on the open-source [LSProxy](https://github.com/agentic-labs/lsproxy) project, and provides a convenient and easy to use TypeScript client library and CLI. Additional language support, process and performance improvements, along with additional capabilities are on Nuanced LSP's roadmap.

## Prerequisites

Before installing Nuanced LSP, you'll need:

- Node.js and npm installed on your system
- Docker installed and running on your system

## Installation

Nuanced LSP is offered as a TypeScript client library or CLI. To install:

```bash
npm install -g @nuanced-dev/lsp@latest
```

## Language Support

Nuanced LSP supports the following languages and LSP servers:

| Language    | LSP Server                   |
|-------------|------------------------------|
| C / C++     | `clangd`                     |
| C#          | `omnisharp`                  |
| Go          | `gopls`                      |
| Java        | `jdtls`                      |
| JavaScript  | `typescript-language-server` |
| Python      | `jedi-language-server`       |
| PHP         | `phpactor`                   |
| Ruby        | `ruby-lsp` and `sorbet`      |
| Rust        | `rust-analyzer`              |
| TypeScript  | `typescript-language-server` |

## API Reference

Nuanced LSP's API is grouped into the following categories.

- **[Container Lifecycle API](./api-reference/container-lifecycle.md)**: Docker container management and image operations.
- **[Symbols API](./api-reference/symbols.md)**: Retrieve definitions, identifiers, references, and related graph data.
- **[System API](./api-reference/system.md)**: System health and language readiness.
- **[Workspace API](./api-reference/workspace.md)**: File listing and source access.

> **Note:** Nuanced LSP does not currently support authorization. If your use case requires authorization, please [contact us](mailto:support@nuanced.dev).

<div align="center">

# Nuanced LSP - Precise code navigation via an API

Originally forked from [agentic-labs/lsproxy](https://github.com/agentic-labs/lsproxy).

[Reference Documentation](docs/overview.md)

</div>

## <a name="what-is-lsp">What is Nuanced LSP?</a>

Nuanced LSP is a containerized code navigation service.

- Nuanced LSP is designed to provide precise code navigation to agents or other tools.
- It allows using LSP capabilities where setting up locally running LSP servers is impossible or undesirable (e.g., in cloud deployments). _It is not meant to replace local LSP servers for IDE use._
- It exposes [LSProxy](https://github.com/agentic-labs/lsproxy)'s API to access code navigation information.

It supports [multiple languages](#supported-languages) and helps retrieve code context and symbol resolution and symbol relationships for a mounted workspace.

## Key features

- **Precise Cross-File Code Navigation**: Find symbol definitions and references across your entire project.
- **Unified API**: Access multiple language servers through a single API.
- **Auto-Configuration**: Automatically detect and configure language servers based on your project files.
- **SDK**: A Nuanced LSP TypeScript SDK is available for programmatic access along with a CLI.

## Supported languages

| Language              | Image                             | Language Server            |
|-----------------------|-----------------------------------|----------------------------|
| C/C++                 | `nuanced-lsp-clangd`              | clangd                     |
| C#                    | `nuanced-lsp-csharp`              | omnisharp                  |
| Golang                | `nuanced-lsp-golang`              | gopls                      |
| Java                  | `nuanced-lsp-java`                | eclipse-jdtls              |
| PHP                   | `nuanced-lsp-php`                 | phpactor                   |
| Python                | `nuanced-lsp-python`              | jedi-language-server       |
| Ruby                  | `nuanced-lsp-ruby-VERSION`        | ruby-lsp                   |
| Ruby (Sorbet)         | `nuanced-lsp-ruby-sorbet-VERSION` | sorbet                     |
| Rust                  | `nuanced-lsp-rust`                | rust-analyzer              |
| TypeScript/JavaScript | `nuanced-lsp-typescript`          | typescript-language-server |

We aim to support the Ruby versioned released in the last year.

## Getting started

Start using Nuanced LSP using the TypeScript [client](clients/typescript/README.md).

## Documentation

See the `docs/` for more detailed documentation.

## Support and Contributing

Nuanced LSP is maintained but not under active development. We do accept bug fixes, documentation improvements, and small, well-scoped extensions. Supporting larger extensions, feature requests, or support with custom integration and deployment scenarios are out of scope.

For more details see [support](SUPPORT.md) and [contribution](CONTRIBUTING.md) guidelines.

## License

This work is licensed under the terms of the MIT license. For a copy, see [LICENSE](LICENSE) or <https://opensource.org/licenses/MIT>.

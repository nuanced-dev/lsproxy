# Nuanced LSP VSCode Extension

VSCode extension for Nuanced LSP server.

_This is not meant to be a full-featured language plugin! Merely a way to showcase the LSP capabilities and do interactive debugging._

## Requirements

- VSCode 1.75.0 or higher
- `nuanced-lsp` command available in your PATH

## Development

### Building

```bash
npm install
npm run build
```

## Running from Command Line

To start VSCode with the extension enabled for development:

```bash
# From the vscode directory
scripts/code-with-nuanced-lsp
```

## Packaging

To create a VSIX package for distribution:

```bash
npm run package
```

This will create a `.vsix` file that can be installed in VSCode via:

```bash
code --install-extension nuanced-lsp-vscode-0.1.0.vsix
```

## License

This work is licensed under the terms of the MIT license. For a copy, see [LICENSE](LICENSE) or <https://opensource.org/licenses/MIT>.

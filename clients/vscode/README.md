# Nuanced LSP VSCode Extension

VSCode extension for Nuanced LSP server providing advanced language features.

## Development

### Building

```bash
npm install
npm run build
```

### Running from Command Line

To start VSCode with the extension enabled for development:

```bash
# From the vscode directory
code --extensionDevelopmentPath="$(pwd)" /path/to/your/workspace
```

### Packaging

To create a VSIX package for distribution:

```bash
npm run package
```

This will create a `.vsix` file that can be installed in VSCode via:

```bash
code --install-extension nuanced-lsp-vscode-0.1.0.vsix
```

## Requirements

- VSCode 1.75.0 or higher
- `nuanced-lsp` command available in your PATH

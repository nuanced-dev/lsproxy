# Nuanced LSP plugin for Claude Code

Plugin to use the Nuanced LSP server with Claude Code.

## Installation

Install Nuanced LSP:

```bash
npm i -g @nuanced-dev/lsp
```

Add the Nuanced marketplace and install the plugin:

```bash
claude plugin marketplace add nuanced-dev/lsp
claude plugin install lsp@nuanced-dev
```

Start using the LSP tool in Claude Code.

## Caveats

- The plugin requires that Claude Code is started in the project root, otherwise the LSP workspace will be incorrect.

- Some people report that the LSP tool only works when running Claude Code thropugh NPM, not when using the native binaries.

## More Information

See [@nuanced-dev/lsp](https://www.npmjs.com/package/@nuanced-dev/lsp) package.

# Nuanced LSP

Nuanced LSP is a TypeScript client library and CLI wrapper around [`agentic-labs/lsproxy`](https://github.com/agentic-labs/lsproxy).
It exposes the upstream LSProxy API while adding container lifecycle commands and an ergonomic developer experience.

## Quick start

**Install the package:**

```shell
npm install -g @nuanced-dev/lsp
```

Assuming all system dependencies are satisfied and the TypeScript client was successfully built, you should see confirmation the `bin/nuanced-lsp` binary was added.

**Run the CLI:**

```bash
nuanced-lsp --help
nuanced-lsp up /path/to/workspace
nuanced-lsp list-files
nuanced-lsp down
```

**Example library usage:**

```ts
import { NuancedLspClient } from '@nuanced-dev/lsp';

const client = new NuancedLspClient();
await client.up({ workspace: '/path/to/ws' });
console.log(await client.listFiles());
await client.down();
```

---

## API overview

Below is a high-level overview of available API (arguments/options omitted here for brevity).

For all options and arguments, please see the `[api reference](https://docs.nuanced.dev/lsp/api-reference/container-lifecycle).

```bash
nuanced-lsp <command> --help
```

### Lifecycle
- `up` – Start the Nuanced LSP Docker container
- `down` – Stop the container
- `logs` – Print or stream container logs
- `pull` – Pull the Nuanced LSProxy Docker image
- `run` – Run a script inside the container
- `status` – Show Docker lifecycle status

### System
- `health` – Check server health and language readiness flags

### Workspace
- `list-files` – List files detected in the workspace
- `read-source` – Read file contents (optionally a range)

### Symbols
- `definitions-in-file` – List symbol definitions in a file
- `find-definition` – Find the definition at a given position
- `find-identifier` – Find identifiers by name in a file
- `find-referenced-symbols` – Find symbols referenced by the identifier at a given position
- `find-references` – Find all references to the identifier at a given position

This should map 1:1 against the upstream `LSProxy` [API reference](https://docs.lsproxy.dev/api-reference).

---

## Configuration

### Container Images

Nuanced LSP uses multiple Docker containers to provide LSP functionality:

- **Proxy container** - Main LSP proxy server that handles requests
- **Wrapper container** - Wraps language servers for containerized execution
- **Watchdog container** - Monitors and manages language server processes
- **Language containers** - Isolated environments for different programming languages

You can override the default container images using CLI flags:

```bash
nuanced-lsp up /path/to/workspace \
  --proxy-image ghcr.io/nuanced-dev/nuanced-lsp-proxy:0.4.8 \
  --wrapper-image ghcr.io/nuanced-dev/nuanced-lsp-wrapper:0.4.8 \
  --watchdog-image ghcr.io/nuanced-dev/nuanced-lsp-watchdog:0.4.8 \
  --language-container-version 1.0.0
```

**Environment variables** (when used via Nuanced MCP):

When Nuanced LSP is run through the Nuanced MCP server, you can override images using environment variables:

- `PROXY_IMAGE` - Override the LSP proxy container image
- `WRAPPER_IMAGE` - Override the LSP wrapper container image
- `WATCHDOG_IMAGE` - Override the LSP watchdog container image
- `LANGUAGE_IMAGE_VERSION` - Override the language container version

These are useful for testing development builds or using custom container images.

# Nuanced LSP

This is the TypeScript library and CLI for the Nuanced LSP containerized code navigation service.

- Nuanced LSP is designed to provide precise code navigation to agents or other tools.
- It allows using LSP capabilities where setting up locally running LSP servers is impossible or undesirable (e.g., in cloud deployments). _It is not meant to replace local LSP servers for IDE use._
- It exposes [LSProxy](https://github.com/agentic-labs/lsproxy)'s API to access code navigation information.

It supports [multiple languages](#supported-languages) and helps retrieve code context and symbol resolution and symbol relationships for a mounted workspace.

## Requirements

**System dependencies:**

- Recent Node.js version installed
- Docker installed and running

**Resource requirements:**

- Memory usage is related to the size of the workspaces being used. The service containers use ~100MB memory. The language containers memory usage depends on the individual language servers and can run into GB's for large repos.
- Disk usage is ~700MD for the service images, and on average ~1GB for per language image.

## Quick start

**Install Nuanced LSP:**

Install the package:

```bash
npm install -g @nuanced-dev/lsp
```

_Assuming all system dependencies are satisfied and the TypeScript client was successfully built, you should see confirmation the `bin/nuanced-lsp` binary was added._

**Run the CLI:**

```bash
# Start the container with your workspace
nuanced-lsp up /path/to/workspace

# The first time it can take a while for the service to start because it needs to pull the necessary Docker images._

# Check container status
nuanced-lsp status

# Check service health
nuanced-lsp health

# List all files in the workspace
nuanced-lsp list-files

# Get symbol definitions in a file
nuanced-lsp definitions-in-file src/index.ts

# Find definition at a specific position (line:char, 0-indexed)
nuanced-lsp find-definition src/index.ts --position 10:5

# Find all references to a symbol
nuanced-lsp find-references src/index.ts --position 10:5

# Stop the container
nuanced-lsp down
```

See all available commands:

```bash
nuanced-lsp --help
```

**Use as a library:**

Add package dependency:

```bash
npm i -S @nuanced-dev/lsp
```

Example usage:

```typescript
import { NuancedLspClient } from '@nuanced-dev/lsp';

const client = new NuancedLspClient();

// Start container with workspace
await client.up({ workspace: '/path/to/workspace' });

// List workspace files
const files = await client.listFiles();
console.log(files);

// Get definitions in a file
const definitions = await client.definitionsInFile({ file: 'src/index.ts' });

// Find definition at position
const definition = await client.findDefinition({
  file: 'src/index.ts',
  position: { line: 10, character: 5 }
});

// Find all references
const references = await client.findReferences({
  file: 'src/index.ts',
  position: { line: 10, character: 5 }
});

// Clean up
await client.down();
```

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

## API overview

Below is a high-level overview of available API (arguments/options omitted here for brevity).

**Lifecycle [(reference docs)](https://docs.nuanced.dev/lsp/api-reference/container-lifecycle):**

- `up` – Start the Nuanced LSP Docker container
- `down` – Stop the container
- `logs` – Print or stream container logs
- `pull` – Pull the Nuanced LSProxy Docker image
- `run` – Run a script inside the container
- `status` – Show Docker lifecycle status

**System [(reference docs)](https://docs.nuanced.dev/lsp/api-reference/system):**

- `health` – Check server health and language readiness flags

**Workspace [(reference docs)](https://docs.nuanced.dev/lsp/api-reference/workspace):**

- `list-files` – List files detected in the workspace
- `read-source` – Read file contents (optionally a range)

**Symbols [(reference docs)](https://docs.nuanced.dev/lsp/api-reference/symbols):**

- `definitions-in-file` – List symbol definitions in a file
- `find-definition` – Find the definition at a given position
- `find-identifier` – Find identifiers by name in a file
- `find-referenced-symbols` – Find symbols referenced by the identifier at a given position
- `find-references` – Find all references to the identifier at a given position

This should map 1:1 against the upstream `LSProxy` [API reference](https://docs.lsproxy.dev/api-reference).

## Configuration

### Container Images

Nuanced LSP uses multiple Docker containers to provide LSP functionality.

You can override the default container images using CLI flags:

- `--service-image-version` overrides the version of the service images
- `--language-image-version` overrides the version of the language images
- `--container-registry` overrides the container registry where images are pulled from

```bash
nuanced-lsp up /path/to/workspace \
  --service-image-version 0.4.9 \
  --language-image-version 1.0.0
```

**Environment variables** (when used via Nuanced MCP):

When Nuanced LSP is run through the Nuanced MCP server, you can override images using environment variables:

- `CONTAINER_REGISTRY` - Override the container registry
- `LANGUAGE_IMAGE_VERSION` - Override the language image version
- `SERVICE_IMAGE_VERSION` - Override the service image version

These are useful for testing development builds or using custom container images.

## Troubleshooting

If Nuanced LSP is not working as expected, check the following common issues:

- _Docker is not running._ Docker is required to start the containerized LSP servers.

- _The Docker socker is not exposed._ The services requires access to the Docker socker to be able to start language containers on demand.

- _Nuanced LSP is already running for another workspace._ If Nuanced LSP is already running for another workspace, it cannot bind to the default API port. To run the service multiple times, explicitly specify which port to use.

## Support and Contributing

Nuanced LSP is maintained but not under active development. We do accept bug fixes, documentation improvements, and small, well-scoped extensions. Supporting larger extensions, feature requests, or support with custom integration and deployment scenarios are out of scope.

For more details see [support](https://github.com/nuanced-dev/lsp/blob/main/SUPPORT.md) and [contribution](https://github.com/nuanced-dev/lsp/blob/main/CONTRIBUTING.md) guidelines in the repository.

## License

Copyright 2025 Nuanced

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

# Nuanced LSP

This is the TypeScript library and CLI for the Nuanced LSP containerized code navigation service.
It exposes [LSProxy](https://github.com/agentic-labs/lsproxy)'s API while adding container lifecycle commands and an ergonomic developer experience.

## Quick start

**Install Nuanced LSP:**

Requirement dependencies:

- A recent Node.js installation
- Docker daemon installed running

Install the package:

```bash
npm install -g @nuanced-dev/lsp
```

_Assuming all system dependencies are satisfied and the TypeScript client was successfully built, you should see confirmation the `bin/nuanced-lsp` binary was added._

**Run the CLI:**

Start Nuanced LSP for a workspace:

```bash
nuanced-lsp up /path/to/workspace
```

_The first time it can take a while for the service to start because it needs to pull the necessary Docker images._

List all source files in the workspace:

```bash
nuanced-lsp list-files
```

Shut down Nuanced LSP:

```bash
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

```ts
import { NuancedLspClient } from '@nuanced-dev/lsp';

const client = new NuancedLspClient();
await client.up({ workspace: '/path/to/ws' });
console.log(await client.listFiles());
await client.down();
```

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

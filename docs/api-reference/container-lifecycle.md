# Container Lifecycle API Reference

Nuanced LSP runs as a Docker container. The following outlines the API for starting, stopping, modifying the container environment, viewing logs, and managing the Nuanced LSP Docker container and image.

| Command                   | Description                                                       |
|---------------------------|-------------------------------------------------------------------|
| [Up](#up)                 | Start the container.                                              |
| [Down](#down)             | Stop the container.                                               |
| [Logs](#logs)             | View the container logs.                                          |
| [Pull](#pull)             | Pull the `nuanced-lsproxy` image.                                 |
| [Run](#run)               | Run a script within the container environment.                    |
| [Status](#status)         | Check the status of the container.                                |

> **Tip:** The `Pull` command is useful for pre-pulling the `nuanced-lsproxy` image for faster startup times, or to update to the latest version.

## Up

Starts the Nuanced LSP container.

### CLI

```
> nuanced-lsp up /path/to/workspace
```

#### Arguments

| Argument    | Description                                                          | Required |
|-------------|----------------------------------------------------------------------|----------|
| `workspace` | Path to the host workspace to mount within the Nuanced LSP container | Yes      |

#### CLI options

| Option                    | Description                                                                 | Default                                                     |
|---------------------------|-----------------------------------------------------------------------------|-------------------------------------------------------------|
| `--host-port <n>`         | Host port to bind to container process                                      | `4444`                                                      |
| `--bind-host <host>`      | Host/IP to bind (e.g. `127.0.0.1`, `0.0.0.0`, `192.168.1.10`)               | `"127.0.0.1"`                                               |
| `--image <ref>`           | Docker image to use for standing up the container process                   | `"ghcr.io/nuanced-dev/nuanced-lsproxy:<version>"`           |
| `--container-name <name>` | Container name                                                              | `"nuanced-lsp"`                                             |
| `--timeout <s>`           | Health check poll loop timeout seconds (`<=0` to skip)                      | `120`                                                       |
| `--sudo`                  | Docker commands are executed with `sudo`                                    | Docker commands are executed without sudo                   |
| `--stream`                | Process stdout, stderr, and container logs are streamed to the CLI's stdout | Process stdout, stderr, and container logs are not streamed |
| `--ro`                    | Mount workspace as read-only                                                | Mount workspace is read-write                               |
| `--json`                  | Output machine readable JSON                                                | Text output                                                 |
| `--debug`                 | Enable debug log level for container process                                | Process log level is info                                   |

#### Result

Success result confirms the container is running and bound to the specified host and port:

```
Nuanced LSP container 'nuanced-lsp' bound at 127.0.0.1:4444
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

await lsp.up({
  workspace: "/path/to/workspace",
});
```

#### Constructor options

| Option         | Type      | Default / Notes                               | Description                                                                   |
|----------------|-----------|-----------------------------------------------|-------------------------------------------------------------------------------|
| `containerName`| `string`  | `"nuanced-lsp"`                               | Docker container name.                                                        |
| `lsProxyUrl`   | `string`  | `"http://127.0.0.1"`                          | Base URL used to reach Nuanced LSP. Trailing `/` is trimmed.                  |
| `lsProxyPort`  | `number`  | `4444`                                        | Port used to reach LSProxy.                                                   |
| `timeoutSecs`  | `number`  | `120`                                         | Overall timeout for lifecycle operations (seconds).                           |
| `retries`      | `number`  | `3`                                           | Number of retry attempts for HTTP requests to account for transient failures. |
| `sudo`         | `boolean` | `false`                                       | Docker commands are executed with `sudo`.                                     |
| `json`         | `boolean` | `false`                                       | Output machine readable JSON                                                  |

#### Signature

```typescript
up(opts: {
  workspace: string;
  mountDir?: string;
  image?: string;
  timeout?: number;
  stream?: boolean;
  ro?: boolean;
  bindHost?: string;
  json?: boolean;
  debug?: boolean;
}): Promise<UpResult>;
```

| Option       | Type      | Default / Notes                                   | Description                                                 |
|--------------|-----------|---------------------------------------------------|--------------------------------------------------------------|
| `workspace`  | `string`  | **required**                                      | Host workspace directory to mount.                           |
| `mountDir`   | `string`  | `"/workspace"`                                    | Path inside the container where the workspace is mounted.    |
| `image`      | `string`  | `"ghcr.io/nuanced-dev/nuanced-lsproxy:<version>"` | Docker image to use for starting the container.              |
| `timeout`    | `number`  | `120`                                             | Health check poll loop timeout (seconds). `<=0` to skip.     |
| `stream`     | `boolean` | `false`                                           | Stream process stdout, stderr, and container logs on startup.|
| `ro`         | `boolean` | `false`                                           | Mount workspace as read-only (default is read-write).        |
| `bindHost`   | `string`  | `"127.0.0.1"`                                     | Host/IP to bind (e.g. `127.0.0.1`, `0.0.0.0`).               |
| `json`       | `boolean` | `false`                                           | Output machine readable JSON                                 |
| `debug`      | `boolean` | `false`                                           | Enable debug log level for container process                 |

#### Return type

```typescript
interface UpResult {
  host_port: number;
  base_url: string;
}
```

## Down

Stops the Nuanced LSP container.

### CLI

```
> nuanced-lsp down
```

#### CLI options

| Option                   | Description                                  | Default                                   |
|--------------------------|----------------------------------------------|-------------------------------------------|
| `--container-name <name>`| Container name                               | `"nuanced-lsp"`                           |
| `--timeout <s>`          | Timeout seconds (`<=0` to skip)              | `120`                                     |
| `--sudo`                 | Docker commands are executed with `sudo`      | Docker commands are executed without sudo |
| `--json`                 | Output machine readable JSON                 | Text output                               |

#### Result

Success result confirms the container was stopped:

```
Nuanced LSP Stopped container 'nuanced-lsp'.
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const result = await lsp.down();
```

#### Signature

```typescript
down(): Promise<CommandResult>;
```

#### Return type

```typescript
export interface CommandResult {
  ok: boolean;
  error?: string;
}
```

- `ok: true` — container stopped successfully
- `ok: false` + `error` — container stop failed

## Logs

Print logs from the Nuanced LSP container.

### CLI

```
> nuanced-lsp logs
```

#### CLI options

| Option                   | Description                                                           | Default                                                     |
|--------------------------|-----------------------------------------------------------------------|-------------------------------------------------------------|
| `--container-name <name>`| Container name                                                        | `"nuanced-lsp"`                                             |
| `--timeout <s>`          | Timeout seconds (`<=0` to skip)                                       | `120`                                                       |
| `--sudo`                 | Docker commands are executed with `sudo`                              | Docker commands are executed without sudo                   |
| `--stream`               | Process stdout, stderr, and container logs are streamed to the CLI's stdout | Process stdout, stderr, and container logs are not streamed |
| `--since <when>`         | Only show logs since (e.g. `10s`, `5m`, or RFC3339 timestamp)         | —                                                           |
| `--tail <n\|all>`        | Show last N lines, or `all` for full history                          | —                                                           |
| `--json`                 | Output machine readable JSON                                          | Text output                                                 |

#### Result

On success, logs are printed to stdout.

Example:
```
[2025-09-02T17:02:11Z] Starting Nuanced LSP...
[2025-09-02T17:02:12Z] Listening on 127.0.0.1:4444
```

If the container is not running, an error is shown.

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const result = await lsp.logs({
  since: "5m",
  tail: 100,
  stream: false,
});
```

#### Signature

```typescript
logs(opts?: {
  since?: string;
  tail?: number | "all";
  stream?: boolean;
}): Promise<LogsResult>;
```

#### Return type

```typescript
export interface LogsResult extends CommandResult {
  ok: boolean;
  error?: string;  // present when ok === false
  output?: string; // present when ok === true
}
```

#### Example result

```json
{
  "ok": true,
  "output": "2025-09-02T20:53:06.552155919Z 2025-09-02T20:53:06.552018Z  INFO lsproxy: Starting on port 4444\n2025-09-02T20..."
}
```

## Pull

Pull the Nuanced LSP image.

### CLI

```
> nuanced-lsp pull
```

#### CLI options

| Option            | Description                                        | Default                                               |
|-------------------|----------------------------------------------------|-------------------------------------------------------|
| `--image <ref>`   | Nuanced LSP Docker image to pull                   | `"ghcr.io/nuanced-dev/nuanced-lsproxy:<version>"`     |
| `--sudo`          | Docker commands are executed with `sudo`            | Docker commands are executed without sudo             |
| `--stream`        | Process stdout, stderr, and container logs are streamed to the CLI's stdout | Process stdout, stderr, and container logs are not streamed |
| `--json`          | Output machine readable JSON                       | Text output                                           |

#### Result

On success, the image is pulled and confirmation is shown.

Example:
```
Pulled image ghcr.io/nuanced-dev/nuanced-lsproxy:0.3.8
```

### TypeScript

```typescript
import { NuancedLSP } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLSP();

const image = "ghcr.io/nuanced-dev/nuanced-lsproxy:0.3.8";
const stream = true;

const result = await lsp.pull(
  image,
  stream
);
```

#### Signature

```typescript
pull(
  image?: string,
  stream?: boolean
): Promise<PullResult>;
```

#### Return type

```typescript
export interface PullResult {
  ok: boolean;
  error?: string;
  image?: string;   // present when ok === true
  output?: string;  // raw docker pull logs if available
}
```

#### Example result

```json
{
  "ok": true,
  "image": "ghcr.io/nuanced-dev/nuanced-lsproxy:0.3.8",
  "output": "Digest: sha256:abc123..."
}
```

- `ok: true` — `image` and optional `output` returned
- `ok: false` + `error` — pull failed

## Run

Run a script inside the Nuanced LSP container.

> **Note:** The script is copied to the container and is executed within the container as the root user.

### CLI

```
> nuanced-lsp run ./scripts/setup.sh
```

#### Arguments

| Argument    | Description                                | Required |
|-------------|--------------------------------------------|----------|
| `script`    | Path to a script to copy to the container and execute | Yes      |

#### CLI options

| Option                   | Description                                              | Default                                                     |
|--------------------------|----------------------------------------------------------|-------------------------------------------------------------|
| `--container-name <name>`| Container name                                           | `"nuanced-lsp"`                                             |
| `--timeout <s>`          | Timeout seconds (`<=0` to skip)                          | `120`                                                       |
| `--sudo`                 | Docker commands are executed with `sudo`                  | Docker commands are executed without sudo                   |
| `--stream`               | Process stdout, stderr, and container logs are streamed to the CLI's stdout | Process stdout, stderr, and container logs are not streamed |
| `--json`                 | Output machine readable JSON                             | Text output                                                 |

#### Result

On success:
```
Script executed successfully in nuanced-lsp.
```

On failure, the error is printed, and the process exits with exit code 1.

### TypeScript

```typescript
import { NuancedLSP } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const script =  "scripts/setup.sh";

// Run a script and stream the process stdout / stderr inside the container
const result = await lsp.run(script, { stream: true });
```

#### Signature

```typescript
run(
  script: string,
  opts?: {
    stream: boolean;
  }
): Promise<CommandResult>;
```

#### Return type

```typescript
export interface CommandResult {
  ok: boolean;
  error?: string;
}
```

#### Example result

```json
{
  "ok": true
}
```

- `ok: true` — script executed successfully
- `ok: false` + `error` — script failed or container not running

## Status

Show the container lifecycle status of the Nuanced LSP container.

This command does not perform a `/health` probe. It only checks Docker's view of the container.

### CLI

```
> nuanced-lsp status
```

#### CLI options

| Option                   | Description                              | Default         |
|--------------------------|------------------------------------------|-----------------|
| `--container-name <name>`| Container name                           | `"nuanced-lsp"` |
| `--timeout <s>`          | Timeout seconds (`<=0` to skip)          | `120`           |
| `--sudo`                 | Docker commands are executed with `sudo`  | Docker commands are executed without sudo |
| `--json`                 | Output machine readable JSON             | Text output     |

#### Result

**Default output:**
```
Container: nuanced-lsp
Running:   yes
Docker:    Up 29 minutes
```

**With `--json`:**
```json
{
  "container_name": "nuanced-lsp",
  "running": true,
  "container_status": "Up 28 minutes"
}
```

### TypeScript

```typescript
import { NuancedLSP } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLSP();

const status = await lsp.status();
```

#### Signature

```typescript
status(): Promise<Status>;
```

#### Return type

```typescript
export interface Status {
  container_name: string;
  running: boolean;
  container_status?: string | null; // e.g. "Up 10 seconds"
}
```

#### Example result

```json
{
  "container_name": "nuanced-lsp",
  "running": true,
  "container_status": "Up 28 minutes"
}
```

- `running: true` — container is active
- `running: false` — container stopped or not found

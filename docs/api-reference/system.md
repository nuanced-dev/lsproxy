# System API Reference

Check system health and LSP server readiness.

## Health

Check Nuanced LSP server status and language readiness.

### CLI

```
> nuanced-lsp health
```

#### CLI options

| Option            | Description                     | Default             |
|-------------------|---------------------------------|---------------------|
| `--lsp-url <url>` | Nuanced LSP base URL            | `"http://127.0.0.1"`|
| `--lsp-port <n>`  | Port for Nuanced LSP            | `4444`              |
| `--timeout <s>`   | Timeout seconds (`<=0` to skip) | `120`               |
| `--json`          | Output machine readable JSON    | Text output         |

#### Result

Outputs JSON matching `HealthResult`:

```json
{
  "status": "ok",
  "version": "0.4.4",
  "languages": {
    "php": false,
    "python": false,
    "java": false,
    "cpp": false,
    "typescript_javascript": true,
    "csharp": false,
    "ruby": false,
    "golang": false,
    "rust": false
  }
}
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const health = await lsp.health();
```

#### Signature

```typescript
health(timeoutSecs?: number): Promise<HealthResult>;
```

#### Return type

```typescript
export interface HealthResult {
  status: "ok" | "not ok";
  version?: string;
  languages?: Record<string, boolean>;
  error?: string;
}
```

#### Example result

```json
{
  "status": "ok",
  "version": "0.4.4",
  "languages": { "typescript_javascript": true }
}
```

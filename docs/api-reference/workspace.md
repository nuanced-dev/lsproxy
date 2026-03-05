# Workspace API Reference

List all files and read source code from files in the workspace.

| Command                          | Description                                        |
|----------------------------------|----------------------------------------------------|
| [List Files](#list-files)        | List all files in the workspace.                   |
| [Read Source](#read-source)      | Read an individual source file in the workspace.   |

## List Files

List all files in the Nuanced LSP workspace.

### CLI

```
> nuanced-lsp list-files
```

#### CLI options

| Option            | Description                     | Default             |
|-------------------|---------------------------------|---------------------|
| `--lsp-url <url>` | Nuanced LSP base URL            | `"http://127.0.0.1"`|
| `--lsp-port <n>`  | Port for Nuanced LSP            | `4444`              |
| `--timeout <s>`   | Timeout seconds (`<=0` to skip) | `120`               |
| `--json`          | Output machine readable JSON    | Text output         |

#### Result

Prints one path per line, e.g.:

```
eslint.config.js
jest.config.js
src/index.ts
src/utils.ts
...
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

// Default timeout
const files = await lsp.listFiles();

// With timeout override (seconds)
const timeoutSecs = 60;
const filesWithTimeout = await lsp.listFiles(timeoutSecs);
```

#### Signature

```typescript
listFiles(timeoutSecs?: number): Promise<string[]>;
```

#### Example result

```json
[
  "eslint.config.js",
  "src/index.ts",
  "src/utils.ts"
]
```

---

## Read Source

Read the full source of a file (or an optional range).

### CLI

```
> nuanced-lsp read-source src/index.ts
```

Read a range (0-indexed `line:char-line:char`):
```
> nuanced-lsp read-source --range 0:0-10:5 src/index.ts
```

#### CLI options

| Option                         | Description                                             | Default              |
|--------------------------------|---------------------------------------------------------|----------------------|
| `file` (argument)              | Path relative to workspace root                         | —                    |
| `--range <line:char-line:char>`| Optional range (e.g. `0:0-10:5`)                        | —                    |
| `--lsp-url <url>`              | Nuanced LSP base URL                                    | `"http://127.0.0.1"` |
| `--lsp-port <n>`               | Port for Nuanced LSP                                    | `4444`               |
| `--timeout <s>`                | Timeout seconds (`<=0` to skip)                         | `120`                |
| `--json`                       | Output machine readable JSON                            | Text output          |

#### Result

Returns the file contents to stdout. Example (truncated):

```
import js from '@eslint/js';
import typescript from '@typescript-eslint/eslint-plugin';
...
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const filePath = "src/index.ts";

// Entire file
const full = await lsp.readSource(filePath);

// Specific range
const range: Range = {
  start: { line: 0, character: 0 },
  end: { line: 10, character: 5 }
};
const timeoutSecs = 60;
const snippet = await lsp.readSource(filePath, range, timeoutSecs);
```

#### Signature

```typescript
readSource(
  filePath: string,
  range?: Range | null,
  timeoutSecs?: number
): Promise<ReadSourceResult>;
```

#### Types

```typescript
export interface Position { line: number; character: number }

export interface Range { start: Position; end: Position }

export interface ReadSourceResult {
  source_code: string;
}
```

#### Example result

```json
{
  "source_code": "import js from '@eslint/js';\nimport typescript from '@typescript-eslint/eslint-plugin';\n..."
}
```

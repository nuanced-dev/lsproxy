# Symbols API Reference

Retrieve definitions, identifiers, references, and related graph data.

Nuanced LSP provides symbol-level APIs for exploring your codebase.
These commands let you list definitions, locate identifiers, and find references or symbol relationships within the workspace.

| Command                                              | Description                                                                 |
|------------------------------------------------------|-----------------------------------------------------------------------------|
| [Definitions in File](#definitions-in-file)          | List all symbol definitions in a specific file.                            |
| [Find Definition](#find-definition)                  | Find the definition of the identifier at a given position.                 |
| [Find Identifier](#find-identifier)                  | Find identifiers by name within a file.                                    |
| [Find Referenced Symbols](#find-referenced-symbols)  | Find symbols referenced by the identifier at a given position.             |
| [Find References](#find-references)                  | Find all references to the identifier at a given position.                 |

> **Tip:** These commands use **0-indexed positions** in the format `line:char` (e.g. `0:0` for the first character of the first line).

## Definitions in File

List all symbol definitions within a single file.

> **Note:** The file path argument should always be relative to the workspace root.

### CLI

```
> nuanced-lsp definitions-in-file path/to/file
```

#### Arguments

| Argument    | Description                                | Required |
|-------------|--------------------------------------------|----------|
| `file`      | Path relative to workspace root            | Yes      |

#### CLI options

| Option            | Description                     | Default             |
|-------------------|---------------------------------|---------------------|
| `--lsp-url <url>` | Nuanced LSP base URL            | `"http://127.0.0.1"`|
| `--lsp-port <n>`  | Port for Nuanced LSP            | `4444`              |
| `--timeout <s>`   | Timeout seconds (`<=0` to skip) | `120`               |
| `--json`          | Output machine readable JSON    | Text output         |

#### Result

Returns JSON array of definitions:

```json
[
  {
    "name": "registerTools",
    "kind": "function",
    "identifier_position": {
      "path": "src/tools/index.ts",
      "position": { "line": 4, "character": 16 }
    },
    "file_range": {
      "path": "src/tools/index.ts",
      "range": {
        "start": { "line": 4, "character": 0 },
        "end": { "line": 7, "character": 1 }
      }
    }
  }
]
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const filePath = "path/to/file";
const defs = await lsp.definitionsInFile(filePath);
```

#### Signature

```typescript
definitionsInFile(
  filePath: string,
  timeoutSecs?: number
): Promise<DefinitionsInFileResult>;
```

#### Types

```typescript
export type DefinitionsInFileResult = DefinitionInFile[];

export interface DefinitionInFile {
  file_range: FileRange;
  identifier_position: IdentifierPosition;
  kind: string;
  name: string;
}

export interface FileRange { path: string; range: Range }

export interface Range { start: Position; end: Position }

export interface Position { line: number; character: number }

export interface IdentifierPosition { path: string; position: Position }
```

#### Example result

```json
[
  {
    "name": "registerTools",
    "kind": "function",
    "identifier_position": {
      "path": "src/tools/index.ts",
      "position": { "line": 4, "character": 16 }
    },
    "file_range": {
      "path": "src/tools/index.ts",
      "range": {
        "start": { "line": 4, "character": 0 },
        "end": { "line": 7, "character": 1 }
      }
    }
  }
]
```

## Find Definition

Find the definition of the identifier at a given position.

> **Note:** Positions are always **0-indexed** in the format `line:char` (e.g., `0:0` for the first character of the first line).

### CLI

```
> nuanced-lsp find-definition path/to/file 4:16
```

#### Arguments

| Argument    | Description                                | Required |
|-------------|--------------------------------------------|----------|
| `file`      | Path relative to workspace root            | Yes      |
| `position`  | Position in `line:char` format (0-indexed) | Yes      |

#### CLI options

| Option                | Description                     | Default             |
|-----------------------|---------------------------------|---------------------|
| `--lsp-url <url>`     | Nuanced LSP base URL            | `"http://127.0.0.1"`|
| `--lsp-port <n>`      | Port for Nuanced LSP            | `4444`              |
| `--timeout <s>`       | Timeout seconds (`<=0` to skip) | `120`               |
| `--json`              | Output machine readable JSON    | Text output         |

#### Result

Returns JSON matching `FindDefinitionResult`:

```json
{
  "raw_response": [
    {
      "range": {
        "end": { "character": 29, "line": 4 },
        "start": { "character": 16, "line": 4 }
      },
      "uri": "file:///workspace/src/tools/index.ts"
    }
  ],
  "definitions": [
    {
      "path": "src/tools/index.ts",
      "position": { "line": 4, "character": 16 }
    }
  ],
  "source_code_context": [
    {
      "file_range": {
        "path": "src/tools/index.ts",
        "range": {
          "start": { "line": 4, "character": 7 },
          "end": { "line": 7, "character": 1 }
        }
      },
      "source_code": "function registerTools(server: McpServer) {\n  registerInitTool(server);\n  registerEnrichTool(server);\n}"
    }
  ],
  "selected_identifier": {
    "name": "registerTools",
    "file_range": {
      "path": "src/tools/index.ts",
      "range": {
        "start": { "line": 4, "character": 16 },
        "end": { "line": 4, "character": 29 }
      }
    },
    "kind": null
  }
}
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const filePath = "path/to/file";
const line = 4;
const char = 16;

const result = await lsp.findDefinition("path/to/file", line, char);
```

#### Signature

```typescript
findDefinition(
  filePath: string,
  line: number,
  character: number,
  timeoutSecs?: number
): Promise<FindDefinitionResult>;
```

#### Types

```typescript
export interface FindDefinitionResult {
  definitions: DefinitionLocation[];
  selected_identifier: SelectedIdentifier;
  raw_response: unknown;
  source_code_context: ContextSnippet[] | null;
}

export interface DefinitionLocation {
  path: string;
  position: Position;
}

export interface ContextSnippet {
  file_range: FileRange;
  source_code: string;
}

export interface SelectedIdentifier {
  file_range: FileRange;
  kind: string | null;
  name: string;
}

export interface FileRange { path: string; range: Range }

export interface Range { start: Position; end: Position }

export interface Position { line: number; character: number }
```

#### Example result

```json
{
  "definitions": [
    { "path": "src/tools/index.ts", "position": { "line": 4, "character": 16 } }
  ],
  "selected_identifier": {
    "name": "registerTools",
    "file_range": {
      "path": "src/tools/index.ts",
      "range": {
        "start": { "line": 4, "character": 16 },
        "end": { "line": 4, "character": 29 }
      }
    },
    "kind": null
  },
  "raw_response": {},
  "source_code_context": null
}
```

## Find Identifier

Find identifiers by name within a file. Optionally provide a seed position.

> **Note:** You can call this endpoint with or without a position to affect its behavior:
> * Without a position returns all matching identifiers in the file.
> * With a position scopes the identifier returned to the identifier and position provided, or if none are found at the precise position, the 3 closest (by position) identifiers matching the name.

### CLI

```
> nuanced-lsp find-identifier path/to/file registerTools
```

With a seed position:
```
> nuanced-lsp find-identifier path/to/file registerTools --position 15:29
```

#### Arguments

| Argument    | Description                                | Required |
|-------------|--------------------------------------------|----------|
| `file`      | Path relative to workspace root            | Yes      |
| `name`      | Identifier name to search for              | Yes      |

#### CLI options

| Option                  | Description                                              | Default             |
|-------------------------|----------------------------------------------------------|---------------------|
| `--position <line:char>`| Optional seed position (0-indexed `line:char`)           | —                   |
| `--lsp-url <url>`       | Nuanced LSP base URL                                     | `"http://127.0.0.1"`|
| `--lsp-port <n>`        | Port for Nuanced LSP                                     | `4444`              |
| `--timeout <s>`         | Timeout seconds (`<=0` to skip)                          | `120`               |
| `--json`                | Output machine readable JSON                             | Text output         |

#### Result

Returns JSON matching `FindIdentifierResult`:

**Without position**
```json
{
  "identifiers": [
    {
      "name": "registerTools",
      "file_range": {
        "path": "src/tools/index.ts",
        "range": {
          "start": { "line": 4, "character": 16 },
          "end":   { "line": 4, "character": 29 }
        }
      },
      "kind": null
    }
  ]
}
```

**With position**
```json
{
  "identifiers": [
    {
      "name": "registerTools",
      "file_range": {
        "path": "src/tools/index.ts",
        "range": {
          "start": { "line": 4, "character": 16 },
          "end":   { "line": 4, "character": 29 }
        }
      },
      "kind": null
    }
  ]
}
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const filePath = "path/to/file";
const identifierName = "registerTools";

// Without position
const res1 = await lsp.findIdentifier(filePath, identifierName);

// With position
const position = { line: 15, character: 29 };
const res2 = await lsp.findIdentifier(filePath, identifierName, position);
```

#### Signature

```typescript
findIdentifier(
  filePath: string,
  name: string,
  position?: { line: number; character: number } | null,
  timeoutSecs?: number
): Promise<FindIdentifierResult>;
```

#### Types

```typescript
export interface FindIdentifierResult {
  identifiers: Identifier[];
}

export interface Identifier {
  file_range: FileRange;
  kind: string | null;
  name: string;
}

export interface FileRange { path: string; range: Range }

export interface Range { start: Position; end: Position }

export interface Position { line: number; character: number }
```

#### Example result

```json
{
  "identifiers": [
    {
      "name": "registerTools",
      "file_range": {
        "path": "src/tools/index.ts",
        "range": {
          "start": { "line": 4, "character": 16 },
          "end":   { "line": 4, "character": 29 }
        }
      },
      "kind": null
    }
  ]
}
```

## Find Referenced Symbols

For a target symbol, returns all symbols referenced within the target's implementation.

The referenced symbols are categorized into the following:

* **Workspace symbols**: symbols found in the workspace along with their definitions.
* **External symbols**: symbols from external libraries or built-in functions.
* **Not found symbols**: symbols Nuanced LSP was unable to resolve.

> **Note:** The full scan option uses more permissive rules for finding referenced symbols. Depending on the LSP server, may use type hints and chained indirection.

### CLI

```
> nuanced-lsp find-referenced-symbols --full-scan path/to/file 4:16
```

#### Arguments and options

| Option             | Description                                        | Default             |
|--------------------|----------------------------------------------------|---------------------|
| `file`             | Path relative to workspace root                    | —                   |
| `position`         | Position in `line:char` format (0-indexed)         | —                   |
| `--full-scan`      | Perform a broader workspace scan for references    | `false`             |
| `--lsp-url <url>`  | Nuanced LSP base URL                               | `"http://127.0.0.1"`|
| `--lsp-port <n>`   | Port for Nuanced LSP                               | `4444`              |
| `--timeout <s>`    | Timeout seconds (`<=0` to skip)                    | `120`               |
| `--json`           | Output machine readable JSON                       | Text output         |

#### Result

Returns JSON matching `FindReferencedSymbolsResult` (excerpt):

```json
{
  "workspace_symbols": [
    {
      "reference": {
        "name": "registerInitTool",
        "file_range": {
          "path": "src/tools/index.ts",
          "range": {
            "start": { "line": 5, "character": 2 },
            "end":   { "line": 5, "character": 18 }
          }
        },
        "kind": "all-references"
      },
      "definitions": [
        {
          "name": "registerInitTool",
          "kind": "function",
          "identifier_position": {
            "path": "src/tools/init.ts",
            "position": { "line": 12, "character": 16 }
          },
          "file_range": {
            "path": "src/tools/init.ts",
            "range": {
              "start": { "line": 12, "character": 0 },
              "end":   { "line": 22, "character": 1 }
            }
          }
        }
      ]
    }
  ],
  "external_symbols": [],
  "not_found": []
}
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const filePath = "path/to/file";
const line = 4;
const char = 16;

// Basic usage
const res = await lsp.findReferencedSymbols(filePath, line, char);

// Full workspace scan with timeout override (secs)
const fullScan = true;
const timeoutSecs = 60;
const resFull = await lsp.findReferencedSymbols(filePath, line, char, fullScan, timeoutSecs);
```

#### Signature

```typescript
findReferencedSymbols(
  filePath: string,
  line: number,
  character: number,
  fullScan?: boolean,
  timeoutSecs?: number
): Promise<FindReferencedSymbolsResult>;
```

#### Types

```typescript
export interface FindReferencedSymbolsResult {
  external_symbols: ExternalSymbol[];
  not_found: NotFoundSymbol[];
  workspace_symbols: WorkspaceSymbol[];
}

export interface ExternalSymbol {
  file_range: FileRange;
  kind: string | null;
  name: string;
}

export interface NotFoundSymbol {
  file_range: FileRange;
  kind: string | null;
  name: string;
}

export interface WorkspaceSymbol {
  reference: WorkspaceReference;
  definitions: WorkspaceDefinition[];
}

export interface WorkspaceReference {
  file_range: FileRange;
  kind: string | null;
  name: string;
}

export interface WorkspaceDefinition {
  file_range: FileRange;
  identifier_position: IdentifierPosition;
  kind: string | null;
  name: string;
}

export interface FileRange { path: string; range: Range }
export interface IdentifierPosition { path: string; position: Position }
export interface Range { start: Position; end: Position }
export interface Position { line: number; character: number }
```

#### Example result (excerpt)

```json
{
  "workspace_symbols": [
    {
      "reference": {
        "name": "registerEnrichTool",
        "file_range": {
          "path": "src/tools/index.ts",
          "range": {
            "start": { "line": 6, "character": 2 },
            "end":   { "line": 6, "character": 20 }
          }
        },
        "kind": "all-references"
      },
      "definitions": [
        { "name": "registerEnrichTool", "kind": "function" }
      ]
    }
  ],
  "external_symbols": [],
  "not_found": []
}
```

## Find References

Find all references to the identifier at a given position. Optionally include lines of code context around each reference.

> **Note:** The optional context lines parameter includes lines of source code around each found reference.

### CLI

```
> nuanced-lsp find-references path/to/file 6:21
```

Include context lines:
```
> nuanced-lsp find-references --context-lines 3 path/to/file 6:21
```

#### Arguments

| Argument    | Description                                | Required |
|-------------|--------------------------------------------|----------|
| `file`      | Path relative to workspace root            | Yes      |
| `position`  | Position in `line:char` format (0-indexed) | Yes      |

#### CLI options

| Option                 | Description                                              | Default             |
|------------------------|----------------------------------------------------------|---------------------|
| `--context-lines <n>`  | Include `n` lines of surrounding code for each reference | `0`                 |
| `--lsp-url <url>`      | Nuanced LSP base URL                                     | `"http://127.0.0.1"`|
| `--lsp-port <n>`       | Port for Nuanced LSP                                     | `4444`              |
| `--timeout <s>`        | Timeout seconds (`<=0` to skip)                          | `120`               |
| `--json`               | Output machine readable JSON                             | Text output         |

#### Result

Returns JSON matching `FindReferencesResult` (excerpt):

```json
{
  "raw_response": [{ "range": { "start": { "line": 4, "character": 30 }, "end": { "line": 4, "character": 36 } }, "uri": "file:///workspace/src/tools/index.ts" }],
  "references": [
    { "path": "src/tools/index.ts", "position": { "line": 4, "character": 30 } },
    { "path": "src/tools/index.ts", "position": { "line": 5, "character": 19 } },
    { "path": "src/tools/index.ts", "position": { "line": 6, "character": 21 } }
  ],
  "context": [
    {
      "file_range": {
        "path": "src/tools/index.ts",
        "range": { "start": { "line": 4, "character": 0 }, "end": { "line": 4, "character": 0 } }
      },
      "source_code": ""
    }
  ],
  "selected_identifier": {
    "name": "server",
    "file_range": {
      "path": "src/tools/index.ts",
      "range": { "start": { "line": 6, "character": 21 }, "end": { "line": 6, "character": 27 } }
    },
    "kind": null
  }
}
```

### TypeScript

```typescript
import { NuancedLspClient } from '@nuanced-dev/nuanced-lsp';

const lsp = new NuancedLspClient();

const filePath = "path/to/file";
const line = 6;
const char = 21;

// Basic usage
const refs = await lsp.findReferences(filePath, line, char);

// Include 3 lines of context per reference, with a timeout override (secs)
const contextLines = 3;
const timeoutSecs = 60;
const refsWithContext = await lsp.findReferences(filePath, line, char, contextLines, timeoutSecs);
```

#### Signature

```typescript
findReferences(
  filePath: string,
  line: number,
  character: number,
  contextLines?: number,   // default 0
  timeoutSecs?: number
): Promise<FindReferencesResult>;
```

#### Types

```typescript
export interface FindReferencesResult {
  references: ReferenceLocation[];
  selected_identifier: SelectedIdentifier;
  context: ContextSnippet[] | null;
  raw_response: unknown; // raw language server response
}

export interface ReferenceLocation {
  path: string;
  position: Position;
}

export interface Position { line: number; character: number }

export interface SelectedIdentifier {
  file_range: FileRange;
  kind: string | null;
  name: string;
}

export interface FileRange { path: string; range: Range }

export interface Range { start: Position; end: Position }

export interface ContextSnippet {
  file_range: FileRange;
  source_code: string;
}
```

#### Example result (excerpt)

```json
{
  "references": [
    { "path": "src/tools/index.ts", "position": { "line": 4, "character": 30 } }
  ],
  "selected_identifier": {
    "name": "server",
    "file_range": {
      "path": "src/tools/index.ts",
      "range": { "start": { "line": 6, "character": 21 }, "end": { "line": 6, "character": 27 } }
    },
    "kind": null
  },
  "context": null,
  "raw_response": {}
}
```

import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

import {
  ListFilesResultSchema,
  ReadSourceResultSchema,
  DefinitionsInFileResultSchema,
  FindDefinitionResultSchema,
  FindIdentifierResultSchema,
  FindReferencedSymbolsResultSchema,
  FindReferencesResultSchema,
} from "../../src/types.js";
import { isGeneratedPath } from "./constants.js";

// Map of endpoint names to their Zod schemas
const ENDPOINT_SCHEMAS = {
  "list-files": ListFilesResultSchema,
  "read-source": ReadSourceResultSchema,
  "definitions-in-file": DefinitionsInFileResultSchema,
  "find-definition": FindDefinitionResultSchema,
  "find-identifier": FindIdentifierResultSchema,
  "find-identifier-narrowed": FindIdentifierResultSchema,
  "find-referenced-symbols": FindReferencedSymbolsResultSchema,
  "find-references": FindReferencesResultSchema,
} as const;

type EndpointName = keyof typeof ENDPOINT_SCHEMAS;

export interface FixtureMeta {
  endpoint: string;
  sourcePath: string;
  params: Record<string, unknown>;
  label?: string | null;
}

const SEP = "__";

export const RECORD_FIXTURES =
  (process.env.RECORD_FIXTURES ?? "").toLowerCase() === "1" ||
  (process.env.RECORD_FIXTURES ?? "").toLowerCase() === "true" ||
  (process.env.RECORD_FIXTURES ?? "").toLowerCase() === "yes";

// ============================================================================
// Shared utility functions
// ============================================================================

function slug(input: string | null | undefined): string {
  if (!input) return "x";
  const lowered = input.toLowerCase();
  const cleaned = lowered.replace(/[^a-z0-9._/-]+/g, "-");
  const collapsed = cleaned.replace(/-+/g, "-").replace(/^-|-$/g, "");
  return collapsed || "x";
}

function stableStringify(value: unknown): string {
  if (value === null || value === undefined) {
    return "null";
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(",")}]`;
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(([key, val]) => `${JSON.stringify(key)}:${stableStringify(val)}`);
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value);
}

function stableHash(obj: unknown): string {
  const json = stableStringify(obj);
  return crypto.createHash("sha1").update(json).digest("hex").slice(0, 10);
}

function buildFixtureFilename(meta: FixtureMeta): string {
  const { endpoint, sourcePath, params, label } = meta;
  const flatRel = sourcePath
    .replace(/\\/g, "/")
    .split("/")
    .filter((segment) => segment.length > 0)
    .join(SEP);
  const relSlug = slug(flatRel).slice(0, 120);
  const labelSlug = label ? `${SEP}${slug(label).slice(0, 80)}` : "";

  const hashInput: Record<string, unknown> = { params };
  if (label) {
    hashInput.label = label;
  }
  const digest = stableHash(hashInput);
  return `${slug(endpoint)}${SEP}${relSlug}${labelSlug}${SEP}${digest}.json`;
}

export function relToWorkspace(workspace: string, target: string): string {
  if (!path.isAbsolute(target)) {
    return target.replace(/\\/g, "/");
  }
  try {
    const rel = path.relative(workspace, target);
    return rel.replace(/\\/g, "/");
  } catch {
    return target.replace(/\\/g, "/");
  }
}

// ============================================================================
// Position and identifier extraction helpers
// ============================================================================

export function pickPosition(
  entry: Record<string, any>,
): [number, number] | null {
  const candidates: Array<Record<string, any>> = [];
  if (entry?.position) candidates.push(entry.position);
  const range = entry?.range?.start;
  if (range) candidates.push(range);
  const identifierPosition = entry?.identifier_position?.position;
  if (identifierPosition) candidates.push(identifierPosition);
  const fileRange = entry?.file_range?.range?.start;
  if (fileRange) candidates.push(fileRange);

  for (const pos of candidates) {
    const line = Number(pos?.line);
    const character = Number(pos?.character);
    if (Number.isInteger(line) && Number.isInteger(character)) {
      return [line, character];
    }
  }
  return null;
}

export function pickIdentifier(entry: Record<string, any>): string | null {
  if (typeof entry?.name === "string" && entry.name) return entry.name;
  if (typeof entry?.identifier === "string" && entry.identifier)
    return entry.identifier;
  const symName = entry?.symbol?.name;
  if (typeof symName === "string" && symName) return symName;
  return null;
}

// ============================================================================
// Schema-based fixture validation and normalization
// ============================================================================

function upgradeContextSnippets(snippets: unknown): unknown {
  if (!Array.isArray(snippets)) {
    return snippets;
  }

  return snippets.map((snippet) => {
    if (!snippet || typeof snippet !== "object") {
      return snippet;
    }

    const record = snippet as Record<string, unknown>;
    if ("file_range" in record || !("range" in record)) {
      return record;
    }

    const range = record.range;
    if (!range || typeof range !== "object") {
      return record;
    }

    const { range: _, ...rest } = record;
    return {
      ...rest,
      file_range: range,
    };
  });
}

function upgradePayloadForSchema(
  endpoint: EndpointName,
  data: unknown,
): unknown {
  if (!data || typeof data !== "object") {
    return data;
  }

  const base = data as Record<string, unknown>;

  switch (endpoint) {
    case "find-definition": {
      const copy: Record<string, unknown> = { ...base };
      if (Array.isArray(copy.source_code_context)) {
        copy.source_code_context = upgradeContextSnippets(
          copy.source_code_context,
        );
      }
      if (
        !("source_code_context" in copy) ||
        copy.source_code_context === undefined
      ) {
        copy.source_code_context = null;
      }
      return copy;
    }
    case "find-references": {
      const copy: Record<string, unknown> = { ...base };
      if (Array.isArray(copy.context)) {
        copy.context = upgradeContextSnippets(copy.context);
      }
      if (!("context" in copy) || copy.context === undefined) {
        copy.context = null;
      }
      return copy;
    }
    default:
      return data;
  }
}

function normalizeWithSchema(endpoint: EndpointName, data: unknown): unknown {
  const schema = ENDPOINT_SCHEMAS[endpoint];

  const upgraded = upgradePayloadForSchema(endpoint, data);

  // First validate that the data conforms to the expected schema
  const parseResult = schema.safeParse(upgraded);
  if (!parseResult.success) {
    throw new Error(
      `Schema validation failed for endpoint ${endpoint}: ${parseResult.error.message}`,
    );
  }

  const validData = parseResult.data;

  // Apply endpoint-specific normalization and sorting
  switch (endpoint) {
    case "list-files":
      // Sort file paths alphabetically
      return Array.isArray(validData) ? validData.slice().sort() : validData;

    case "read-source":
      // No special normalization needed for read-source, just return the validated data
      return validData;

    case "definitions-in-file":
      // Sort definitions by file path, line, character, then name
      return Array.isArray(validData)
        ? validData.slice().sort((a, b) => {
            const aPath = a.file_range?.path || "";
            const bPath = b.file_range?.path || "";
            if (aPath !== bPath) return aPath.localeCompare(bPath);

            const aLine = a.identifier_position?.position?.line || 0;
            const bLine = b.identifier_position?.position?.line || 0;
            if (aLine !== bLine) return aLine - bLine;

            const aChar = a.identifier_position?.position?.character || 0;
            const bChar = b.identifier_position?.position?.character || 0;
            if (aChar !== bChar) return aChar - bChar;

            return (a.name || "").localeCompare(b.name || "");
          })
        : validData;

    case "find-definition": {
      // Sort definitions by path, line, character
      const sorted = { ...validData };
      if (Array.isArray(sorted.definitions)) {
        sorted.definitions = sorted.definitions.slice().sort((a, b) => {
          if (a.path !== b.path) return a.path.localeCompare(b.path);
          if (a.position.line !== b.position.line)
            return a.position.line - b.position.line;
          return a.position.character - b.position.character;
        });
      }
      if (Array.isArray(sorted.source_code_context)) {
        sorted.source_code_context = sorted.source_code_context
          .slice()
          .sort((a, b) =>
            (a.file_range?.path || "").localeCompare(b.file_range?.path || ""),
          );
      }
      // Remove raw_response as it's unstable
      delete (sorted as any).raw_response;
      return sorted;
    }

    case "find-identifier":
    case "find-identifier-narrowed": {
      // Sort identifiers by file path, line, character, then name
      const sortedIdents = { ...validData };
      if (Array.isArray(sortedIdents.identifiers)) {
        sortedIdents.identifiers = sortedIdents.identifiers
          .slice()
          .sort((a, b) => {
            const aPath = a.file_range?.path || "";
            const bPath = b.file_range?.path || "";
            if (aPath !== bPath) return aPath.localeCompare(bPath);

            const aLine = a.file_range?.range?.start?.line || 0;
            const bLine = b.file_range?.range?.start?.line || 0;
            if (aLine !== bLine) return aLine - bLine;

            const aChar = a.file_range?.range?.start?.character || 0;
            const bChar = b.file_range?.range?.start?.character || 0;
            if (aChar !== bChar) return aChar - bChar;

            return (a.name || "").localeCompare(b.name || "");
          });
      }
      return sortedIdents;
    }

    case "find-referenced-symbols": {
      // Sort all symbol arrays consistently
      const sortedRefs = { ...validData };
      const sortSymbols = (symbols: any[]) =>
        symbols.slice().sort((a, b) => {
          const aPath = a.file_range?.path || "";
          const bPath = b.file_range?.path || "";
          if (aPath !== bPath) return aPath.localeCompare(bPath);

          const aLine = a.file_range?.range?.start?.line || 0;
          const bLine = b.file_range?.range?.start?.line || 0;
          if (aLine !== bLine) return aLine - bLine;

          const aChar = a.file_range?.range?.start?.character || 0;
          const bChar = b.file_range?.range?.start?.character || 0;
          if (aChar !== bChar) return aChar - bChar;

          const aName = a.name || "";
          const bName = b.name || "";
          if (aName !== bName) return aName.localeCompare(bName);

          return (a.kind || "").localeCompare(b.kind || "");
        });

      if (Array.isArray(sortedRefs.external_symbols)) {
        sortedRefs.external_symbols = sortSymbols(sortedRefs.external_symbols);
      }
      if (Array.isArray(sortedRefs.not_found)) {
        sortedRefs.not_found = sortSymbols(sortedRefs.not_found);
      }
      if (Array.isArray(sortedRefs.workspace_symbols)) {
        sortedRefs.workspace_symbols = sortedRefs.workspace_symbols
          .slice()
          .sort((a, b) =>
            (a.reference?.name || "").localeCompare(b.reference?.name || ""),
          );
      }
      return sortedRefs;
    }

    case "find-references": {
      // Sort references and context
      const sortedRefsResult = { ...validData };
      if (Array.isArray(sortedRefsResult.references)) {
        sortedRefsResult.references = sortedRefsResult.references
          .slice()
          .sort((a, b) => {
            if (a.path !== b.path) return a.path.localeCompare(b.path);
            if (a.position.line !== b.position.line)
              return a.position.line - b.position.line;
            return a.position.character - b.position.character;
          });
      }
      if (Array.isArray(sortedRefsResult.context)) {
        sortedRefsResult.context = sortedRefsResult.context
          .slice()
          .sort((a, b) =>
            (a.file_range?.path || "").localeCompare(b.file_range?.path || ""),
          );
      }
      // Remove raw_response as it's unstable
      delete (sortedRefsResult as any).raw_response;
      return sortedRefsResult;
    }

    default:
      return validData;
  }
}

// ============================================================================
// Fixture file paths
// ============================================================================

function fixtureWorkspaceFor(runnerWorkspace: string): string {
  const workspaceName = path.basename(runnerWorkspace);
  const testsRoot = path.resolve(runnerWorkspace, "..", "..");
  return path.join(testsRoot, "fixtures", workspaceName);
}

function schemaFixturesDirFor(fixtureWorkspace: string): string {
  const dir = path.join(fixtureWorkspace, "schema-fixtures");
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

function schemaFixturePath(
  meta: FixtureMeta & { fixtureWorkspace: string },
): string {
  const dir = schemaFixturesDirFor(meta.fixtureWorkspace);
  return path.join(dir, buildFixtureFilename(meta));
}

// ============================================================================
// Schema fixture assertion (new approach)
// ============================================================================

export interface SchemaFixtureOptions {
  runnerWorkspace: string;
  endpoint: EndpointName;
  sourcePath: string;
  params: Record<string, unknown>;
  payload: unknown;
  label?: string | null;
  allowBasenameFallback?: boolean;
}

export function assertMatchesSchemaFixture(
  options: SchemaFixtureOptions,
): void {
  const {
    runnerWorkspace,
    endpoint,
    sourcePath,
    params,
    payload,
    label,
    allowBasenameFallback = false,
  } = options;

  const fixtureWorkspace = fixtureWorkspaceFor(runnerWorkspace);

  const rel = relToWorkspace(runnerWorkspace, sourcePath);
  if (isGeneratedPath(rel)) {
    return;
  }

  const meta: FixtureMeta = {
    endpoint,
    sourcePath: rel,
    params,
    label: label ?? undefined,
  };

  // Normalize the payload using the schema
  const normalizedPayload = normalizeWithSchema(endpoint, payload);

  if (RECORD_FIXTURES) {
    const pathToWrite = schemaFixturePath({
      ...meta,
      fixtureWorkspace,
    });
    fs.mkdirSync(path.dirname(pathToWrite), { recursive: true });
    fs.writeFileSync(
      pathToWrite,
      JSON.stringify(normalizedPayload, null, 2) + "\n",
      "utf8",
    );
    return;
  }

  const candidates = [rel];
  const base = path.basename(rel);
  if (allowBasenameFallback && base !== rel) {
    candidates.push(base);
  }

  let expectedPath: string | null = null;
  let expectedValue: unknown = undefined;
  let readError: Error | null = null;

  for (const candidate of candidates) {
    try {
      const targetPath = schemaFixturePath({
        ...meta,
        sourcePath: candidate,
        fixtureWorkspace,
      });
      const raw = fs.readFileSync(targetPath, "utf8");
      expectedValue = JSON.parse(raw);
      expectedPath = targetPath;
      break;
    } catch (error) {
      readError = error as Error;
      expectedPath = null;
    }
  }

  if (expectedPath == null) {
    const tried = candidates
      .map((candidate) =>
        schemaFixturePath({
          ...meta,
          sourcePath: candidate,
          fixtureWorkspace,
        }),
      )
      .map((candidatePath) => `  tried: ${candidatePath}`)
      .join("\n");
    const message = [
      `Missing schema fixture for ${endpoint}`,
      `  workspace: ${runnerWorkspace}`,
      `  source:    ${sourcePath}`,
      tried,
    ].join("\n");
    throw new Error(message, { cause: readError ?? undefined });
  }

  // Normalize the expected value as well
  const normalizedExpected = normalizeWithSchema(endpoint, expectedValue);

  if (
    JSON.stringify(normalizedPayload) === JSON.stringify(normalizedExpected)
  ) {
    return;
  }

  // Generate a helpful diff message
  const actualStr = JSON.stringify(normalizedPayload, null, 2);
  const expectedStr = JSON.stringify(normalizedExpected, null, 2);

  const message = [
    `${endpoint} schema fixture mismatch (fixture: ${expectedPath})`,
    "Expected (normalized):",
    expectedStr,
    "Actual (normalized):",
    actualStr,
  ].join("\n");

  throw new Error(message);
}

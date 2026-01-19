import { z } from "zod";

// ---- Named types utility ----------------------------------------------------
// This provides a 0-cost way to "brand" types derived from Zod schemas
// with a unique name. This nudges the TypeScript compiler to use the provided
// name in error messages and automcompletion. Otherwise, inferring types
// from Zod schemas alone results in anonymous types that capture the JSON
// structure but lose the exported type name.
declare const __name__: unique symbol;
type Named<S extends z.ZodTypeAny, N extends string> = z.output<S> & {
  readonly [__name__]?: N;
};

// ---- General Result type ----------------------------------------------------
export type Result<T, E> = Ok<T> | Err<E>;
export interface Err<E> {
  ok: false;
  data: E;
}
export interface Ok<T> {
  ok: true;
  data: T;
}

export const ok = <T>(data: T): Ok<T> => ({ ok: true, data });
export const err = <E>(data: E): Err<E> => ({ ok: false, data });

export const isOk = <T, E>(r: Result<T, E>): r is Ok<T> => r.ok;
export const isErr = <T, E>(r: Result<T, E>): r is Err<E> => !r.ok;

// ---- Docker Lifecycle -------------------------------------------------------
export const DockerErrSchema = z.object({
  error_code: z.number(),
  message: z.string(),
  stdout: z.string(),
  stderr: z.string(),
});
export type DockerErr = Named<typeof DockerErrSchema, "DockerErr">;
export type DockerResult<T> = Result<T, DockerErr>;

export const DownResultSchema = z.object({
  stdout: z.string(),
});
export type DownResult = Named<typeof DownResultSchema, "DownResult">;

export const UpResultSchema = z.object({
  host_port: z.number(),
  base_url: z.string(),
});
export type UpResult = Named<typeof UpResultSchema, "UpResult">;

export const RunResultSchema = z.object({
  stdout: z.string(),
});
export type RunResult = Named<typeof RunResultSchema, "RunResult">;

export const StatusResultSchema = z.object({
  container_name: z.string(),
  container_status: z.string(),
});
export type StatusResult = Named<typeof StatusResultSchema, "StatusResult">;

export const LogsResultSchema = z.object({
  stdout: z.string(),
});
export type LogsResult = Named<typeof LogsResultSchema, "LogsResult">;

export const PullResultSchema = z.object({
  image: z.string(),
  stdout: z.string(),
});
export type PullResult = Named<typeof PullResultSchema, "PullResult">;

// ---- General HttpResult types -----------------------------------------------
export const HttpErrSchema = z.object({
  status_code: z.number().nullable(),
  error: z.record(z.any()),
});
export type HttpErr = Named<typeof HttpErrSchema, "HttpErr">;
export type HttpResult<T> = Result<T, HttpErr>;

// ---- Health (data-plane /v1/system/health) ----------------------------------
export const HealthResultSchema = z.object({
  status: z.union([z.literal("ok"), z.literal("not ok")]),
  version: z.string().optional(),
  languages: z.record(z.boolean()).optional(),
});
export type HealthResult = Named<typeof HealthResultSchema, "HealthResult">;

// ---- Common shapes used across symbol/workspace endpoints -------------------
export const LspPositionSchema = z.object({
  line: z.number().int().min(0),
  character: z.number().int().min(0),
});
export type LspPosition = Named<typeof LspPositionSchema, "LspPosition">;

export const LspRangeSchema = z.object({
  start: LspPositionSchema,
  end: LspPositionSchema,
});
export type LspRange = Named<typeof LspRangeSchema, "LspRange">;

export const FilePositionSchema = z.object({
  path: z.string(),
  position: LspPositionSchema,
});
export type FilePosition = Named<typeof FilePositionSchema, "FilePosition">;

export const FileRangeSchema = z.object({
  path: z.string(),
  range: LspRangeSchema,
});
export type FileRange = Named<typeof FileRangeSchema, "FileRange">;

export const IdentifierPositionSchema = z.object({
  path: z.string(),
  position: LspPositionSchema,
});
export type IdentifierPosition = Named<
  typeof IdentifierPositionSchema,
  "IdentifierPosition"
>;

export const SelectedIdentifierSchema = z.object({
  file_range: FileRangeSchema,
  kind: z.string().nullable(),
  name: z.string(),
});
export type SelectedIdentifier = Named<
  typeof SelectedIdentifierSchema,
  "SelectedIdentifier"
>;

// ---- Workspace --------------------------------------------------------------
export const ListFilesResultSchema = z.array(z.string());
export type ListFilesResult = Named<
  typeof ListFilesResultSchema,
  "ListFilesResult"
>;

export const ReadSourceResultSchema = z.object({
  source_code: z.string(),
});
export type ReadSourceResult = Named<
  typeof ReadSourceResultSchema,
  "ReadSourceResult"
>;

// ---- Symbols ---------------------------------------------------------------
export const DefinitionLocationSchema = z.object({
  path: z.string(),
  position: LspPositionSchema,
});
export type DefinitionLocation = Named<
  typeof DefinitionLocationSchema,
  "DefinitionLocation"
>;

export const ContextSnippetSchema = z
  .object({
    file_range: FileRangeSchema.optional(),
    range: FileRangeSchema.optional(),
    source_code: z.string(),
  })
  .superRefine((data, ctx) => {
    if (!data.file_range && !data.range) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "one of file_range or range is required",
      });
    }
  });
export type ContextSnippet = Named<
  typeof ContextSnippetSchema,
  "ContextSnippet"
>;

export const DefinitionInFileSchema = z.object({
  file_range: FileRangeSchema,
  identifier_position: IdentifierPositionSchema,
  kind: z.string(),
  name: z.string(),
});
export type DefinitionInFile = Named<
  typeof DefinitionInFileSchema,
  "DefinitionInFile"
>;

export const DefinitionsInFileResultSchema = z.array(DefinitionInFileSchema);
export type DefinitionsInFileResult = Named<
  typeof DefinitionsInFileResultSchema,
  "DefinitionsInFileResult"
>;

// ---------- FindDefinitionResult types ---------------------------------------
export const FindDefinitionResultSchema = z.object({
  definitions: z.array(DefinitionLocationSchema),
  selected_identifier: SelectedIdentifierSchema,
  raw_response: z.unknown(),
  source_code_context: z.array(ContextSnippetSchema).nullable(),
});
export type FindDefinitionResult = Named<
  typeof FindDefinitionResultSchema,
  "FindDefinitionResult"
>;

// ---------- FindIdentifierResult types ---------------------------------------
export const IdentifierSchema = z.object({
  file_range: FileRangeSchema,
  kind: z.string().nullable(),
  name: z.string(),
});
export type Identifier = Named<typeof IdentifierSchema, "Identifier">;

export const FindIdentifierResultSchema = z.object({
  identifiers: z.array(IdentifierSchema),
});
export type FindIdentifierResult = Named<
  typeof FindIdentifierResultSchema,
  "FindIdentifierResult"
>;

// ---------- FindReferencedSymbolsResult types --------------------------------
export const ExternalSymbolSchema = z.object({
  file_range: FileRangeSchema,
  kind: z.string().nullable(),
  name: z.string(),
});
export type ExternalSymbol = Named<
  typeof ExternalSymbolSchema,
  "ExternalSymbol"
>;

export const NotFoundSymbolSchema = ExternalSymbolSchema;
export type NotFoundSymbol = Named<
  typeof NotFoundSymbolSchema,
  "NotFoundSymbol"
>;

export const WorkspaceDefinitionSchema = z.object({
  file_range: FileRangeSchema,
  identifier_position: IdentifierPositionSchema,
  kind: z.string().nullable(),
  name: z.string(),
});
export type WorkspaceDefinition = Named<
  typeof WorkspaceDefinitionSchema,
  "WorkspaceDefinition"
>;

export const WorkspaceReferenceSchema = z.object({
  file_range: FileRangeSchema,
  kind: z.string().nullable(),
  name: z.string(),
});
export type WorkspaceReference = Named<
  typeof WorkspaceReferenceSchema,
  "WorkspaceReference"
>;

export const WorkspaceSymbolSchema = z.object({
  reference: WorkspaceReferenceSchema,
  definitions: z.array(WorkspaceDefinitionSchema),
});
export type WorkspaceSymbol = Named<
  typeof WorkspaceSymbolSchema,
  "WorkspaceSymbol"
>;

export const FindReferencedSymbolsResultSchema = z.object({
  external_symbols: z.array(ExternalSymbolSchema),
  not_found: z.array(NotFoundSymbolSchema),
  workspace_symbols: z.array(WorkspaceSymbolSchema),
});
export type FindReferencedSymbolsResult = Named<
  typeof FindReferencedSymbolsResultSchema,
  "FindReferencedSymbolsResult"
>;

// ---------- FindReferencesResult types ---------------------------------------
export const ReferenceLocationSchema = z.object({
  path: z.string(),
  position: LspPositionSchema,
});
export type ReferenceLocation = Named<
  typeof ReferenceLocationSchema,
  "ReferenceLocation"
>;

export const FindReferencesResultSchema = z.object({
  references: z.array(ReferenceLocationSchema),
  selected_identifier: SelectedIdentifierSchema,
  context: z.array(ContextSnippetSchema).nullable(),
  raw_response: z.unknown(),
});
export type FindReferencesResult = Named<
  typeof FindReferencesResultSchema,
  "FindReferencesResult"
>;

// ---- JSON-RPC types ---------------------------------------------------------

export interface JsonRpcMessage {
  jsonrpc: "2.0";
  id?: string | number | null;
  method?: string;
  params?: any;
  result?: any;
  error?: JsonRpcError;
}

export interface JsonRpcError {
  code: JsonRpcErrorCode;
  message: string;
  data?: any;
}

export enum JsonRpcErrorCode {
  ParseError = -32700,
  InvalidRequest = -32600,
  MethodNotFound = -32601,
  InvalidParams = -32602,
  InternalError = -32603,
}

// ---- CLI Command Options ---------------------------------------------------
export const BaseCommandOptionsSchema = z.object({
  json: z.boolean().optional(),
});
export type BaseCommandOptions = Named<
  typeof BaseCommandOptionsSchema,
  "BaseCommandOptions"
>;

export const UpCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  containerName: z.string().optional(),
  hostPort: z.number().int().min(0).optional(),
  containerRegistry: z.string().optional(),
  languageImageVersion: z.string().optional(),
  serviceImageVersion: z.string().optional(),
  timeout: z.number().optional(),
  sudo: z.boolean().optional(),
  stream: z.boolean().optional(),
  ro: z.boolean().optional(),
  bindHost: z.string().optional(),
  debug: z.boolean().optional(),
  env: z.array(z.string()).optional(),
  envFile: z.string().optional(),
});
export type UpCommandOptions = Named<
  typeof UpCommandOptionsSchema,
  "UpCommandOptions"
>;

export const DownCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  containerName: z.string().optional(),
  sudo: z.boolean().optional(),
  timeout: z.number().optional(),
});
export type DownCommandOptions = Named<
  typeof DownCommandOptionsSchema,
  "DownCommandOptions"
>;

export const LogsCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  containerName: z.string().optional(),
  timeout: z.number().optional(),
  sudo: z.boolean().optional(),
  stream: z.boolean().optional(),
  since: z.string().optional(),
  tail: z.union([z.number().int().min(0), z.literal("all")]).optional(),
});
export type LogsCommandOptions = Named<
  typeof LogsCommandOptionsSchema,
  "LogsCommandOptions"
>;

export const RunCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  containerName: z.string().optional(),
  sudo: z.boolean().optional(),
  stream: z.boolean().optional(),
  timeout: z.number().optional(),
  env: z.array(z.string()).optional(),
  envFile: z.string().optional(),
});
export type RunCommandOptions = Named<
  typeof RunCommandOptionsSchema,
  "RunCommandOptions"
>;

export const StatusCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  containerName: z.string().optional(),
  sudo: z.boolean().optional(),
  timeout: z.number().optional(),
});
export type StatusCommandOptions = Named<
  typeof StatusCommandOptionsSchema,
  "StatusCommandOptions"
>;

export const PullCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  containerRegistry: z.string().optional(),
  allLanguages: z.boolean().optional(),
  allServices: z.boolean().optional(),
  languageImageVersion: z.string().optional(),
  languages: z.string().optional(),
  serviceImageVersion: z.string().optional(),
  services: z.string().optional(),
  stream: z.boolean().optional(),
  sudo: z.boolean().optional(),
});
export type PullCommandOptions = Named<
  typeof PullCommandOptionsSchema,
  "PullCommandOptions"
>;

export const LspCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  lspUrl: z.string().optional(),
  lspPort: z.number().int().min(0).optional(),
  timeout: z.number().optional(),
});
export type LspCommandOptions = Named<
  typeof LspCommandOptionsSchema,
  "LspCommandOptions"
>;

export const FindDefinitionOptionsSchema = LspCommandOptionsSchema.extend({
  includeRawResponse: z.boolean().optional(),
  includeSourceCode: z.boolean().optional(),
});
export type FindDefinitionOptions = Named<
  typeof FindDefinitionOptionsSchema,
  "FindDefinitionOptions"
>;

export const ReadSourceCommandOptionsSchema = LspCommandOptionsSchema.extend({
  range: z.lazy(() => LspRangeSchema).nullish(), // keep laziness safe for type references
});
export type ReadSourceCommandOptions = Named<
  typeof ReadSourceCommandOptionsSchema,
  "ReadSourceCommandOptions"
>;

export const HealthCommandOptionsSchema = LspCommandOptionsSchema.extend({});
export type HealthCommandOptions = Named<
  typeof HealthCommandOptionsSchema,
  "HealthCommandOptions"
>;

export const FindIdentifierOptionsSchema = LspCommandOptionsSchema.extend({
  position: z.string().optional(),
});
export type FindIdentifierOptions = Named<
  typeof FindIdentifierOptionsSchema,
  "FindIdentifierOptions"
>;

export const FindReferencedSymbolsOptionsSchema =
  LspCommandOptionsSchema.extend({
    fullScan: z.boolean().optional(),
  });
export type FindReferencedSymbolsOptions = Named<
  typeof FindReferencedSymbolsOptionsSchema,
  "FindReferencedSymbolsOptions"
>;

export const FindReferencesOptionsSchema = LspCommandOptionsSchema.extend({
  includeCodeContextLines: z.number().int().min(0).optional(),
  includeRawResponse: z.boolean().optional(),
});
export type FindReferencesOptions = Named<
  typeof FindReferencesOptionsSchema,
  "FindReferencesOptions"
>;

export const ServerCommandOptionsSchema = BaseCommandOptionsSchema.extend({
  hostPort: z.number().int().min(0).optional(),
  containerRegistry: z.string().optional(),
  languageImageVersion: z.string().optional(),
  serviceImageVersion: z.string().optional(),
  timeout: z.number().optional(),
  sudo: z.boolean().optional(),
  ro: z.boolean().optional(),
  bindHost: z.string().optional(),
  debug: z.boolean().optional(),
  env: z.array(z.string()).optional(),
  envFile: z.string().optional(),
  shared: z.boolean().optional(),
  sharedMode: z
    .union([z.literal("up"), z.literal("down"), z.literal("use")])
    .optional(),
});
export type ServerCommandOptions = Named<
  typeof ServerCommandOptionsSchema,
  "ServerCommandOptions"
>;

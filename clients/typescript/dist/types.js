import { z } from "zod";
export const ok = (data) => ({ ok: true, data });
export const err = (data) => ({ ok: false, data });
export const isOk = (r) => r.ok;
export const isErr = (r) => !r.ok;
// ---- Docker Lifecycle -------------------------------------------------------
export const DockerErrSchema = z.object({
    error_code: z.number(),
    message: z.string(),
    stdout: z.string(),
    stderr: z.string(),
});
export const DownResultSchema = z.object({
    stdout: z.string(),
});
export const UpResultSchema = z.object({
    host_port: z.number(),
    base_url: z.string(),
});
export const RunResultSchema = z.object({
    stdout: z.string(),
});
export const StatusResultSchema = z.object({
    container_name: z.string(),
    container_status: z.string(),
});
export const LogsResultSchema = z.object({
    stdout: z.string(),
});
export const PullResultSchema = z.object({
    image: z.string(),
    stdout: z.string(),
});
// ---- General HttpResult types -----------------------------------------------
export const HttpErrSchema = z.object({
    status_code: z.number().nullable(),
    error: z.record(z.any()),
});
// ---- Health (data-plane /v1/system/health) ----------------------------------
export const HealthResultSchema = z.object({
    status: z.union([z.literal("ok"), z.literal("not ok")]),
    version: z.string().optional(),
    languages: z.record(z.boolean()).optional(),
});
// ---- Common shapes used across symbol/workspace endpoints -------------------
export const LspPositionSchema = z.object({
    line: z.number().int().min(0),
    character: z.number().int().min(0),
});
export const LspRangeSchema = z.object({
    start: LspPositionSchema,
    end: LspPositionSchema,
});
export const FilePositionSchema = z.object({
    path: z.string(),
    position: LspPositionSchema,
});
export const FileRangeSchema = z.object({
    path: z.string(),
    range: LspRangeSchema,
});
export const IdentifierPositionSchema = z.object({
    path: z.string(),
    position: LspPositionSchema,
});
export const SelectedIdentifierSchema = z.object({
    file_range: FileRangeSchema,
    kind: z.string().nullable(),
    name: z.string(),
});
// ---- Workspace --------------------------------------------------------------
export const ListFilesResultSchema = z.array(z.string());
export const ReadSourceResultSchema = z.object({
    source_code: z.string(),
});
// ---- Symbols ---------------------------------------------------------------
export const DefinitionLocationSchema = z.object({
    path: z.string(),
    position: LspPositionSchema,
});
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
export const DefinitionInFileSchema = z.object({
    file_range: FileRangeSchema,
    identifier_position: IdentifierPositionSchema,
    kind: z.string(),
    name: z.string(),
});
export const DefinitionsInFileResultSchema = z.array(DefinitionInFileSchema);
// ---------- FindDefinitionResult types ---------------------------------------
export const FindDefinitionResultSchema = z.object({
    definitions: z.array(DefinitionLocationSchema),
    selected_identifier: SelectedIdentifierSchema,
    raw_response: z.unknown(),
    source_code_context: z.array(ContextSnippetSchema).nullable(),
});
// ---------- FindIdentifierResult types ---------------------------------------
export const IdentifierSchema = z.object({
    file_range: FileRangeSchema,
    kind: z.string().nullable(),
    name: z.string(),
});
export const FindIdentifierResultSchema = z.object({
    identifiers: z.array(IdentifierSchema),
});
// ---------- FindReferencedSymbolsResult types --------------------------------
export const ExternalSymbolSchema = z.object({
    file_range: FileRangeSchema,
    kind: z.string().nullable(),
    name: z.string(),
});
export const NotFoundSymbolSchema = ExternalSymbolSchema;
export const WorkspaceDefinitionSchema = z.object({
    file_range: FileRangeSchema,
    identifier_position: IdentifierPositionSchema,
    kind: z.string().nullable(),
    name: z.string(),
});
export const WorkspaceReferenceSchema = z.object({
    file_range: FileRangeSchema,
    kind: z.string().nullable(),
    name: z.string(),
});
export const WorkspaceSymbolSchema = z.object({
    reference: WorkspaceReferenceSchema,
    definitions: z.array(WorkspaceDefinitionSchema),
});
export const FindReferencedSymbolsResultSchema = z.object({
    external_symbols: z.array(ExternalSymbolSchema),
    not_found: z.array(NotFoundSymbolSchema),
    workspace_symbols: z.array(WorkspaceSymbolSchema),
});
// ---------- FindReferencesResult types ---------------------------------------
export const ReferenceLocationSchema = z.object({
    path: z.string(),
    position: LspPositionSchema,
});
export const FindReferencesResultSchema = z.object({
    references: z.array(ReferenceLocationSchema),
    selected_identifier: SelectedIdentifierSchema,
    context: z.array(ContextSnippetSchema).nullable(),
    raw_response: z.unknown(),
});
// ---- CLI Command Options ---------------------------------------------------
export const BaseCommandOptionsSchema = z.object({
    json: z.boolean().optional(),
});
export const UpCommandOptionsSchema = BaseCommandOptionsSchema.extend({
    containerName: z.string().optional(),
    hostPort: z.number().int().min(0).optional(),
    languageContainerVersion: z.string().optional(),
    proxyImage: z.string().optional(),
    wrapperImage: z.string().optional(),
    watchdogImage: z.string().optional(),
    timeout: z.number().optional(),
    sudo: z.boolean().optional(),
    stream: z.boolean().optional(),
    ro: z.boolean().optional(),
    bindHost: z.string().optional(),
    debug: z.boolean().optional(),
    env: z.array(z.string()).optional(),
    envFile: z.string().optional(),
});
export const DownCommandOptionsSchema = BaseCommandOptionsSchema.extend({
    containerName: z.string().optional(),
    sudo: z.boolean().optional(),
    timeout: z.number().optional(),
});
export const LogsCommandOptionsSchema = BaseCommandOptionsSchema.extend({
    containerName: z.string().optional(),
    timeout: z.number().optional(),
    sudo: z.boolean().optional(),
    stream: z.boolean().optional(),
    since: z.string().optional(),
    tail: z.union([z.number().int().min(0), z.literal("all")]).optional(),
});
export const RunCommandOptionsSchema = BaseCommandOptionsSchema.extend({
    containerName: z.string().optional(),
    sudo: z.boolean().optional(),
    stream: z.boolean().optional(),
    timeout: z.number().optional(),
    env: z.array(z.string()).optional(),
    envFile: z.string().optional(),
});
export const StatusCommandOptionsSchema = BaseCommandOptionsSchema.extend({
    containerName: z.string().optional(),
    sudo: z.boolean().optional(),
    timeout: z.number().optional(),
});
export const PullCommandOptionsSchema = BaseCommandOptionsSchema.extend({
    image: z.string().optional(),
    stream: z.boolean().optional(),
    sudo: z.boolean().optional(),
});
export const LspCommandOptionsSchema = BaseCommandOptionsSchema.extend({
    lspUrl: z.string().optional(),
    lspPort: z.number().int().min(0).optional(),
    timeout: z.number().optional(),
});
export const FindDefinitionOptionsSchema = LspCommandOptionsSchema.extend({
    includeRawResponse: z.boolean().optional(),
    includeSourceCode: z.boolean().optional(),
});
export const ReadSourceCommandOptionsSchema = LspCommandOptionsSchema.extend({
    range: z.lazy(() => LspRangeSchema).nullish(), // keep laziness safe for type references
});
export const HealthCommandOptionsSchema = LspCommandOptionsSchema.extend({});
export const FindIdentifierOptionsSchema = LspCommandOptionsSchema.extend({
    position: z.string().optional(),
});
export const FindReferencedSymbolsOptionsSchema = LspCommandOptionsSchema.extend({
    fullScan: z.boolean().optional(),
});
export const FindReferencesOptionsSchema = LspCommandOptionsSchema.extend({
    includeCodeContextLines: z.number().int().min(0).optional(),
    includeRawResponse: z.boolean().optional(),
});

import { ExecOptions, Instance, InstanceBootOptions, InstanceListOptions, InstanceStartOptions, InstanceStatus, MorphCloudClient, Snapshot } from 'morphcloud';

export const SOURCE_ARCHIVE = "nuanced-lsp-src.zip";
export const SOURCE_DIR = "nuanced-lsp-src";

export const BASE_DIGEST = "nuanced-lsp-base";
export const BUILDER_DIGEST = "nuanced-lsp-builder";
export const SOURCE_DIGEST = "nuanced-lsp-source";
export const SERVICE_DIGEST = "nuanced-lsp-service";

export const LABEL_NUANCED_ROLE = "nuanced.role"

export const NUANCED_ROLE_BASE = "base"
export const NUANCED_ROLE_BUILDER = "builder"
export const NUANCED_ROLE_SOURCE = "source"
export const NUANCED_ROLE_SERVICE = "service"

export const NUANCED_ROLE_WORKSPACE = "workspace"

export const LABEL_NUANCED_WORKSPACE_DIGEST = "nuanced.workspace.digest"

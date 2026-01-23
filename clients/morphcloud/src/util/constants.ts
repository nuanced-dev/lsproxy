import nodeMachineId from "node-machine-id";

export const NUANCED_LSP_VERSION = "@^0.5";

export const MACHINE_ID = nodeMachineId.machineIdSync();

export const LABEL_NUANCED_LSP_ROLE = "nuanced-lsp.role";
export const NUANCED_LSP_ROLE_SERVICE = "service";
export const NUANCED_LSP_ROLE_WORKSPACE = "workspace";

export const LABEL_NUANCED_LSP_WORKSPACE_HOSTNAME =
  "nuanced-lsp.workspace.hostname";

export const LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID =
  "nuanced-lsp.workspace.machine_id";

export const LABEL_NUANCED_LSP_WORKSPACE_PATH = "nuanced-lsp.workspace.path";

import { realpathSync } from "fs";
import { Instance, MorphCloudClient } from "morphcloud";
import { machineIdSync } from "node-machine-id";
import {
  LABEL_NUANCED_LSP_ROLE,
  LABEL_NUANCED_LSP_WORKSPACE_PATH,
  LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID,
  NUANCED_LSP_ROLE_WORKSPACE,
} from "../util/constants";
import { findInstance } from "../util/morphcloud";

const MACHINE_ID = machineIdSync();

export async function workspaceStatus(workspaceDir: string) {
  const client = new MorphCloudClient();

  const workspaceRealDir = realpathSync(workspaceDir);
  const metadata = {
    [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_WORKSPACE,
    [LABEL_NUANCED_LSP_WORKSPACE_PATH]: workspaceRealDir,
    [LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID]: MACHINE_ID,
  };

  let workspaceInstance: Instance | undefined;
  try {
    workspaceInstance = await findInstance(client, { metadata });
    console.log(
      `${workspaceRealDir.padEnd(2)} : ${workspaceInstance?.id ?? "<missing>"}`,
    );
  } catch (error) {
    throw new Error(`Workspace status failed: ${error}`);
  }
}

import { realpathSync } from "fs";
import { Instance, MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_LSP_ROLE,
  LABEL_NUANCED_LSP_WORKSPACE_PATH,
  LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID,
  NUANCED_LSP_ROLE_WORKSPACE,
  MACHINE_ID,
} from "../util/constants.js";
import { findInstance } from "../util/morphcloud.js";
import { getSshConfig } from "../util/ssh.js";

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
    if (workspaceInstance) {
      // refreshed the SSH config--can be useful when manually debugging Mutagen issues
      await getSshConfig(workspaceInstance);
    }
  } catch (error) {
    throw new Error(`Workspace status failed: ${error}`);
  }
}

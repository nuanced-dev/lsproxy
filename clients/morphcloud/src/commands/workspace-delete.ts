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
import { MutagenClient } from "../util/mutagen";

const MACHINE_ID = machineIdSync();

export async function workspaceDelete(workspaceDir: string) {
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

    if (workspaceInstance) {
      const mutagen = new MutagenClient(workspaceInstance);
      await mutagen.stopSync().catch((e) => {
        console.error(`Error: Cannot remove sync ${workspaceRealDir}: ${e}`);
      });
      await workspaceInstance.stop().catch((e) => {
        console.error(
          `Error: Cannot stop instance ${workspaceInstance!.id}: ${e}`,
        );
      });
    }
  } catch (error) {
    throw new Error(`Workspace stop failed: ${error}`);
  }
}

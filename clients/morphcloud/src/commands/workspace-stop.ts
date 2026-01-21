import { realpathSync } from "fs";
import { Instance, MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_ROLE,
  LABEL_NUANCED_WORKSPACE_DIGEST,
  NUANCED_ROLE_WORKSPACE,
} from "../util/constants";
import { findInstance } from "../util/morphcloud";
import { MutagenClient } from "../util/mutagen";
import { getGloballyUniqueDigest } from "../util/digest";

export async function workspaceStop(workspaceDir: string) {
  const client = new MorphCloudClient();

  const workspaceRealDir = realpathSync(workspaceDir);
  const workspaceDigest = getGloballyUniqueDigest(workspaceRealDir);
  const metadata = {
    [LABEL_NUANCED_ROLE]: NUANCED_ROLE_WORKSPACE,
    [LABEL_NUANCED_WORKSPACE_DIGEST]: workspaceDigest,
  };

  let workspaceInstance: Instance | undefined;
  try {
    workspaceInstance = await findInstance(client, { metadata });

    if (workspaceInstance) {
      const mutagen = new MutagenClient(workspaceInstance);
      await mutagen.stopSync().catch((e) => {
        console.error(`Error: Cannot remove sync ${workspaceDigest}: ${e}`);
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

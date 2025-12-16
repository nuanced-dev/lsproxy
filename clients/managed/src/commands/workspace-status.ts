import { realpathSync } from "fs";
import { Instance, MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_ROLE,
  LABEL_NUANCED_WORKSPACE_DIGEST,
  NUANCED_ROLE_WORKSPACE,
} from "../util/constants";
import { findInstance } from "../util/morphcloud";
import { getGloballyUniqueDigest } from "../util/digest";

export async function workspaceStatus(workspaceDir: string) {
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
    console.log(
      `${workspaceDigest.padEnd(2)} : ${workspaceInstance?.id ?? "<missing>"}`,
    );
  } catch (error) {
    throw new Error(`Workspace status failed: ${error}`);
  }
}

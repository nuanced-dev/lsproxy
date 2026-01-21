import { MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_LSP_ROLE,
  LABEL_NUANCED_LSP_WORKSPACE_PATH,
  NUANCED_LSP_ROLE_SERVICE,
  NUANCED_LSP_ROLE_WORKSPACE,
} from "../util/constants";
import {
  findSnapshot,
  formatInstanceStatus,
  listInstances,
} from "../util/morphcloud";

export async function serviceStatus() {
  const client = new MorphCloudClient();
  try {
    console.log("System snapshots:");
    const serviceSnapshot = await findSnapshot(client, {
      metadata: { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_SERVICE },
    });
    console.log(
      `- service${" ".repeat(13)} : ${serviceSnapshot ? serviceSnapshot.id : "<missing>"}`,
    );
    console.log("Workspace instances:");
    for (const instance of await listInstances(client, {
      metadata: { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_WORKSPACE },
    })) {
      const path = instance.metadata![LABEL_NUANCED_LSP_WORKSPACE_PATH]!;
      console.log(
        `- ${path.padEnd(20)} : ${instance.id} (${formatInstanceStatus(instance.status)})`,
      );
    }
  } catch (error) {
    throw new Error(`Service status failed: ${error}`);
  }
}

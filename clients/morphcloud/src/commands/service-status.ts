import { MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_LSP_ROLE,
  LABEL_NUANCED_LSP_WORKSPACE_PATH,
  LABEL_NUANCED_LSP_WORKSPACE_HOSTNAME,
  NUANCED_LSP_ROLE_SERVICE,
  NUANCED_LSP_ROLE_WORKSPACE,
} from "../util/constants.js";
import {
  findSnapshot,
  formatInstanceStatus,
  listInstances,
} from "../util/morphcloud.js";

export async function serviceStatus() {
  const client = new MorphCloudClient();
  try {
    console.log("Service snapshots:");
    const serviceSnapshot = await findSnapshot(client, {
      metadata: { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_SERVICE },
    });
    console.log(
      `- service${" ".repeat(13)} : ${serviceSnapshot ? serviceSnapshot.id : "<missing>"}`,
    );
    console.log();
    console.log("Workspace instances:");
    for (const instance of await listInstances(client, {
      metadata: { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_WORKSPACE },
    })) {
      const hostname =
        instance.metadata![LABEL_NUANCED_LSP_WORKSPACE_HOSTNAME] ?? "<unknown>";
      const path = instance.metadata![LABEL_NUANCED_LSP_WORKSPACE_PATH]!;
      const label = `${hostname}:${path}`;
      console.log(
        `- ${label.padEnd(40)} : ${instance.id} (${formatInstanceStatus(instance.status)})`,
      );
    }
  } catch (error) {
    throw new Error(`Service status failed: ${error}`);
  }
}

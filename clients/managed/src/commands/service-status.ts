import { MorphCloudClient } from "morphcloud";
import {
  SERVICE_DIGEST,
  BUILDER_DIGEST,
  LABEL_NUANCED_ROLE,
  LABEL_NUANCED_WORKSPACE_DIGEST,
  BASE_DIGEST,
  NUANCED_ROLE_WORKSPACE,
  SOURCE_DIGEST,
} from "../util/constants";
import {
  findSnapshotByDigest,
  formatInstanceStatus,
  listInstances,
} from "../util/morphcloud";

export async function serviceStatus() {
  const client = new MorphCloudClient();
  try {
    console.log("System snapshots:");
    for (const digest of [
      BASE_DIGEST,
      BUILDER_DIGEST,
      SOURCE_DIGEST,
      SERVICE_DIGEST,
    ]) {
      const snapshot = await findSnapshotByDigest(client, digest);
      console.log(
        `- ${digest.padEnd(20)} : ${snapshot ? snapshot.id : "<missing>"}`,
      );
    }
    console.log("Workspace instances:");
    for (const instance of await listInstances(client, {
      metadata: { [LABEL_NUANCED_ROLE]: NUANCED_ROLE_WORKSPACE },
    })) {
      const digest = instance.metadata![LABEL_NUANCED_WORKSPACE_DIGEST]!;
      console.log(
        `- ${digest.padEnd(20)} : ${instance.id} (${formatInstanceStatus(instance.status)})`,
      );
    }
  } catch (error) {
    throw new Error(`Service status failed: ${error}`);
  }
}

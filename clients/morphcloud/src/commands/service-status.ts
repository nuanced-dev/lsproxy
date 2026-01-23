import { MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_LSP_ROLE,
  NUANCED_LSP_ROLE_SERVICE,
} from "../util/constants.js";
import { findSnapshot } from "../util/morphcloud.js";

export async function serviceStatus() {
  const client = new MorphCloudClient();
  try {
    const serviceSnapshot = await findSnapshot(client, {
      metadata: { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_SERVICE },
    });
    console.log(
      `Service snapshot: ${serviceSnapshot ? serviceSnapshot.id : "<missing>"}`,
    );
  } catch (error) {
    throw new Error(`Service status failed: ${error}`);
  }
}

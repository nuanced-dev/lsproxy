import { MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_LSP_ROLE,
  NUANCED_LSP_ROLE_SERVICE,
} from "../util/constants.js";
import { findSnapshot } from "../util/morphcloud.js";

export async function serviceDelete() {
  const client = new MorphCloudClient();

  try {
    const metadata = { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_SERVICE };
    const serviceSnapshot = await findSnapshot(client, { metadata });

    if (serviceSnapshot) {
      console.log(`Deleting service snapshot: ${serviceSnapshot.id}`);
      await serviceSnapshot.delete();
      console.log("Service snapshot deleted");
    } else {
      console.log("No service snapshot found");
    }
  } catch (error) {
    throw new Error(`Service delete failed: ${error}`);
  }
}

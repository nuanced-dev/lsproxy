#!/usr/bin/env node

import { realpathSync } from "fs";
import { Instance, MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_ROLE,
  LABEL_NUANCED_WORKSPACE_DIGEST,
  NUANCED_ROLE_WORKSPACE,
} from "./util/constants";
import { findInstance } from "./util/morphcloud";
import { getGloballyUniqueDigest } from "./util/digest";

async function main() {
  const client = new MorphCloudClient();

  if (process.argv.length !== 3) {
    console.error(`Usage: ${process.argv[1]} WORKSPACE_DIR`);
    process.exit(1);
  }
  const workspaceDir = realpathSync(process.argv[2]);
  const workspaceDigest = getGloballyUniqueDigest(workspaceDir);
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
    console.error("Error:", error);
    process.exitCode = 1;
  }
}

main().catch(console.error);

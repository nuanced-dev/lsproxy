import { Instance, MorphCloudClient } from "morphcloud";
import {
  LABEL_NUANCED_LSP_ROLE,
  NUANCED_LSP_ROLE_SERVICE,
} from "../util/constants.js";
import {
  execOrThrow,
  findSnapshot,
  startInstance,
} from "../util/morphcloud.js";

const VCPU_COUNT = 2;
const MEM_SIZE_MB = 16384;
const DISK_SIZE_MB = 16384;

const VERBOSE = true;

export async function serviceCreate() {
  const client = new MorphCloudClient();
  let serviceInstance: Instance | undefined;
  try {
    const metadata = { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_SERVICE };

    let serviceSnapshot = await findSnapshot(client, {
      metadata,
    });
    if (serviceSnapshot) {
      console.log(`Found service snapshot: ${serviceSnapshot.id}`);
      return serviceSnapshot;
    }

    console.log("Creating service snapshot...");

    console.log("Creating minimal snapshot...");
    const minimalSnapshot = await client.snapshots.create({
      imageId: "morphvm-minimal",
      vcpus: VCPU_COUNT,
      memory: MEM_SIZE_MB,
      diskSize: DISK_SIZE_MB,
    });
    console.log(`Created minimal snapshot: ${minimalSnapshot.id}`);

    console.log("Starting service instance...");
    serviceInstance = await startInstance(client, minimalSnapshot, {
      metadata,
    });
    console.log(`Started service instance: ${serviceInstance.id}`);

    console.log("Installing packages...");
    console.log("- Installing system commands...");
    await execOrThrow(
      serviceInstance,
      `
            apt-get update -qq \
            && apt-get install -qq -y curl nodejs npm
        `,
      { verbose: VERBOSE, prefix: "  | " },
    );

    console.log("- Installing Docker...");
    await execOrThrow(
      serviceInstance,
      `
            curl -sSf https://get.docker.com | sh -s -- \
            && systemctl enable docker.service \
            && systemctl enable containerd.service \
            && systemctl start docker.service
        `,
      { verbose: VERBOSE, prefix: "  | " },
    );
    const dockerVersion = await execOrThrow(
      serviceInstance,
      "docker --version",
    );
    console.log(`  Installed ${dockerVersion.trim()}`);

    console.log("- Installing Nuanced LSP...");
    await execOrThrow(serviceInstance, "npm install -g @nuanced-dev/lsp", {
      verbose: VERBOSE,
      prefix: "  | ",
    });
    console.log("Installed packages");

    console.log("Pulling service images...");
    await execOrThrow(serviceInstance, "nuanced-lsp pull --all-services", {
      verbose: VERBOSE,
      prefix: "| ",
    });
    console.log("Pulled service images");

    console.log("Creating service snapshot...");
    serviceSnapshot = await serviceInstance.snapshot({
      metadata,
    });
    console.log(`Created service snapshot: ${serviceSnapshot.id}`);

    console.log("Cleaning up...");
    if (serviceInstance) {
      console.log("- service instance...");
      await serviceInstance.stop();
    }
    console.log("Cleaned up");
  } catch (error) {
    if (serviceInstance) {
      console.log(`Left service instance running: ${serviceInstance.id}`);
    }
    throw new Error(`Service setup failed: ${error}`);
  }
}

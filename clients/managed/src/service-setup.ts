#!/usr/bin/env node

import { Instance, MorphCloudClient, Snapshot } from "morphcloud";
import { NodeSSH } from "node-ssh";
import {
  SERVICE_DIGEST,
  BUILDER_DIGEST,
  LABEL_NUANCED_ROLE,
  BASE_DIGEST,
  NUANCED_ROLE_BASE,
  SOURCE_DIGEST,
  NUANCED_ROLE_BUILDER,
  NUANCED_ROLE_SOURCE,
  NUANCED_ROLE_SERVICE,
  SOURCE_ARCHIVE,
  SOURCE_DIR,
} from "./util/constants";
import {
  execOrThrow,
  findSnapshotByDigest,
  startInstance,
} from "./util/morphcloud";

const VCPU_COUNT = 2;
const MEM_SIZE_MB = 16384;
const DISK_SIZE_MB = 16384;

const VERBOSE = true;

async function ensureBase(client: MorphCloudClient): Promise<Snapshot> {
  let baseSnapshot = await findSnapshotByDigest(client, BUILDER_DIGEST);
  if (baseSnapshot) {
    console.log(`Found base snapshot: ${baseSnapshot.id}`);
    return baseSnapshot;
  }

  console.log("Creating minimal snapshot...");

  baseSnapshot = await client.snapshots.create({
    imageId: "morphvm-minimal",
    vcpus: VCPU_COUNT,
    memory: MEM_SIZE_MB,
    diskSize: DISK_SIZE_MB,
    digest: BASE_DIGEST,
    metadata: {
      [LABEL_NUANCED_ROLE]: NUANCED_ROLE_BASE,
    },
  });

  console.log(`Created base snapshot: ${baseSnapshot.id}`);

  return baseSnapshot;
}

async function ensureBuilder(
  client: MorphCloudClient,
  minimalSnapshot: Snapshot,
): Promise<Snapshot> {
  let builderSnapshot = await findSnapshotByDigest(client, BUILDER_DIGEST);
  if (builderSnapshot) {
    console.log(`Found builder snapshot: ${builderSnapshot.id}`);
    return builderSnapshot;
  }

  console.log("Creating builder snapshot...");

  let builderInstance: Instance | undefined;
  try {
    console.log("Starting builder instance...");
    builderInstance = await startInstance(client, minimalSnapshot, {
      metadata: {
        [LABEL_NUANCED_ROLE]: NUANCED_ROLE_BUILDER,
      },
    });
    console.log(`Started builder instance: ${builderInstance.id}`);

    console.log("Installing build tools...");
    console.log("- Installing system commands...");
    await execOrThrow(
      builderInstance,
      `
            apt-get update -qq \
            && apt-get install -qq -y curl git nodejs npm unzip pkg-config
        `,
      { verbose: VERBOSE, prefix: "  | " },
    );

    console.log("- Installing Docker...");
    await execOrThrow(
      builderInstance,
      `
            curl -sSf https://get.docker.com | sh -s -- \
            && systemctl enable docker.service \
            && systemctl enable containerd.service \
            && systemctl start docker.service
        `,
      { verbose: VERBOSE, prefix: "  | " },
    );
    const dockerVersion = await execOrThrow(
      builderInstance,
      "docker --version",
    );
    console.log(`  Installed ${dockerVersion.trim()}`);

    console.log("- Installing Rust...");
    await execOrThrow(
      builderInstance,
      `
            curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        `,
      { verbose: VERBOSE, prefix: "  | " },
    );
    const rustcVersion = await execOrThrow(builderInstance, "rustc --version");
    console.log(`  Installed ${rustcVersion.trim()}`);

    console.log("Installed build tools");

    console.log("Creating builder snapshot...");
    builderSnapshot = await builderInstance.snapshot({
      digest: BUILDER_DIGEST,
      metadata: {
        [LABEL_NUANCED_ROLE]: NUANCED_ROLE_BUILDER,
      },
    });
    console.log(`Created builder snapshot: ${builderSnapshot.id}`);

    console.log("Cleaning up...");
    if (builderInstance) {
      console.log("- builder instance...");
      await builderInstance.stop();
    }
    console.log("Cleaned up");
  } catch (e) {
    if (builderInstance) {
      console.log(`Left builder instance running: ${builderInstance.id}`);
    }
    throw e;
  }

  return builderSnapshot;
}

async function ensureSource(
  client: MorphCloudClient,
  builderSnapshot: Snapshot,
): Promise<Snapshot> {
  let sourceSnapshot = await findSnapshotByDigest(client, SOURCE_DIGEST);
  if (sourceSnapshot) {
    console.log(`Found source snapshot: ${sourceSnapshot.id}`);
    return sourceSnapshot;
  }

  console.log("Creating source snapshot...");

  let sourceInstance: Instance | undefined;
  try {
    console.log("Starting source instance...");
    sourceInstance = await startInstance(client, builderSnapshot, {
      metadata: {
        [LABEL_NUANCED_ROLE]: NUANCED_ROLE_SOURCE,
      },
    });
    console.log(`Started source instance: ${sourceInstance.id}`);

    console.log("Uploading repository...");
    let ssh: NodeSSH | undefined;
    try {
      const ssh = await sourceInstance.ssh();
      await ssh.putFile(SOURCE_ARCHIVE, SOURCE_ARCHIVE);
    } finally {
      if (ssh) {
        ssh.dispose();
      }
    }
    await execOrThrow(
      sourceInstance,
      `
            unzip -q ${SOURCE_ARCHIVE} -d ${SOURCE_DIR} \
            && rm -f ${SOURCE_ARCHIVE}
        `,
      { verbose: VERBOSE, prefix: "| " },
    );
    console.log("Uploaded repository");

    console.log("Creating source snapshot...");
    sourceSnapshot = await sourceInstance.snapshot({
      digest: SOURCE_DIGEST,
      metadata: {
        [LABEL_NUANCED_ROLE]: NUANCED_ROLE_SOURCE,
      },
    });
    console.log(`Created source snapshot: ${sourceSnapshot.id}`);

    console.log("Cleaning up...");
    if (sourceInstance) {
      console.log("- source instance");
      await sourceInstance.stop();
    }
    console.log("Cleaned up");
  } catch (e) {
    if (sourceInstance) {
      console.log(`Left source instance running: ${sourceInstance.id}`);
    }
    throw e;
  }

  return sourceSnapshot;
}

async function ensureService(
  client: MorphCloudClient,
  sourceSnapshot: Snapshot,
): Promise<Snapshot> {
  let serviceSnapshot = await findSnapshotByDigest(client, SERVICE_DIGEST);
  if (serviceSnapshot) {
    console.log(`Found service snapshot: ${serviceSnapshot.id}`);
    return serviceSnapshot;
  }

  console.log("Creating service snapshot...");

  let serviceInstance: Instance | undefined;
  try {
    console.log("Starting service instance...");
    serviceInstance = await startInstance(client, sourceSnapshot, {
      metadata: {
        [LABEL_NUANCED_ROLE]: NUANCED_ROLE_SERVICE,
      },
    });
    console.log(`Started service instance: ${serviceInstance.id}`);

    console.log("Building Nuanced LSP...");
    const buildStartTime = Date.now();
    console.log("- Rust Docker images");
    await execOrThrow(
      serviceInstance,
      `
            cd ${SOURCE_DIR} \
            && scripts/build-rust-images.sh
        `,
      { verbose: VERBOSE, prefix: "  | " },
    );
    console.log("- TypeScript CLI");
    await execOrThrow(
      serviceInstance,
      `
            cd ${SOURCE_DIR}/clients/typescript \
            && npm install \
            && npm run build \
            && npm install -g
        `,
      { verbose: VERBOSE, prefix: "  | " },
    );
    const buildDuration = ((Date.now() - buildStartTime) / 1000).toFixed(2);
    console.log(`Built Nuanced LSP in ${buildDuration}s`);

    console.log("Cleaning up repository...");
    await execOrThrow(
      serviceInstance,
      `
            rm -rf ${SOURCE_DIR}
        `,
      { verbose: VERBOSE, prefix: "| " },
    );
    console.log("Cleaned up repository");

    console.log("Creating service snapshot...");
    serviceSnapshot = await serviceInstance.snapshot({
      digest: SERVICE_DIGEST,
      metadata: {
        [LABEL_NUANCED_ROLE]: NUANCED_ROLE_SERVICE,
      },
    });
    console.log(`Created service snapshot: ${serviceSnapshot.id}`);

    console.log("Cleaning up...");
    if (serviceInstance) {
      console.log("- service instance...");
      await serviceInstance.stop();
    }
    console.log("Cleaned up");
  } catch (e) {
    if (serviceInstance) {
      console.log(`Left service instance running: ${serviceInstance.id}`);
    }
    throw e;
  }

  return serviceSnapshot;
}

async function main() {
  const client = new MorphCloudClient();
  try {
    const baseSnapshot = await ensureBase(client);
    const builderSnapshot = await ensureBuilder(client, baseSnapshot);
    const sourceSnapshot = await ensureSource(client, builderSnapshot);
    const _serviceSnapshot = await ensureService(client, sourceSnapshot);
    process.exit(0);
  } catch (error) {
    console.error("Error:", error);
    process.exit(1);
  }
}

main().catch(console.error);

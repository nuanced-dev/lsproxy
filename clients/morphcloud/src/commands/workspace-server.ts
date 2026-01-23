import { realpathSync } from "fs";
import { hostname } from "os";
import { Instance, MorphCloudClient } from "morphcloud";
import {
  NUANCED_LSP_ROLE_SERVICE,
  NUANCED_LSP_ROLE_WORKSPACE,
  LABEL_NUANCED_LSP_ROLE,
  LABEL_NUANCED_LSP_WORKSPACE_PATH,
  LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID,
  LABEL_NUANCED_LSP_WORKSPACE_HOSTNAME,
  MACHINE_ID,
} from "../util/constants.js";
import {
  findSnapshot,
  startInstance,
  findInstance,
  execOrThrow,
} from "../util/morphcloud.js";
import { MutagenClient } from "../util/mutagen.js";
import { sshExec } from "../util/ssh.js";
import { getWorkspaceProcessRcPath } from "../util/config.js";
import { ProcessRc } from "../util/process-rc.js";

async function ensureWorkspaceInstance(
  client: MorphCloudClient,
  metadata: Record<string, string>,
): Promise<void> {
  let workspaceInstance = await findInstance(client, {
    metadata,
    ensureReady: true,
  });
  if (workspaceInstance) {
    return;
  }

  console.error("Creating workspace instance...");
  const serviceSnapshot = await findSnapshot(client, {
    metadata: { [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_SERVICE },
  });
  if (!serviceSnapshot) {
    throw new Error("Missing service snapshot. Run setup.ts first.");
  }
  workspaceInstance = await startInstance(client, serviceSnapshot, {
    metadata,
  });

  console.error("Starting Nuanced LSP container...");
  await execOrThrow(
    workspaceInstance,
    "nuanced-lsp up --container-name nuanced-lsp workspace",
  );

  console.error(`Created workspace instance: ${workspaceInstance.id}`);
}

export async function workspaceServer(workspaceDir: string) {
  const client = new MorphCloudClient();

  const workspaceRealDir = realpathSync(workspaceDir);
  const metadata = {
    [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_WORKSPACE,
    [LABEL_NUANCED_LSP_WORKSPACE_PATH]: workspaceRealDir,
    [LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID]: MACHINE_ID,
    [LABEL_NUANCED_LSP_WORKSPACE_HOSTNAME]: hostname(),
  };

  const processRcPath = await getWorkspaceProcessRcPath(workspaceRealDir);
  const processRc = new ProcessRc(processRcPath);

  let workspaceInstance: Instance | undefined;
  let mutagen: MutagenClient | undefined;
  try {
    await processRc.acquire(async () => {
      await ensureWorkspaceInstance(client, metadata);
    });

    workspaceInstance = await findInstance(client, {
      metadata,
      ensureReady: true,
    });
    if (!workspaceInstance) {
      throw new Error("Workspace instance not found after acquire");
    }
    console.error(`Workspace instance: ${workspaceInstance.id}`);

    mutagen = new MutagenClient(workspaceInstance);

    console.error("Setting up file sync...");
    try {
      if (!(await mutagen.findSync())) {
        console.error("Creating file sync...");
        await mutagen.createSync(workspaceRealDir);
      } else {
        console.error("Resuming file sync...");
        await mutagen.resumeSync();
        await mutagen.flushSync();
      }
      console.error("Waiting for file sync...");
      await mutagen.waitForSyncReady();
    } catch (e) {
      throw new Error(`File sync failure: ${e}`);
    }

    console.error("Starting nuanced-lsp server...");
    const ssh = await workspaceInstance.ssh();
    const cmd = "nuanced-lsp";
    const args = ["server", "--container-name", "nuanced-lsp"];
    const stream = await sshExec(ssh, cmd, args, {
      stdin: process.stdin,
      stdout: process.stdout,
      stderr: process.stderr,
    });
    stream.on("close", async () => {
      ssh.dispose();
      await processRc.release(async () => {
        if (mutagen) {
          await mutagen.pauseSync().catch((e) => {
            console.error(`Error: Cannot pause sync ${workspaceRealDir}: ${e}`);
          });
        }
        if (workspaceInstance) {
          await workspaceInstance.pause().catch((e) => {
            console.error(
              `Error: Cannot pause instance ${workspaceInstance!.id}: ${e}`,
            );
          });
        }
      });
      process.exit(0);
    });
  } catch (e) {
    await processRc.release(async () => {
      if (mutagen) {
        await mutagen.stopSync().catch((e) => {
          console.error(
            `Workspace LSP error: Cannot remove sync ${workspaceRealDir}: ${e}`,
          );
        });
      }
      if (workspaceInstance) {
        await workspaceInstance.stop().catch((e) => {
          console.error(
            `Workspace LSP error: Cannot stop instance ${workspaceInstance!.id}: ${e}`,
          );
        });
      }
    });
    console.error(`Workspace LSP error: ${e}`);
    process.exit(1);
  }
}

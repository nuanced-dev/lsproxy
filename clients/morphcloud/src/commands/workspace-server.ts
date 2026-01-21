import { realpathSync } from "fs";
import { Instance, MorphCloudClient } from "morphcloud";
import { machineIdSync } from "node-machine-id";
import {
  NUANCED_LSP_ROLE_SERVICE,
  NUANCED_LSP_ROLE_WORKSPACE,
  LABEL_NUANCED_LSP_ROLE,
  LABEL_NUANCED_LSP_WORKSPACE_PATH,
  LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID,
} from "../util/constants";
import {
  findSnapshot,
  startInstance,
  findInstance,
  execOrThrow,
} from "../util/morphcloud";
import { MutagenClient } from "../util/mutagen";
import { sshExec } from "../util/ssh";
import { getWorkspaceProcessRcPath } from "../util/config";
import { ProcessRc } from "../util/process-rc";

const MACHINE_ID = machineIdSync();

async function ensureWorkspaceInstance(
  client: MorphCloudClient,
  metadata: Record<string, string>,
  workspaceRealDir: string,
): Promise<{ instance: Instance; mutagen: MutagenClient }> {
  let workspaceInstance = await findInstance(client, {
    metadata,
    ensureReady: true,
  });

  let mutagen: MutagenClient;

  if (!workspaceInstance) {
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

    console.error(`Workspace instance: ${workspaceInstance.id}`);

    mutagen = new MutagenClient(workspaceInstance);

    console.error("Creating file sync...");
    try {
      await mutagen.createSync(workspaceRealDir);
      await mutagen.flushSync();
    } catch (e) {
      throw new Error(`Failed to setup file sync: ${e}`);
    }
  } else {
    console.error(`Workspace instance: ${workspaceInstance.id}`);

    mutagen = new MutagenClient(workspaceInstance);

    if (!(await mutagen.findSync({ ensureReady: true }))) {
      throw new Error(
        "File sync not found for existing workspace. Delete the workspace and start over.",
      );
    }

    console.error("Resuming file sync...");
    try {
      await mutagen.resumeSync();
    } catch (e) {
      throw new Error(`Failed to resume file sync: ${e}`);
    }
  }

  return { instance: workspaceInstance, mutagen };
}

export async function workspaceServer(workspaceDir: string) {
  const client = new MorphCloudClient();

  const workspaceRealDir = realpathSync(workspaceDir);
  const metadata = {
    [LABEL_NUANCED_LSP_ROLE]: NUANCED_LSP_ROLE_WORKSPACE,
    [LABEL_NUANCED_LSP_WORKSPACE_PATH]: workspaceRealDir,
    [LABEL_NUANCED_LSP_WORKSPACE_MACHINE_ID]: MACHINE_ID,
  };

  const processRcPath = await getWorkspaceProcessRcPath(workspaceRealDir);
  const processRc = new ProcessRc(processRcPath);

  let workspaceInstance: Instance | undefined;
  let mutagen: MutagenClient | undefined;
  try {
    await processRc.acquire(async () => {
      const result = await ensureWorkspaceInstance(
        client,
        metadata,
        workspaceRealDir,
      );
      workspaceInstance = result.instance;
      mutagen = result.mutagen;
    });

    if (!workspaceInstance) {
      workspaceInstance = await findInstance(client, {
        metadata,
        ensureReady: true,
      });
      if (!workspaceInstance) {
        throw new Error("Workspace instance not found after acquire");
      }
      console.error(`Workspace instance: ${workspaceInstance.id}`);
    }

    console.error("Executing nuanced-lsp server...");
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

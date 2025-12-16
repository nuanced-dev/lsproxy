import { realpathSync } from "fs";
import { Instance, MorphCloudClient } from "morphcloud";
import {
  SERVICE_DIGEST,
  NUANCED_ROLE_WORKSPACE,
  LABEL_NUANCED_ROLE,
  LABEL_NUANCED_WORKSPACE_DIGEST,
} from "../util/constants";
import {
  findSnapshotByDigest,
  startInstance,
  findInstance,
} from "../util/morphcloud";
import { MutagenClient } from "../util/mutagen";
import { getGloballyUniqueDigest } from "../util/digest";
import { sshExec } from "../util/ssh";
import { getWorkspaceProcessRcPath } from "../util/config";
import { ProcessRc } from "../util/process-rc";

export async function workspaceLsp(workspaceDir: string) {
  const client = new MorphCloudClient();

  const workspaceRealDir = realpathSync(workspaceDir);
  const workspaceDigest = getGloballyUniqueDigest(workspaceRealDir);
  const metadata = {
    [LABEL_NUANCED_ROLE]: NUANCED_ROLE_WORKSPACE,
    [LABEL_NUANCED_WORKSPACE_DIGEST]: workspaceDigest,
  };

  const processRcPath = await getWorkspaceProcessRcPath(workspaceDigest);
  const processRc = new ProcessRc(processRcPath);

  let workspaceInstance: Instance | undefined;
  let mutagen: MutagenClient | undefined;
  try {
    await processRc.acquire(async () => {
      workspaceInstance = await findInstance(client, {
        metadata,
        ensureReady: true,
      });

      if (!workspaceInstance) {
        console.error("Creating workspace instance...");
        const serviceSnapshot = await findSnapshotByDigest(
          client,
          SERVICE_DIGEST,
        );
        if (!serviceSnapshot) {
          throw new Error("Missing service snapshot. Run setup.ts first.");
        }
        workspaceInstance = await startInstance(client, serviceSnapshot, {
          metadata,
        });
      }
      console.error(`Workspace instance: ${workspaceInstance.id}`);
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

    mutagen = new MutagenClient(workspaceInstance);

    if (await mutagen.findSync({ ensureReady: true })) {
      console.error("Resuming file sync...");
      try {
        await mutagen.resumeSync();
      } catch (e) {
        throw new Error(`Failed to resume file sync: ${e}`);
      }
    } else {
      console.error("Creating file sync...");
      try {
        await mutagen.createSync(workspaceRealDir);
        await mutagen.flushSync();
      } catch (e) {
        throw new Error(`Failed to setup file sync: ${e}`);
      }
    }

    console.error("Executing nuanced-lsp...");
    const ssh = await workspaceInstance.ssh();
    const cmd = "nuanced-lsp";
    const args = [
      "server",
      "--proxy-image",
      "nuanced-lsp-proxy:latest",
      "--watchdog-image",
      "nuanced-lsp-watchdog:latest",
      "--wrapper-image",
      "nuanced-lsp-wrapper:latest",
      "--shared",
      "workspace",
    ];
    console.error(`nuanced-lsp ${args.join(" ")}`);
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
            console.error(`Error: Cannot pause sync ${workspaceDigest}: ${e}`);
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
            `Workspace LSP error: Cannot remove sync ${workspaceDigest}: ${e}`,
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

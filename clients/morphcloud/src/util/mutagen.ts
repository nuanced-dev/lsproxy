import { join } from "path";
import process from "process";
import { mkdir } from "fs/promises";
import { Instance } from "morphcloud";
import { mutagen } from "@nuanced-dev/mutagen";
import { ensureConfigDirectory } from "./config.js";
import { getSshConfig, removeSshConfig } from "./ssh.js";

const MISSING_SESSION_ERROR = "unable to locate requested session";

export class MutagenClient {
  constructor(private readonly instance: Instance) {}

  private syncName(): string {
    return `nuanced-lsp-${this.instance.id.replace("_", "-")}`;
  }

  // Status: Staging files on beta
  // Status: Applying changes
  // Status: Watching for changes

  async createSync(localDirectory: string): Promise<void> {
    try {
      const sshConfig = await getSshConfig(this.instance);
      await spawnMutagen(
        [
          "sync",
          "create",
          "--name",
          this.syncName(),
          "--mode",
          "two-way-safe",
          localDirectory,
          `${sshConfig.user}@${sshConfig.host}:workspace`,
        ],
        { env: { MUTAGEN_SSH_CONFIG_BETA: sshConfig.configPath } },
      );
    } catch (e) {
      throw new Error(`Failed to create sync: ${(e as any).stderr}`);
    }
  }

  async findSync(_opts?: { ensureReady?: boolean }): Promise<boolean> {
    try {
      await spawnMutagen(["sync", "list", this.syncName()]);
      // TODO implement waiting on ready status
      return true;
    } catch (e) {
      const stderr = (e as any).stderr.toLowerCase();
      if (stderr.includes(MISSING_SESSION_ERROR)) {
        return false;
      } else {
        throw new Error(`Failed to find sync: ${(e as any).stderr}`);
      }
    }
  }

  async flushSync(): Promise<void> {
    try {
      await spawnMutagen(["sync", "flush", this.syncName()]);
    } catch (e) {
      throw new Error(`Failed to flush sync: ${(e as any).stderr}`);
    }
  }

  async pauseSync(): Promise<void> {
    try {
      await spawnMutagen(["sync", "pause", this.syncName()]);
    } catch (e) {
      throw new Error(`Failed to pause sync: ${(e as any).stderr}`);
    }
  }

  async resumeSync(): Promise<void> {
    try {
      const sshConfig = await getSshConfig(this.instance);
      await spawnMutagen(["sync", "resume", this.syncName()], {
        env: { MUTAGEN_SSH_CONFIG_BETA: sshConfig.configPath },
      });
    } catch (e) {
      throw new Error(`Failed to resume sync: ${(e as any).stderr}`);
    }
  }

  async stopSync(): Promise<void> {
    try {
      await spawnMutagen(["sync", "terminate", this.syncName()]);
    } catch (e) {
      const stderr = (e as any).stderr.toLowerCase();
      if (stderr.includes(MISSING_SESSION_ERROR)) {
        return;
      } else {
        throw new Error(`Failed to stop sync: ${(e as any).stderr}`);
      }
    } finally {
      await removeSshConfig(this.instance);
    }
  }
}

export async function spawnMutagen(
  args: string[],
  opts?: { env?: Record<string, string> },
): Promise<void> {
  const mutagenDataDir = join(await ensureConfigDirectory(), "mutagen");
  await mkdir(mutagenDataDir, { recursive: true });
  await mutagen(args, {
    env: {
      ...process.env,
      ...opts?.env,
      MUTAGEN_DATA_DIRECTORY: mutagenDataDir,
    },
  });
}

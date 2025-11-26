import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, ChildProcessWithoutNullStreams } from "node:child_process";

export interface LogFollower {
  proc: ChildProcessWithoutNullStreams;
  stream: fs.WriteStream;
  path: string;
}

function safeFragment(name: string): string {
  return name.replace(/[^a-z0-9._-]+/gi, "-");
}

export function startLogFollow(containerName: string): LogFollower | undefined {
  try {
    const safe = safeFragment(containerName);
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), `nuanced-lsp-${safe}-`));
    const filePath = path.join(dir, "logs.txt");
    const stream = fs.createWriteStream(filePath, { encoding: "utf8" });

    const proc = spawn("docker", ["logs", "-f", containerName], {
      stdio: ["ignore", "pipe", "pipe"],
    });

    proc.stdout?.pipe(stream, { end: false });
    proc.stderr?.pipe(stream, { end: false });

    proc.on("close", () => {
      stream.end();
    });

    return { proc, stream, path: filePath };
  } catch {
    // If we fail to start log following (e.g., docker missing), ignore.
    return undefined;
  }
}

export async function stopLogFollow(follower?: LogFollower): Promise<void> {
  if (!follower) return;

  const { proc, stream } = follower;
  try {
    if (proc.exitCode === null) {
      proc.kill("SIGINT");
      const timeout = new Promise<void>((resolve) => setTimeout(resolve, 2000));
      await Promise.race([
        new Promise<void>((resolve) => {
          proc.once("exit", () => resolve());
        }),
        timeout,
      ]);
      if (proc.exitCode === null) {
        proc.kill("SIGKILL");
      }
    }
  } catch {
    // ignore
  }

  try {
    await new Promise<void>((resolve) => {
      stream.end(() => resolve());
    });
  } catch {
    // ignore
  }
}

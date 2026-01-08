import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { describe, it, expect } from "vitest";

import { LANGUAGES, DOCKER_AVAILABLE } from "../helpers/constants.js";
import { ClientRunner, createRunner } from "../helpers/runner.js";

const describeDocker = DOCKER_AVAILABLE ? describe : describe.skip;
const MAX_LIFECYCLE_CONCURRENCY = Math.max(
  1,
  Number(
    process.env.NUANCED_LIFECYCLE_MAX_CONCURRENCY ?? os.cpus()?.length ?? 1,
  ),
);

function createSemaphore(limit: number): {
  acquire: () => Promise<() => void>;
} {
  let active = 0;
  const queue: Array<() => void> = [];

  async function acquire(): Promise<() => void> {
    if (active >= limit) {
      await new Promise<void>((resolve) => queue.push(resolve));
    }
    active += 1;
    return () => {
      active -= 1;
      const next = queue.shift();
      if (next) {
        next();
      }
    };
  }

  return { acquire };
}

const lifecycleSemaphore = createSemaphore(MAX_LIFECYCLE_CONCURRENCY);

describe("TypeScript client", () => {
  describeDocker("lifecycle", () => {
    for (const language of LANGUAGES) {
      describe(language.label, () => {
        it.concurrent(
          "manages container lifecycle: up, status, run scripts, logs, down",
          async () => {
            const release = await lifecycleSemaphore.acquire();

            console.info(
              `[lifecycle] starting ${language.key} (${language.label})`,
            );
            const runner: ClientRunner = createRunner({ language });

            try {
              // Test: bring container up
              runner.up();

              // Test: status
              const status = runner.statusJson();
              expect(status?.container_name).toEqual(runner.containerName);
              const containerStatus = status?.container_status;
              expect(typeof containerStatus).toBe("string");
              expect(String(containerStatus).startsWith("Up")).toBeTruthy();

              // Test: run custom script
              const sid = runner.containerName.replace(/\//g, "_");
              const scriptName = `.nuanced_test_boot_${sid}.sh`;
              const probeName = `.nuanced_probe_${sid}.txt`;
              const scriptPath = path.join(runner.workspace, scriptName);
              const probePath = path.join(runner.workspace, probeName);

              try {
                fs.writeFileSync(
                  scriptPath,
                  `#!/bin/sh\necho "hello-from-container" > /mnt/workspace/${probeName}\n`,
                  { encoding: "utf8" },
                );

                runner.runScript(scriptPath);

                expect(fs.existsSync(probePath)).toBeTruthy();
                const contents = fs.readFileSync(probePath, "utf8").trim();
                expect(contents).toBe("hello-from-container");
              } finally {
                fs.rmSync(scriptPath, { force: true });
                fs.rmSync(probePath, { force: true });
              }

              // Test: logs
              const logs = runner.logs();
              expect(typeof logs).toBe("string");
            } finally {
              // Test: tear down
              if (runner) {
                await runner.down();
              }
              release();
            }
          },
        );
      });
    }
  });
});

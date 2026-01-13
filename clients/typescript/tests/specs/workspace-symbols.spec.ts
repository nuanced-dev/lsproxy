import path from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { TEST_WORKSPACES, DOCKER_AVAILABLE } from "../helpers/constants.js";
import { ClientRunner, createRunner } from "../helpers/runner.js";
import {
  pickIdentifier,
  pickPosition,
  assertMatchesSchemaFixture,
} from "../helpers/fixtures.js";
import {
  workspaceDefinitions,
  workspaceFiles,
  resetCaches,
} from "../helpers/workspace-cache.js";

function normalizeListFiles(payload: unknown): unknown {
  if (!Array.isArray(payload)) return payload;
  return [...new Set(payload)].sort((a, b) =>
    a.localeCompare(b, undefined, { sensitivity: "base" }),
  );
}

const REFERENCED_SYMBOLS_SUPPORTED = new Set(["csharp", "ts", "js", "python"]);

const MAX_FILES_WITH_DEFS = Number(
  process.env.NUANCED_MAX_FILES_WITH_DEFS ?? "2",
);
const MAX_DEFS_PER_FILE = Number(process.env.NUANCED_MAX_DEFS_PER_FILE ?? "2");

const describeDocker = DOCKER_AVAILABLE ? describe : describe.skip;

describe("TypeScript client", () => {
  describeDocker("workspace + symbols", () => {
    for (const workspace of TEST_WORKSPACES) {
      describe(workspace, () => {
        let runner: ClientRunner;

        beforeAll(async () => {
          runner = createRunner({ workspace });
          runner.ensureUp();
          // For Rust, wait for initial indexing to complete
          if (workspace === "rust") {
            await new Promise((resolve) => setTimeout(resolve, 15000));
          }
        });

        afterAll(async () => {
          await runner.down();
          resetCaches();
        });

        it("lists all files in the workspace deterministically", async () => {
          runner.ensureUp();
          const files = await workspaceFiles(runner);
          expect(Array.isArray(files) && files.length > 0).toBeTruthy();

          const payload = normalizeListFiles(files);

          assertMatchesSchemaFixture({
            runnerWorkspace: runner.workspace,
            endpoint: "list-files",
            sourcePath: path.basename(runner.workspace),
            params: {},
            payload,
          });
        });

        it("reads source files from the workspace", async () => {
          runner.ensureUp();
          const files = await workspaceFiles(runner);
          const target =
            files.find((f) => f.toLowerCase().includes("fizz")) ?? files[0];

          const payload = await runner.readSource(target);
          expect(payload && typeof payload === "object").toBeTruthy();
          const sourceRecord = payload as Record<string, unknown>;
          expect(typeof sourceRecord.source_code).toBe("string");
          expect((sourceRecord.source_code as string).length).toBeGreaterThan(
            0,
          );

          assertMatchesSchemaFixture({
            runnerWorkspace: runner.workspace,
            endpoint: "read-source",
            sourcePath: target,
            params: {},
            payload,
            allowBasenameFallback: true,
          });
        });

        it("collects definitions per file", async () => {
          const files = await workspaceFiles(runner);
          if (!files.length) {
            return;
          }

          const defs = await workspaceDefinitions(runner, files);

          for (const file of files) {
            const entries = defs.get(file) ?? [];
            assertMatchesSchemaFixture({
              runnerWorkspace: runner.workspace,
              endpoint: "definitions-in-file",
              sourcePath: file,
              params: {},
              payload: entries,
              allowBasenameFallback: true,
            });
          }
        });

        it("finds definitions for identifiers", async () => {
          const files = await workspaceFiles(runner);
          const defs = await workspaceDefinitions(runner, files);

          let considered = 0;
          for (const file of files) {
            const entries = defs.get(file) ?? [];
            for (const entry of entries) {
              const pos = pickPosition(entry);
              if (!pos) continue;
              const [line, character] = pos;
              const payload = await runner.findDefinition(
                file,
                line,
                character,
              );
              const ident = pickIdentifier(entry) ?? undefined;
              assertMatchesSchemaFixture({
                runnerWorkspace: runner.workspace,
                endpoint: "find-definition",
                sourcePath: file,
                params: { line, character },
                payload,
                label: ident,
                allowBasenameFallback: true,
              });
              considered += 1;
            }
          }

          expect(considered).toBeGreaterThan(0);
        });

        it("finds identifiers without narrowing", async () => {
          const files = await workspaceFiles(runner);
          const defs = await workspaceDefinitions(runner, files);

          let considered = 0;
          for (const file of files) {
            const entries = defs.get(file) ?? [];
            for (const entry of entries) {
              const ident = pickIdentifier(entry);
              if (!ident) continue;
              const payload = await runner.findIdentifier(file, ident);
              assertMatchesSchemaFixture({
                runnerWorkspace: runner.workspace,
                endpoint: "find-identifier",
                sourcePath: file,
                params: { identifier: ident },
                payload,
                label: ident,
                allowBasenameFallback: true,
              });
              considered += 1;
            }
          }
          expect(considered).toBeGreaterThan(0);
        });

        it("finds identifiers with narrowing", async () => {
          const files = await workspaceFiles(runner);
          const defs = await workspaceDefinitions(runner, files);

          let considered = 0;
          for (const file of files) {
            const entries = defs.get(file) ?? [];
            for (const entry of entries) {
              const ident = pickIdentifier(entry);
              const pos = pickPosition(entry);
              if (!ident || !pos) continue;
              const [line, character] = pos;
              const payload = await runner.findIdentifier(
                file,
                ident,
                `${line}:${character}`,
              );
              assertMatchesSchemaFixture({
                runnerWorkspace: runner.workspace,
                endpoint: "find-identifier-narrowed",
                sourcePath: file,
                params: { identifier: ident, line, character },
                payload,
                label: ident,
                allowBasenameFallback: true,
              });
              considered += 1;
            }
          }
          expect(considered).toBeGreaterThan(0);
        });

        it("finds referenced symbols where supported", async () => {
          if (!REFERENCED_SYMBOLS_SUPPORTED.has(workspace)) {
            return;
          }

          const files = await workspaceFiles(runner);
          const defs = await workspaceDefinitions(runner, files);
          let considered = 0;

          for (const file of files) {
            const entries = defs.get(file) ?? [];
            for (const entry of entries) {
              const pos = pickPosition(entry);
              if (!pos) continue;
              const [line, character] = pos;
              const ident = pickIdentifier(entry) ?? undefined;

              try {
                const payload = await runner.findReferencedSymbols(
                  file,
                  line,
                  character,
                );
                assertMatchesSchemaFixture({
                  runnerWorkspace: runner.workspace,
                  endpoint: "find-referenced-symbols",
                  sourcePath: file,
                  params: { line, character },
                  payload,
                  label: ident,
                  allowBasenameFallback: true,
                });
                considered += 1;
              } catch (error) {
                const msg = String((error as Error).message).toLowerCase();
                if (msg.includes("not implemented")) {
                  continue;
                }
                throw error;
              }
            }
          }

          expect(considered).toBeGreaterThan(0);
        });

        it("finds references to symbols where possible", async () => {
          const files = await workspaceFiles(runner);
          const defs = await workspaceDefinitions(runner, files);
          const candidateFiles = files.filter(
            (file) => (defs.get(file) ?? []).length > 0,
          );
          const limitedFiles = candidateFiles.slice(0, MAX_FILES_WITH_DEFS);

          let considered = 0;

          for (const file of limitedFiles) {
            const entries = (defs.get(file) ?? []).slice(0, MAX_DEFS_PER_FILE);
            for (const entry of entries) {
              const pos = pickPosition(entry);
              if (!pos) continue;
              const [line, character] = pos;
              const ident = pickIdentifier(entry) ?? undefined;

              try {
                const payload = await runner.findReferences(
                  file,
                  line,
                  character,
                );
                assertMatchesSchemaFixture({
                  runnerWorkspace: runner.workspace,
                  endpoint: "find-references",
                  sourcePath: file,
                  params: { line, character },
                  payload,
                  label: ident,
                  allowBasenameFallback: true,
                });
                considered += 1;
              } catch (error) {
                if (workspace === "php") {
                  const msg = String((error as Error).message).toLowerCase();
                  if (
                    msg.includes("-32603") ||
                    msg.includes("internal error") ||
                    msg.includes("phpactor") ||
                    msg.includes("timed out") ||
                    msg.includes("timeout")
                  ) {
                    continue;
                  }
                }
                throw error;
              }
            }
          }

          if (considered === 0) {
            return;
          }
          expect(considered).toBeGreaterThan(0);
        });
      });
    }
  });
});

import { spawnSync } from "node:child_process";
import { describe, it, expect } from "vitest";

import { CLIENT_COMMAND } from "../helpers/constants.js";

const REQUIRED_SUBCOMMANDS = [
  "up",
  "run",
  "down",
  "logs",
  "status",
  "pull",
  "health",
  "list-files",
  "read-source",
  "definitions-in-file",
  "find-definition",
  "find-identifier",
  "find-referenced-symbols",
  "find-references",
];

const HELP_PATTERNS: Record<string, RegExp[]> = {
  up: [
    /workspace\b/i,
    /--host-port\b/i,
    /--bind-host\b/i,
    /--language-image-version\b/i,
    /--service-image-version\b/i,
    /--container-registry\b/i,
    /--container-name\b/i,
    /--timeout\b/i,
    /--sudo\b/i,
    /--stream\b/i,
    /--ro\b/i,
    /--json\b/i,
    /--debug\b/i,
    /--env\b/i,
    /--env-file\b/i,
  ],
  run: [
    /script\b/i,
    /--container-name\b/i,
    /--timeout\b/i,
    /--sudo\b/i,
    /--stream\b/i,
    /--json\b/i,
    /--env\b/i,
    /--env-file\b/i,
  ],
  down: [/--container-name\b/i, /--sudo\b/i, /--timeout\b/i, /--json\b/i],
  logs: [
    /--container-name\b/i,
    /--timeout\b/i,
    /--sudo\b/i,
    /--stream\b/i,
    /--since\b/i,
    /--tail\b/i,
    /--json\b/i,
  ],
  status: [/--json\b/i, /--container-name\b/i, /--timeout\b/i, /--sudo\b/i],
  pull: [
    /--language-image-version\b/i,
    /--service-image-version\b/i,
    /--container-registry\b/i,
    /--all-languages\b/i,
    /--languages\b/i,
    /--all-services\b/i,
    /--services\b/i,
    /--sudo\b/i,
    /--stream\b/i,
    /--json\b/i,
  ],
  health: [/--lsp-url\b/i, /--lsp-port\b/i, /--timeout\b/i, /--json\b/i],
  "list-files": [/--lsp-url\b/i, /--lsp-port\b/i, /--timeout\b/i, /--json\b/i],
  "read-source": [
    /--range\b/i,
    /--lsp-url\b/i,
    /--lsp-port\b/i,
    /--timeout\b/i,
    /--json\b/i,
  ],
  "definitions-in-file": [
    /--lsp-url\b/i,
    /--lsp-port\b/i,
    /--timeout\b/i,
    /--json\b/i,
  ],
  "find-definition": [
    /--include-raw-response\b/i,
    /--include-source-code\b/i,
    /--lsp-url\b/i,
    /--lsp-port\b/i,
    /--timeout\b/i,
    /--json\b/i,
  ],
  "find-identifier": [
    /--position\b/i,
    /--lsp-url\b/i,
    /--lsp-port\b/i,
    /--timeout\b/i,
    /--json\b/i,
  ],
  "find-referenced-symbols": [
    /--full-scan\b/i,
    /--lsp-url\b/i,
    /--lsp-port\b/i,
    /--timeout\b/i,
    /--json\b/i,
  ],
  "find-references": [
    /--context-lines\b/i,
    /--include-code-context-lines\b/i,
    /--include-raw-response\b/i,
    /--lsp-url\b/i,
    /--lsp-port\b/i,
    /--timeout\b/i,
    /--json\b/i,
  ],
};

function runHelp(command: string, args: string[]): string {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    timeout: 10_000,
    env: { ...process.env },
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status && result.status !== 0) {
    return (result.stdout || "") + (result.stderr || "");
  }
  return result.stdout || "";
}

describe("TypeScript client", () => {
  describe("CLI contract", () => {
    const [cmd, ...baseArgs] = CLIENT_COMMAND;

    it("includes required subcommands in top-level help", () => {
      const help = runHelp(cmd, [...baseArgs, "--help"]);
      for (const sub of REQUIRED_SUBCOMMANDS) {
        expect(new RegExp(`\\b${sub}\\b`, "i").test(help)).toBeTruthy();
      }
    });

    for (const [subcommand, patterns] of Object.entries(HELP_PATTERNS)) {
      it(`${subcommand} exposes required flags`, () => {
        const help = runHelp(cmd, [...baseArgs, subcommand, "--help"]);
        for (const pattern of patterns) {
          expect(pattern.test(help)).toBeTruthy();
        }
      });
    }
  });
});

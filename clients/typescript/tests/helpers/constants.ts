import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { selectLanguages } from "../../src/constants.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, "..", "..");

export const CLIENT_COMMAND = [path.join(ROOT, "dist/cli.cjs")];

const WORKSPACES_DIR = path.join(ROOT, "tests", "workspaces");

const ALL_WORKSPACES: string[] = (() => {
  try {
    return readdirSync(WORKSPACES_DIR).filter((entry: string) => {
      const fullPath = path.join(WORKSPACES_DIR, entry);
      return statSync(fullPath).isDirectory();
    });
  } catch {
    return [];
  }
})();

export const TEST_WORKSPACES: string[] = (function () {
  if (process.env.TEST_WORKSPACES) {
    const names = process.env.TEST_WORKSPACES.split(",")
      .map((value: string) => value.trim().toLowerCase())
      .filter(Boolean);
    return selectLanguages(names, ALL_WORKSPACES);
  } else {
    return ALL_WORKSPACES;
  }
})();

const USE_EPHEMERAL_PORTS = (() => {
  const value =
    process.env.NUANCED_EPHEMERAL_PORTS ??
    process.env.NUANCED_USE_EPHEMERAL_PORTS ??
    "";
  const normalized = value.toLowerCase();
  // Default to ephemeral ports unless explicitly disabled
  if (normalized === "0" || normalized === "false" || normalized === "no") {
    return false;
  }
  return true;
})();

const BASE_PORT = 54000;
const WORKER_STRIDE = 1000;
const LANG_STRIDE = 10;

export function workerIndex(): number {
  const envId = process.env.VITEST_WORKER_ID ?? process.env.VITEST_POOL_ID;
  if (!envId) {
    return 0;
  }
  const parsed = Number(envId);
  return Number.isFinite(parsed) ? Number(parsed) : 0;
}

function languageIndex(lang: string): number {
  const idx = TEST_WORKSPACES.findIndex((l) => l === lang);
  return idx >= 0 ? idx : 0;
}

export function fixedPort(lang: string): number {
  const override = process.env.NUANCED_FIXED_HOST_PORT;
  if (override && override.length > 0) {
    const parsed = Number(override);
    if (Number.isFinite(parsed) && parsed > 0) {
      return parsed;
    }
  }

  if (USE_EPHEMERAL_PORTS) {
    return 0;
  }

  return (
    BASE_PORT +
    WORKER_STRIDE * workerIndex() +
    LANG_STRIDE * languageIndex(lang)
  );
}

export function dockerSafe(raw: string): string {
  const lower = raw.toLowerCase();
  const collapsed = lower.replace(/[^a-z0-9-]+/g, "-");
  const normalized = collapsed.replace(/-+/g, "-").replace(/^-|-$/g, "");
  return normalized || "nuanced-test";
}

export function workspacePath(langKey: string): string {
  return path.join(ROOT, "tests", "workspaces", langKey);
}

export const TIMEOUT_SECONDS = Number(process.env.NUANCED_LSP_TIMEOUT ?? "120");
export const LANGUAGE_IMAGE_VERSION = process.env.LANGUAGE_TAG ?? "";
export const SERVICE_IMAGE_VERSION = process.env.SERVICE_TAG ?? "";
export const CONTAINER_REGISTRY = process.env.REGISTRY ?? "";
export const SYMBOL_SCENARIO_DELAY = Number(
  process.env.SYMBOL_SCENARIO_DELAY ?? "0",
);
export const WORKSPACE_SCENARIO_DELAY = Number(
  process.env.WORKSPACE_SCENARIO_DELAY ?? "0",
);

const FOLLOW_LOGS_ENV = (process.env.NUANCED_FOLLOW_LOGS ?? "").toLowerCase();
export const FOLLOW_LOGS =
  FOLLOW_LOGS_ENV === "1" ||
  FOLLOW_LOGS_ENV === "true" ||
  FOLLOW_LOGS_ENV === "yes" ||
  FOLLOW_LOGS_ENV === "on";

export function isGeneratedPath(input: string): boolean {
  const parts = input
    .replace(/\\/g, "/")
    .split("/")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  return parts.some((segment) => segment === "vendor" || segment === "obj");
}

function detectDockerAvailability(): boolean {
  const result = spawnSync("docker", ["ps"], {
    encoding: "utf8",
    timeout: 5000,
  });

  if (result.error) {
    return false;
  }

  if (typeof result.status === "number" && result.status !== 0) {
    const stderr = result.stderr?.toString() ?? "";
    if (
      /permission denied/i.test(stderr) ||
      /operation not permitted/i.test(stderr)
    ) {
      return false;
    }
    if (/docker: cannot connect/i.test(stderr)) {
      return false;
    }
  }

  return true;
}

export const DOCKER_AVAILABLE = detectDockerAvailability();

if (!DOCKER_AVAILABLE) {
  console.warn(
    "[nuanced-lsp] Docker is not available; skipping container-based tests.",
  );
} else {
  if (USE_EPHEMERAL_PORTS) {
    console.warn(
      "[nuanced-lsp] Test containers will use Docker-assigned ephemeral host ports (default).",
    );
  } else {
    console.warn("[nuanced-lsp] Test containers will use fixed host ports.");
  }
  const selected = TEST_WORKSPACES.join(", ");
  console.warn(`[nuanced-lsp] Selected workspaces: ${selected}`);
}

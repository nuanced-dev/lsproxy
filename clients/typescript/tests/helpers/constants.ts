import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export interface LanguageSpec {
  key: string;
  label: string;
}

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, "..", "..");

export const CLIENT_COMMAND = [path.join(ROOT, "dist/cli.cjs")];

const SUPPORTED_RUBY_VERSIONS = [
  "3.2.2",
  "3.2.6",
  "3.3.5",
  "3.3.6",
  "3.4.1",
  "3.4.2",
  "3.4.4",
] as const;

const RUBY_LANGUAGES: LanguageSpec[] = [
  { key: "ruby", label: "Ruby (ruby-lsp + sorbet)" },
  ...SUPPORTED_RUBY_VERSIONS.map((version) => ({
    key: `ruby-${version}`,
    label: `Ruby ${version} (ruby-lsp + sorbet)`,
  })),
  { key: "ruby-no-sorbet", label: "Ruby (ruby-lsp)" },
  ...SUPPORTED_RUBY_VERSIONS.map((version) => ({
    key: `ruby-no-sorbet-${version}`,
    label: `Ruby ${version} (ruby-lsp)`,
  })),
];

const ALL_LANGUAGES: LanguageSpec[] = [
  { key: "csharp", label: "C# (omnisharp)" },
  { key: "cpp", label: "C/C++ (clangd)" },
  { key: "go", label: "Go (gopls)" },
  { key: "java", label: "Java (jdtls)" },
  { key: "php", label: "PHP (phpactor)" },
  { key: "python", label: "Python (pyright-langserver)" },
  ...RUBY_LANGUAGES,
  { key: "rust", label: "Rust (rust-analyzer)" },
  { key: "ts", label: "TypeScript (typescript-language-server)" },
  { key: "js", label: "JavaScript (typescript-language-server)" },
];

function selectLanguages(): LanguageSpec[] {
  const raw = process.env.NUANCED_LANGUAGES ?? "all";
  const tokens = raw
    .split(",")
    .map((value) => value.trim().toLowerCase())
    .filter(Boolean);

  if (tokens.length === 0 || tokens.includes("all") || tokens.includes("*")) {
    return ALL_LANGUAGES;
  }

  const selected = ALL_LANGUAGES.filter((lang) => {
    const key = lang.key.toLowerCase();
    return tokens.some((token) => key === token || key.startsWith(`${token}-`));
  });
  if (selected.length === 0) {
    // If no languages match, default to all languages instead of just PHP
    return ALL_LANGUAGES;
  }
  return selected;
}

export const LANGUAGES: LanguageSpec[] = selectLanguages();

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

function languageIndex(key: string): number {
  const idx = LANGUAGES.findIndex((l) => l.key === key);
  return idx >= 0 ? idx : 0;
}

export function fixedPort(langKey: string): number {
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
    LANG_STRIDE * languageIndex(langKey)
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
export const PROXY_IMAGE_OVERRIDE = process.env.PROXY_IMAGE ?? "";
export const WATCHDOG_IMAGE_OVERRIDE = process.env.WATCHDOG_IMAGE ?? "";
export const WRAPPER_IMAGE_OVERRIDE = process.env.WRAPPER_IMAGE ?? "";
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
  const selected = LANGUAGES.map((lang) => lang.key).join(", ");
  console.warn(`[nuanced-lsp] Selected languages: ${selected}`);
}

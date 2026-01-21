#!/usr/bin/env node
import { Command } from "commander";
import { isErr } from "./types.js";
import type {
  BaseCommandOptions,
  DefinitionsInFileResult,
  DockerErr,
  DownCommandOptions,
  DownResult,
  FindDefinitionOptions,
  FindDefinitionResult,
  FindIdentifierOptions,
  FindIdentifierResult,
  FindReferencedSymbolsOptions,
  FindReferencedSymbolsResult,
  FindReferencesOptions,
  FindReferencesResult,
  HealthCommandOptions,
  HealthResult,
  HttpErr,
  ListFilesResult,
  LogsCommandOptions,
  LogsResult,
  LspCommandOptions,
  LspPosition,
  PullCommandOptions,
  PullResult,
  LspRange,
  ReadSourceCommandOptions,
  ReadSourceResult,
  Result,
  RunCommandOptions,
  RunResult,
  ServerCommandOptions,
  StatusCommandOptions,
  StatusResult,
  UpCommandOptions,
  UpResult,
} from "./types.js";
import {
  DEFAULT_BIND_HOST,
  DEFAULT_CONTAINER_NAME,
  DEFAULT_CONTAINER_REGISTRY,
  DEFAULT_HOST_PORT,
  DEFAULT_HOST_URL,
  DEFAULT_LANGUAGE_IMAGE_VERSION,
  DEFAULT_SERVICE_IMAGE_VERSION,
  DEFAULT_TIMEOUT_SECS,
  VERSION,
} from "./defaults.js";
import type { NuancedLspClient } from "./client.js";

async function generateRandomContainerName(): Promise<string> {
  const { randomUUID } = await import("node:crypto");
  const id = randomUUID().slice(0, 8);
  return `nuanced-lsp-${id}`;
}

// Lazy-load client (avoids startup cost if user only runs --help, etc.)
async function lspClient(opts: {
  containerName?: string;
  lspUrl?: string;
  lspPort?: number;
  timeout?: number;
  sudo?: boolean;
}): Promise<NuancedLspClient> {
  const { NuancedLspClient } = await import("./client.js");
  return new NuancedLspClient({
    containerName: opts.containerName ?? process.env.NUANCED_LSP_CONTAINER_NAME,
    proxyUrl: opts.lspUrl ?? process.env.NUANCED_LSP_URL,
    proxyPort:
      opts.lspPort ??
      (process.env.NUANCED_LSP_PORT
        ? parseInt(process.env.NUANCED_LSP_PORT)
        : undefined),
    timeoutSecs:
      opts.timeout ??
      (process.env.NUANCED_LSP_TIMEOUT
        ? parseInt(process.env.NUANCED_LSP_TIMEOUT)
        : undefined),
    sudo: opts.sudo,
  });
}

const ansi = {
  pink: (s: string) => `\x1b[38;2;248;172;255m${s}\x1b[0m`,
  yellow: (s: string) => `\x1b[38;2;255;221;51m${s}\x1b[0m`,
};
const log = {
  info: (msg: string) => console.log(ansi.pink(msg)),
  err: (msg: string) => console.error(msg),
  passThrough: (msg: string) => console.log(msg),
};
const ANSI_RE =
  // eslint-disable-next-line no-control-regex
  /[\u001B\u009B][[\]()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g;
const stripAnsi = (s: string) => s.replace(ANSI_RE, "");

function versionCommand(opts: BaseCommandOptions): void {
  if (opts.json) {
    log.passThrough(JSON.stringify({ version: VERSION }, null, 2));
    return;
  }
  log.info(`Nuanced LSP TypeScript CLI version: ${VERSION}`);
}

function parsePositionArg(position: string): LspPosition {
  const m = /^(\d+):(\d+)$/.exec(position.trim());
  if (!m)
    throw new Error(
      "Invalid <position> format. Expected 0-indexed 'line:char' (e.g., 0:0, 12:4).",
    );
  const line = parseInt(m[1], 10);
  const character = parseInt(m[2], 10);
  if (Number.isNaN(line) || Number.isNaN(character)) {
    throw new Error(
      "Invalid <position> numbers. Both line and char must be integers (0-indexed).",
    );
  }
  return { line, character };
}
function parseRange(spec: string): LspRange {
  const parts = spec.trim().split(/\s*-\s*/);
  if (parts.length !== 2) {
    throw new Error(
      `Invalid --range: ${spec} (expected format L:C-L:C, e.g. 0:0-10:5)`,
    );
  }
  const start = parsePositionArg(parts[0]);
  const end = parsePositionArg(parts[1]);
  const endsBeforeStart =
    end.line < start.line ||
    (end.line === start.line && end.character < start.character);
  if (endsBeforeStart) {
    throw new Error(
      `--range end must be >= start (got ${parts[0]} -> ${parts[1]})`,
    );
  }
  return { start, end };
}
function getSudoFlag(opts: unknown): boolean {
  return !!(opts as any)?.sudo;
}

function isDockerErr(x: unknown): x is DockerErr {
  return !!x && typeof x === "object" && "error_code" in x && "message" in x;
}
function isHttpErr(x: unknown): x is HttpErr {
  return !!x && typeof x === "object" && "status_code" in x && "error" in x;
}

export function prettyErr(err: unknown, fallback?: string): string {
  if (isDockerErr(err)) return prettyDockerErr(err, fallback);
  if (isHttpErr(err)) return prettyHttpErr(err, fallback);
  return fallback ?? "Error";
}

function prettyDockerErr(err: DockerErr, fallback?: string): string {
  const lines: string[] = [];
  const code = Number.isFinite(err.error_code) ? err.error_code : "n/a";
  lines.push(`Docker error (code: ${code})`);
  lines.push(`Message: ${err.message?.trim() || fallback || "Docker error"}`);
  if (err.stderr?.trim()) lines.push("stderr:\n" + err.stderr.trim());
  if (err.stdout?.trim()) lines.push("stdout:\n" + err.stdout.trim());
  return lines.join("\n");
}

function prettyHttpErr(err: HttpErr, fallback?: string): string {
  const code = err.status_code ?? "n/a";
  const details =
    typeof err.error === "string"
      ? err.error
      : JSON.stringify(err.error, null, 2);
  const header = `HTTP error (status: ${code})`;
  if (!details && fallback) return `${header}\nMessage: ${fallback}`;
  return `${header}\nDetails:\n${details}`;
}

function handleResult<T, E>(
  res: Result<T, E>,
  opts: {
    json?: boolean;
    fallbackErrMsg: string;
    onError?: (data: E) => void;
    onErrorJson?: (data: E) => void;
    onSuccess?: (data: T) => void;
    onSuccessJson?: (data: T) => void;
  },
): void {
  // Handle errors first. All errors set process.exitCode = 1.
  if (isErr(res)) {
    const e = res.data;
    process.exitCode = 1;

    // If JSON output is requsted, prefer onErrorJson if provided, otherwise fallback to default JSON output.
    if (opts.json) {
      if (opts.onErrorJson) return opts.onErrorJson(e);

      // Default JSON output.
      return log.err(JSON.stringify(e, null, 2));
    }

    if (opts.onError) return opts.onError(e);
    return log.err(prettyErr(e, opts.fallbackErrMsg));
  }

  // If JSON output is requsted, prefer onSuccessJson if provided, otherwise fallback to default JSON output.
  if (opts.json) {
    if (opts.onSuccessJson) return opts.onSuccessJson(res.data);

    // Default JSON output.
    return log.passThrough(JSON.stringify(res.data, null, 2));
  }

  // If we have a custom onSuccess handler, use it.
  if (opts.onSuccess) return opts.onSuccess(res.data);

  // Otherwise, default to just printing the data as a string.
  return log.passThrough(String(res.data));
}

async function upCommand(
  workspace: string,
  opts: UpCommandOptions,
): Promise<void> {
  const client = await lspClient({
    ...opts,
    lspPort: opts.hostPort,
  });

  if (!opts.json)
    log.info(`Starting Nuanced LSP container '${client.containerName}'...`);

  const res = await client.up(workspace, {
    containerRegistry:
      opts.containerRegistry ?? process.env.CONTAINER_REGISTRTY,
    languageImageVersion:
      opts.languageImageVersion ?? process.env.LANGUAGE_IMAGE_VERSION,
    serviceImageVersion:
      opts.serviceImageVersion ?? process.env.SERVICE_IMAGE_VERSION,
    timeout: opts.timeout,
    stream: opts.stream,
    ro: opts.ro,
    bindHost: opts.bindHost,
    debug: opts.debug,
    env: opts.env,
    envFile: opts.envFile,
  });

  handleResult<UpResult, DockerErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: `Failed to start container '${client.containerName}'`,
    onSuccess: (data: UpResult) => {
      log.info(
        `Nuanced LSP container '${client.containerName}' bound at ${data.base_url}:${data.host_port}`,
      );
    },
  });
}

async function serverCommand(
  workspace: string,
  opts: ServerCommandOptions,
): Promise<void> {
  const client = await lspClient({
    ...opts,
    containerName: opts.containerName ?? (await generateRandomContainerName()),
    lspPort: opts.hostPort ?? 0,
  });

  try {
    // Run the LSP server stdio loop
    const { runLspServer } = await import("./server.js");
    await runLspServer(client, workspace, opts, process.stdin, process.stdout);
  } catch {
    // Fatal errors are already logged via window/logMessage
    process.exitCode = 1;
  }
}

async function downCommand(opts: DownCommandOptions): Promise<void> {
  const client = await lspClient({
    ...opts,
    sudo: getSudoFlag(opts),
  });

  if (!opts.json)
    log.info(`Stopping Nuanced LSP container '${client.containerName}'...`);
  const res = await client.down();

  handleResult<DownResult, DockerErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: `Failed to stop container '${client.containerName}'`,
    onSuccess: (_data: DownResult) => {
      log.info(`Nuanced LSP container '${client.containerName}' stopped.`);
    },
  });
}

async function logsCommand(opts: LogsCommandOptions): Promise<void> {
  const client = await lspClient(opts);
  const tail: number | "all" | undefined =
    opts.tail === undefined
      ? undefined
      : typeof opts.tail === "number"
        ? opts.tail
        : opts.tail === "all"
          ? "all"
          : Number.isFinite(Number(opts.tail))
            ? Number(opts.tail)
            : undefined;

  // Streaming in this context blocks until the user interrupts (e.g., Ctrl-C), so it is not handled via handleResult.
  if (opts.stream) {
    await client.logs({ stream: opts.stream, since: opts.since, tail });
    return;
  }

  const res = await client.logs({ since: opts.since, tail });

  handleResult<LogsResult, DockerErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: `Failed to read logs for '${client.containerName}'`,
    onSuccess: (data: LogsResult) => {
      log.passThrough(data.stdout ?? "");
    },
    onSuccessJson: (data: LogsResult) => {
      // Strip ANSI color codes from logs in JSON mode.
      const cleanLogs = data.stdout ? stripAnsi(data.stdout) : "";
      log.passThrough(JSON.stringify({ ...data, stdout: cleanLogs }, null, 2));
    },
  });
}

async function runCommand(
  script: string,
  opts: RunCommandOptions,
): Promise<void> {
  const client = await lspClient({
    ...opts,
    sudo: getSudoFlag(opts),
  });

  if (!opts.json)
    log.info(
      `Running script '${script}' in container '${client.containerName}'...`,
    );

  const res = await client.run(script, {
    stream: opts.stream,
    env: opts.env,
    envFile: opts.envFile,
  });

  handleResult<RunResult, DockerErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: `Script execution failed in '${client.containerName}'`,
    onSuccess: (_data: RunResult) =>
      log.info(`Script executed successfully in '${client.containerName}'`),
  });
}

async function statusCommand(opts: StatusCommandOptions): Promise<void> {
  const client = await lspClient({
    ...opts,
    sudo: getSudoFlag(opts),
  });

  if (!opts.json)
    log.info(`Checking status for container '${client.containerName}'...`);

  const res = await client.status();

  handleResult<StatusResult, DockerErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: `Failed to get status for '${client.containerName}'`,
    onSuccess: (data: StatusResult) => {
      log.info(`Container: ${data.container_name}`);
      if (data.container_status)
        log.info(`Docker:    ${data.container_status}`);
    },
  });
}

async function pullCommand(opts: PullCommandOptions): Promise<void> {
  const client = await lspClient({
    sudo: getSudoFlag(opts),
  });

  // Parse services
  let services: "all" | string[] | undefined;
  if (opts.allServices) {
    services = "all";
  } else if (opts.services) {
    services = opts.services
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
  }

  // Parse languages
  let languages: "all" | string[] | undefined;
  if (opts.allLanguages) {
    languages = "all";
  } else if (opts.languages) {
    languages = opts.languages
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
  }

  // Validate at least one of services or languages is specified
  if (services == undefined && languages == undefined) {
    log.err(
      "At least one of, --all-languages --all-services, --languages=, or --services= must be specified",
    );
    process.exit(1);
  }

  const res = await client.pull({
    services,
    languages,
    containerRegistry: opts.containerRegistry,
    languageImageVersion: opts.languageImageVersion,
    serviceImageVersion: opts.serviceImageVersion,
    stream: opts.stream,
  });

  handleResult<PullResult[], DockerErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Failed to pull images",
    onSuccess: (data: PullResult[]) => {
      for (const result of data) {
        log.info(`Successfully pulled image: ${result.image}`);
      }
    },
  });
}

async function healthCommand(opts: HealthCommandOptions): Promise<void> {
  const client = await lspClient(opts);

  const res = await client.health(opts.timeout);

  handleResult<HealthResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Health check failed",
    onSuccess: (data: HealthResult) => {
      log.info("Nuanced LSP system is healthy.");
      log.passThrough(JSON.stringify(data, null, 2));
    },
  });
}

async function listFilesCommand(opts: LspCommandOptions): Promise<void> {
  const client = await lspClient(opts);

  const res = await client.listFiles(opts.timeout);

  handleResult<ListFilesResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "List files failed",
    onSuccess: (data: ListFilesResult) => {
      for (const f of data) log.passThrough(f);
    },
  });
}

async function readSourceCommand(
  file: string,
  opts: ReadSourceCommandOptions,
): Promise<void> {
  const client = await lspClient(opts);

  const res = await client.readSource(file, opts.range ?? null, opts.timeout);

  handleResult<ReadSourceResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Read source failed",
    onSuccess: (data: ReadSourceResult) => {
      log.passThrough(data.source_code);
    },
  });
}

async function definitionsInFileCommand(
  file: string,
  opts: LspCommandOptions,
): Promise<void> {
  const client = await lspClient(opts);

  const res = await client.definitionsInFile(file, (opts as any).timeout);

  handleResult<DefinitionsInFileResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Definitions in file failed",
    onSuccess: (data: DefinitionsInFileResult) => {
      log.passThrough(JSON.stringify(data, null, 2));
    },
  });
}

async function findDefinitionCommand(
  file: string,
  rawPosition: string,
  opts: FindDefinitionOptions,
): Promise<void> {
  const position = parsePositionArg(rawPosition);
  const client = await lspClient(opts);

  const res = await client.findDefinition(
    { path: file, position },
    !!opts.includeRawResponse,
    !!opts.includeSourceCode,
    opts.timeout,
  );

  handleResult<FindDefinitionResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Find definition failed",
    onSuccess: (data: FindDefinitionResult) => {
      log.passThrough(JSON.stringify(data, null, 2));
    },
  });
}

async function findIdentifierCommand(
  file: string,
  name: string,
  opts: FindIdentifierOptions,
): Promise<void> {
  const position = opts.position ? parsePositionArg(opts.position) : null;
  const client = await lspClient(opts);

  const res = await client.findIdentifier(file, name, position, opts.timeout);

  handleResult<FindIdentifierResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Find identifier failed",
    onSuccess: (data: FindIdentifierResult) => {
      log.passThrough(JSON.stringify(data, null, 2));
    },
  });
}

async function findReferencedSymbolsCommand(
  file: string,
  rawPosition: string,
  opts: FindReferencedSymbolsOptions,
): Promise<void> {
  const position = parsePositionArg(rawPosition);
  const identifierPosition = { path: file, position };
  const client = await lspClient(opts);

  const res = await client.findReferencedSymbols(
    identifierPosition,
    !!opts.fullScan,
    (opts as any).timeout,
  );

  handleResult<FindReferencedSymbolsResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Find referenced symbols failed",
    onSuccess: (data: FindReferencedSymbolsResult) => {
      log.passThrough(JSON.stringify(data, null, 2));
    },
  });
}

async function findReferencesCommand(
  file: string,
  rawPosition: string,
  opts: FindReferencesOptions,
): Promise<void> {
  const position = parsePositionArg(rawPosition);
  const identifierPosition = { path: file, position };
  const client = await lspClient(opts);

  const res = await client.findReferences(
    identifierPosition,
    opts.includeCodeContextLines,
    opts.includeRawResponse,
    (opts as any).timeout,
  );

  handleResult<FindReferencesResult, HttpErr>(res, {
    json: !!opts.json,
    fallbackErrMsg: "Find references failed",
    onSuccess: (data: FindReferencesResult) => {
      log.passThrough(JSON.stringify(data, null, 2));
    },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Banner
// ─────────────────────────────────────────────────────────────────────────────
const banner = String.raw`
┌───┐       ┌───┐       ┌───┐
│ N │ ━━━━━ │ u │ ━━━━━ │ a │
└───┘       └───┘       └───┘
              │
              │
              │
            ┌───┐       ┌───┐
            │ n │ ━━━━━ │ c │
            └───┘       └───┘
                          │
                          │
                          │
                        ┌───┐       ┌───┐
                        │ e │ ━━━━━ │ d │
                        └───┘       └───┘
                                      │
                                      │
                                      │
                                    ┌───┐       ┌───┐       ┌───┐
                                    │ L │ ━━━━━ │ S │ ━━━━━ │ P │
                                    └───┘       └───┘       └───┘
`.trimEnd();

function gsub(
  s: string,
  from: string | RegExp,
  to: string | ((m: string) => string),
): string {
  const re =
    typeof from === "string"
      ? new RegExp(from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "g")
      : from;
  return typeof to === "string"
    ? s.replace(re, to)
    : s.replace(re, (m: string) => to(m));
}
function renderBanner(): string {
  let out = banner;
  out = gsub(out, /[┌┐└┘─━│]/g, (m: string) => ansi.pink(m));
  out = out.replace(/│\s([A-Za-z])\s│/g, (_m, ch: string) => `│ ${ch} │`);
  return out;
}

// ─────────────────────────────────────────────────────────────────────────────
// Commander wiring
// ─────────────────────────────────────────────────────────────────────────────
const program = new Command();
program.addHelpText("before", () => renderBanner() + "\n");

program
  .name("nuanced-lsp")
  .description(ansi.pink("Nuanced LSP CLI"))
  .version(`${VERSION}`);

program
  .command("version")
  .description(ansi.pink("Print the Nuanced LSP client version"))
  .option("--json", "Output machine-readable JSON") // harmless, consistent
  .action(versionCommand);

program.commandsGroup("Lifecycle commands:");

program
  .command("up")
  .description(ansi.pink("Start the Nuanced LSP container locally in Docker."))
  .argument("<workspace>", "Host workspace directory to mount")
  .option(
    "--host-port <n>",
    `Host port to map to ${DEFAULT_HOST_PORT}. Note: Use port 0 for a dynamically assigned host port from Docker.`,
    (v: string) => parseInt(v, 10),
  )
  .option(
    "--bind-host <host>",
    "Host/IP to bind (e.g. 127.0.0.1, 0.0.0.0, 192.168.1.10)",
    DEFAULT_BIND_HOST,
  )
  .option(
    "--container-registry <registry>",
    `Container registry (default: ${DEFAULT_CONTAINER_REGISTRY})`,
  )
  .option(
    "--language-image-version <version>",
    "Language image version (default: from service)",
  )
  .option(
    "--service-image-version <version>",
    `Nuanced LSP service image version (default: ${DEFAULT_SERVICE_IMAGE_VERSION})`,
  )
  .option(
    "--container-name <name>",
    `Container name (default: ${DEFAULT_CONTAINER_NAME})`,
  )
  .option(
    "--timeout <s>",
    `Health check poll loop timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--sudo", "Run Docker commands with sudo")
  .option(
    "--stream",
    "Stream process stdout, stderr, and container logs during startup",
  )
  .option("--ro", "Mount workspace as read-only (default is read-write)")
  .option("--json", "Output machine-readable JSON")
  .option("--debug", "Run the container with debug logging enabled")
  .option(
    "--env <k=v>",
    "Set environment variable (repeatable)",
    (v: string, prev: string[] | undefined) => (prev ? prev.concat(v) : [v]),
  )
  .option("--env-file <path>", "Path to an env file")
  .action(upCommand);

program
  .command("server")
  .description(
    ansi.pink(
      "Start a stdio LSP server that forwards requests to the Nuanced LSP container.",
    ),
  )
  .argument("<workspace>", "Host workspace directory to mount")
  .option(
    "--container-name <name>",
    "Name of an already running container to use (default: start new container with random name).",
  )
  .option(
    "--host-port <n>",
    `Host port to map to ${DEFAULT_HOST_PORT} (default: port 0 for a dynamically assigned host port from Docker).`,
    (v: string) => parseInt(v, 10),
  )
  .option(
    "--bind-host <host>",
    "Host/IP to bind (e.g. 127.0.0.1, 0.0.0.0, 192.168.1.10)",
    DEFAULT_BIND_HOST,
  )
  .option(
    "--container-registry <registry>",
    `Container registry (default: ${DEFAULT_CONTAINER_REGISTRY})`,
  )
  .option(
    "--language-image-version <version>",
    "Language image version (default: from service)",
  )
  .option(
    "--service-image-version <version>",
    `Nuanced LSP service image version (default: ${DEFAULT_SERVICE_IMAGE_VERSION})`,
  )
  .option(
    "--timeout <s>",
    `Health check poll loop timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--sudo", "Run Docker commands with sudo")
  .option("--ro", "Mount workspace as read-only (default is read-write)")
  .option("--debug", "Run the container with debug logging enabled")
  .option(
    "--env <k=v>",
    "Set environment variable (repeatable)",
    (v: string, prev: string[] | undefined) => (prev ? prev.concat(v) : [v]),
  )
  .option("--env-file <path>", "Path to an env file")
  .action(serverCommand);

program
  .command("down")
  .description(ansi.pink("Stop the Nuanced LSP container."))
  .option(
    "--container-name <name>",
    `Container name (default: ${DEFAULT_CONTAINER_NAME})`,
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--sudo", "Run Docker commands with sudo")
  .option("--json", "Output machine-readable JSON")
  .action(downCommand);

program
  .command("logs")
  .description(ansi.pink("Print container logs."))
  .option(
    "--container-name <name>",
    `Container name (default: ${DEFAULT_CONTAINER_NAME})`,
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--sudo", "Run Docker commands with sudo")
  .option("--stream", "Stream Docker logs (i.e. docker logs -f)")
  .option("--since <when>", "Only show logs since (e.g., 10s, 5m, RFC3339)")
  .option("--tail <n|all>", "Show last N lines (or 'all')")
  .option("--json", "Output machine-readable JSON")
  .action(logsCommand);

program
  .command("run")
  .description(
    ansi.pink("Run a workspace script inside the container (as root)."),
  )
  .argument("<script>", "Path to a script relative to the mounted workspace")
  .option(
    "--container-name <name>",
    `Container name (default: ${DEFAULT_CONTAINER_NAME})`,
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--sudo", "Run Docker commands with sudo")
  .option("--stream", "Stream script process stdout / stderr")
  .option("--json", "Output machine-readable JSON")
  .option(
    "--env <k=v>",
    "Set environment variable (repeatable)",
    (v: string, prev: string[] | undefined) => (prev ? prev.concat(v) : [v]),
  )
  .option("--env-file <path>", "Path to an env file")
  .action(runCommand);

program
  .command("status")
  .description(ansi.pink("Show Docker lifecycle status (no /health probe)."))
  .option("--json", "Output machine-readable JSON")
  .option(
    "--container-name <name>",
    `Container name (default: ${DEFAULT_CONTAINER_NAME})`,
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--sudo", "Run Docker commands with sudo")
  .action(statusCommand);

program
  .command("pull")
  .description(ansi.pink("Pull Nuanced LSP images."))
  .option(
    "--container-registry <registry>",
    `Container registry (default: ${DEFAULT_CONTAINER_REGISTRY})`,
  )
  .option("--all-languages", "Pull all language images")
  .option("--all-services", "Pull all service images")
  .option(
    "--language-image-version <version>",
    `Language image version (default: ${DEFAULT_LANGUAGE_IMAGE_VERSION})`,
  )
  .option(
    "--languages <list>",
    "Pull specific language images (comma-separated)",
  )
  .option(
    "--service-image-version <version>",
    `Service image version (default: ${DEFAULT_SERVICE_IMAGE_VERSION})`,
  )
  .option(
    "--services <list>",
    "Pull specific service images (comma-separated: proxy,watchdog,wrapper)",
  )
  .option("--sudo", "Run Docker commands with sudo")
  .option("--stream", "Stream process stdout and stderr")
  .option("--json", "Output machine-readable JSON")
  .action(pullCommand);

program.commandsGroup("System commands:");

program
  .command("health")
  .description(ansi.pink("Check Nuanced LSP system health."))
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(healthCommand);

program.commandsGroup("Workspace commands:");

program
  .command("list-files")
  .description(ansi.pink("List files in the workspace."))
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(listFilesCommand);

program
  .command("read-source <file>")
  .description(ansi.pink("Read full source of file."))
  .option(
    "--range <line:char-line:char>",
    "Optional range as 0-indexed line:char-line:char (e.g., 0:0-10:5)",
    parseRange,
  )
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(readSourceCommand);

program.commandsGroup("Symbols commands:");

program
  .command("definitions-in-file <file>")
  .description(ansi.pink("List definitions in a file."))
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(definitionsInFileCommand);

program
  .command("find-definition <file> <position>")
  .description(
    ansi.pink("Find definition at position (0-indexed line:char, e.g., 0:0)."),
  )
  .option("--include-raw-response", "Include raw LSP response in output")
  .option("--include-source-code", "Include raw LSP response in output")
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(findDefinitionCommand);

program
  .command("find-identifier <file> <name>")
  .description(ansi.pink("Find identifiers by name in a file."))
  .option(
    "--position <line:char>",
    "Optional position as 0-indexed line:char (e.g., 0:0)",
  )
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(findIdentifierCommand);

program
  .command("find-referenced-symbols <file> <position>")
  .description(
    ansi.pink(
      "Find symbols referenced by the identifier at the given position (0-indexed line:char, e.g., 0:0).",
    ),
  )
  .option("--full-scan", "Perform a full workspace scan for references")
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(findReferencedSymbolsCommand);

program
  .command("find-references <file> <position>")
  .description(
    ansi.pink(
      "Find all references to the identifier at the given position (0-indexed line:char, e.g., 0:0).",
    ),
  )
  .option(
    "--context-lines, --include-code-context-lines <n>",
    "Include N lines of context around symbols retrieved",
    (v) => parseInt(v, 10),
    0,
  )
  .option("--include-raw-response", "Include raw LSP response in output")
  .option("--lsp-url <url>", `Nuanced LSP URL (default: ${DEFAULT_HOST_URL})`)
  .option(
    "--lsp-port <n>",
    `Port for Nuanced LSP (default: ${DEFAULT_HOST_PORT})`,
    (v) => parseInt(v, 10),
  )
  .option(
    "--timeout <s>",
    `Timeout seconds (<=0 to skip) (default: ${DEFAULT_TIMEOUT_SECS})`,
    (v: string) => parseInt(v, 10),
  )
  .option("--json", "Output machine-readable JSON")
  .action(findReferencesCommand);

try {
  program.parse(process.argv);
} catch (err: any) {
  log.err(String(err?.message ?? err));
  process.exit(1);
}

"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __esm = (fn, res) => function __init() {
  return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/__generated/version.ts
var DEFAULT_PROXY_IMAGE, DEFAULT_WRAPPER_IMAGE, DEFAULT_WATCHDOG_IMAGE, LANGUAGE_CONTAINER_VERSION;
var init_version = __esm({
  "src/__generated/version.ts"() {
    "use strict";
    DEFAULT_PROXY_IMAGE = "ghcr.io/nuanced-dev/nuanced-lsp-proxy:0.4.8";
    DEFAULT_WRAPPER_IMAGE = "ghcr.io/nuanced-dev/nuanced-lsp-wrapper:0.4.8";
    DEFAULT_WATCHDOG_IMAGE = "ghcr.io/nuanced-dev/nuanced-lsp-watchdog:0.4.8";
    LANGUAGE_CONTAINER_VERSION = "1.0.0";
  }
});

// src/defaults.ts
var DEFAULT_BIND_HOST, DEFAULT_CONTAINER_NAME, DEFAULT_CONTAINER_PORT, DEFAULT_HOST_PORT, DEFAULT_HOST_URL, DEFAULT_MOUNT_DIR, DEFAULT_PROXY_IMAGE2, DEFAULT_RETRIES, DEFAULT_TIMEOUT_SECS, DEFAULT_WATCHDOG_IMAGE2, DEFAULT_WRAPPER_IMAGE2;
var init_defaults = __esm({
  "src/defaults.ts"() {
    "use strict";
    init_version();
    DEFAULT_BIND_HOST = "127.0.0.1";
    DEFAULT_CONTAINER_NAME = "nuanced-lsp";
    DEFAULT_CONTAINER_PORT = 4444;
    DEFAULT_HOST_PORT = 4444;
    DEFAULT_HOST_URL = "http://127.0.0.1";
    DEFAULT_MOUNT_DIR = "/mnt/workspace";
    DEFAULT_PROXY_IMAGE2 = DEFAULT_PROXY_IMAGE;
    DEFAULT_RETRIES = 5;
    DEFAULT_TIMEOUT_SECS = 120;
    DEFAULT_WATCHDOG_IMAGE2 = DEFAULT_WATCHDOG_IMAGE;
    DEFAULT_WRAPPER_IMAGE2 = DEFAULT_WRAPPER_IMAGE;
  }
});

// src/types.ts
var import_zod, ok, err, isOk, isErr, DockerErrSchema, DownResultSchema, UpResultSchema, RunResultSchema, StatusResultSchema, LogsResultSchema, PullResultSchema, HttpErrSchema, HealthResultSchema, LspPositionSchema, LspRangeSchema, FilePositionSchema, FileRangeSchema, IdentifierPositionSchema, SelectedIdentifierSchema, ListFilesResultSchema, ReadSourceResultSchema, DefinitionLocationSchema, ContextSnippetSchema, DefinitionInFileSchema, DefinitionsInFileResultSchema, FindDefinitionResultSchema, IdentifierSchema, FindIdentifierResultSchema, ExternalSymbolSchema, NotFoundSymbolSchema, WorkspaceDefinitionSchema, WorkspaceReferenceSchema, WorkspaceSymbolSchema, FindReferencedSymbolsResultSchema, ReferenceLocationSchema, FindReferencesResultSchema, BaseCommandOptionsSchema, UpCommandOptionsSchema, DownCommandOptionsSchema, LogsCommandOptionsSchema, RunCommandOptionsSchema, StatusCommandOptionsSchema, PullCommandOptionsSchema, LspCommandOptionsSchema, FindDefinitionOptionsSchema, ReadSourceCommandOptionsSchema, HealthCommandOptionsSchema, FindIdentifierOptionsSchema, FindReferencedSymbolsOptionsSchema, FindReferencesOptionsSchema;
var init_types = __esm({
  "src/types.ts"() {
    "use strict";
    import_zod = require("zod");
    ok = (data) => ({ ok: true, data });
    err = (data) => ({ ok: false, data });
    isOk = (r) => r.ok;
    isErr = (r) => !r.ok;
    DockerErrSchema = import_zod.z.object({
      error_code: import_zod.z.number(),
      message: import_zod.z.string(),
      stdout: import_zod.z.string(),
      stderr: import_zod.z.string()
    });
    DownResultSchema = import_zod.z.object({
      stdout: import_zod.z.string()
    });
    UpResultSchema = import_zod.z.object({
      host_port: import_zod.z.number(),
      base_url: import_zod.z.string()
    });
    RunResultSchema = import_zod.z.object({
      stdout: import_zod.z.string()
    });
    StatusResultSchema = import_zod.z.object({
      container_name: import_zod.z.string(),
      container_status: import_zod.z.string()
    });
    LogsResultSchema = import_zod.z.object({
      stdout: import_zod.z.string()
    });
    PullResultSchema = import_zod.z.object({
      image: import_zod.z.string(),
      stdout: import_zod.z.string()
    });
    HttpErrSchema = import_zod.z.object({
      status_code: import_zod.z.number().nullable(),
      error: import_zod.z.record(import_zod.z.any())
    });
    HealthResultSchema = import_zod.z.object({
      status: import_zod.z.union([import_zod.z.literal("ok"), import_zod.z.literal("not ok")]),
      version: import_zod.z.string().optional(),
      languages: import_zod.z.record(import_zod.z.boolean()).optional()
    });
    LspPositionSchema = import_zod.z.object({
      line: import_zod.z.number().int().min(0),
      character: import_zod.z.number().int().min(0)
    });
    LspRangeSchema = import_zod.z.object({
      start: LspPositionSchema,
      end: LspPositionSchema
    });
    FilePositionSchema = import_zod.z.object({
      path: import_zod.z.string(),
      position: LspPositionSchema
    });
    FileRangeSchema = import_zod.z.object({
      path: import_zod.z.string(),
      range: LspRangeSchema
    });
    IdentifierPositionSchema = import_zod.z.object({
      path: import_zod.z.string(),
      position: LspPositionSchema
    });
    SelectedIdentifierSchema = import_zod.z.object({
      file_range: FileRangeSchema,
      kind: import_zod.z.string().nullable(),
      name: import_zod.z.string()
    });
    ListFilesResultSchema = import_zod.z.array(import_zod.z.string());
    ReadSourceResultSchema = import_zod.z.object({
      source_code: import_zod.z.string()
    });
    DefinitionLocationSchema = import_zod.z.object({
      path: import_zod.z.string(),
      position: LspPositionSchema
    });
    ContextSnippetSchema = import_zod.z.object({
      file_range: FileRangeSchema.optional(),
      range: FileRangeSchema.optional(),
      source_code: import_zod.z.string()
    }).superRefine((data, ctx) => {
      if (!data.file_range && !data.range) {
        ctx.addIssue({
          code: import_zod.z.ZodIssueCode.custom,
          message: "one of file_range or range is required"
        });
      }
    });
    DefinitionInFileSchema = import_zod.z.object({
      file_range: FileRangeSchema,
      identifier_position: IdentifierPositionSchema,
      kind: import_zod.z.string(),
      name: import_zod.z.string()
    });
    DefinitionsInFileResultSchema = import_zod.z.array(DefinitionInFileSchema);
    FindDefinitionResultSchema = import_zod.z.object({
      definitions: import_zod.z.array(DefinitionLocationSchema),
      selected_identifier: SelectedIdentifierSchema,
      raw_response: import_zod.z.unknown(),
      source_code_context: import_zod.z.array(ContextSnippetSchema).nullable()
    });
    IdentifierSchema = import_zod.z.object({
      file_range: FileRangeSchema,
      kind: import_zod.z.string().nullable(),
      name: import_zod.z.string()
    });
    FindIdentifierResultSchema = import_zod.z.object({
      identifiers: import_zod.z.array(IdentifierSchema)
    });
    ExternalSymbolSchema = import_zod.z.object({
      file_range: FileRangeSchema,
      kind: import_zod.z.string().nullable(),
      name: import_zod.z.string()
    });
    NotFoundSymbolSchema = ExternalSymbolSchema;
    WorkspaceDefinitionSchema = import_zod.z.object({
      file_range: FileRangeSchema,
      identifier_position: IdentifierPositionSchema,
      kind: import_zod.z.string().nullable(),
      name: import_zod.z.string()
    });
    WorkspaceReferenceSchema = import_zod.z.object({
      file_range: FileRangeSchema,
      kind: import_zod.z.string().nullable(),
      name: import_zod.z.string()
    });
    WorkspaceSymbolSchema = import_zod.z.object({
      reference: WorkspaceReferenceSchema,
      definitions: import_zod.z.array(WorkspaceDefinitionSchema)
    });
    FindReferencedSymbolsResultSchema = import_zod.z.object({
      external_symbols: import_zod.z.array(ExternalSymbolSchema),
      not_found: import_zod.z.array(NotFoundSymbolSchema),
      workspace_symbols: import_zod.z.array(WorkspaceSymbolSchema)
    });
    ReferenceLocationSchema = import_zod.z.object({
      path: import_zod.z.string(),
      position: LspPositionSchema
    });
    FindReferencesResultSchema = import_zod.z.object({
      references: import_zod.z.array(ReferenceLocationSchema),
      selected_identifier: SelectedIdentifierSchema,
      context: import_zod.z.array(ContextSnippetSchema).nullable(),
      raw_response: import_zod.z.unknown()
    });
    BaseCommandOptionsSchema = import_zod.z.object({
      json: import_zod.z.boolean().optional()
    });
    UpCommandOptionsSchema = BaseCommandOptionsSchema.extend({
      containerName: import_zod.z.string().optional(),
      hostPort: import_zod.z.number().int().min(0).optional(),
      languageContainerVersion: import_zod.z.string().optional(),
      proxyImage: import_zod.z.string().optional(),
      wrapperImage: import_zod.z.string().optional(),
      watchdogImage: import_zod.z.string().optional(),
      timeout: import_zod.z.number().optional(),
      sudo: import_zod.z.boolean().optional(),
      stream: import_zod.z.boolean().optional(),
      ro: import_zod.z.boolean().optional(),
      bindHost: import_zod.z.string().optional(),
      debug: import_zod.z.boolean().optional(),
      env: import_zod.z.array(import_zod.z.string()).optional(),
      envFile: import_zod.z.string().optional()
    });
    DownCommandOptionsSchema = BaseCommandOptionsSchema.extend({
      containerName: import_zod.z.string().optional(),
      sudo: import_zod.z.boolean().optional(),
      timeout: import_zod.z.number().optional()
    });
    LogsCommandOptionsSchema = BaseCommandOptionsSchema.extend({
      containerName: import_zod.z.string().optional(),
      timeout: import_zod.z.number().optional(),
      sudo: import_zod.z.boolean().optional(),
      stream: import_zod.z.boolean().optional(),
      since: import_zod.z.string().optional(),
      tail: import_zod.z.union([import_zod.z.number().int().min(0), import_zod.z.literal("all")]).optional()
    });
    RunCommandOptionsSchema = BaseCommandOptionsSchema.extend({
      containerName: import_zod.z.string().optional(),
      sudo: import_zod.z.boolean().optional(),
      stream: import_zod.z.boolean().optional(),
      timeout: import_zod.z.number().optional(),
      env: import_zod.z.array(import_zod.z.string()).optional(),
      envFile: import_zod.z.string().optional()
    });
    StatusCommandOptionsSchema = BaseCommandOptionsSchema.extend({
      containerName: import_zod.z.string().optional(),
      sudo: import_zod.z.boolean().optional(),
      timeout: import_zod.z.number().optional()
    });
    PullCommandOptionsSchema = BaseCommandOptionsSchema.extend({
      image: import_zod.z.string().optional(),
      stream: import_zod.z.boolean().optional(),
      sudo: import_zod.z.boolean().optional()
    });
    LspCommandOptionsSchema = BaseCommandOptionsSchema.extend({
      lspUrl: import_zod.z.string().optional(),
      lspPort: import_zod.z.number().int().min(0).optional(),
      timeout: import_zod.z.number().optional()
    });
    FindDefinitionOptionsSchema = LspCommandOptionsSchema.extend({
      includeRawResponse: import_zod.z.boolean().optional(),
      includeSourceCode: import_zod.z.boolean().optional()
    });
    ReadSourceCommandOptionsSchema = LspCommandOptionsSchema.extend({
      range: import_zod.z.lazy(() => LspRangeSchema).nullish()
      // keep laziness safe for type references
    });
    HealthCommandOptionsSchema = LspCommandOptionsSchema.extend({});
    FindIdentifierOptionsSchema = LspCommandOptionsSchema.extend({
      position: import_zod.z.string().optional()
    });
    FindReferencedSymbolsOptionsSchema = LspCommandOptionsSchema.extend({
      fullScan: import_zod.z.boolean().optional()
    });
    FindReferencesOptionsSchema = LspCommandOptionsSchema.extend({
      includeCodeContextLines: import_zod.z.number().int().min(0).optional(),
      includeRawResponse: import_zod.z.boolean().optional()
    });
  }
});

// src/http.ts
var http_exports = {};
__export(http_exports, {
  doRequest: () => doRequest,
  httpRequestWithRetries: () => httpRequestWithRetries,
  pollHttpWithRetries: () => pollHttpWithRetries
});
function parseErrObject(err2) {
  if (!err2) return null;
  if (typeof err2 === "object") return err2;
  if (typeof err2 === "string") {
    try {
      return JSON.parse(err2);
    } catch {
    }
    const m = /code=([A-Z_]+)/.exec(err2);
    return m ? { code: m[1] } : { message: err2 };
  }
  return { message: String(err2) };
}
function shouldRetryResult(res) {
  if (res.ok) return false;
  const e = parseErrObject(res.data?.error);
  const name = e?.name;
  if (name === "AbortError") return true;
  const code = e?.code;
  const RETRYABLE_CODES = /* @__PURE__ */ new Set([
    "ETIMEDOUT",
    "ECONNRESET",
    "ECONNREFUSED",
    "EAI_AGAIN",
    "ENETUNREACH",
    "EHOSTUNREACH",
    "EPIPE",
    "EAGAIN",
    "UND_ERR_SOCKET"
  ]);
  if (code && RETRYABLE_CODES.has(code)) return true;
  return false;
}
function formatHttpErrorForResult(err2, url, timeoutMs) {
  const c = err2?.cause ?? {};
  return {
    name: err2?.name ?? "Error",
    message: err2?.message ?? String(err2),
    code: c.code,
    errno: c.errno,
    syscall: c.syscall,
    address: c.address,
    port: c.port,
    timeoutMs,
    url: url.toString()
  };
}
async function doRequest(method, baseUrl, path2, json, params, timeoutMs = 1e4) {
  const url = new URL(path2, baseUrl);
  if (params)
    for (const [k, v] of Object.entries(params))
      url.searchParams.set(k, String(v));
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, {
      method,
      headers: method === "POST" ? { "Content-Type": "application/json" } : void 0,
      body: method === "POST" ? JSON.stringify(json ?? {}) : void 0,
      signal: controller.signal
    });
    if (!res.ok) {
      let body = null;
      try {
        body = await res.json();
      } catch {
        body = null;
      }
      return err({
        status_code: res.status,
        error: {
          message: body?.error ?? `HTTP ${res.status}`,
          url: url.toString()
        }
      });
    }
    const data = await res.json();
    return ok(data);
  } catch (e) {
    return err({
      status_code: null,
      error: formatHttpErrorForResult(e, url, timeoutMs)
    });
  } finally {
    clearTimeout(timer);
  }
}
async function httpRequestWithRetries(method, path2, baseUrl, json, params, retries = DEFAULT_RETRIES, timeoutSecs = DEFAULT_TIMEOUT_SECS) {
  for (let attempt = 0; ; attempt += 1) {
    const res = await doRequest(
      method,
      baseUrl,
      path2,
      json,
      params,
      timeoutSecs * 1e3
    );
    if (res.ok) return res;
    if (!shouldRetryResult(res)) return res;
    if (attempt >= retries) return res;
    const base = 500 * 2 ** attempt;
    const jittered = Math.floor(base * (0.8 + Math.random() * 0.4));
    await sleep(jittered);
  }
}
async function pollHttpWithRetries(method, path2, baseUrl, {
  json,
  params,
  retriesPerAttempt = 1,
  perAttemptTimeoutSecs = 1,
  overallTimeoutSecs = DEFAULT_TIMEOUT_SECS,
  shouldStop = (res, _attempt) => res.ok,
  // <- fixed arity
  initialDelayMs = 200,
  maxDelayMs = 1e3,
  backoffFactor = 1.5
} = {}) {
  const start = Date.now();
  let delay = initialDelayMs;
  let attempt = 0;
  let last = null;
  while (Date.now() - start < overallTimeoutSecs * 1e3) {
    const res = await httpRequestWithRetries(
      method,
      path2,
      baseUrl,
      json,
      params,
      retriesPerAttempt,
      perAttemptTimeoutSecs
    );
    last = res;
    if (shouldStop(res, attempt)) {
      return res;
    }
    await sleep(delay);
    delay = Math.min(maxDelayMs, Math.floor(delay * backoffFactor));
    attempt += 1;
  }
  if (last) return last;
  return err({
    status_code: 408,
    error: {
      message: `Timed out after ${overallTimeoutSecs}s without a response`
    }
  });
}
var sleep;
var init_http = __esm({
  "src/http.ts"() {
    "use strict";
    init_defaults();
    init_types();
    sleep = (ms) => ms > 0 ? new Promise((r) => setTimeout(r, ms)) : Promise.resolve();
  }
});

// src/proxy.ts
var proxy_exports = {};
__export(proxy_exports, {
  down: () => down,
  logs: () => logs,
  port: () => port,
  pull: () => pull,
  run: () => run,
  status: () => status,
  up: () => up
});
function http() {
  return _http ? Promise.resolve(_http) : Promise.resolve().then(() => (init_http(), http_exports)).then((m) => _http = m);
}
function dockerCmd(sudo, args) {
  return sudo ? { cmd: "sudo", fullArgs: ["docker", ...args] } : { cmd: "docker", fullArgs: args };
}
function dockerFailureMessage(r) {
  const detail = (r.stderr || r.stdout || `${r.cmd} failed (exit ${r.code})`).replace(/\n?Run 'docker .*? --help' for more information\.?/i, "").trim();
  if (/is already in use by container/i.test(detail)) {
    const m = /The container name "([^"]+)"/i.exec(detail);
    let name = m?.[1] ?? "<container>";
    if (name.startsWith("/")) {
      name = name.slice(1);
    }
    return `
${detail}

Hint: First stop the "${name}" container via the \`down\` command, or use a different --container-name.`;
  }
  const cleaned = detail.replace(/\s+$/g, "");
  if (/No such container/i.test(cleaned)) {
    const m = /No such container:\s+(.+)$/i.exec(cleaned);
    const name = m?.[1]?.replace(/^\/+/, "") ?? "<container>";
    return `
${detail}

Hint: Start the "${name}" container via the \`up\` command.`;
  }
  return detail;
}
function runDockerCmd(args, opts = {}) {
  const { cmd, fullArgs } = dockerCmd(opts.sudo, args);
  const streamed = !!opts.stream;
  const stdio = streamed ? "inherit" : ["ignore", "pipe", "pipe"];
  const res = (0, import_node_child_process.spawnSync)(cmd, fullArgs, { encoding: "utf8", stdio });
  const code = res.status ?? 0;
  const stdout = res.stdout ?? "";
  const stderr = res.stderr ?? "";
  return {
    ok: code === 0,
    code,
    stdout,
    stderr,
    cmd: `${cmd} ${fullArgs.join(" ")}`,
    streamed
  };
}
function pollLogs(containerName, sudo, intervalMs) {
  let since = /* @__PURE__ */ new Date();
  let stopped = false;
  let inFlight = false;
  let timer = null;
  let lastKey = "";
  const tick = async () => {
    if (stopped || inFlight) return;
    inFlight = true;
    try {
      const res = await logs(containerName, sudo, {
        since,
        tail: 20
      });
      if (res.ok) {
        const logs2 = res.data;
        const lines = logs2.stdout.split(/\r?\n/).filter(Boolean);
        for (const line of lines) {
          const m = /^(\d{4}-\d{2}-\d{2}T[^\s]+)\s(.*)$/.exec(line);
          if (m) {
            const [, iso, msg] = m;
            const key = iso + "\n" + msg;
            if (key !== lastKey) {
              process.stdout.write(msg + "\n");
              lastKey = key;
            }
            const t = Date.parse(iso);
            if (!Number.isNaN(t)) since = new Date(t + 1);
          } else {
            if (line !== lastKey) {
              process.stdout.write(line + "\n");
              lastKey = line;
            }
          }
        }
      }
    } catch (e) {
      console.error("Error polling logs:", e);
    } finally {
      inFlight = false;
      if (!stopped) timer = setTimeout(tick, intervalMs);
    }
  };
  timer = setTimeout(tick, intervalMs);
  return () => {
    stopped = true;
    if (timer) clearTimeout(timer);
  };
}
async function up(workspace, opts) {
  const {
    containerName = DEFAULT_CONTAINER_NAME,
    languageContainerVersion = LANGUAGE_CONTAINER_VERSION,
    proxyImage = DEFAULT_PROXY_IMAGE2,
    watchdogImage = DEFAULT_WATCHDOG_IMAGE2,
    wrapperImage = DEFAULT_WRAPPER_IMAGE2,
    timeout = DEFAULT_TIMEOUT_SECS,
    sudo = false,
    stream = false,
    ro = false,
    bindHost = DEFAULT_BIND_HOST,
    debug = false
  } = opts;
  let { hostPort = DEFAULT_HOST_PORT } = opts;
  if (!Number.isInteger(hostPort) || hostPort < 0) {
    throw new Error(
      "up(): hostPort is required and must be a positive integer >= 0"
    );
  }
  if (workspace.trim() === "") {
    throw new Error(
      "Invalid argument: 'workspace' is required and cannot be empty."
    );
  }
  const expanded = workspace.startsWith("~") ? import_node_path.default.join(import_node_os.default.homedir(), workspace.slice(1)) : workspace;
  const abs = import_node_path.default.resolve(expanded);
  const mountMode = ro ? "ro" : "rw";
  const args = [
    "run",
    "-d",
    "--rm",
    "--name",
    containerName,
    "-p",
    `${bindHost}:${hostPort}:${DEFAULT_CONTAINER_PORT}`,
    // Mount the workspace (repo)
    "-v",
    `${abs}:${DEFAULT_MOUNT_DIR}:${mountMode}`,
    // Mount Docker socket for Docker-outside-of--Docker support
    "-v",
    "/var/run/docker.sock:/var/run/docker.sock",
    // Mask `.ruby-lsp/` with a tmpfs so the LSProxy process NEVER sees previous composed bundles in `.ruby-lsp/`.
    // This prevents many issues with LSProxy's use of the root user to install and bundle gems, which can lead
    // to mutation of the host's `.ruby-lsp/`, and conflict with the host's use of IDE's that are using `.ruby-lsp/`.
    // It also ensures that every container start is a clean slate with respect to `.ruby-lsp/`.
    // Note: this mounting strategy will leave behind an empty `.ruby-lsp/` on the host.
    // If a `.ruby-lsp/` already exists on the host it is not touched or impacted by the container process.
    "--mount",
    `type=tmpfs,destination=${DEFAULT_MOUNT_DIR}/.ruby-lsp`,
    "-e",
    "USE_AUTH=false",
    "-e",
    `RUST_LOG=info${debug ? ",lsproxy=debug,lsp_wrapper=debug" : ""}`,
    "-e",
    `WATCHDOG_IMAGE=${watchdogImage}`,
    "-e",
    `WRAPPER_IMAGE=${wrapperImage}`,
    "-e",
    `LANGUAGE_CONTAINER_VERSION=${languageContainerVersion}`
    // env flags inserted below
  ];
  if (opts.envFile && String(opts.envFile).trim()) {
    args.push("--env-file", String(opts.envFile));
  }
  if (Array.isArray(opts.env)) {
    for (const e of opts.env) {
      if (e && String(e).trim()) {
        args.push("-e", String(e));
      }
    }
  }
  args.push(proxyImage, "nuanced-lsp-proxy");
  const r = runDockerCmd(args, { sudo, stream });
  if (!r.ok) {
    return err({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim()
    });
  }
  if (hostPort === 0) {
    const portRes = runDockerCmd(
      ["port", containerName, `${DEFAULT_CONTAINER_PORT}/tcp`],
      { sudo }
    );
    if (!portRes.ok) {
      return err({
        message: dockerFailureMessage(portRes),
        error_code: portRes.code,
        stdout: portRes.stdout.trim(),
        stderr: portRes.stderr.trim()
      });
    }
    const out = portRes.stdout.trim();
    const parts = out.split(":");
    if (parts.length !== 2) {
      return err({
        message: `Unexpected docker port output: '${out}'`,
        error_code: 1,
        stdout: out,
        stderr: portRes.stderr.trim()
      });
    }
    const parsedPort = Number.parseInt(parts[1], 10);
    if (!Number.isFinite(parsedPort)) {
      return err({
        message: `Failed to parse port from '${out}'`,
        error_code: 1,
        stdout: out,
        stderr: portRes.stderr.trim()
      });
    }
    hostPort = parsedPort;
  }
  let stopLogs = null;
  if (stream) stopLogs = pollLogs(containerName, sudo, 500);
  try {
    if (timeout > 0) {
      const healthHost = bindHost === "0.0.0.0" ? "127.0.0.1" : bindHost;
      const base = `http://${healthHost}:${hostPort}`;
      const { pollHttpWithRetries: pollHttpWithRetries2 } = await http();
      const h = await pollHttpWithRetries2(
        "GET",
        "/v1/system/health",
        base,
        {
          // Each attempt: quick timeout, tiny retry budget
          perAttemptTimeoutSecs: 1,
          retriesPerAttempt: 1,
          // Overall: respect the `--timeout` the user passed
          overallTimeoutSecs: timeout,
          // Stop condition: only when server reports healthy
          shouldStop: (res) => res.ok && res.data.status === "ok",
          // Backoff cadence while waiting for startup
          initialDelayMs: 200,
          maxDelayMs: 800,
          backoffFactor: 1.5
        }
      );
      if (!h.ok) {
        const httpErr = h.data;
        return err({
          error_code: httpErr.status_code ?? 1,
          message: `Health check polling loop timed out before successful response: ${httpErr.error}`,
          stdout: "",
          stderr: ""
        });
      }
    }
    return ok({ host_port: hostPort, base_url: bindHost });
  } finally {
    if (stopLogs) stopLogs();
  }
}
async function down(containerName, sudo) {
  const r = runDockerCmd(["stop", containerName], { sudo });
  if (!r.ok) {
    return err({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim()
    });
  }
  return ok({ stdout: r.stdout.trim() });
}
async function port(containerName, containerPort, sudo) {
  const r = runDockerCmd(["port", containerName, `${containerPort}/tcp`], {
    sudo
  });
  if (!r.ok) {
    return null;
  }
  const output = r.stdout.trim();
  const parts = output.split(":");
  if (parts.length !== 2) {
    return null;
  }
  const port2 = Number.parseInt(parts[1], 10);
  return Number.isFinite(port2) ? port2 : null;
}
async function logs(containerName, sudo, opts) {
  const args = ["logs"];
  if (opts?.stream) args.push("-f");
  if (opts?.since instanceof Date)
    args.push("--since", opts.since.toISOString());
  else if (opts?.since) args.push("--since", String(opts.since));
  if (opts?.tail !== void 0) args.push("--tail", String(opts.tail));
  args.push("--timestamps");
  args.push(containerName);
  const r = runDockerCmd(args, { sudo, stream: !!opts?.stream });
  if (!r.ok) {
    return err({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim()
    });
  }
  return ok({ stdout: r.stdout.trim() });
}
async function run(script, containerName, sudo, stream, env, envFile) {
  const containerTmpDir = "/tmp/nuanced-lsp-run";
  const uniqueSuffix = String(Date.now() / 1e3 | 0);
  const hostScriptPath = import_node_path.default.resolve(script);
  const destName = `${import_node_path.default.basename(hostScriptPath)}-${uniqueSuffix}`;
  const containerScriptPath = `${containerTmpDir}/${destName}`;
  let r = runDockerCmd(
    ["exec", containerName, "sh", "-lc", `mkdir -p ${containerTmpDir}`],
    { sudo, stream }
  );
  if (!r.ok) {
    return err({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim()
    });
  }
  r = runDockerCmd(
    ["cp", hostScriptPath, `${containerName}:${containerScriptPath}`],
    { sudo, stream }
  );
  if (!r.ok) {
    return err({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim()
    });
  }
  const execArgs = ["exec", "-u", "0"];
  if (envFile && String(envFile).trim())
    execArgs.push("--env-file", String(envFile));
  if (Array.isArray(env)) {
    for (const e of env) {
      if (e && String(e).trim()) execArgs.push("-e", String(e));
    }
  }
  execArgs.push(
    containerName,
    "sh",
    "-lc",
    `chmod +x ${containerScriptPath} || true; ${containerScriptPath}`
  );
  const runResult = runDockerCmd(execArgs, { sudo, stream });
  if (!runResult.ok) {
    return err({
      message: dockerFailureMessage(runResult),
      error_code: runResult.code,
      stdout: runResult.stdout.trim(),
      stderr: runResult.stderr.trim()
    });
  }
  r = runDockerCmd(
    ["exec", containerName, "sh", "-lc", `rm -f ${containerScriptPath}`],
    { sudo, stream }
  );
  if (!r.ok) {
    return err({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim()
    });
  }
  return ok({ stdout: runResult.stdout.trim() });
}
function dockerInspectField(containerName, fieldTemplate, sudo) {
  return runDockerCmd(["inspect", "-f", fieldTemplate, containerName], {
    sudo
  });
}
async function status(containerName, sudo) {
  const runningRes = dockerInspectField(
    containerName,
    "{{.State.Running}}",
    sudo
  );
  if (!runningRes.ok) {
    return err({
      message: dockerFailureMessage(runningRes),
      error_code: runningRes.code,
      stdout: runningRes.stdout.trim(),
      stderr: runningRes.stderr.trim()
    });
  }
  const running = runningRes.stdout.trim().toLowerCase() === "true";
  if (!running) {
    const exitRes = dockerInspectField(
      containerName,
      "{{.State.ExitCode}}",
      sudo
    );
    const errorRes = dockerInspectField(
      containerName,
      "{{.State.Error}}",
      sudo
    );
    let exitCode;
    if (exitRes.ok) {
      const raw = exitRes.stdout.trim();
      const n = parseInt(raw, 10);
      exitCode = Number.isFinite(n) ? n : void 0;
    }
    const dockerError = errorRes.ok ? errorRes.stdout.trim() : "";
    const combinedStdout = [runningRes.stdout, exitRes.stdout, errorRes.stdout].filter(Boolean).join("\n");
    const combinedStderr = [runningRes.stderr, exitRes.stderr, errorRes.stderr].filter(Boolean).join("\n");
    const msgParts = [`Container '${containerName}' is not running.`];
    if (typeof exitCode === "number") msgParts.push(`ExitCode=${exitCode}.`);
    if (dockerError) msgParts.push(`Error="${dockerError}".`);
    return err({
      error_code: typeof exitCode === "number" ? exitCode : runningRes.code,
      // prefer the container exit code when present
      message: msgParts.join(" "),
      stdout: combinedStdout,
      stderr: combinedStderr
    });
  }
  const psRes = runDockerCmd(
    ["ps", "--filter", `name=${containerName}`, "--format", "{{.Status}}"],
    { sudo }
  );
  if (!psRes.ok) {
    return err({
      error_code: psRes.code,
      message: `docker ps failed while checking status for '${containerName}'`,
      stdout: psRes.stdout,
      stderr: psRes.stderr
    });
  }
  const container_status = psRes.stdout.trim() || "Up";
  return ok({
    container_name: containerName,
    container_status
  });
}
async function pull(image, sudo, stream) {
  const img = image ?? DEFAULT_PROXY_IMAGE2;
  const args = ["pull", img];
  const r = runDockerCmd(args, { sudo, stream });
  if (!r.ok) {
    return err({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim()
    });
  }
  return ok({ image: img, stdout: r.stdout.trim() });
}
var import_node_child_process, import_node_path, import_node_os, _http;
var init_proxy = __esm({
  "src/proxy.ts"() {
    "use strict";
    import_node_child_process = require("node:child_process");
    import_node_path = __toESM(require("node:path"), 1);
    import_node_os = __toESM(require("node:os"), 1);
    init_types();
    init_defaults();
    init_version();
    _http = null;
  }
});

// src/index.ts
var index_exports = {};
__export(index_exports, {
  BaseCommandOptionsSchema: () => BaseCommandOptionsSchema,
  ContextSnippetSchema: () => ContextSnippetSchema,
  DefinitionInFileSchema: () => DefinitionInFileSchema,
  DefinitionLocationSchema: () => DefinitionLocationSchema,
  DefinitionsInFileResultSchema: () => DefinitionsInFileResultSchema,
  DockerErrSchema: () => DockerErrSchema,
  DownCommandOptionsSchema: () => DownCommandOptionsSchema,
  DownResultSchema: () => DownResultSchema,
  ExternalSymbolSchema: () => ExternalSymbolSchema,
  FilePositionSchema: () => FilePositionSchema,
  FileRangeSchema: () => FileRangeSchema,
  FindDefinitionOptionsSchema: () => FindDefinitionOptionsSchema,
  FindDefinitionResultSchema: () => FindDefinitionResultSchema,
  FindIdentifierOptionsSchema: () => FindIdentifierOptionsSchema,
  FindIdentifierResultSchema: () => FindIdentifierResultSchema,
  FindReferencedSymbolsOptionsSchema: () => FindReferencedSymbolsOptionsSchema,
  FindReferencedSymbolsResultSchema: () => FindReferencedSymbolsResultSchema,
  FindReferencesOptionsSchema: () => FindReferencesOptionsSchema,
  FindReferencesResultSchema: () => FindReferencesResultSchema,
  HealthCommandOptionsSchema: () => HealthCommandOptionsSchema,
  HealthResultSchema: () => HealthResultSchema,
  HttpErrSchema: () => HttpErrSchema,
  IdentifierPositionSchema: () => IdentifierPositionSchema,
  IdentifierSchema: () => IdentifierSchema,
  ListFilesResultSchema: () => ListFilesResultSchema,
  LogsCommandOptionsSchema: () => LogsCommandOptionsSchema,
  LogsResultSchema: () => LogsResultSchema,
  LspCommandOptionsSchema: () => LspCommandOptionsSchema,
  LspPositionSchema: () => LspPositionSchema,
  LspRangeSchema: () => LspRangeSchema,
  NotFoundSymbolSchema: () => NotFoundSymbolSchema,
  NuancedLspClient: () => NuancedLspClient,
  PullCommandOptionsSchema: () => PullCommandOptionsSchema,
  PullResultSchema: () => PullResultSchema,
  ReadSourceCommandOptionsSchema: () => ReadSourceCommandOptionsSchema,
  ReadSourceResultSchema: () => ReadSourceResultSchema,
  ReferenceLocationSchema: () => ReferenceLocationSchema,
  RunCommandOptionsSchema: () => RunCommandOptionsSchema,
  RunResultSchema: () => RunResultSchema,
  SelectedIdentifierSchema: () => SelectedIdentifierSchema,
  StatusCommandOptionsSchema: () => StatusCommandOptionsSchema,
  StatusResultSchema: () => StatusResultSchema,
  UpCommandOptionsSchema: () => UpCommandOptionsSchema,
  UpResultSchema: () => UpResultSchema,
  WorkspaceDefinitionSchema: () => WorkspaceDefinitionSchema,
  WorkspaceReferenceSchema: () => WorkspaceReferenceSchema,
  WorkspaceSymbolSchema: () => WorkspaceSymbolSchema,
  err: () => err,
  isErr: () => isErr,
  isOk: () => isOk,
  ok: () => ok
});
module.exports = __toCommonJS(index_exports);

// src/client.ts
init_defaults();
var TRAILING_SLASH_RE = /\/$/;
var _http2 = null;
async function http2() {
  if (_http2) return _http2;
  _http2 = await Promise.resolve().then(() => (init_http(), http_exports));
  return _http2;
}
var _proxy = null;
async function proxy() {
  if (_proxy) return _proxy;
  _proxy = await Promise.resolve().then(() => (init_proxy(), proxy_exports));
  return _proxy;
}
var NuancedLspClient = class {
  // 15 seconds
  constructor(opts) {
    // Cache for list-files results to avoid redundant calls
    // TODO: Add proper cache invalidation when files are modified
    // Currently uses 15 second TTL as a simple invalidation strategy
    this.fileListCache = null;
    this.CACHE_TTL_MS = 15e3;
    this.lsProxyBaseUrl = (opts?.lsProxyUrl ?? DEFAULT_HOST_URL).replace(
      TRAILING_SLASH_RE,
      ""
    );
    const lsProxyPort = typeof opts?.lsProxyPort === "number" ? opts.lsProxyPort : DEFAULT_HOST_PORT;
    this.lsProxyPort = lsProxyPort;
    this.fullLsProxyUrl = lsProxyPort === 0 ? void 0 : `${this.lsProxyBaseUrl}:${lsProxyPort}`;
    this.containerName = opts?.containerName ?? DEFAULT_CONTAINER_NAME;
    this.timeoutSecs = opts?.timeoutSecs ?? DEFAULT_TIMEOUT_SECS;
    this.retries = opts?.retries ?? DEFAULT_RETRIES;
    this.sudo = opts?.sudo;
  }
  resolveTimeout(t) {
    return Number.isFinite(t) ? t : this.timeoutSecs;
  }
  /**
   * Get the LSProxy URL. If the URL is not set (because port was 0 in constructor),
   * dynamically determine the port from the running container.
   */
  async lsProxyUrl() {
    if (this.fullLsProxyUrl) {
      return this.fullLsProxyUrl;
    }
    const { port: port2 } = await proxy();
    const lsProxyPort = await port2(
      this.containerName,
      DEFAULT_CONTAINER_PORT,
      this.sudo
    );
    if (port2 === null) {
      throw new Error(
        `Cannot determine port for container ${this.containerName}. Is the container running?`
      );
    }
    this.fullLsProxyUrl = `${this.lsProxyBaseUrl}:${lsProxyPort}`;
    return this.fullLsProxyUrl;
  }
  // ---- Lifecycle (docker) ---------------------------------------------------
  async up(workspace, opts) {
    const { up: up2 } = await proxy();
    const result = await up2(workspace, {
      containerName: this.containerName,
      hostPort: this.lsProxyPort,
      languageContainerVersion: opts.languageContainerVersion,
      proxyImage: opts.proxyImage,
      watchdogImage: opts.watchdogImage,
      wrapperImage: opts.wrapperImage,
      timeout: this.resolveTimeout(opts.timeout),
      sudo: this.sudo,
      stream: opts.stream,
      ro: !!opts.ro,
      bindHost: opts.bindHost,
      debug: !!opts.debug,
      env: opts.env,
      envFile: opts.envFile
    });
    if (result.ok) {
      this.fullLsProxyUrl = `${this.lsProxyBaseUrl}:${result.data.host_port}`;
    }
    return result;
  }
  /** Stop container. Never throws; returns { ok, error? }. */
  async down() {
    const { down: down2 } = await proxy();
    return down2(this.containerName, this.sudo);
  }
  async logs(opts) {
    const { logs: logs2 } = await proxy();
    return logs2(this.containerName, this.sudo, opts);
  }
  async run(script, opts) {
    const { run: run2 } = await proxy();
    return run2(
      script,
      this.containerName,
      this.sudo,
      opts?.stream,
      opts?.env,
      opts?.envFile
    );
  }
  // ---- Status (docker-only) -------------------------------------------------
  async status() {
    const { status: status2 } = await proxy();
    return status2(this.containerName, this.sudo);
  }
  async pull(image, stream) {
    const { pull: pull2 } = await proxy();
    return pull2(image, this.sudo, stream);
  }
  // ---- Health (data-plane) --------------------------------------------------
  async health(timeoutSecs) {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    return httpRequestWithRetries2(
      "GET",
      "/v1/system/health",
      url,
      void 0,
      void 0,
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
  }
  // ---- Workspace ------------------------------------------------------------
  async listFiles(timeoutSecs) {
    const now = Date.now();
    if (this.fileListCache && now - this.fileListCache.timestamp < this.CACHE_TTL_MS) {
      return { ok: true, data: this.fileListCache.data };
    }
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    const result = await httpRequestWithRetries2(
      "GET",
      "/v1/workspace/list-files",
      url,
      void 0,
      void 0,
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
    if (result.ok) {
      this.fileListCache = {
        data: result.data,
        timestamp: now
      };
    }
    return result;
  }
  async readSource(filePath, range, timeoutSecs) {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    return httpRequestWithRetries2(
      "POST",
      "/v1/workspace/read-source-code",
      url,
      { path: filePath, range: range ?? null },
      void 0,
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
  }
  // ---- Symbols --------------------------------------------------------------
  async definitionsInFile(filePath, timeoutSecs) {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    return httpRequestWithRetries2(
      "GET",
      "/v1/symbol/definitions-in-file",
      url,
      void 0,
      { file_path: filePath },
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
  }
  async findDefinition(filePosition, includeRawResponse, includeSourceCode, timeoutSecs) {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    return httpRequestWithRetries2(
      "POST",
      "/v1/symbol/find-definition",
      url,
      {
        include_raw_response: !!includeRawResponse,
        include_source_code: !!includeSourceCode,
        position: filePosition
      },
      void 0,
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
  }
  async findIdentifier(filePath, name, position, timeoutSecs) {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    return httpRequestWithRetries2(
      "POST",
      "/v1/symbol/find-identifier",
      url,
      { name, path: filePath, position },
      void 0,
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
  }
  async findReferencedSymbols(identifierPosition, fullScan = false, timeoutSecs) {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    return httpRequestWithRetries2(
      "POST",
      "/v1/symbol/find-referenced-symbols",
      url,
      {
        full_scan: !!fullScan,
        identifier_position: identifierPosition
      },
      void 0,
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
  }
  async findReferences(identifierPosition, contextLines = 0, includeRawResponse, timeoutSecs) {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries: httpRequestWithRetries2 } = await http2();
    return httpRequestWithRetries2(
      "POST",
      "/v1/symbol/find-references",
      url,
      {
        identifier_position: identifierPosition,
        include_code_context_lines: contextLines,
        include_raw_response: !!includeRawResponse
      },
      void 0,
      this.retries,
      this.resolveTimeout(timeoutSecs)
    );
  }
};

// src/index.ts
init_types();
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  BaseCommandOptionsSchema,
  ContextSnippetSchema,
  DefinitionInFileSchema,
  DefinitionLocationSchema,
  DefinitionsInFileResultSchema,
  DockerErrSchema,
  DownCommandOptionsSchema,
  DownResultSchema,
  ExternalSymbolSchema,
  FilePositionSchema,
  FileRangeSchema,
  FindDefinitionOptionsSchema,
  FindDefinitionResultSchema,
  FindIdentifierOptionsSchema,
  FindIdentifierResultSchema,
  FindReferencedSymbolsOptionsSchema,
  FindReferencedSymbolsResultSchema,
  FindReferencesOptionsSchema,
  FindReferencesResultSchema,
  HealthCommandOptionsSchema,
  HealthResultSchema,
  HttpErrSchema,
  IdentifierPositionSchema,
  IdentifierSchema,
  ListFilesResultSchema,
  LogsCommandOptionsSchema,
  LogsResultSchema,
  LspCommandOptionsSchema,
  LspPositionSchema,
  LspRangeSchema,
  NotFoundSymbolSchema,
  NuancedLspClient,
  PullCommandOptionsSchema,
  PullResultSchema,
  ReadSourceCommandOptionsSchema,
  ReadSourceResultSchema,
  ReferenceLocationSchema,
  RunCommandOptionsSchema,
  RunResultSchema,
  SelectedIdentifierSchema,
  StatusCommandOptionsSchema,
  StatusResultSchema,
  UpCommandOptionsSchema,
  UpResultSchema,
  WorkspaceDefinitionSchema,
  WorkspaceReferenceSchema,
  WorkspaceSymbolSchema,
  err,
  isErr,
  isOk,
  ok
});
//# sourceMappingURL=index.cjs.map

import { spawnSync, type StdioOptions } from "node:child_process";
import path from "node:path";
import os from "node:os";
import { ok, err } from "./types.js";
import type {
  DockerErr,
  DockerResult,
  DownResult,
  LogsResult,
  HealthResult,
  PullResult,
  RunResult,
  StatusResult,
  UpResult,
} from "./types.js";
import {
  DEFAULT_BIND_HOST,
  DEFAULT_CONTAINER_NAME,
  DEFAULT_CONTAINER_PORT,
  DEFAULT_HOST_PORT,
  DEFAULT_MOUNT_DIR,
  DEFAULT_PROXY_IMAGE,
  DEFAULT_TIMEOUT_SECS,
  DEFAULT_WATCHDOG_IMAGE,
  DEFAULT_WRAPPER_IMAGE,
} from "./defaults.js";
import { LANGUAGE_CONTAINER_VERSION } from "./__generated/version.js";

// Lazy import http module.
type HttpModule = typeof import("./http.js");
let _http: HttpModule | null = null;
function http(): Promise<HttpModule> {
  return _http
    ? Promise.resolve(_http)
    : import("./http.js").then((m) => (_http = m));
}

function dockerCmd(sudo: boolean | undefined, args: string[]) {
  return sudo
    ? { cmd: "sudo", fullArgs: ["docker", ...args] as string[] }
    : { cmd: "docker", fullArgs: args as string[] };
}

type RunDockerOpts = {
  sudo?: boolean;
  stream?: boolean; // true = print live (inherit), false/undefined = quiet (capture)
};

export type RunDockerResult = {
  ok: boolean; // True when code === 0
  code: number; // exit code
  stdout: string;
  stderr: string;
  cmd: string; // full "docker …" string
  streamed: boolean; // whether stdio was inherited
};

function dockerFailureMessage(r: RunDockerResult): string {
  const detail = (r.stderr || r.stdout || `${r.cmd} failed (exit ${r.code})`)
    .replace(/\n?Run 'docker .*? --help' for more information\.?/i, "")
    .trim();

  // Common error case: container name already in use (exit 125)
  if (/is already in use by container/i.test(detail)) {
    // Try to extract the name for a better hint; fallback to generic.
    const m = /The container name "([^"]+)"/i.exec(detail);
    let name = m?.[1] ?? "<container>";
    if (name.startsWith("/")) {
      name = name.slice(1);
    }
    return `\n${detail}\n\nHint: First stop the "${name}" container via the \`down\` command, or use a different --container-name.`;
  }

  // Common error case: no such container (exit 1)
  const cleaned = detail.replace(/\s+$/g, "");
  if (/No such container/i.test(cleaned)) {
    // Try to extract the container name after the colon
    const m = /No such container:\s+(.+)$/i.exec(cleaned);
    const name = m?.[1]?.replace(/^\/+/, "") ?? "<container>";
    return `\n${detail}\n\nHint: Start the "${name}" container via the \`up\` command.`;
  }

  return detail;
}

/** One-shot docker command. Stream or capture. Throws on non-zero unless ignoreError. */
function runDockerCmd(
  args: string[],
  opts: RunDockerOpts = {},
): RunDockerResult {
  const { cmd, fullArgs } = dockerCmd(opts.sudo, args);
  const streamed = !!opts.stream;
  const stdio: StdioOptions = streamed ? "inherit" : ["ignore", "pipe", "pipe"];

  const res = spawnSync(cmd, fullArgs, { encoding: "utf8", stdio });
  const code = res.status ?? 0;
  const stdout = res.stdout ?? "";
  const stderr = res.stderr ?? "";

  return {
    ok: code === 0,
    code,
    stdout,
    stderr,
    cmd: `${cmd} ${fullArgs.join(" ")}`,
    streamed,
  };
}

// Log polling helper used to stream Docker container logs.
// This is annoyingly complex, but far simpler than dealing with
// process groups and restricted privileges when users spin `up`
// a container using `sudo`.
// Rather than deal with that, this polls the logs until the health
// polling check succeeds. No privileged process group handling required.
function pollLogs(
  containerName: string,
  sudo: boolean,
  intervalMs: number,
): () => void {
  let since = new Date();
  let stopped = false;
  let inFlight = false;
  let timer: NodeJS.Timeout | null = null;
  let lastKey = "";

  const tick = async () => {
    if (stopped || inFlight) return;
    inFlight = true;
    try {
      const res = await logs(containerName, sudo, {
        since,
        tail: 20,
      });

      if (res.ok) {
        const logs = res.data as LogsResult;
        const lines = logs.stdout.split(/\r?\n/).filter(Boolean);
        for (const line of lines) {
          // "<RFC3339Nano> <message>"
          const m = /^(\d{4}-\d{2}-\d{2}T[^\s]+)\s(.*)$/.exec(line);
          if (m) {
            const [, iso, msg] = m;
            const key = iso + "\n" + msg;
            if (key !== lastKey) {
              process.stdout.write(msg + "\n");
              lastKey = key;
            }
            const t = Date.parse(iso);
            if (!Number.isNaN(t)) since = new Date(t + 1); // advance past this line
          } else {
            // Fallback if timestamps ever missing
            if (line !== lastKey) {
              process.stdout.write(line + "\n");
              lastKey = line;
            }
          }
        }
      }
      // If !res.ok (e.g., sudo needs TTY), we just skip this tick and try again.
    } catch (e) {
      console.error("Error polling logs:", e);
    } finally {
      inFlight = false;
      // If the timer is still active, restart the tick loop and hope for the best!
      if (!stopped) timer = setTimeout(tick, intervalMs);
    }
  };

  timer = setTimeout(tick, intervalMs);
  return () => {
    stopped = true;
    if (timer) clearTimeout(timer);
  };
}

// ───────────────────────────────────────────────────────────────────────────────
// Lifecycle commands
// ───────────────────────────────────────────────────────────────────────────────
export async function up(
  workspace: string,
  opts: {
    hostPort?: number;
    containerName?: string;
    languageContainerVersion?: string;
    proxyImage?: string;
    watchdogImage?: string;
    wrapperImage?: string;
    timeout?: number;
    sudo?: boolean;
    stream?: boolean;
    ro?: boolean;
    bindHost?: string;
    debug?: boolean;
    env?: string[];
    envFile?: string;
  },
): Promise<DockerResult<UpResult>> {
  const {
    containerName = DEFAULT_CONTAINER_NAME,
    languageContainerVersion = LANGUAGE_CONTAINER_VERSION,
    proxyImage: proxyImage = DEFAULT_PROXY_IMAGE,
    watchdogImage: watchdogImage = DEFAULT_WATCHDOG_IMAGE,
    wrapperImage: wrapperImage = DEFAULT_WRAPPER_IMAGE,
    timeout = DEFAULT_TIMEOUT_SECS,
    sudo = false,
    stream = false,
    ro = false,
    bindHost = DEFAULT_BIND_HOST,
    debug = false,
  } = opts;

  let { hostPort = DEFAULT_HOST_PORT } = opts;

  if (!Number.isInteger(hostPort) || hostPort < 0) {
    throw new Error(
      "up(): hostPort is required and must be a positive integer >= 0",
    );
  }
  if (workspace.trim() === "") {
    throw new Error(
      "Invalid argument: 'workspace' is required and cannot be empty.",
    );
  }

  const expanded = workspace.startsWith("~")
    ? path.join(os.homedir(), workspace.slice(1))
    : workspace;
  const abs = path.resolve(expanded);
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
    `LANGUAGE_CONTAINER_VERSION=${languageContainerVersion}`,
    // env flags inserted below
  ];

  // Append env-file if provided
  if (opts.envFile && String(opts.envFile).trim()) {
    args.push("--env-file", String(opts.envFile));
  }

  // Append additional -e flags if provided
  if (Array.isArray(opts.env)) {
    for (const e of opts.env) {
      if (e && String(e).trim()) {
        args.push("-e", String(e));
      }
    }
  }

  // Image and entry-point command
  args.push(proxyImage, "nuanced-lsp-proxy");

  // Run container
  const r = runDockerCmd(args, { sudo, stream });
  if (!r.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim(),
    });
  }

  if (hostPort === 0) {
    const portRes = runDockerCmd(
      ["port", containerName, `${DEFAULT_CONTAINER_PORT}/tcp`],
      { sudo },
    );
    if (!portRes.ok) {
      return err<DockerErr>({
        message: dockerFailureMessage(portRes),
        error_code: portRes.code,
        stdout: portRes.stdout.trim(),
        stderr: portRes.stderr.trim(),
      });
    }

    const out = portRes.stdout.trim();
    const parts = out.split(":");
    if (parts.length !== 2) {
      return err<DockerErr>({
        message: `Unexpected docker port output: '${out}'`,
        error_code: 1,
        stdout: out,
        stderr: portRes.stderr.trim(),
      });
    }

    const parsedPort = Number.parseInt(parts[1], 10);
    if (!Number.isFinite(parsedPort)) {
      return err<DockerErr>({
        message: `Failed to parse port from '${out}'`,
        error_code: 1,
        stdout: out,
        stderr: portRes.stderr.trim(),
      });
    }

    hostPort = parsedPort;
  }

  let stopLogs: (() => void) | null = null;
  if (stream) stopLogs = pollLogs(containerName, sudo, 500);

  try {
    if (timeout > 0) {
      const healthHost = bindHost === "0.0.0.0" ? "127.0.0.1" : bindHost;
      const base = `http://${healthHost}:${hostPort}`;

      const { pollHttpWithRetries } = await http();

      const h = await pollHttpWithRetries<HealthResult>(
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
          backoffFactor: 1.5,
        },
      );

      if (!h.ok) {
        const httpErr = h.data;
        return err<DockerErr>({
          error_code: httpErr.status_code ?? 1,
          message: `Health check polling loop timed out before successful response: ${httpErr.error}`,
          stdout: "",
          stderr: "",
        });
      }
    }
    return ok<UpResult>({ host_port: hostPort, base_url: bindHost });
  } finally {
    if (stopLogs) stopLogs();
  }
}

export async function down(
  containerName: string,
  sudo?: boolean,
): Promise<DockerResult<DownResult>> {
  const r = runDockerCmd(["stop", containerName], { sudo });
  if (!r.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim(),
    });
  }
  return ok<DownResult>({ stdout: r.stdout.trim() });
}

export async function port(
  containerName: string,
  containerPort: number,
  sudo?: boolean,
): Promise<number | null> {
  const r = runDockerCmd(["port", containerName, `${containerPort}/tcp`], {
    sudo,
  });
  if (!r.ok) {
    return null;
  }

  // Output format: "127.0.0.1:PORT" or "0.0.0.0:PORT"
  const output = r.stdout.trim();
  const parts = output.split(":");
  if (parts.length !== 2) {
    return null;
  }

  const port = Number.parseInt(parts[1], 10);
  return Number.isFinite(port) ? port : null;
}

export async function logs(
  containerName: string,
  sudo?: boolean,
  opts?: {
    stream?: boolean;
    since?: string | Date; // Date is allowed for polling in `up`.
    tail?: number | "all";
  },
): Promise<DockerResult<LogsResult>> {
  const args = ["logs"];
  if (opts?.stream) args.push("-f");
  if (opts?.since instanceof Date)
    args.push("--since", opts.since.toISOString());
  else if (opts?.since) args.push("--since", String(opts.since));
  if (opts?.tail !== undefined) args.push("--tail", String(opts.tail));
  args.push("--timestamps");
  args.push(containerName);

  const r = runDockerCmd(args, { sudo, stream: !!opts?.stream });
  if (!r.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim(),
    });
  }
  return ok<LogsResult>({ stdout: r.stdout.trim() });
}

export async function run(
  script: string,
  containerName: string,
  sudo?: boolean,
  stream?: boolean,
  env?: string[],
  envFile?: string,
): Promise<DockerResult<RunResult>> {
  const containerTmpDir = "/tmp/nuanced-lsp-run";
  const uniqueSuffix = String((Date.now() / 1000) | 0);
  const hostScriptPath = path.resolve(script);
  const destName = `${path.basename(hostScriptPath)}-${uniqueSuffix}`;
  const containerScriptPath = `${containerTmpDir}/${destName}`;

  // Prepare workspace inside container.
  let r = runDockerCmd(
    ["exec", containerName, "sh", "-lc", `mkdir -p ${containerTmpDir}`],
    { sudo, stream },
  );
  if (!r.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim(),
    });
  }

  // Copy script.
  r = runDockerCmd(
    ["cp", hostScriptPath, `${containerName}:${containerScriptPath}`],
    { sudo, stream },
  );
  if (!r.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim(),
    });
  }

  // Execute the script.
  const execArgs = ["exec", "-u", "0"] as string[];

  // Insert env options before container name
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
    `chmod +x ${containerScriptPath} || true; ${containerScriptPath}`,
  );

  const runResult = runDockerCmd(execArgs, { sudo, stream });
  if (!runResult.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(runResult),
      error_code: runResult.code,
      stdout: runResult.stdout.trim(),
      stderr: runResult.stderr.trim(),
    });
  }

  // Clean up the script.
  r = runDockerCmd(
    ["exec", containerName, "sh", "-lc", `rm -f ${containerScriptPath}`],
    { sudo, stream },
  );
  if (!r.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim(),
    });
  }

  return ok<RunResult>({ stdout: runResult.stdout.trim() });
}

function dockerInspectField(
  containerName: string,
  fieldTemplate: string, // e.g. "{{.State.Running}}"
  sudo?: boolean,
): RunDockerResult {
  return runDockerCmd(["inspect", "-f", fieldTemplate, containerName], {
    sudo,
  });
}

export async function status(
  containerName: string,
  sudo?: boolean,
): Promise<DockerResult<StatusResult>> {
  const runningRes = dockerInspectField(
    containerName,
    "{{.State.Running}}",
    sudo,
  );
  if (!runningRes.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(runningRes),
      error_code: runningRes.code,
      stdout: runningRes.stdout.trim(),
      stderr: runningRes.stderr.trim(),
    });
  }

  const running = runningRes.stdout.trim().toLowerCase() === "true";
  if (!running) {
    // 2) Not running → check ExitCode and Error
    const exitRes = dockerInspectField(
      containerName,
      "{{.State.ExitCode}}",
      sudo,
    );
    const errorRes = dockerInspectField(
      containerName,
      "{{.State.Error}}",
      sudo,
    );

    // Parse exit code if available
    let exitCode: number | undefined;
    if (exitRes.ok) {
      const raw = exitRes.stdout.trim();
      const n = parseInt(raw, 10);
      exitCode = Number.isFinite(n) ? n : undefined;
    }

    // Extract docker's error string if available (may be empty)
    const dockerError = errorRes.ok ? errorRes.stdout.trim() : "";

    // Combine stdout/stderr from all three calls for maximal debugging context
    const combinedStdout = [runningRes.stdout, exitRes.stdout, errorRes.stdout]
      .filter(Boolean)
      .join("\n");
    const combinedStderr = [runningRes.stderr, exitRes.stderr, errorRes.stderr]
      .filter(Boolean)
      .join("\n");

    const msgParts = [`Container '${containerName}' is not running.`];
    if (typeof exitCode === "number") msgParts.push(`ExitCode=${exitCode}.`);
    if (dockerError) msgParts.push(`Error="${dockerError}".`);

    return err<DockerErr>({
      error_code: typeof exitCode === "number" ? exitCode : runningRes.code, // prefer the container exit code when present
      message: msgParts.join(" "),
      stdout: combinedStdout,
      stderr: combinedStderr,
    });
  }

  const psRes = runDockerCmd(
    ["ps", "--filter", `name=${containerName}`, "--format", "{{.Status}}"],
    { sudo },
  );
  if (!psRes.ok) {
    return err<DockerErr>({
      error_code: psRes.code,
      message: `docker ps failed while checking status for '${containerName}'`,
      stdout: psRes.stdout,
      stderr: psRes.stderr,
    });
  }

  const container_status = psRes.stdout.trim() || "Up";
  return ok<StatusResult>({
    container_name: containerName,
    container_status,
  });
}

/** Pull image. Supports optional streaming. Never throws. */
export async function pull(
  image?: string,
  sudo?: boolean,
  stream?: boolean,
): Promise<DockerResult<PullResult>> {
  const img = image ?? DEFAULT_PROXY_IMAGE;
  const args = ["pull", img];
  const r = runDockerCmd(args, { sudo, stream });
  if (!r.ok) {
    return err<DockerErr>({
      message: dockerFailureMessage(r),
      error_code: r.code,
      stdout: r.stdout.trim(),
      stderr: r.stderr.trim(),
    });
  }
  return ok<PullResult>({ image: img, stdout: r.stdout.trim() });
}

import fs from "node:fs";
import { spawnSync, SpawnSyncOptions } from "node:child_process";

import {
  LanguageSpec,
  TIMEOUT_SECONDS,
  PROXY_IMAGE_OVERRIDE,
  WATCHDOG_IMAGE_OVERRIDE,
  WRAPPER_IMAGE_OVERRIDE,
  SYMBOL_SCENARIO_DELAY,
  WORKSPACE_SCENARIO_DELAY,
  workspacePath,
  dockerSafe,
  fixedPort,
  workerIndex,
  CLIENT_COMMAND,
  FOLLOW_LOGS,
} from "./constants.js";
import { LogFollower, startLogFollow, stopLogFollow } from "./logs.js";

export interface RunnerOptions {
  language: LanguageSpec;
}

export class ClientRunner {
  public readonly language: LanguageSpec;
  public readonly workspace: string;
  public readonly containerName: string;
  public hostPort: number;
  public readonly timeoutSeconds: number;
  public readonly proxyImage: string;
  public readonly watchdogImage: string;
  public readonly wrapperImage: string;
  public readonly symbolDelay: number;
  public readonly workspaceDelay: number;

  private logFollower?: LogFollower;
  private running = false;
  private healthReady = false;
  private readonly requestedHostPort: number;

  constructor(private readonly options: RunnerOptions) {
    this.language = options.language;
    this.workspace = workspacePath(options.language.key);
    this.containerName = dockerSafe(
      `nuanced-test-${options.language.key}-worker-${workerIndex()}`,
    );
    this.requestedHostPort = fixedPort(options.language.key);
    this.hostPort = this.requestedHostPort;
    this.timeoutSeconds = TIMEOUT_SECONDS;
    this.proxyImage = PROXY_IMAGE_OVERRIDE;
    this.watchdogImage = WATCHDOG_IMAGE_OVERRIDE;
    this.wrapperImage = WRAPPER_IMAGE_OVERRIDE;
    this.symbolDelay = SYMBOL_SCENARIO_DELAY;
    this.workspaceDelay = WORKSPACE_SCENARIO_DELAY;
  }

  private sleepSync(milliseconds: number): void {
    if (milliseconds > 0) {
      const buf = new SharedArrayBuffer(4);
      const arr = new Int32Array(buf);
      Atomics.wait(arr, 0, 0, milliseconds);
    }
  }

  private ensureWorkspaceExists(): void {
    if (!fs.existsSync(this.workspace)) {
      throw new Error(`workspace ${this.workspace} missing`);
    }
  }

  private runCommand(...args: string[]): string {
    const [command, ...baseArgs] = CLIENT_COMMAND;
    const fullArgs = [...baseArgs, ...args];

    const opts: SpawnSyncOptions = {
      encoding: "utf8",
      maxBuffer: 20 * 1024 * 1024,
      timeout: this.timeoutSeconds * 1000,
      env: { ...process.env },
    };

    const result = spawnSync(command, fullArgs, opts);

    if (result.error) {
      if (result.error.message.includes("ETIMEDOUT")) {
        throw new Error(
          `Command timed out after ${this.timeoutSeconds}s: ${command} ${fullArgs.join(" ")}`,
          { cause: result.error },
        );
      }
      throw new Error(`Failed to execute ${command}: ${result.error.message}`, {
        cause: result.error,
      });
    }

    if (result.status && result.status !== 0) {
      const stdout = result.stdout ?? "";
      const stderr = result.stderr ?? "";
      const dockerPs = spawnSync(
        "docker",
        ["ps", "--format", "{{.Names}}\t{{.Status}}\t{{.Ports}}"],
        { encoding: "utf8", timeout: 5000 },
      );
      const dockerInfo = dockerPs.stdout
        ? `\n---- docker ps ----\n${dockerPs.stdout.trim()}`
        : "";
      throw new Error(
        `Command failed (exit ${result.status}): ${command} ${fullArgs.join(" ")}\n` +
          (stdout ? `---- stdout ----\n${stdout.trim()}\n` : "") +
          (stderr ? `---- stderr ----\n${stderr.trim()}\n` : "") +
          dockerInfo,
      );
    }

    return result.stdout ?? "";
  }

  private async sleep(seconds: number): Promise<void> {
    if (seconds > 0) {
      await new Promise((resolve) => setTimeout(resolve, seconds * 1000));
    }
  }

  private assertContainerPort(): void {
    const inspect = spawnSync(
      "docker",
      [
        "inspect",
        this.containerName,
        "--format",
        "{{json .NetworkSettings.Ports}}",
      ],
      {
        encoding: "utf8",
        timeout: 5000,
      },
    );

    if (inspect.status !== 0 || !inspect.stdout) {
      throw new Error(
        `Failed to inspect published ports for ${this.containerName}: ${inspect.stderr ?? inspect.error}`,
      );
    }

    try {
      const ports = JSON.parse(inspect.stdout.trim());
      const mapping = ports?.["4444/tcp"] ?? [];
      const hostPorts = Array.isArray(mapping)
        ? mapping
            .map((entry: any) =>
              entry && typeof entry === "object" ? entry.HostPort : null,
            )
            .filter(
              (value): value is string =>
                typeof value === "string" && value.length > 0,
            )
        : [];

      if (hostPorts.length === 0) {
        throw new Error(
          "Container is running but did not publish 4444/tcp as expected.\n" +
            `  container: ${this.containerName}`,
        );
      }

      const numericPorts = hostPorts
        .map((value) => Number(value))
        .filter((value) => Number.isFinite(value) && value > 0);

      if (numericPorts.length === 0) {
        throw new Error(
          `Docker reported non-numeric ports for ${this.containerName}: ${hostPorts.join(", ")}`,
        );
      }

      if (this.requestedHostPort !== 0) {
        if (!hostPorts.includes(String(this.requestedHostPort))) {
          throw new Error(
            "Container is running but host port is not published as requested.\n" +
              `  container: ${this.containerName}\n` +
              `  requested: 127.0.0.1:${this.requestedHostPort}->4444/tcp\n` +
              `  actual   : ${hostPorts.join(", ") || "<none>"}`,
          );
        }
        this.hostPort = this.requestedHostPort;
      } else {
        this.hostPort = numericPorts[0];
      }
    } catch {
      throw error instanceof Error
        ? error
        : new Error(
            `Unable to validate published ports for ${this.containerName}`,
          );
    }
  }

  private startLogs(): void {
    if (FOLLOW_LOGS && !this.logFollower) {
      this.logFollower = startLogFollow(this.containerName);
    }
  }

  private async stopLogs(): Promise<void> {
    if (this.logFollower) {
      await stopLogFollow(this.logFollower);
    }
    this.logFollower = undefined;
  }

  /**
   * Checks if an error is due to a Docker container name conflict.
   * Used by up() to detect when a container with the same name already exists,
   * allowing automatic cleanup and retry.
   */
  private isNameConflict(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error);
    return message.includes("is already in use by container");
  }

  /**
   * Removes a conflicting container to allow reusing the container name.
   * This makes tests self-healing by automatically cleaning up leftover containers from:
   * - Previous test runs interrupted by Ctrl+C or crashes
   * - Failed test cleanup due to errors or timeouts
   * - Race conditions during parallel test execution
   * Without this, leftover containers would require manual cleanup via `docker rm -f`.
   */
  private cleanupConflictingContainer(): void {
    // Try docker rm -f directly (handles both running and stopped containers)
    const rm = spawnSync("docker", ["rm", "-f", this.containerName], {
      encoding: "utf8",
      timeout: 10000,
    });

    // If direct removal failed, try the CLI down command
    if (rm.status && rm.status !== 0) {
      try {
        this.runCommand(
          "down",
          "--container-name",
          this.containerName,
          "--timeout",
          String(this.timeoutSeconds),
          "--json",
        );
      } catch {
        // Final attempt: try rm -f again in case the container state changed
        spawnSync("docker", ["rm", "-f", this.containerName], {
          encoding: "utf8",
          timeout: 5000,
        });
      }
    }

    this.running = false;
    this.healthReady = false;
  }

  up(extraArgs: string[] = []): string {
    this.ensureWorkspaceExists();

    const desiredPort = this.requestedHostPort === 0 ? 0 : this.hostPort;

    const args = [
      "up",
      this.workspace,
      "--container-name",
      this.containerName,
      "--host-port",
      String(desiredPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
      ...extraArgs,
    ];
    if (this.proxyImage) {
      args.push("--proxy-image", this.proxyImage);
    }
    if (this.watchdogImage) {
      args.push("--watchdog-image", this.watchdogImage);
    }
    if (this.wrapperImage) {
      args.push("--wrapper-image", this.wrapperImage);
    }

    const attempt = () => {
      const out = this.runCommand(...args);
      this.assertContainerPort();
      this.startLogs();
      this.running = true;
      this.healthReady = false;
      this.waitForHealth();
      return out;
    };

    try {
      return attempt();
    } catch (error) {
      if (!this.isNameConflict(error)) {
        throw error;
      }

      this.cleanupConflictingContainer();
      return attempt();
    }
  }

  async down(): Promise<void> {
    try {
      await this.stopLogs();
      this.runCommand(
        "down",
        "--container-name",
        this.containerName,
        "--timeout",
        String(this.timeoutSeconds),
        "--json",
      );
    } catch {
      // If down command fails, try docker rm -f as fallback
      try {
        spawnSync("docker", ["rm", "-f", this.containerName], {
          encoding: "utf8",
          timeout: 10000,
        });
      } catch {
        // Ignore - we tried our best
      }
    } finally {
      this.running = false;
      this.healthReady = false;
    }
  }

  private checkLanguages(
    languageStatus: Record<string, unknown> | undefined,
    languages: Record<string, unknown> | undefined,
  ): [pending: string[], failed: string[]] {
    if (languageStatus && typeof languageStatus === "object") {
      const pending: string[] = [];
      const failed: string[] = [];
      for (const [key, value] of Object.entries(languageStatus)) {
        if (value === "unhealthy") {
          failed.push(key);
        } else if (value !== "healthy") {
          pending.push(key);
        }
      }
      return [pending, failed];
    }

    // Fall back to legacy languages boolean field
    if (languages && typeof languages === "object") {
      const pending: string[] = [];
      for (const [key, value] of Object.entries(languages)) {
        if (value !== true) {
          pending.push(key);
        }
      }
      return [pending, []];
    }

    return [["<languages-unreported>"], []];
  }

  private checkHealthOnce(): Record<string, unknown> | null {
    try {
      const out = this.runCommand(
        "health",
        "--lsp-url",
        "http://127.0.0.1",
        "--lsp-port",
        String(this.hostPort),
        "--timeout",
        String(this.timeoutSeconds),
        "--json",
      );
      return parseJson(out);
    } catch {
      return null;
    }
  }

  private waitForHealth(): void {
    const deadline = Date.now() + this.timeoutSeconds * 1000;
    let lastHealth: Record<string, unknown> | null = null;

    while (Date.now() < deadline) {
      lastHealth = this.checkHealthOnce();
      const status =
        lastHealth && typeof lastHealth === "object"
          ? (lastHealth as any).status
          : undefined;
      const languageStatus =
        lastHealth &&
        typeof lastHealth === "object" &&
        "language_status" in lastHealth
          ? (lastHealth as any).language_status
          : undefined;
      const languages =
        lastHealth &&
        typeof lastHealth === "object" &&
        "languages" in lastHealth
          ? (lastHealth as any).languages
          : undefined;

      const [pending, failed] = this.checkLanguages(languageStatus, languages);

      if (failed.length > 0) {
        throw new Error(
          `Language containers failed to start (unhealthy): ${failed.join(", ")}`,
        );
      }

      if (status === "ok" && pending.length === 0) {
        this.healthReady = true;
        return;
      }

      this.sleepSync(1000);
    }

    const [pending] = this.checkLanguages(
      lastHealth && typeof lastHealth === "object"
        ? (lastHealth as any).language_status
        : undefined,
      lastHealth && typeof lastHealth === "object"
        ? (lastHealth as any).languages
        : undefined,
    );
    const pendingList = pending.length > 0 ? pending.join(", ") : "unknown";
    throw new Error(
      `Service did not become healthy within ${this.timeoutSeconds}s (pending languages: ${pendingList})`,
    );
  }

  public ensureUp(): void {
    if (!this.running) {
      this.up();
    } else if (!this.healthReady) {
      this.waitForHealth();
    }
  }

  statusJson(): Record<string, unknown> {
    this.ensureUp();
    const out = this.runCommand(
      "status",
      "--json",
      "--container-name",
      this.containerName,
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    );
    return parseJson(out);
  }

  logs(): string {
    this.ensureUp();
    return this.runCommand(
      "logs",
      "--container-name",
      this.containerName,
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    );
  }

  runScript(scriptPath: string): string {
    this.ensureUp();
    return this.runCommand(
      "run",
      scriptPath,
      "--container-name",
      this.containerName,
      "--timeout",
      String(this.timeoutSeconds),
      "--stream",
      "--json",
    );
  }

  async health(): Promise<Record<string, unknown>> {
    this.ensureUp();
    await this.sleep(this.workspaceDelay);
    const out = this.runCommand(
      "health",
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    );
    return parseJson(out);
  }

  async listFiles(): Promise<string[]> {
    this.ensureUp();
    await this.sleep(this.workspaceDelay);
    const out = this.runCommand(
      "list-files",
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    );
    const data = parseJson(out);
    if (Array.isArray(data)) {
      return data.map((item) => String(item));
    }
    if (
      data &&
      typeof data === "object" &&
      Array.isArray((data as any).files)
    ) {
      return (data as any).files.map((item: unknown) => String(item));
    }
    return out
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
  }

  async readSource(targetPath: string): Promise<any> {
    this.ensureUp();
    await this.sleep(this.workspaceDelay);
    const out = this.runCommand(
      "read-source",
      targetPath,
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    );
    return parseJson(out);
  }

  async definitionsInFile(targetPath: string): Promise<any> {
    this.ensureUp();
    await this.sleep(this.symbolDelay);
    const out = this.runCommand(
      "definitions-in-file",
      targetPath,
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    );
    return parseJson(out);
  }

  async findDefinition(
    targetPath: string,
    line: number,
    col: number,
  ): Promise<any> {
    this.ensureUp();
    await this.sleep(this.symbolDelay);
    const out = this.runCommand(
      "find-definition",
      targetPath,
      `${line}:${col}`,
      "--include-raw-response",
      "--include-source-code",
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    );
    return parseJson(out);
  }

  async findIdentifier(
    targetPath: string,
    identifier: string,
    position?: string,
  ): Promise<any> {
    this.ensureUp();
    await this.sleep(this.symbolDelay);
    const args = [
      "find-identifier",
      targetPath,
      identifier,
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    ];
    if (position) {
      args.push("--position", position);
    }
    const out = this.runCommand(...args);
    return parseJson(out);
  }

  async findReferencedSymbols(
    targetPath: string,
    line: number,
    col: number,
    fullScan = false,
  ): Promise<any> {
    this.ensureUp();
    await this.sleep(this.symbolDelay);
    const args = [
      "find-referenced-symbols",
      targetPath,
      `${line}:${col}`,
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    ];
    if (fullScan) args.push("--full-scan");
    const out = this.runCommand(...args);
    return parseJson(out);
  }

  async findReferences(
    targetPath: string,
    line: number,
    col: number,
    contextLines = 0,
  ): Promise<any> {
    this.ensureUp();
    await this.sleep(this.symbolDelay);
    const args = [
      "find-references",
      targetPath,
      `${line}:${col}`,
      "--include-raw-response",
      "--lsp-url",
      "http://127.0.0.1",
      "--lsp-port",
      String(this.hostPort),
      "--timeout",
      String(this.timeoutSeconds),
      "--json",
    ];
    if (contextLines) {
      args.push("--context-lines", String(contextLines));
    }
    const out = this.runCommand(...args);
    return parseJson(out);
  }
}

function parseJson(raw: string): any {
  try {
    return JSON.parse(raw);
  } catch {
    throw new Error(
      `Failed to parse JSON output: ${(error as Error).message}\n${raw}`,
    );
  }
}

export function createRunner(options: RunnerOptions): ClientRunner {
  return new ClientRunner(options);
}

import type WebSocket from "ws";
import type {
  ClientPullOptions,
  DefinitionsInFileResult,
  DockerResult,
  DownResult,
  FilePosition,
  FindDefinitionResult,
  FindIdentifierResult,
  FindReferencedSymbolsResult,
  FindReferencesResult,
  HealthResult,
  HttpResult,
  IdentifierPosition,
  JsonRpcMessage,
  ListFilesResult,
  LogsResult,
  LspPosition,
  LspRange,
  PullResult,
  ReadSourceResult,
  RunResult,
  StatusResult,
  UpResult,
} from "./types.js";

import {
  DEFAULT_CONTAINER_NAME,
  DEFAULT_CONTAINER_PORT,
  DEFAULT_CONTAINER_REGISTRY,
  DEFAULT_HOST_PORT,
  DEFAULT_HOST_URL,
  DEFAULT_LANGUAGE_IMAGE_VERSION,
  DEFAULT_RETRIES,
  DEFAULT_SERVICE_IMAGE_VERSION,
  DEFAULT_TIMEOUT_SECS,
} from "./defaults.js";
import { err, ok } from "./types.js";

// Precompiled tiny helpers
const TRAILING_SLASH_RE = /\/$/;

type HttpModule = typeof import("./http.js");
async function http(): Promise<HttpModule> {
  return import("./http.js");
}

type ProxyModule = typeof import("./proxy.js");
async function proxy(): Promise<ProxyModule> {
  return await import("./proxy.js");
}

export class NuancedLspClient {
  readonly containerName: string;
  private readonly sudo?: boolean;
  private readonly retries: number;
  private readonly proxyPort: number;
  private readonly proxyBaseUrl: string;
  private timeoutSecs: number;

  private fullProxyUrl: string | undefined;
  private nextId = 1;

  // Cache for list-files results to avoid redundant calls
  // TODO: Add proper cache invalidation when files are modified
  // Currently uses 15 second TTL as a simple invalidation strategy
  private fileListCache: {
    data: string[];
    timestamp: number;
  } | null = null;
  private readonly CACHE_TTL_MS = 15000; // 15 seconds

  constructor(opts?: {
    containerName?: string;
    proxyUrl?: string;
    proxyPort?: number;
    timeoutSecs?: number;
    retries?: number;
    sudo?: boolean;
  }) {
    this.proxyBaseUrl = (opts?.proxyUrl ?? DEFAULT_HOST_URL).replace(
      TRAILING_SLASH_RE,
      "",
    );
    const proxyPort =
      typeof opts?.proxyPort === "number" ? opts.proxyPort : DEFAULT_HOST_PORT;
    this.proxyPort = proxyPort;

    // If port is 0 (dynamic assignment), leave fullProxyUrl undefined
    // It will be resolved lazily when needed
    this.fullProxyUrl =
      proxyPort === 0 ? undefined : `${this.proxyBaseUrl}:${proxyPort}`;

    this.containerName = opts?.containerName ?? DEFAULT_CONTAINER_NAME;
    this.timeoutSecs = opts?.timeoutSecs ?? DEFAULT_TIMEOUT_SECS;
    this.retries = opts?.retries ?? DEFAULT_RETRIES;
    this.sudo = opts?.sudo;
  }

  private resolveTimeout(t?: number): number {
    return Number.isFinite(t as number) ? (t as number) : this.timeoutSecs;
  }

  /**
   * Get the LSProxy URL. If the URL is not set (because port was 0 in constructor),
   * dynamically determine the port from the running container.
   */
  private async proxyUrl(): Promise<string> {
    if (this.fullProxyUrl) {
      return this.fullProxyUrl;
    }

    // Port not set - need to detect it from the running container
    const { port } = await proxy();
    const proxyPort = await port(
      this.containerName,
      DEFAULT_CONTAINER_PORT,
      this.sudo,
    );

    if (port === null) {
      throw new Error(
        `Cannot determine port for container ${this.containerName}. Is the container running?`,
      );
    }

    // Cache the URL for future calls
    this.fullProxyUrl = `${this.proxyBaseUrl}:${proxyPort}`;
    return this.fullProxyUrl;
  }

  /**
   * Create a WebSocket connection for bidirectional LSP communication
   */
  async lsp_ws(): Promise<WebSocket> {
    const httpUrl = await this.proxyUrl();
    const wsUrl = httpUrl.replace(/^http/, "ws");
    const WS = (await import("ws")).default;
    return new WS(`${wsUrl}/lsp/ws`);
  }

  // ---- Lifecycle (docker) ---------------------------------------------------
  async up(
    workspace: string,
    opts: {
      containerRegistry?: string;
      languageImageVersion?: string;
      serviceImageVersion?: string;
      timeout?: number;
      stream?: boolean;
      ro?: boolean;
      bindHost?: string;
      debug?: boolean;
      env?: string[]; // Array of "KEY=VALUE".
      envFile?: string; // Path to file with KEY=VALUE lines.
    },
  ): Promise<DockerResult<UpResult>> {
    const { up } = await proxy();
    const result = await up(workspace, {
      containerName: this.containerName,
      hostPort: this.proxyPort,
      containerRegistry: opts.containerRegistry,
      languageImageVersion: opts.languageImageVersion,
      serviceImageVersion: opts.serviceImageVersion,
      timeout: this.resolveTimeout(opts.timeout),
      sudo: this.sudo,
      stream: opts.stream,
      ro: !!opts.ro,
      bindHost: opts.bindHost,
      debug: !!opts.debug,
      env: opts.env,
      envFile: opts.envFile,
    });

    // If up succeeded and we don't have a URL yet, set it now
    if (result.ok) {
      this.fullProxyUrl = `${this.proxyBaseUrl}:${result.data.host_port}`;
    }

    return result;
  }

  /** Stop container. Never throws; returns { ok, error? }. */
  async down(): Promise<DockerResult<DownResult>> {
    const { down } = await proxy();
    return down(this.containerName, this.sudo);
  }

  async logs(opts?: {
    since?: string;
    tail?: number | "all";
    stream?: boolean;
  }): Promise<DockerResult<LogsResult>> {
    const { logs } = await proxy();
    return logs(this.containerName, this.sudo, opts);
  }

  async run(
    script: string,
    opts?: {
      stream?: boolean;
      env?: string[]; // Array of "KEY=VALUE".
      envFile?: string; // Path to file with KEY=VALUE lines.
    },
  ): Promise<DockerResult<RunResult>> {
    const { run } = await proxy();
    return run(
      script,
      this.containerName,
      this.sudo,
      opts?.stream,
      opts?.env,
      opts?.envFile,
    );
  }

  // ---- Status (docker-only) -------------------------------------------------

  async status(): Promise<DockerResult<StatusResult>> {
    const { status } = await proxy();
    return status(this.containerName, this.sudo);
  }

  async pull(opts: ClientPullOptions): Promise<DockerResult<PullResult[]>> {
    const { pull } = await proxy();

    // Import constants
    const { ALL_SERVICES, ALL_LANGUAGES, selectLanguages } =
      await import("./constants.js");

    // Parse services
    let services: string[] = [];
    if (opts.services === "all") {
      services = [...ALL_SERVICES];
    } else if (Array.isArray(opts.services)) {
      services = opts.services;
    }

    // Parse languages and expand "ruby" and "ruby-sorbet" into all supported versions
    let languages: string[] = [];
    if (opts.languages === "all") {
      languages = [...ALL_LANGUAGES];
    } else if (Array.isArray(opts.languages)) {
      languages = selectLanguages(opts.languages, ALL_LANGUAGES);
    }

    // Get registry and image versions
    const containerRegistry =
      opts.containerRegistry ??
      process.env.CONTAINER_REGISTRTY ??
      DEFAULT_CONTAINER_REGISTRY;
    const languageVersion =
      opts.languageImageVersion ??
      process.env.LANGUAGE_IMAGE_VERSION ??
      DEFAULT_LANGUAGE_IMAGE_VERSION;
    const serviceVersion =
      opts.serviceImageVersion ??
      process.env.SERVICE_IMAGE_VERSION ??
      DEFAULT_SERVICE_IMAGE_VERSION;

    // Build list of images to pull
    const images: string[] = [];
    for (const service of services) {
      images.push(
        `${containerRegistry}/nuanced-lsp-${service}:${serviceVersion}`,
      );
    }
    for (const language of languages) {
      images.push(
        `${containerRegistry}/nuanced-lsp-${language}:${languageVersion}`,
      );
    }

    // Pull each image
    const results: PullResult[] = [];
    for (const image of images) {
      const res = await pull(image, this.sudo, opts.stream);
      if (!res.ok) {
        return res as DockerResult<PullResult[]>;
      }
      results.push(res.data);
    }

    return { ok: true, data: results };
  }

  // ---- Health (data-plane) --------------------------------------------------

  async health(timeoutSecs?: number): Promise<HttpResult<HealthResult>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<HealthResult>(
      "GET",
      "/v1/system/health",
      url,
      undefined,
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }

  // ---- Workspace ------------------------------------------------------------

  async listFiles(timeoutSecs?: number): Promise<HttpResult<ListFilesResult>> {
    // Check cache first
    const now = Date.now();
    if (
      this.fileListCache &&
      now - this.fileListCache.timestamp < this.CACHE_TTL_MS
    ) {
      return { ok: true, data: this.fileListCache.data };
    }

    // Cache miss or expired - fetch from server
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    const result = await httpRequestWithRetries<ListFilesResult>(
      "GET",
      "/v1/workspace/list-files",
      url,
      undefined,
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );

    // Update cache on successful response
    if (result.ok) {
      this.fileListCache = {
        data: result.data,
        timestamp: now,
      };
    }

    return result;
  }

  async readSource(
    filePath: string,
    range?: LspRange | null,
    timeoutSecs?: number,
  ): Promise<HttpResult<ReadSourceResult>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<ReadSourceResult>(
      "POST",
      "/v1/workspace/read-source-code",
      url,
      { path: filePath, range: range ?? null },
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }

  // ---- Symbols --------------------------------------------------------------

  async definitionsInFile(
    filePath: string,
    timeoutSecs?: number,
  ): Promise<HttpResult<DefinitionsInFileResult>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<DefinitionsInFileResult>(
      "GET",
      "/v1/symbol/definitions-in-file",
      url,
      undefined,
      { file_path: filePath },
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }

  async findDefinition(
    filePosition: FilePosition,
    includeRawResponse?: boolean,
    includeSourceCode?: boolean,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindDefinitionResult>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<FindDefinitionResult>(
      "POST",
      "/v1/symbol/find-definition",
      url,
      {
        include_raw_response: !!includeRawResponse,
        include_source_code: !!includeSourceCode,
        position: filePosition,
      },
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }

  async findIdentifier(
    filePath: string,
    name: string,
    position?: LspPosition | null,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindIdentifierResult>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<FindIdentifierResult>(
      "POST",
      "/v1/symbol/find-identifier",
      url,
      { name, path: filePath, position },
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }

  async findReferencedSymbols(
    identifierPosition: IdentifierPosition,
    fullScan = false,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindReferencedSymbolsResult>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<FindReferencedSymbolsResult>(
      "POST",
      "/v1/symbol/find-referenced-symbols",
      url,
      {
        full_scan: !!fullScan,
        identifier_position: identifierPosition,
      },
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }

  async findReferences(
    identifierPosition: IdentifierPosition,
    contextLines = 0,
    includeRawResponse?: boolean,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindReferencesResult>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<FindReferencesResult>(
      "POST",
      "/v1/symbol/find-references",
      url,
      {
        identifier_position: identifierPosition,
        include_code_context_lines: contextLines,
        include_raw_response: !!includeRawResponse,
      },
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }

  // ---- LSP JSON-RPC forwarding -----------------------------------------------

  async request(
    method: string,
    params: any | undefined,
    timeoutSecs?: number,
  ): Promise<HttpResult<any>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    const id = this.nextId++;
    const request: JsonRpcMessage = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };
    const result = await httpRequestWithRetries<JsonRpcMessage>(
      "POST",
      "/lsp",
      url,
      request,
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );

    if (!result.ok) {
      return result;
    }

    // If JSON-RPC response contains an error, translate to 500 HTTP error
    if (result.data.error) {
      return err({
        status_code: 500,
        error: result.data.error,
      });
    }

    // Return just the result data
    return ok(result.data.result);
  }

  async notify(
    method: string,
    params: any | undefined,
    timeoutSecs?: number,
  ): Promise<HttpResult<undefined>> {
    const url = await this.proxyUrl();
    const { httpRequestWithRetries } = await http();
    const request: JsonRpcMessage = {
      jsonrpc: "2.0",
      method,
      params,
    };
    return httpRequestWithRetries<undefined>(
      "POST",
      "/lsp",
      url,
      request,
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }
}

import type {
  DefinitionsInFileResult,
  DockerResult,
  DownResult,
  FilePosition,
  FindDefinitionResult,
  FindIdentifierResult,
  FindReferencesResult,
  FindReferencedSymbolsResult,
  HealthResult,
  HttpResult,
  IdentifierPosition,
  UpResult,
  ListFilesResult,
  LspPosition,
  LspRange,
  ReadSourceResult,
  RunResult,
  StatusResult,
  LogsResult,
  PullResult,
  JsonRpcRequest,
  JsonRpcResponse,
} from "./types.js";

import {
  DEFAULT_CONTAINER_NAME,
  DEFAULT_CONTAINER_PORT,
  DEFAULT_HOST_PORT,
  DEFAULT_HOST_URL,
  DEFAULT_TIMEOUT_SECS,
  DEFAULT_RETRIES,
} from "./defaults.js";
import { err, ok } from "./types.js";

// Precompiled tiny helpers
const TRAILING_SLASH_RE = /\/$/;

type HttpModule = typeof import("./http.js");
let _http: HttpModule | null = null;
async function http(): Promise<HttpModule> {
  // Lazy-load only when a data-plane method is called
  if (_http) return _http;
  _http = await import("./http.js");
  return _http!;
}

type ProxyModule = typeof import("./proxy.js");
let _proxy: ProxyModule | null = null;
async function proxy(): Promise<ProxyModule> {
  // Lazy-load only when a docker lifecycle method is called
  if (_proxy) return _proxy;
  _proxy = await import("./proxy.js");
  return _proxy!;
}

export class NuancedLspClient {
  readonly containerName: string;
  private readonly sudo?: boolean;
  private readonly retries: number;
  private readonly lsProxyPort: number;
  private readonly lsProxyBaseUrl: string;
  private timeoutSecs: number;

  private fullLsProxyUrl: string | undefined;
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
    lsProxyUrl?: string;
    lsProxyPort?: number;
    timeoutSecs?: number;
    retries?: number;
    sudo?: boolean;
  }) {
    this.lsProxyBaseUrl = (opts?.lsProxyUrl ?? DEFAULT_HOST_URL).replace(
      TRAILING_SLASH_RE,
      "",
    );
    const lsProxyPort =
      typeof opts?.lsProxyPort === "number"
        ? opts.lsProxyPort
        : DEFAULT_HOST_PORT;
    this.lsProxyPort = lsProxyPort;

    // If port is 0 (dynamic assignment), leave fullLsProxyUrl undefined
    // It will be resolved lazily when needed
    this.fullLsProxyUrl =
      lsProxyPort === 0 ? undefined : `${this.lsProxyBaseUrl}:${lsProxyPort}`;

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
  private async lsProxyUrl(): Promise<string> {
    if (this.fullLsProxyUrl) {
      return this.fullLsProxyUrl;
    }

    // Port not set - need to detect it from the running container
    const { port } = await proxy();
    const lsProxyPort = await port(
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
    this.fullLsProxyUrl = `${this.lsProxyBaseUrl}:${lsProxyPort}`;
    return this.fullLsProxyUrl;
  }

  // ---- Lifecycle (docker) ---------------------------------------------------
  async up(
    workspace: string,
    opts: {
      languageContainerVersion?: string;
      proxyImage?: string;
      watchdogImage?: string;
      wrapperImage?: string;
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
      envFile: opts.envFile,
    });

    // If up succeeded and we don't have a URL yet, set it now
    if (result.ok) {
      this.fullLsProxyUrl = `${this.lsProxyBaseUrl}:${result.data.host_port}`;
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

  async pull(
    image?: string,
    stream?: boolean,
  ): Promise<DockerResult<PullResult>> {
    const { pull } = await proxy();
    return pull(image, this.sudo, stream);
  }

  // ---- Health (data-plane) --------------------------------------------------

  async health(timeoutSecs?: number): Promise<HttpResult<HealthResult>> {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries } = await http();
    return httpRequestWithRetries<HealthResult>(
      "GET",
      "/v2/system/health",
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

    // Cache miss or expired - fetch from server via LSP command
    const result = await this.request(
      "lsproxy/workspace/listFiles",
      undefined,
      timeoutSecs,
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
    return this.request(
      "lsproxy/workspace/readSourceCode",
      { path: filePath, range: range ?? null },
      timeoutSecs,
    );
  }

  // ---- Symbols --------------------------------------------------------------

  async definitionsInFile(
    filePath: string,
    timeoutSecs?: number,
  ): Promise<HttpResult<DefinitionsInFileResult>> {
    return this.request(
      "lsproxy/symbol/definitionsInFile",
      { file_path: filePath },
      timeoutSecs,
    );
  }

  async findDefinition(
    filePosition: FilePosition,
    includeRawResponse?: boolean,
    includeSourceCode?: boolean,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindDefinitionResult>> {
    return this.request(
      "lsproxy/symbol/findDefinition",
      {
        include_raw_response: !!includeRawResponse,
        include_source_code: !!includeSourceCode,
        position: filePosition,
      },
      timeoutSecs,
    );
  }

  async findIdentifier(
    filePath: string,
    name: string,
    position?: LspPosition | null,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindIdentifierResult>> {
    return this.request(
      "lsproxy/symbol/findIdentifier",
      { name, path: filePath, position },
      timeoutSecs,
    );
  }

  async findReferencedSymbols(
    identifierPosition: IdentifierPosition,
    fullScan = false,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindReferencedSymbolsResult>> {
    return this.request(
      "lsproxy/symbol/findReferencedSymbols",
      {
        full_scan: !!fullScan,
        identifier_position: identifierPosition,
      },
      timeoutSecs,
    );
  }

  async findReferences(
    identifierPosition: IdentifierPosition,
    contextLines = 0,
    includeRawResponse?: boolean,
    timeoutSecs?: number,
  ): Promise<HttpResult<FindReferencesResult>> {
    return this.request(
      "lsproxy/symbol/findReferences",
      {
        identifier_position: identifierPosition,
        include_code_context_lines: contextLines,
        include_raw_response: !!includeRawResponse,
      },
      timeoutSecs,
    );
  }

  // ---- LSP JSON-RPC forwarding -----------------------------------------------

  async request(
    method: string,
    params: any | undefined,
    timeoutSecs?: number,
  ): Promise<HttpResult<any>> {
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries } = await http();
    const id = this.nextId++;
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };
    const result = await httpRequestWithRetries<JsonRpcResponse>(
      "POST",
      "/v2/lsp",
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
    const url = await this.lsProxyUrl();
    const { httpRequestWithRetries } = await http();
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      method,
      params,
    };
    return httpRequestWithRetries<undefined>(
      "POST",
      "/v2/lsp",
      url,
      request,
      undefined,
      this.retries,
      this.resolveTimeout(timeoutSecs),
    );
  }
}

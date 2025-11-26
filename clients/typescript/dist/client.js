import { DEFAULT_CONTAINER_NAME, DEFAULT_CONTAINER_PORT, DEFAULT_HOST_PORT, DEFAULT_HOST_URL, DEFAULT_TIMEOUT_SECS, DEFAULT_RETRIES, } from "./defaults.js";
// Precompiled tiny helpers
const TRAILING_SLASH_RE = /\/$/;
let _http = null;
async function http() {
    // Lazy-load only when a data-plane method is called
    if (_http)
        return _http;
    _http = await import("./http.js");
    return _http;
}
let _proxy = null;
async function proxy() {
    // Lazy-load only when a docker lifecycle method is called
    if (_proxy)
        return _proxy;
    _proxy = await import("./proxy.js");
    return _proxy;
}
export class NuancedLspClient {
    constructor(opts) {
        // Cache for list-files results to avoid redundant calls
        // TODO: Add proper cache invalidation when files are modified
        // Currently uses 15 second TTL as a simple invalidation strategy
        this.fileListCache = null;
        this.CACHE_TTL_MS = 15000; // 15 seconds
        this.lsProxyBaseUrl = (opts?.lsProxyUrl ?? DEFAULT_HOST_URL).replace(TRAILING_SLASH_RE, "");
        const lsProxyPort = typeof opts?.lsProxyPort === "number"
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
        // Port not set - need to detect it from the running container
        const { port } = await proxy();
        const lsProxyPort = await port(this.containerName, DEFAULT_CONTAINER_PORT, this.sudo);
        if (port === null) {
            throw new Error(`Cannot determine port for container ${this.containerName}. Is the container running?`);
        }
        // Cache the URL for future calls
        this.fullLsProxyUrl = `${this.lsProxyBaseUrl}:${lsProxyPort}`;
        return this.fullLsProxyUrl;
    }
    // ---- Lifecycle (docker) ---------------------------------------------------
    async up(workspace, opts) {
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
    async down() {
        const { down } = await proxy();
        return down(this.containerName, this.sudo);
    }
    async logs(opts) {
        const { logs } = await proxy();
        return logs(this.containerName, this.sudo, opts);
    }
    async run(script, opts) {
        const { run } = await proxy();
        return run(script, this.containerName, this.sudo, opts?.stream, opts?.env, opts?.envFile);
    }
    // ---- Status (docker-only) -------------------------------------------------
    async status() {
        const { status } = await proxy();
        return status(this.containerName, this.sudo);
    }
    async pull(image, stream) {
        const { pull } = await proxy();
        return pull(image, this.sudo, stream);
    }
    // ---- Health (data-plane) --------------------------------------------------
    async health(timeoutSecs) {
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        return httpRequestWithRetries("GET", "/v1/system/health", url, undefined, undefined, this.retries, this.resolveTimeout(timeoutSecs));
    }
    // ---- Workspace ------------------------------------------------------------
    async listFiles(timeoutSecs) {
        // Check cache first
        const now = Date.now();
        if (this.fileListCache &&
            now - this.fileListCache.timestamp < this.CACHE_TTL_MS) {
            return { ok: true, data: this.fileListCache.data };
        }
        // Cache miss or expired - fetch from server
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        const result = await httpRequestWithRetries("GET", "/v1/workspace/list-files", url, undefined, undefined, this.retries, this.resolveTimeout(timeoutSecs));
        // Update cache on successful response
        if (result.ok) {
            this.fileListCache = {
                data: result.data,
                timestamp: now,
            };
        }
        return result;
    }
    async readSource(filePath, range, timeoutSecs) {
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        return httpRequestWithRetries("POST", "/v1/workspace/read-source-code", url, { path: filePath, range: range ?? null }, undefined, this.retries, this.resolveTimeout(timeoutSecs));
    }
    // ---- Symbols --------------------------------------------------------------
    async definitionsInFile(filePath, timeoutSecs) {
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        return httpRequestWithRetries("GET", "/v1/symbol/definitions-in-file", url, undefined, { file_path: filePath }, this.retries, this.resolveTimeout(timeoutSecs));
    }
    async findDefinition(filePosition, includeRawResponse, includeSourceCode, timeoutSecs) {
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        return httpRequestWithRetries("POST", "/v1/symbol/find-definition", url, {
            include_raw_response: !!includeRawResponse,
            include_source_code: !!includeSourceCode,
            position: filePosition,
        }, undefined, this.retries, this.resolveTimeout(timeoutSecs));
    }
    async findIdentifier(filePath, name, position, timeoutSecs) {
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        return httpRequestWithRetries("POST", "/v1/symbol/find-identifier", url, { name, path: filePath, position }, undefined, this.retries, this.resolveTimeout(timeoutSecs));
    }
    async findReferencedSymbols(identifierPosition, fullScan = false, timeoutSecs) {
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        return httpRequestWithRetries("POST", "/v1/symbol/find-referenced-symbols", url, {
            full_scan: !!fullScan,
            identifier_position: identifierPosition,
        }, undefined, this.retries, this.resolveTimeout(timeoutSecs));
    }
    async findReferences(identifierPosition, contextLines = 0, includeRawResponse, timeoutSecs) {
        const url = await this.lsProxyUrl();
        const { httpRequestWithRetries } = await http();
        return httpRequestWithRetries("POST", "/v1/symbol/find-references", url, {
            identifier_position: identifierPosition,
            include_code_context_lines: contextLines,
            include_raw_response: !!includeRawResponse,
        }, undefined, this.retries, this.resolveTimeout(timeoutSecs));
    }
}

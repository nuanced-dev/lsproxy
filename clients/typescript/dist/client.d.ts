import type { DefinitionsInFileResult, DockerResult, DownResult, FilePosition, FindDefinitionResult, FindIdentifierResult, FindReferencesResult, FindReferencedSymbolsResult, HealthResult, HttpResult, IdentifierPosition, UpResult, ListFilesResult, LspPosition, LspRange, ReadSourceResult, RunResult, StatusResult, LogsResult, PullResult } from "./types.js";
export declare class NuancedLspClient {
    readonly containerName: string;
    private readonly sudo?;
    private readonly retries;
    private readonly lsProxyPort;
    private readonly lsProxyBaseUrl;
    private timeoutSecs;
    private fullLsProxyUrl;
    private fileListCache;
    private readonly CACHE_TTL_MS;
    constructor(opts?: {
        containerName?: string;
        lsProxyUrl?: string;
        lsProxyPort?: number;
        timeoutSecs?: number;
        retries?: number;
        sudo?: boolean;
    });
    private resolveTimeout;
    /**
     * Get the LSProxy URL. If the URL is not set (because port was 0 in constructor),
     * dynamically determine the port from the running container.
     */
    private lsProxyUrl;
    up(workspace: string, opts: {
        languageContainerVersion?: string;
        proxyImage?: string;
        watchdogImage?: string;
        wrapperImage?: string;
        timeout?: number;
        stream?: boolean;
        ro?: boolean;
        bindHost?: string;
        debug?: boolean;
        env?: string[];
        envFile?: string;
    }): Promise<DockerResult<UpResult>>;
    /** Stop container. Never throws; returns { ok, error? }. */
    down(): Promise<DockerResult<DownResult>>;
    logs(opts?: {
        since?: string;
        tail?: number | "all";
        stream?: boolean;
    }): Promise<DockerResult<LogsResult>>;
    run(script: string, opts?: {
        stream?: boolean;
        env?: string[];
        envFile?: string;
    }): Promise<DockerResult<RunResult>>;
    status(): Promise<DockerResult<StatusResult>>;
    pull(image?: string, stream?: boolean): Promise<DockerResult<PullResult>>;
    health(timeoutSecs?: number): Promise<HttpResult<HealthResult>>;
    listFiles(timeoutSecs?: number): Promise<HttpResult<ListFilesResult>>;
    readSource(filePath: string, range?: LspRange | null, timeoutSecs?: number): Promise<HttpResult<ReadSourceResult>>;
    definitionsInFile(filePath: string, timeoutSecs?: number): Promise<HttpResult<DefinitionsInFileResult>>;
    findDefinition(filePosition: FilePosition, includeRawResponse?: boolean, includeSourceCode?: boolean, timeoutSecs?: number): Promise<HttpResult<FindDefinitionResult>>;
    findIdentifier(filePath: string, name: string, position?: LspPosition | null, timeoutSecs?: number): Promise<HttpResult<FindIdentifierResult>>;
    findReferencedSymbols(identifierPosition: IdentifierPosition, fullScan?: boolean, timeoutSecs?: number): Promise<HttpResult<FindReferencedSymbolsResult>>;
    findReferences(identifierPosition: IdentifierPosition, contextLines?: number, includeRawResponse?: boolean, timeoutSecs?: number): Promise<HttpResult<FindReferencesResult>>;
}
//# sourceMappingURL=client.d.ts.map
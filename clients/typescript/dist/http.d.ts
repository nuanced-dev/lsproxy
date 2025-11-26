import type { HttpResult } from "./types.js";
export declare function doRequest<T>(method: "GET" | "POST", baseUrl: string, path: string, json?: any, params?: Record<string, any>, timeoutMs?: number): Promise<HttpResult<T>>;
export declare function httpRequestWithRetries<T>(method: "GET" | "POST", path: string, baseUrl: string, json?: any, params?: Record<string, any>, retries?: number, timeoutSecs?: number): Promise<HttpResult<T>>;
export declare function pollHttpWithRetries<T>(method: "GET" | "POST", path: string, baseUrl: string, { json, params, retriesPerAttempt, perAttemptTimeoutSecs, overallTimeoutSecs, shouldStop, // <- fixed arity
initialDelayMs, maxDelayMs, backoffFactor, }?: {
    json?: any;
    params?: Record<string, any>;
    retriesPerAttempt?: number;
    perAttemptTimeoutSecs?: number;
    overallTimeoutSecs?: number;
    shouldStop?: (res: HttpResult<T>, attempt: number) => boolean;
    initialDelayMs?: number;
    maxDelayMs?: number;
    backoffFactor?: number;
}): Promise<HttpResult<T>>;
//# sourceMappingURL=http.d.ts.map
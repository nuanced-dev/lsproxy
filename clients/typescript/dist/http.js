import { DEFAULT_RETRIES, DEFAULT_TIMEOUT_SECS } from "./defaults.js";
import { ok, err } from "./types.js";
const sleep = (ms) => ms > 0 ? new Promise((r) => setTimeout(r, ms)) : Promise.resolve();
function parseErrObject(err) {
    if (!err)
        return null;
    if (typeof err === "object")
        return err;
    if (typeof err === "string") {
        // try JSON first
        try {
            return JSON.parse(err);
        }
        catch {
            // ignore
        }
        // fall back to simple code extraction (e.g. "code=ECONNRESET")
        const m = /code=([A-Z_]+)/.exec(err);
        return m ? { code: m[1] } : { message: err };
    }
    return { message: String(err) };
}
function shouldRetryResult(res) {
    if (res.ok)
        return false;
    // res is the Err arm: res.data is HttpErr
    const e = parseErrObject(res.data?.error);
    const name = e?.name;
    if (name === "AbortError")
        return true;
    const code = e?.code;
    const RETRYABLE_CODES = new Set([
        "ETIMEDOUT",
        "ECONNRESET",
        "ECONNREFUSED",
        "EAI_AGAIN",
        "ENETUNREACH",
        "EHOSTUNREACH",
        "EPIPE",
        "EAGAIN",
        "UND_ERR_SOCKET",
    ]);
    if (code && RETRYABLE_CODES.has(code))
        return true;
    return false;
}
function formatHttpErrorForResult(err, url, timeoutMs) {
    const c = err?.cause ?? {};
    return {
        name: err?.name ?? "Error",
        message: err?.message ?? String(err),
        code: c.code,
        errno: c.errno,
        syscall: c.syscall,
        address: c.address,
        port: c.port,
        timeoutMs,
        url: url.toString(),
    };
}
export async function doRequest(method, baseUrl, path, json, params, timeoutMs = 10000) {
    const url = new URL(path, baseUrl);
    if (params)
        for (const [k, v] of Object.entries(params))
            url.searchParams.set(k, String(v));
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
        const res = await fetch(url, {
            method,
            headers: method === "POST" ? { "Content-Type": "application/json" } : undefined,
            body: method === "POST" ? JSON.stringify(json ?? {}) : undefined,
            signal: controller.signal,
        });
        if (!res.ok) {
            // Expecting body: { error: ... }
            let body = null;
            try {
                body = await res.json();
            }
            catch {
                body = null;
            }
            return err({
                status_code: res.status,
                error: {
                    message: body?.error ?? `HTTP ${res.status}`,
                    url: url.toString(),
                },
            });
        }
        const data = (await res.json());
        return ok(data);
    }
    catch (e) {
        return err({
            status_code: null,
            error: formatHttpErrorForResult(e, url, timeoutMs),
        });
    }
    finally {
        clearTimeout(timer);
    }
}
export async function httpRequestWithRetries(method, path, baseUrl, json, params, retries = DEFAULT_RETRIES, timeoutSecs = DEFAULT_TIMEOUT_SECS) {
    for (let attempt = 0;; attempt += 1) {
        const res = await doRequest(method, baseUrl, path, json, params, timeoutSecs * 1000);
        if (res.ok)
            return res;
        // stop if we shouldn't retry this particular error
        if (!shouldRetryResult(res))
            return res;
        // stop if we've exhausted attempts
        if (attempt >= retries)
            return res;
        const base = 500 * 2 ** attempt;
        const jittered = Math.floor(base * (0.8 + Math.random() * 0.4));
        await sleep(jittered);
    }
}
export async function pollHttpWithRetries(method, path, baseUrl, { json, params, retriesPerAttempt = 1, perAttemptTimeoutSecs = 1, overallTimeoutSecs = DEFAULT_TIMEOUT_SECS, shouldStop = (res, _attempt) => res.ok, // <- fixed arity
initialDelayMs = 200, maxDelayMs = 1000, backoffFactor = 1.5, } = {}) {
    const start = Date.now();
    let delay = initialDelayMs;
    let attempt = 0;
    let last = null;
    while (Date.now() - start < overallTimeoutSecs * 1000) {
        const res = await httpRequestWithRetries(method, path, baseUrl, json, params, retriesPerAttempt, perAttemptTimeoutSecs);
        last = res;
        if (shouldStop(res, attempt)) {
            return res;
        }
        await sleep(delay);
        delay = Math.min(maxDelayMs, Math.floor(delay * backoffFactor));
        attempt += 1;
    }
    // Deadline hit: return best info available
    if (last)
        return last;
    return err({
        status_code: 408,
        error: {
            message: `Timed out after ${overallTimeoutSecs}s without a response`,
        },
    });
}

import type { DockerResult, DownResult, LogsResult, PullResult, RunResult, StatusResult, UpResult } from "./types.js";
export type RunDockerResult = {
    ok: boolean;
    code: number;
    stdout: string;
    stderr: string;
    cmd: string;
    streamed: boolean;
};
export declare function up(workspace: string, opts: {
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
}): Promise<DockerResult<UpResult>>;
export declare function down(containerName: string, sudo?: boolean): Promise<DockerResult<DownResult>>;
export declare function port(containerName: string, containerPort: number, sudo?: boolean): Promise<number | null>;
export declare function logs(containerName: string, sudo?: boolean, opts?: {
    stream?: boolean;
    since?: string | Date;
    tail?: number | "all";
}): Promise<DockerResult<LogsResult>>;
export declare function run(script: string, containerName: string, sudo?: boolean, stream?: boolean, env?: string[], envFile?: string): Promise<DockerResult<RunResult>>;
export declare function status(containerName: string, sudo?: boolean): Promise<DockerResult<StatusResult>>;
/** Pull image. Supports optional streaming. Never throws. */
export declare function pull(image?: string, sudo?: boolean, stream?: boolean): Promise<DockerResult<PullResult>>;
//# sourceMappingURL=proxy.d.ts.map
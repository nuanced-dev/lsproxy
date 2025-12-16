import * as fs from 'fs/promises';
import * as lockfile from 'proper-lockfile';

export class ProcessRc {
    private readonly filePath: string;
    private readonly lockOptions: lockfile.LockOptions;

    constructor(filePath: string) {
        this.filePath = filePath;
        this.lockOptions = {
            retries: {
                retries: 5,
                minTimeout: 100,
                maxTimeout: 1000,
            },
        };
    }

    async acquire(init?: () => Promise<void>): Promise<void> {
        const release = await this.lockFile();
        try {
            const pids = await this.readPids();
            const runningPids = await this.filterRunningPids(pids);

            if (runningPids.length === 0 && init) {
                await init();
            }

            runningPids.push(process.pid);
            await this.writePids(runningPids);
        } finally {
            await release();
        }
    }

    async release(cleanup?: () => Promise<void>): Promise<void> {
        const release = await this.lockFile();
        try {
            const pids = await this.readPids();
            const runningPids = await this.filterRunningPids(pids);
            const filteredPids = runningPids.filter(pid => pid !== process.pid);
            await this.writePids(filteredPids);

            if (filteredPids.length === 0 && cleanup) {
                await cleanup();
            }
        } finally {
            await release();
        }
    }

    private async lockFile(): Promise<() => Promise<void>> {
        await this.ensureFileExists();
        return await lockfile.lock(this.filePath, this.lockOptions);
    }

    private async ensureFileExists(): Promise<void> {
        try {
            await fs.access(this.filePath);
        } catch {
            await fs.writeFile(this.filePath, JSON.stringify([]));
        }
    }

    private async readPids(): Promise<number[]> {
        const content = await fs.readFile(this.filePath, 'utf-8');
        return JSON.parse(content);
    }

    private async writePids(pids: number[]): Promise<void> {
        await fs.writeFile(this.filePath, JSON.stringify(pids));
    }

    private async filterRunningPids(pids: number[]): Promise<number[]> {
        const runningPids: number[] = [];
        for (const pid of pids) {
            if (await this.isProcessRunning(pid)) {
                runningPids.push(pid);
            }
        }
        return runningPids;
    }

    private async isProcessRunning(pid: number): Promise<boolean> {
        try {
            process.kill(pid, 0);
            return true;
        } catch {
            return false;
        }
    }
}

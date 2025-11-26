import { cpus } from "node:os";
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/**/*.spec.ts"],
    testTimeout: Number(process.env.NUANCED_LSP_TIMEOUT ?? "120") * 1000,
    hookTimeout: Number(process.env.NUANCED_LSP_TIMEOUT ?? "120") * 1000,
    retry: 0,
    globals: true,
    reporters: process.env.CI ? ["default"] : ["default"],
    minWorkers: process.env.WORKERS ? Number(process.env.WORKERS) : undefined,
    maxWorkers: process.env.WORKERS ? Number(process.env.WORKERS) : undefined,
    bail: process.env.FAIL_FAST ? Number(process.env.FAIL_FAST) : undefined,
    pool: "threads",
    poolOptions: {
      threads: {
        maxThreads: cpus().length,
        minThreads: cpus().length,
      },
    },
  },
});

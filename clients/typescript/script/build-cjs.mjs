#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");

// Mark runtime deps as external so we don't vendor them into the CJS file
const pkg = JSON.parse(await readFile(path.join(root, "package.json"), "utf8"));
const externals = [
  ...Object.keys(pkg.dependencies ?? {}),
  ...Object.keys(pkg.peerDependencies ?? {}),
];

await build({
  entryPoints: ["src/index.ts"],
  platform: "node",
  target: "node20",
  format: "cjs",
  bundle: true,
  external: externals,
  sourcemap: true,
  outfile: "dist/index.cjs",
});

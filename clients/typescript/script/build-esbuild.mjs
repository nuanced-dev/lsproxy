import { build } from "esbuild";

await build({
  entryPoints: ["src/cli.ts"],
  bundle: true,
  platform: "node",
  target: "node20", // match your supported Node
  outfile: "dist/cli.cjs", // single-file CLI output
  format: "cjs", // your project is "type": "module"
  sourcemap: false,
  external: [
    // add any native/optional deps you don't want bundled, e.g.:
    // 'fsevents'
  ],
});

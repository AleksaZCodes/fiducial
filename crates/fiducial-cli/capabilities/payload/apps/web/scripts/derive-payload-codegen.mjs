import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";

if (!existsSync("node_modules/payload/bin.js")) {
  process.exit(0);
}

function run(...args) {
  const result = spawnSync(process.execPath, args, { stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

run(
  "--import", "./scripts/payload-ts-resolve.mjs",
  "node_modules/payload/bin.js",
  "generate:types",
  "--disable-transpile",
);
run(
  "--import", "./scripts/payload-ts-resolve.mjs",
  "node_modules/payload/bin.js",
  "generate:importmap",
  "--disable-transpile",
);

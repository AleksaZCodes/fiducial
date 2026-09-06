#!/usr/bin/env node
/**
 * @fiducial/cli — thin Node.js shim for the `fid` Rust binary.
 *
 * When installed via `npm install -g @fiducial/cli` or added as a dev
 * dependency, this shim locates the native `fid` binary and exec-replaces
 * the process with it, passing all arguments through unchanged.
 *
 * Resolution order:
 *   1. `FID_BIN` environment variable (override for CI or custom installs)
 *   2. `fid` on PATH (installed via `cargo install fiducial-cli` or system pkg)
 *
 * If neither is found, an actionable error is printed with install instructions.
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const args = process.argv.slice(2);

// ── Locate the binary ────────────────────────────────────────────────────────

function findBin() {
  // 1. Explicit override.
  const override = process.env.FID_BIN;
  if (override) {
    const abs = resolve(override);
    if (existsSync(abs)) return abs;
    console.error(
      `[fid] FID_BIN is set to "${override}" but the file does not exist.`,
    );
    process.exit(1);
  }

  // 2. PATH — let the OS find it.
  return "fid";
}

const bin = findBin();

// ── Exec ─────────────────────────────────────────────────────────────────────

const result = spawnSync(bin, args, {
  stdio: "inherit",
  // On Windows, shell: true is required to resolve PATH entries.
  shell: process.platform === "win32",
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(
      `[fid] The \`fid\` binary was not found.\n\n` +
        `Install it with:\n` +
        `  cargo install fiducial-cli\n\n` +
        `Or set the FID_BIN environment variable to its path.`,
    );
  } else {
    console.error(`[fid] Failed to launch fid: ${result.error.message}`);
  }
  process.exit(1);
}

process.exit(result.status ?? 0);

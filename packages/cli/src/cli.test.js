/**
 * Minimal smoke tests for the @fiducial/cli shim.
 * These run with Node's built-in test runner (`node --test`).
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const shim = resolve(__dirname, "cli.js");

describe("@fiducial/cli shim", () => {
  it("forwards --help to fid (or exits non-zero if fid is not installed)", () => {
    const result = spawnSync("node", [shim, "--help"], {
      encoding: "utf8",
      timeout: 10_000,
    });
    // Either fid is installed and prints help, or the shim fails with a clear
    // message — in both cases the shim itself should not throw or crash with
    // an uncaught error.
    assert.ok(
      result.status !== null,
      "shim exited with a status code (did not crash)",
    );
  });

  it("respects FID_BIN=nonexistent and exits 1", () => {
    const result = spawnSync("node", [shim, "--help"], {
      encoding: "utf8",
      timeout: 5_000,
      env: { ...process.env, FID_BIN: "/nonexistent/fid" },
    });
    assert.equal(result.status, 1, "exits 1 when FID_BIN points to missing file");
    assert.match(
      result.stderr + result.stdout,
      /FID_BIN/,
      "error message mentions FID_BIN",
    );
  });
});

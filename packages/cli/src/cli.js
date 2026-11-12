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
 *   3. The release's prebuilt binary, downloaded once and cached — on Linux
 *      x86-64, when a version is known. The version comes from `FID_VERSION`
 *      or from the `FID_VERSION:` line of the product's CI workflow, which is
 *      where a product already declares which `fid` gates it.
 *
 * Step 3 is what makes `pnpm exec fid` work on a machine with no Rust
 * toolchain: a cloud session, a CI runner, a new contributor's laptop.
 *
 * If none is found, an actionable error is printed with install instructions.
 */

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

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

/** The version this product gates with, if it says: env first, then its CI. */
function wantedVersion(cwd = process.cwd(), env = process.env) {
  if (env.FID_VERSION) return env.FID_VERSION.replace(/^v/, "");
  const ci = join(cwd, ".github/workflows/ci.yml");
  if (!existsSync(ci)) return null;
  for (const line of readFileSync(ci, "utf8").split("\n")) {
    const m = /^\s*FID_VERSION:\s*["']?([0-9][^"'\s]*)["']?\s*$/.exec(line);
    if (m) return m[1];
  }
  return null;
}

/** Download the prebuilt binary for `version`, verify it, cache it, return its path. */
async function fetchPrebuilt(version) {
  if (process.platform !== "linux" || process.arch !== "x64") return null;
  const dir = join(process.env.XDG_CACHE_HOME ?? join(homedir(), ".cache"), "fiducial", version);
  const bin = join(dir, "fid");
  if (existsSync(bin)) return bin;

  const asset = `fid-${version}-x86_64-unknown-linux-gnu.tar.gz`;
  const url = `https://github.com/AleksaZCodes/fiducial/releases/download/v${version}/${asset}`;
  const [archive, sums] = await Promise.all([fetch(url), fetch(`${url}.sha256`)]);
  if (!archive.ok || !sums.ok) return null;
  const bytes = Buffer.from(await archive.arrayBuffer());
  const want = (await sums.text()).trim().split(/\s+/)[0];
  const got = createHash("sha256").update(bytes).digest("hex");
  if (want !== got) {
    console.error(`[fid] checksum mismatch for ${asset} — refusing to run it.`);
    process.exit(1);
  }
  mkdirSync(dir, { recursive: true });
  const tarball = join(dir, asset);
  writeFileSync(tarball, bytes);
  const untar = spawnSync("tar", ["-xzf", tarball, "-C", dir, "fid"], { stdio: "inherit" });
  if (untar.status !== 0 || !existsSync(bin)) return null;
  return bin;
}

if (result.error?.code === "ENOENT" && !process.env.FID_BIN) {
  const version = wantedVersion();
  const prebuilt = version ? await fetchPrebuilt(version).catch(() => null) : null;
  if (prebuilt) {
    const rerun = spawnSync(prebuilt, args, { stdio: "inherit" });
    process.exit(rerun.status ?? 1);
  }
}

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(
      `[fid] The \`fid\` binary was not found.\n\n` +
        `Install it with:\n` +
        `  scripts/install-fid.sh <version>   (prebuilt, Linux x86-64)\n` +
        `  cargo install fiducial-cli         (anywhere, from source)\n\n` +
        `Or set FID_VERSION and this shim downloads the prebuilt binary itself.\n` +
        `Or set the FID_BIN environment variable to its path.`,
    );
  } else {
    console.error(`[fid] Failed to launch fid: ${result.error.message}`);
  }
  process.exit(1);
}

process.exit(result.status ?? 0);

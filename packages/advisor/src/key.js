/**
 * Where the advisory key comes from.
 *
 * `fid` itself makes no network calls — every remote thing it does, it does by
 * shelling out to a tool built for it. That posture is deliberate and this
 * package is what preserves it: `fid advise` shells out here, exactly as it
 * shells out to `git`, so the HTTP and TLS stack lands in Node where the
 * tested adapter already lives instead of being linked into the binary and
 * reimplemented in a second language.
 *
 * `fid` still owns *writing* the credentials file, because writing a config
 * file is squarely what `fid` does and a key is not something a product
 * declares in `fiducial.toml` — a secret in a committed file is a leaked
 * secret. This module only reads.
 *
 * Resolution order, first hit wins:
 *
 *   1. `OPENROUTER_API_KEY` in the environment  → the gateway route
 *   2. `TYPESAFE_API_KEY` in the environment    → the direct route
 *   3. `$XDG_CONFIG_HOME/fiducial/credentials.toml`, else
 *      `~/.config/fiducial/credentials.toml`     → written by `fid advise key set`
 *
 * The environment wins over the file so CI can inject a key without writing
 * one, and so a developer can override their stored key for one command.
 */

import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

/** Path to the credentials file, honouring XDG. */
export function credentialsPath() {
  const base = process.env.XDG_CONFIG_HOME || join(homedir(), ".config");
  return join(base, "fiducial", "credentials.toml");
}

/**
 * Read `key = "value"` pairs from the credentials file.
 *
 * Deliberately not a TOML parser: this file has exactly one shape, `fid` is
 * the only thing that writes it, and a dependency added to read three lines is
 * a dependency to audit and update forever. Anything it cannot parse is
 * ignored rather than thrown on — a malformed credentials file should degrade
 * to "no key" and let the caller print the actionable message, not crash a
 * hook.
 */
function readCredentials() {
  const path = credentialsPath();
  if (!existsSync(path)) return {};

  const out = {};
  let text;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return {};
  }

  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#") || trimmed.startsWith("[")) continue;
    const eq = trimmed.indexOf("=");
    if (eq === -1) continue;
    const name = trimmed.slice(0, eq).trim();
    let value = trimmed.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (name && value) out[name] = value;
  }
  return out;
}

/**
 * Resolve a key and the route it implies.
 *
 * Returns `{ vendor, env }` ready to hand to a vendor constructor, or
 * `{ vendor: null, reason }` when nothing is configured. A missing key is a
 * normal state, not an error: advisory checks are unlocked *by* a key, so the
 * absence of one has to be reportable rather than thrown.
 */
export function resolveKey(env = process.env) {
  if (env.OPENROUTER_API_KEY) {
    return {
      vendor: "openrouter",
      env: { OPENROUTER_API_KEY: env.OPENROUTER_API_KEY },
      source: "OPENROUTER_API_KEY in the environment",
    };
  }
  if (env.TYPESAFE_API_KEY) {
    return {
      vendor: "typesafe",
      env: { TYPESAFE_API_KEY: env.TYPESAFE_API_KEY },
      source: "TYPESAFE_API_KEY in the environment",
    };
  }

  const stored = readCredentials();
  if (stored.openrouter_api_key) {
    return {
      vendor: "openrouter",
      env: { OPENROUTER_API_KEY: stored.openrouter_api_key },
      source: credentialsPath(),
    };
  }
  if (stored.typesafe_api_key) {
    return {
      vendor: "typesafe",
      env: { TYPESAFE_API_KEY: stored.typesafe_api_key },
      source: credentialsPath(),
    };
  }

  return {
    vendor: null,
    reason:
      "no advisory key configured. Set one with `fid advise key set` (or export " +
      "OPENROUTER_API_KEY). Advisory checks stay off until you do; nothing " +
      "else changes.",
  };
}

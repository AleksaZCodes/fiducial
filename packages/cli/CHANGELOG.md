# @fiducial/cli

## 0.2.0

### Minor Changes

- 0894e7c: Add `@fiducial/cli` npm shim and Phase 3b capability mechanism.
  
  **`@fiducial/cli`** — new package. Node shim that resolves `fid` from `FID_BIN` or PATH and delegates to the native binary. Enables `npx @fiducial/cli new my-product` without installing Rust.
  
  **`fid` CLI (fiducial-cli v0.1.0)** — new binary crate.
  
  - `fid new <name>` — scaffold a product repository: `fiducial.toml`, `fiducial.lock`, `MISSION.md`, `AGENTS.md`, `.claude/settings.json` (guard hook), `.gitignore`, `README.md`, `git init`.
  - `fid add app next|tauri|worker` / `fid add firmware rp2040` — install built-in capabilities: write template files, activate guard rules in `fiducial.toml`, install `SKILL.md` into `.claude/skills/<id>.md`.
  - `fid capability list [--all]` / `fid capability check` / `fid capability new <name>` — manage capabilities.
  - `fid doctor` — verify `fiducial.toml`, `fiducial.lock`, and SHA-256 template integrity.
  - `fid guard-check` — shell-aware `PreToolUse` hook; tokenizes Bash commands before matching guard rules so `echo "npm install"` never triggers the `npm` rule.
  - Stub commands with full `--help`: `fid derive`, `fid upgrade`, `fid graph` (Phase 4).
  - Built-in capabilities: `web-next` (Next.js 15), `firmware-rp2040` (Embassy), `tauri`, `worker-cloudflare`.
  - 11 guard unit tests; `fiducial.lock` re-baselined after every capability install so `fid doctor` stays clean.

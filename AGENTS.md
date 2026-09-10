# Fiducial — Agent context

> One declaration, many derivations. See `MISSION.md` for the why.

This file covers the same ground as `CLAUDE.md` for agent runtimes that read
`AGENTS.md` (Codex, Copilot Workspace, etc.). The authoritative design is in
`docs/specs/2026-09-06-fiducial-design.md`.

## Current phase

Read `PHASES.md` before every session. It states which phase is active and what
"done" means for it. Do not begin the next phase until the current one is
complete.

## Core rules (from MISSION.md)

1. **One declaration, many derivations.** A value used in two places is declared
   once. If you are typing the same value into a second file, stop.

2. **Artifacts are generated, never hand-edited.** Changing a derived file by
   hand is an error. Change the upstream declaration and re-run the pipeline.

3. **Decisions are appended, not edited.** New decisions go in `docs/specs/`
   with a date. Old decisions are never modified.

4. **The mission is the tiebreaker.** When ambiguous, `MISSION.md` resolves it.

5. **No capability without a real need.** The Rule of Two (§16 of the design
   spec): generalize only when a second real product needs it.

## Repo layout

```
fiducial/
├── MISSION.md, STACK.md, PHASES.md
├── crates/               11 Rust members; no_std spine + `fid` CLI
│   ├── fiducial-core, -protocol, -quantity, -model
│   ├── fiducial-geometry, -mesh   geometry + case generation
│   ├── fiducial-eda               board.interface.json schema
│   └── fiducial-cli, -wasm, -tauri, fiducial
├── firmware/             Separate workspace (Embassy; rp2040 + stm32)
├── packages/             10 JS/TS packages (pnpm + Turborepo)
├── docs/specs/           Design decisions (append-only)
└── .github/workflows/    CI (ci.yml) + release (release.yml)
```

`ls crates packages` beats this tree — it is hand-maintained and drifts.

## Tools — use these before writing library code

### context7 (live documentation)

Before writing code against any named library — Embassy, wasm-bindgen, wasm-pack,
Turborepo, Changesets, Tauri, probe-rs, Next.js, SvelteKit, and anything else with a
versioned API — fetch current docs via context7. Training data goes stale; library
feature names change. The Phase 2 Embassy renames (`arch-cortex-m` → `platform-cortex-m`,
`integrated-timers` removed, `embassy-rp` jumping from 0.3 to 0.10) cost a full
trial-and-error loop because context7 was not used. That is avoidable.

Step 1: `resolve-library-id` with the library name and a topic query.
Step 2: `query-docs` with the returned ID and a specific question.

### Claude Code skills

`/code-review`, `/commit`, `/commit-push-pr`, `/run`, `/security-review`,
`/update-config`, `/claude-api`. Invoke by name or via the Skill tool.

### GitHub MCP plugin

`mcp__plugin_github_github__*` — search code, read files, create PRs, manage issues.
Use `get_me` first to confirm current user context.

## Build

```sh
pnpm build       # JS workspace (turbo)
cargo build      # Rust workspace
cargo test && pnpm test
```

## What not to do

- Do not modify `/home/aleksa/dev/rop-reference/` (real secrets).
- Do not touch ROP production code before Phase 6 is green.
- Do not publish or push the rop-reference directory.

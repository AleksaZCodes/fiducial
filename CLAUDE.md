# Fiducial — Claude Code context

> One declaration, many derivations. See `MISSION.md` for the why; `STACK.md` for the what.

## Project layout

```
fiducial/
├── MISSION.md          Why this exists — the tiebreaker for ambiguous decisions
├── STACK.md            Enumerated technology choices
├── PHASES.md           Build order and current state — read first each session
├── crates/             Rust workspace members (host compilation)
│   └── fiducial/       crates.io placeholder → growing to fiducial-core etc.
├── packages/           JS/TS workspace packages (pnpm + Turborepo)
│   └── fiducial/       @fiducial/fiducial on npm
├── docs/specs/         Design specs (append-only decisions)
└── .github/workflows/  CI + release automation
```

## Current phase: Phase 3 — `fid` CLI

Read `PHASES.md` at the start of every session. The current phase row says what is
in progress and what "done" means. Do not start Phase N+1 work inside Phase N.

## Rules

### Never hand-edit a generated artifact

Generated files are blocked by guard rules (Phase 3). If you need to change a
generated file, change its **upstream declaration** and re-run the pipeline.

### One declaration, many derivations

A value used in two places belongs in one declared location with generated
derivations. If you are about to type the same value a second time, stop — find
or create its declaration.

### Derived artifacts are never committed by hand

Run `cargo build` / `pnpm build` / the appropriate `fid derive` command and let
CI verify freshness. A stale artifact in CI is a bug in the pipeline, not a
prompt to update the artifact by hand.

### Decisions are appended, not edited

When a new decision supersedes an old one, append it to `docs/specs/` with a
date stamp. Do not edit old decisions. The history is the record.

### The mission is the tiebreaker

When a technical decision is genuinely ambiguous, `MISSION.md` resolves it.

## Tools available in this session

### context7 — always use for library/framework questions

Before writing code that touches any named library (Embassy, wasm-bindgen, wasm-pack,
Turborepo, Changesets, Tauri, embassy-rp, embassy-executor, embassy-time, probe-rs,
Next.js, SvelteKit, …) call context7 to get current docs. Training data goes stale;
library APIs change without warning. The Phase 2 Embassy feature renames
(`arch-cortex-m` → `platform-cortex-m`, `integrated-timers` removed, `embassy-rp 0.3`
→ `0.10`) were discovered by trial-and-error because context7 was not used. That
cost is avoidable.

```
# Pattern — always do this before writing library code:
1. mcp__context7__resolve-library-id  (libraryName: "Embassy", query: "...")
2. mcp__context7__query-docs          (libraryId: "/embassy-rs/embassy", query: "...")
```

### Skills available (`/skill-name` or via Skill tool)

| Skill | When to use |
|---|---|
| `claude-api` | Any Claude/Anthropic API question — model IDs, pricing, streaming, tool use |
| `code-review` | Review the current diff or a PR for bugs and simplifications |
| `commit-commands:commit` | Create a well-formed commit |
| `commit-commands:commit-push-pr` | Commit, push, and open a PR |
| `run` | Run and screenshot the app to verify a change works |
| `security-review` | Security audit of changed code |
| `update-config` | Modify Claude Code settings, hooks, permissions |

### MCP plugins available

- **`mcp__plugin_github_github__*`** — GitHub API: PRs, issues, branches, files
- **`mcp__context7__*`** — Live library documentation
- **`mcp__ide__getDiagnostics`** — Pull current IDE errors/warnings into context

## Build commands

```sh
# JS/TS
pnpm build          # turbo build all packages
pnpm typecheck      # turbo typecheck
pnpm test           # turbo test

# Rust
cargo build         # workspace build
cargo test          # workspace tests
cargo clippy        # lint (CI treats warnings as errors)
cargo fmt           # format (CI checks)

# Changesets (versioning)
pnpm changeset      # create a changeset for a PR
pnpm version        # apply pending changesets (done by CI)
pnpm release        # build + publish (done by CI)
```

## CI

- `ci.yml` — runs on every PR and push to `main`. Rust (fmt, clippy, test) + JS
  (build, typecheck, lint, test).
- `release.yml` — runs on merge to `main`. Opens or updates a Release PR while
  changesets are pending; publishes to npm and crates.io when the Release PR is
  merged.

## Secrets needed (set in GitHub repo settings)

| Secret | What it does |
| --- | --- |
| `NPM_TOKEN` | Publish `@fiducial/*` packages to npm |
| `CARGO_REGISTRY_TOKEN` | Publish `fiducial-*` crates to crates.io |

## What not to do

- **Do not modify `/home/aleksa/dev/rop-reference/`** — pre-migration ROP snapshot,
  contains real secrets, never push.
- **Do not start Phase 6 (ROP characterization) before Phase 6 is the active phase.**
- **Do not touch ROP production code before Phase 6 is green.**
- **Do not add a capability or rule unless a real product needs it** (Rule of Two, §16).

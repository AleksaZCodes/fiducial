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

## Current phase: Phase 1 — monorepo skeleton

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

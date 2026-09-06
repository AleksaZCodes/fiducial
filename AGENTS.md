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
├── crates/fiducial/      Rust crate (crates.io)
├── packages/fiducial/    JS package (@fiducial/fiducial on npm)
├── docs/specs/           Design decisions (append-only)
└── .github/workflows/    CI (ci.yml) + release (release.yml)
```

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

---
description: Load Fiducial platform context — run at session start in any Fiducial product
---

You are working in a product built on the **Fiducial platform** (https://github.com/AleksaZCodes/fiducial).

Before doing anything else:
1. Run `fid doctor` — confirms the product is in sync. Fix any issues it reports first.
2. Read `fiducial.toml` — lists installed capabilities and active guard rules.
3. Read `AGENTS.md` — product-specific context written at scaffold time.
4. Read `MISSION.md` — the tiebreaker for every ambiguous technical decision.

## Core commands

| Command | Purpose |
|---|---|
| `fid doctor` | Detect drift: missing files, stale templates, pending migrations |
| `fid upgrade` | Pull upstream template changes (3-way merge) and apply codemods |
| `fid derive` | Run pipelines; `--check` fails CI on stale artifacts |
| `fid add <capability>` | Install a built-in or third-party capability |
| `fid capability list` | Show installed capabilities and their skill status |
| `fid capability check` | Validate capability conformance |

## Guard

A `PreToolUse` hook (contributed by this plugin) runs `fid guard-check` before every Bash call.
The guard reads rules from `fiducial.toml [guard]` and fires only when a forbidden token is in
**command position** — it is shell-aware and never trips on quoted arguments.

Active rules are listed under `fiducial.toml [guard.rules]`. Common rules:

| Rule | What it prevents |
|---|---|
| `no-direct-main-push` | Committing directly to the default branch |
| `no-hand-edit-generated` | Editing files listed in `fiducial.lock` by hand |
| `no-unpinned-cli-fetch` | Fetching unversioned CLI tools at build time |
| `no-direct-schema-migration` | Writing raw DDL outside the declared migration system |
| `no-direct-flash-without-check` | Flashing firmware without running `cargo check` first |

## Propagation

Every template file in this product was recorded in `fiducial.lock` at the version it was installed from. When the platform updates:

- `fid doctor` reports "upstream template updated" for any tracked file that changed.
- `fid upgrade` 3-way-merges upstream changes against your local edits. Clean changes apply automatically; conflicts write standard conflict markers for you to resolve.
- Codemod migrations (API renames, reshape operations) apply once and are recorded in the lock.

## What not to do

- **Do not hand-edit files in `fiducial.lock`.** They are template-tracked; run `fid upgrade` instead.
- **Do not commit directly to the default branch** (`no-direct-main-push` rule).
- **Do not hand-write derived artifacts.** Change the upstream declaration and run `fid derive`.
- **Do not duplicate a value.** One declaration, many derivations — find or create the single source.

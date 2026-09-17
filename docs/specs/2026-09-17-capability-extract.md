# `fid capability extract` — spec

**Date:** 2026-09-17
**Status:** shipped

---

## Problem

A capability built inside one product cannot be reused elsewhere until it is
lifted out of the product and into a standalone directory. The mechanical work
— find the files, copy them, find what is hardcoded — is the same every time.
`fid capability extract` mechanises it.

The hard part is not the copying. It is naming what the extraction could not
generalise: product names embedded in config, absolute paths, `fiducial.toml`
keys with no matching declaration.

---

## Design constraints

**Stage, don't install.** Output lands under `capabilities/<id>/` as a staging
area. The command never installs the extracted capability into any product,
including the one it was extracted from. The staging area is for human review
and generalisation.

**Name what it could not generalise.** The command analyses each staged file
for:
- Occurrences of the product name (from `config.product.name`) — replace with
  `{{name}}` or a declaration
- Absolute paths (token starting with `/`) — make relative

**Warn on single consumer.** Every capability extracted from one product has
exactly one known consumer. The command always warns. `MISSION.md` anti-goal 2:
generalise when a second product needs it, not speculatively.

---

## Command

```
fid capability extract <id> [--from <path>]
```

`--from` is the source directory inside the product to stage. When omitted the
command falls back to files recorded in `fiducial.lock` for this capability
(pipelines only — template files are not tracked per-capability in the lock).

---

## Algorithm

1. Verify `fiducial.toml` exists (product root check).
2. Load `Config` (for `product.name`) and `Lock`.
3. Collect files:
   - If `--from` is given: walk the directory, read each file as UTF-8.
   - Otherwise: look up `lock.capabilities[id].pipelines`; each entry is a
     product-relative path. Warn that template files are not tracked.
4. For each file, write a copy to `capabilities/<id>/<rel-path>`.
5. Write `capabilities/<id>/SKILL.md` stub if none was staged.
6. Write `capabilities/<id>/capability.toml` stub if none was staged.
7. Analyse each staged file for generalisation issues (product name, absolute
   path). Print one line per issue with file, line number, and description.
8. Print single-consumer warning.
9. Print next-steps instructions.

---

## Output format

```
✦ fid capability extract realtime

  Staging to capabilities/realtime/

  staged   src/realtime/index.ts  →  capabilities/realtime/src/realtime/index.ts
  wrote    (stub)  →  capabilities/realtime/SKILL.md
  wrote    (stub)  →  capabilities/realtime/capability.toml

  ⚠ generalisation issues found (review before using as a capability):

    src/realtime/index.ts:3  hardcoded product name "my-app" — replace with {{name}} or a declaration

  ⚠ single consumer: this capability exists in one product only.
    Anti-goal 2 (MISSION.md): generalise when a second product needs it, not speculatively.

  Next steps:
    1. Review and fix the issues above
    2. Add capabilities/realtime/capability.toml with description and declarations
    3. Run: fid capability check --capability realtime
    4. Install in another product: fid add capability realtime --from ./capabilities/realtime
```

---

## Implementation

`crates/fiducial-cli/src/commands/capability.rs` — `Extract` variant added to
`CapabilityAction`, `cmd_extract` function, `collect_dir` / `collect_dir_inner`
helpers.

`Lock::TemplateRecord` does not record which capability installed each file, so
the `--from` flag is required when template files need to be included. Pipeline
files are recoverable from `lock.capabilities[id].pipelines`.

---

## Why fiducial.lock does not link templates to capabilities

`TemplateRecord` was designed for `fid upgrade`'s three-way merge: it tracks
`base_content` (the platform version) and `hash` (current on-disk), not
provenance. Adding a `capability_id` field would be the right long-term change;
`cmd_extract` works around its absence with `--from`.

---

## Tests

Four unit tests in `commands/capability.rs`:

| Test | What it asserts |
|---|---|
| `extract_stages_files_and_writes_stubs` | Files are copied; SKILL.md and capability.toml stubs are written |
| `extract_reports_hardcoded_product_name` | Product name detection logic is correct |
| `extract_fails_for_missing_source_dir` | Clear error when `--from` path does not exist |
| `extract_fails_without_from_and_no_lock_entry` | Clear error when capability is not in the lock and no `--from` given |

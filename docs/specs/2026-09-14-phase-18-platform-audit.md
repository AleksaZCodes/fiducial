# Phase 18 — platform audit: the drift inside the anti-drift platform

**Date:** 2026-09-14
**Status:** accepted
**Supersedes:** nothing. Corrects one claim in the Phase 17 record.

---

## Context

Phases 0–17 built the platform. Nothing was failing: every Rust test passed,
clippy was clean, `cargo fmt` was clean, all 30 JS tasks were green, and there
was not a single `TODO` or `FIXME` in the tree.

This audit therefore asked a different question than "what is broken?". It asked
**"where does this repository violate the rule it exists to enforce?"** — and
found that a green build had been hiding a substantial amount of exactly that.

## What was found

### 1 · The platform declared `serde`'s version five times

There was no `[workspace.dependencies]` table. `serde` was declared in five
manifests, `serde_json` in four, `sha2` in three, and `postcard`, `libm` and
`ed25519-dalek` in two each. Internal path dependencies (`{ path = "../…" }`)
were repeated per consumer.

This is principle 1 violated in the repository that states principle 1.

**Resolved:** every version now lives once in `[workspace.dependencies]` and is
inherited with `{ workspace = true }`. Feature *sets* are deliberately **not**
inherited — a version is one fact shared by every consumer, but a feature set is
a per-crate decision (`fiducial-ota` needs `postcard` without `std`; its
dev-dependencies need it with).

### 2 · The TypeScript version had already drifted three ways

`typescript` was declared in ten `package.json` files as **three different
answers** — `^7.0.2` (seven packages), `^7.0.0` (two) and `^5.0.0` (one) — while
exactly one version, 7.0.2, was installed.

Nothing failed. The declarations had simply stopped describing reality. This is
the precise failure mode the mission describes, sitting in the platform's own
repository, undetected.

**Resolved:** a pnpm `catalog:` block in `pnpm-workspace.yaml` is now the single
declaration; every package references `"catalog:"`.

### 3 · `fid new` generated products in the state its own dashboard flags

`fid new` scaffolded a Claude review workflow and no CI workflow. `fid dash`
reports, as a finding, "no workflow runs `fid derive --check`, so a stale
artifact would reach main unnoticed".

So every product the platform generated was born carrying the defect the platform
reports. Worse, a **test asserted this** — `ci_reports_declared_workflows_…`
required that a fresh scaffold have no freshness guard. The defect had been
written down as the specification.

**Resolved:** `fid new` scaffolds `.github/workflows/ci.yml`, which runs
`fid doctor` and `fid derive --check`. The test now asserts the fixed behaviour,
and keeps a negative case so per-workflow detection is still covered.

### 4 · Scaffolded products were born on a branch nothing referenced

`git init` ran without `--initial-branch`, so the branch came from the user's
`init.defaultBranch` — still `master` on a default install, as the `fon` product
demonstrates. Meanwhile the scaffold shipped:

- a guard rule named `no-direct-main-push`, guarding a branch that did not exist
- a review agent instructing `git diff main...HEAD`, which fails outright
- a CI workflow triggering on `main`, which would never fire

**Resolved:** `fid new` forces `main`, with a `symbolic-ref` fallback for git
older than 2.28.

### 5 · A typecheck that was an `echo`

`@fiducial/ui-svelte`'s `typecheck` script was
`echo 'checked by svelte-check at build time'` — and there was no build step, so
nothing ever checked it. Its `tsconfig.json` compounded this: it included
`src/**/*.ts` and then **excluded `src/index.ts`**, the only TypeScript file, so
even a real `tsc` run would have had nothing to do.

`pnpm typecheck` reported green across the workspace for two phases while one
package was entirely unchecked.

**Resolved:** real `svelte-check`. Its first run found a genuine accessibility
defect in `Dialog.svelte` — an interactive `<div>` with no ARIA role, whose only
purpose was `stopPropagation`. That handler was also **dead code**: the backdrop
handler hit-tests the dialog rect, so a click on the content can never close it.
Removed both.

Similar `echo` scripts that reported success for work that did not exist were
removed from `@fiducial/fiducial` and `@fiducial/ui-react`.

### 6 · The CI spine matrix was nine copies of one list

The `spine` job repeated a near-identical `cargo check` block per crate. The
drift was already present: `fiducial-sim` had joined the workspace without a
tenth block, and `PHASES.md` recorded it as covered.

**Resolved twice over.** The list is now *derived* — a crate is on the embedded
matrix exactly when it declares `#![no_std]`, so the attribute is the declaration
and a `grep` is the derivation. Adding a `no_std` crate enters the matrix
automatically.

And the PHASES.md claim was **corrected rather than quietly satisfied**:
`fiducial-sim` uses `Vec` and rayon, is not `no_std`, and genuinely cannot build
for `thumbv6m`. Its absence was right; the record was wrong.

### 7 · Thirteen crates, zero READMEs

Every crate publishes to crates.io with a bare description line and no front
page. Four packages had no README either.

**Resolved:** all seventeen written — and each crate README is now compiled and
run as a **doctest** via a `#[cfg(doctest)]` include, so its examples are
verified on every `cargo test` rather than merely plausible. This caught a wrong
method name (`derivative` vs `derivatives`) in the first README written.

### 8 · Hand-maintained trees that documented their own unreliability

`CLAUDE.md` and `AGENTS.md` each carried a repository tree, both stale —
missing `fiducial-ota`, `fiducial-sim` and `packages/realtime`, with AGENTS.md
also miscounting (11 crates and 10 packages; really 13 and 11).

Both files carried a disclaimer telling readers to run `ls` instead of trusting
the tree. **A disclaimer is not a fix.** Agents read that tree to decide what
exists, and a crate missing from it is a crate they will rebuild — which is the
one thing this platform exists to prevent.

**Resolved:** both trees corrected and expanded, and a test now fails the build
when a crate or package is missing from either.

### 9 · Stale pnpm workspace globs

`pnpm-workspace.yaml` declared `apps/*`, `workbench` and `cli`. None had ever
existed at the repository root. A stale glob is silent — pnpm matches nothing and
says nothing — so it survives until someone creates the directory and is
surprised by what it picks up.

**Resolved:** removed, and a test asserts every glob resolves.

## Decision

Findings of this class are not fixed by being fixed. They are fixed by being made
**unable to recur**, because the audit that found them was manual and will not be
run again on a schedule.

`crates/fiducial-cli/tests/workspace_hygiene.rs` therefore encodes seven
invariants as tests, each failing with the exact file and line to change:

| Invariant | Catches |
|---|---|
| No crate manifest pins a dependency version | finding 1 |
| No dependency is declared by two packages outside the catalog | finding 2 |
| Every crate inherits all `[workspace.package]` metadata | `fiducial-sim`'s missing MSRV |
| Every crate declares `keywords` and `categories` | crates.io discoverability |
| Every crate ships a README | finding 7 |
| Every pnpm workspace glob resolves | finding 9 |
| Every crate and package is named in the agent-context trees | finding 8 |

These run inside `cargo test --workspace`, which CI already executes, so they cost
no new job and no new minutes.

## Consequences

- Adding a crate that skips workspace inheritance fails the build.
- Adding a second package that pins a shared dependency fails the build.
- A new `no_std` crate joins the four-target matrix with no CI edit.
- A README example that stops compiling fails the build.
- `cargo test --workspace` in CI gained `--all-features`, without which the new
  README doctests requiring `std` would be silently skipped.

## What this phase deliberately did not do

No behaviour changed. No API changed. The enclosure geometry, the wire format,
the OTA state machine and the dashboard all produce byte-identical output — which
is what made it safe to change every manifest in the workspace at once.

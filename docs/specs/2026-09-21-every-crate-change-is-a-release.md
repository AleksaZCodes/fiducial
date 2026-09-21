# Every change to crate source is a release, and must carry a new version

**Date:** 2026-09-21
**Status:** accepted
**Supersedes:** nothing

---

## Context

`fiducial-cli 0.2.2` was published to crates.io on 2026-09-19. The `systemOne`
contract, the security-header module (`crates/fiducial-cli/src/security.rs`) and
the realtime work all merged to main after that, with no version bump. Main and
crates.io both said `0.2.2` and were different binaries.

That is not cosmetic. `PLATFORM_VERSION` (`capability.rs:30`) is
`CARGO_PKG_VERSION`, and it is written into every scaffolded product's
`fiducial.lock` as `source_version`. A product therefore recorded `0.2.2` for
artifacts that only a *newer* `0.2.2` could reproduce. Nothing could detect it,
because the version strings matched. It broke fon's CI twice in one session —
fon pins `0.2.1`, and the artifacts it was handed did not come from `0.2.1`.

A second, quieter drift was sitting underneath. The eleven internal
`fiducial-*` entries in `[workspace.dependencies]` all pinned `version =
"0.2.0"` while the crates themselves were at `0.2.2`, in direct contradiction of
`2026-09-16-the-rust-crates-release-in-lockstep.md`. Nothing failed, because
`^0.2.0` admits `0.2.2`. The drift was invisible until the first *minor* bump,
at which point cargo could not resolve the workspace at all.

`release.yml` publishes on every push to main, taking whatever version the
workspace declares.

## Decision

The workspace version is bumped on every pull request that changes anything
under `crates/`, and `fid release version-check` enforces it: it diffs against
the merge base with the base ref, and fails when published source moved and
`[workspace.package] version` did not. It runs in the `release` job in CI.

The same command enforces lockstep as an unconditional check — every internal
`fiducial-*` workspace dependency must pin exactly the version the workspace
declares, not a range that happens to contain it.

Both are exact checks over files, not judgments, so they are gates rather than
advice.

## Why not the alternatives

**Compare against crates.io at release time.** This is where the truth lives,
and `publish-crates.sh` already queries it. But it fails *after* merge, when the
only remedy is a follow-up commit, and it cannot run locally without network.
The gate has to be where the bump can still be added: the pull request.

**Require a changeset, as the TypeScript packages do.** Changesets do not model
the Rust workspace at all — `Cargo.toml` is hand-edited and no tool reads those
files for it. Teaching changesets about cargo is a larger mechanism than the
invariant deserves, and it would still be a second place where the version is
declared.

**Bump only when the change is "user-visible".** Every attempt to define that
predicate is a judgment call made by the person least able to be objective about
it, at the moment they want to merge. "Published source changed" is a diff.
Version inflation is the price and it is not a cost: if main publishes on every
push, then every change to crate source *is* a release, and numbering it
honestly is the whole point.

**Git tags as the baseline.** There are none for the crates — `git tag` lists
only `@fiducial/*` npm tags — so this would have meant backfilling history to
establish a baseline that the merge base already provides for free.

## Consequences

The workspace moves to `0.3.0` and the eleven internal pins move with it. fon's
pin must move to `0.3.0` once it is published; until then fon is pinned to a
version that does not correspond to main, which is the condition this decision
exists to end.

Versions will climb faster than they used to, and a patch bump becomes part of
the ordinary cost of touching a crate. A branch that changes only docs,
workflows or `packages/` is untouched by the gate.

## What this deliberately does not do

It does not choose *which* component to bump. Patch versus minor stays a human
judgment about what changed; the gate only insists the number is not the same
one that is already on crates.io.

It does not verify that the declared version is actually unpublished. That check
needs the network and belongs in `publish-crates.sh`, which already performs it
and treats an already-published version as an error.

It does not extend to the TypeScript packages. Those release through changesets
on their own cadence, and folding them into the crate version would couple two
release trains that have no reason to move together.

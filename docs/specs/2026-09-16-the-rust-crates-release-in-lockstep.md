# The fifteen Rust crates release in lockstep, and the registry is the gate

**Date:** 2026-09-16
**Status:** accepted

---

## Context

`ROADMAP.md` §*Rust release versioning* recorded that none of the fifteen
crates is on crates.io, that `release.yml`'s loop read `for crate in fiducial`
— one of fifteen — and that three further failures were observed on the real
release of 2026-09-16. It set one precondition before any of this was touched:

> **Do not assume; check the crates.io account before writing any publish
> logic**, because "this name is taken by someone else" and "this name is free"
> need different first steps.

**Checked, 2026-09-16.** All fifteen names return `crate … does not exist`
from `https://crates.io/api/v1/crates/<name>`. Nobody holds them. A 404 means
absent rather than yanked, so the open question — never published, or published
and later removed — resolves to **never published**. The `SHIPPED.md` Phase 0
row recorded an intention.

**Two further failures, found by running `cargo publish --dry-run` for the
first time.** Neither was in the roadmap's list, because the list was written
from workflow logs and these never got as far as a log:

1. The eleven internal entries in `[workspace.dependencies]` declared `path`
   and nothing else. `cargo publish` refuses — *"all dependencies must have a
   version requirement specified when publishing"* — because a published crate
   has no path to resolve. **Not one crate in this workspace could be
   published**, whatever the workflow said.
2. `fiducial-cli`'s `build.rs` reads `MISSION.md` two directories above its
   manifest. `cargo package` cannot reach outside a package directory, so the
   tarball contained no `MISSION.md` and verification panicked on it.

So the visible bugs — the one-crate loop, the dead `if:` gate — sat on top of
two that made the whole path impossible.

## Decision

**Lockstep.** All fifteen crates carry `version` from `[workspace.package]` and
move together. This answers the roadmap's *"lockstep or independent — decide
this explicitly and record it"*, and it is what the code already assumed.

They are one artifact with one story: `fiducial-core` is the `no_std` spine,
and `fiducial-protocol`, `fiducial-quantity`, `fiducial-eda` and the rest are
facets of it that are separate crates so a firmware target can link three of
them instead of fifteen. `WIRE_VERSION` already treats the protocol crates as
moving together. Fifteen independent changelogs would describe fifteen
histories that are, today, one history.

**The registry is the gate, not a workflow output.** `scripts/publish-crates.sh`
asks crates.io which versions exist and passes `--package` for exactly the ones
that do not, letting cargo order them by dependency. The step's `if:` condition
is deleted rather than repaired.

**Nothing is trusted to have worked.** `scripts/verify-published.sh` asks npm,
crates.io and `git ls-remote --tags origin` whether what the repository
declares is actually there, and the workflow runs it after every release with
`if: always()`.

**Tags are pushed.** `git push origin --tags` after a publish.

## Why not the alternatives

**Independent versions per crate.** More correct in the abstract, and the right
answer once these crates have separate consumers with separate upgrade
pressures. Today it would mean fifteen changelogs recording the same commits,
and a bump mechanism to decide fifteen numbers that would all be the same
number. Revisit when a crate first needs a version the others do not — that is
a visible event, not a judgment call.

**Repairing `steps.changesets.outputs.published` instead of deleting the gate.**
The output was wrong twice in a row on runs that did publish, so the repair
would be guesswork against an action's internals. More to the point, the step
does not need a gate: it already asks the registry, and the registry is the
authority on what the registry contains. Gating a registry check on a claim
about the registry is the second declaration this platform exists to delete.

**Keeping the `for crate in …` loop and adding fourteen names.** It would be
correct on the day it was written and wrong the first time a crate was added,
and it would have to encode dependency order by hand. `cargo metadata` holds
the graph and `cargo publish` has ordered multi-package publishing since Rust
1.90.

**Letting the internal `version = "0.1.0"` duplication stand ungated.** Cargo
cannot inherit `[workspace.package] version` into a dependency requirement, so
the copy is forced. Leaving it unchecked means the next bump publishes crates
depending on sibling versions that do not exist — discovered partway through a
fifteen-crate publish that cannot be undone. It is gated by
`internal_dependencies_pin_the_workspace_version` instead.

**Embedding a copy of `MISSION.md` in `crates/fiducial-cli/`.** The principles
would then exist twice, which is the thing `MISSION.md` is about. A symlink is
a pointer, not a copy, and `cargo package` follows it — verified by
`cargo package --list`, which shows `MISSION.md` in the packaged set.

## Consequences

The next merge to `main` publishes all fifteen crates at 0.1.0 and claims those
names. That is intended: they are free today and a name nobody has claimed is a
name somebody can claim.

`build.rs` now resolves `MISSION.md` from two candidates and keys the choice on
*finding a principles section* rather than on the file existing. A checkout
where git did not materialize the symlink (Windows without symlink support)
leaves a small text file holding the link target, which parses as Markdown with
no principles in it — so "the file is there" is not the question worth asking.

`RELEASE_PAT` is read with a fallback to `GITHUB_TOKEN`. Until the secret
exists nothing changes; once it does, Release PR CI stops needing manual
approval. The fallback exists so this file is correct in both states and there
is no second change to remember.

## What this deliberately does not do

- **No version-bump mechanism for Rust.** The roadmap's third question — a Rust
  equivalent of changesets, crate bumps inside `.changeset/*.md`, or a
  `fid release bump` subcommand — is untouched. This item was never about
  adding a decision mechanism to a working publish path; it was about there
  being no publish path. There is one now, and the bump question is answerable
  against something real for the first time.
- **No Zenodo DOI.** It needs a tag, then a GitHub release, then the webhook.
  This supplies the first link and verifies it against the remote.
- **No change to `WIRE_VERSION`.** Deliberately not SemVer, already stricter,
  and explicitly out of scope in the roadmap.

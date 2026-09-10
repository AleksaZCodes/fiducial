# Workbench v0 — `fid dash`, and one reader for pipelines

**Date:** 2026-09-11
**Status:** Accepted
**Implements:** §10 of `2026-09-06-fiducial-design.md` (Phase 16).

The design spec says the workbench is "a view, never a database". This records
what that constraint actually forced, and the duplication it exposed.

---

## 1. Dash owns no store, and that is load-bearing

Every number `fid dash` prints is recomputed on each run from files already in
the repository: `fiducial.toml`, `fiducial.lock`, `pipelines/*.toml`,
`.github/workflows/`, a roadmap file, `docs/specs/`, and git.

**Why this matters more than it sounds.** A dashboard that cached would need
invalidation, and invalidation is the work this system exists to delete. Because
dash holds nothing, there is no state to go stale and no sync step to get wrong.
A test asserts it writes nothing at all — the property is cheap to state and
cheap to check, so it is checked.

**The consequence.** Dash is slower than a cache would be, since it re-hashes
every tracked artifact on every run. On a real product that is milliseconds, and
the alternative buys speed with the exact failure mode the whole platform is
arranged to avoid.

## 2. No network, by default and by design

CI is reported from the workflow files the repository declares — their names,
triggers, and whether any of them runs `fid derive --check`. Not from a live API.

**Why not query GitHub.** A view that needs a token and a connection to render
is a view that stops working on a plane, in CI, or on a fork. And a cached
answer would be a private store by another name, which §10 rules out. There is a
test that renders dash with every proxy pointed at a closed port.

**What is lost, honestly.** Dash cannot tell you whether `main` is currently
green. It tells you something the live status cannot: whether the repository has
a workflow that would *catch* a stale artifact at all. On a freshly scaffolded
product the answer is no, and dash says so — which is a more useful finding than
a green tick.

**Reverse this if** a live status view is wanted, and put it behind an explicit
flag so the default stays hermetic.

## 3. Absence is a finding, not an error

A product with no roadmap, no decisions, or no pipelines gets a section saying
exactly that, and dash still exits 0.

**Why.** The moment a dashboard is most wanted is early, when little exists yet.
Refusing to render — or worse, erroring — would make it useless precisely then.
"No workflow checks freshness" and "nothing has been derived yet" are the two
most valuable things dash says about a young product.

## 4. Dash reports problems; it does not fail on them

Exit code 0 even when everything it shows is broken. `fid derive --check` and
`fid doctor` are the commands that fail.

**Why the split.** A command that exits non-zero cannot be run casually, and a
dashboard has to be runnable casually. Conflating "show me the state" with
"gate the build" would give one command two jobs and make the useful one
unusable in a shell prompt or a pre-commit glance.

## 5. Roadmap progress is counted from markers, not parsed as a schema

Dash counts `✅` / `🟡` / `⬜` and GitHub task-list syntax in `ROADMAP.md` or
`PHASES.md`. A line with no marker is not a roadmap item.

**Why not a structured roadmap file.** That would be a second declaration of
something products already write in prose, and it would mean a migration for
every existing roadmap. Counting markers works on the file people actually keep.

**The trade this accepts.** A roadmap with no markers reports zero tracked
items rather than guessing — asserted by a test, because silently inventing
progress from sentences would be worse than reporting none.

## 6. `--json` exists so workbench v1 needs no second implementation

The same facts, serialised. v1 is a Tauri app (§10); v2 authors. Both need
exactly these reads.

**Why now, before there is a consumer.** Because the alternative is that v1
reimplements freshness hashing, pipeline discovery, and decision listing against
the same files — and then the two disagree, which is the failure §7 below is
about. It is also what agents should use instead of re-deriving product state by
reading files themselves.

## 7. Pipeline discovery was already duplicated, and the copies disagreed

`fid derive` and `fid graph` each parsed `pipelines/*.toml` independently. Dash
would have been the third.

They did not behave the same. `derive` parsed strictly and failed with the
filename on a malformed pipeline; `graph` swallowed the error and labelled the
pipeline `"unknown"`. So the two commands gave different answers about the same
file — and the one that lied was the one whose entire job is explaining the
build.

Now there is one `pipeline::discover`, and all three read it. A malformed
pipeline fails all three, naming the file, which a test asserts across every
command.

**The general point.** This is the one-declaration rule applied to code rather
than to product data, and it had already been violated inside the tool that
enforces it. Adding a third reader would have been the cheapest thing to do and
the easiest to justify.

---

## Known gaps

- **Single repo.** §10 puts multi-repo in v1. `Config::find_root` walks up from
  the cwd, so dash describes one product.
- **No live CI status.** By choice, see §2. It would go behind a flag.
- **Roadmap markers are a convention, not a contract.** A product using different
  symbols gets zero counts and a source line, which is visibly wrong rather than
  quietly wrong — but it is still a convention this file is the only record of.
- **Decisions are listed, not read.** Dash reports dates and titles. Superseded
  decisions are not detected, because nothing in the format marks them.
- **`briefs`, mentioned in §10's v0 row, is not implemented.** No product has one
  yet, and the Rule of Two (§16) says not to build the reader before the thing
  it reads exists.

# Phase 19 — harvest: getting the good parts out of what you already built

**Date:** 2026-09-14
**Status:** accepted

---

## Context

`MISSION.md` names an anti-goal: **we do not rebuild what already works.** It was
written about CAD kernels and browser engines — things nobody should reimplement.
But the rule bites hardest one scale down, on your own prior work.

A person who has shipped before is sitting on business rules that took iterations
to get right, a theme that took a week to tune, a landing page that converted,
and conventions arrived at by getting it wrong twice. Starting the next product
means rebuilding that or copying it, and both are bad:

- **Rebuilding** discards the edge cases — which is the entire value, because the
  happy path was never the expensive part.
- **Copying** imports the assumptions. Two products now share code belonging to
  neither, and keeping them in sync is the work this platform exists to delete.

## Decision

Split the work along the line between *mechanical* and *judgment*, and give each
half to whichever of the two is actually good at it.

| Half | Owner | Why |
|---|---|---|
| Survey — walk, classify, measure, detect stack, stage | `fid harvest` | Deterministic and fast. An agent doing this by hand burns its context window on `ls` output. |
| Extraction — decide what is worth keeping, generalize it | `/fiducial:harvest` skill | Whether a function is a business rule or incidental framing is not a heuristic. |

### The staging rule

> `harvest/` is a staging area. It is never the product.

`fid harvest` writes only into `harvest/<slug>/`. It copies nothing into the
product's source tree, wires nothing in, and overwrites nothing. An end-to-end
test fingerprints every file outside `harvest/` before and after a run and
asserts they are byte-identical.

This is the load-bearing decision. A tool that imported directly would produce
precisely the failure the platform exists to prevent: a second copy of somebody
else's assumptions, pasted into a new product, now needing to be kept in sync
with a repository nobody will open again.

### Eight kinds

`logic`, `ui`, `theme`, `art`, `principle`, `ops`, `contract`, `test` — the four
the request named, plus the three that always travel with them and the one
(`test`) that determines whether porting logic is safe at all.

Every classification records the **evidence** for itself in a `reason` field, and
the survey prints it. A heuristic that states its reasoning is correctable; one
that does not is silently authoritative, which is worse than being absent.

### Ordering by value per unit of risk

The survey does not list findings by size or by how interesting the code looks.
It orders the work `theme → principle → contract → logic → ui → ops → art`:

- **theme first** because tokens are declarative, dependency-free, and carry the
  thing that took longest to get right. Highest value, lowest risk, every time.
- **contract before logic** because a donor almost always has two hand-written
  copies of one shape. Porting it once and deriving the other is an *improvement*
  on what the donor had — the rare case where harvesting makes something better
  rather than merely cheaper.
- **ops late and loudly** because those files are reusable nearly verbatim, which
  is exactly what makes them dangerous: they are full of account IDs and secrets.
- **art last** because the first question is licensing, not taste.

## Safety properties, each asserted

| Property | Test |
|---|---|
| Nothing outside `harvest/` is modified | `harvest_writes_only_inside_the_staging_directory` |
| `.env`, lockfiles and logs are never inventoried **or** staged | `secrets_and_lockfiles_are_never_inventoried_or_staged` |
| `node_modules`, `dist`, `target`, `.git` are never walked | `build_output_and_dependencies_are_skipped` |
| Harvesting a tree into itself is refused | `harvesting_into_the_source_is_refused` |
| Every classification states its evidence | `every_asset_records_why_it_was_classified` |
| `fid doctor` stays clean afterwards | `doctor_stays_clean_after_a_harvest` |

The overlap test earned its place immediately: the first implementation called
`canonicalize()` on the destination, which fails on a path that does not exist —
so the guard was skipped on exactly the first run where it was needed. Replaced
with a resolver that normalizes against the nearest existing ancestor.

## Notes on the classifier

**Markup is UI.** The first implementation had no rule for HTML, which made the
command useless for the commonest donor there is: a static site. A landing page
*is* its markup — the structure, the copy and the layout all live there.

**Test paths beat file extensions.** `Button.test.tsx` is a test, not a
component, or every component test lands in the UI bucket.

**A stylesheet declaring custom properties is theme; one that does not is UI.**
The distinction is read from the file, not guessed from the path, because donors
put tokens in inconsistent places.

**Logic is split by whether it performs I/O**, and both halves are reported as
logic with different reasons. "Logic mixed with effects" is the most common and
most valuable finding in a real donor: a pure rule wrapped in `fetch` and
`process.env`. The rule is the part worth having.

## Consequences

- `fid harvest <path> [--name] [--into] [--json]`
- `/fiducial:harvest` in the Claude Code plugin
- `docs/guides/harvesting.md` — the guide, with a worked landing-page example
- Harvesting is now a repeatable process rather than an afternoon of copy-paste

## What this deliberately does not do

**It does not decide.** The classifier reports evidence; the skill presents
options and waits. A harvest whose outcome is "four of these thirteen are worth
taking" is a better result than one that imports everything, and the skill is
written to make the second column — *what is not worth lifting, and why* — as
prominent as the first.

**It does not stage large binaries.** Over 512 KB is inventoried but left in
place. The point of staging is to give an agent something it can read.

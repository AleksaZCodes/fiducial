# `SHIPPED.md` holds only the record

**Date:** 2026-09-15
**Status:** implemented
**Extends:** [`2026-09-14-phases-renamed-to-shipped.md`](2026-09-14-phases-renamed-to-shipped.md),
which decided the split and asserted this property without anything checking it.

---

## The question

> *Why the heck does `SHIPPED.md` say what's next, and not `ROADMAP.md`? And why
> is it "between phases"?*

Both had the same answer, and it was not a good one.

## What the earlier spec promised

The rename spec says, of the file it renamed:

> The roadmap took over ordering, and **the next-items prose was removed from
> `PHASES.md` precisely so it would hold only the record.**

The commit that wrote that sentence also added, to the renamed file:

```markdown
## Current phase: between phases — next is the capability taxonomy
```

Which is next-items prose, in the file that was supposed to have stopped holding
it. It was then hand-edited on every merge for three phases — by an agent that
maintained it three times without noticing it should not exist.

## Why "between phases" was never a real state

The record is **phase-numbered**; the plan is **item-named**. That split is
correct and deliberate, and the roadmap says why: *"roadmap items are now named,
and a phase entry in `SHIPPED.md` names the item it implements. Neither file
renumbers because of the other."*

A phase, though, is a unit of *shipped* work. It comes into existence when the
work lands. So there is no honest value for "current phase" while work is in
flight — which is exactly when a status line would be useful. "Between phases"
was the line admitting it had nothing to say.

The state it was reaching for — *which item is being worked on* — is a roadmap
fact, and the roadmap already has a marker for it: 🟡.

## The decision

**`SHIPPED.md` looks backwards only.** No "current phase", no "next is".

**Nothing states what is next in prose, anywhere.** The roadmap's order of work
plus its ⬜ 🟡 ✅ markers already say it. `fid dash --section roadmap` derives
it: what is in progress if anything is, otherwise the first to-do item in file
order. A sentence naming the next item is a second declaration of those markers,
and it is the copy that goes stale — demonstrably, three times.

## The bug underneath

`fid dash` reported the platform's own roadmap as **"5 done, 0 in progress,
0 to do"**. Six unstarted items, and the dashboard said the roadmap was
finished.

`roadmap_status` counts ⬜ 🟡 ✅ and ignores unmarked lines — deliberately, so
prose is not counted as work. The cost, unnoticed until now, is that an *item*
without a marker is invisible rather than pending. Items 4–9 carried none.

That is the same failure the prose line caused, one layer down: a fact recorded
only in a form a machine cannot read, and the machine reporting its absence as
zero. Items 4–9 now carry ⬜, and the roadmap reads 5 done, 6 to do.

## Enforced, not asserted

The previous spec stated this property and nothing checked it, which is how it
was violated by its own commit. Two tests in `workspace_hygiene.rs`:

- `shipped_does_not_state_what_is_next` — no heading in `SHIPPED.md` mentions a
  current phase or what is next.
- `every_roadmap_item_carries_a_marker` — every ordered roadmap item carries
  one, so none is invisible to the dashboard.

Both were verified by reintroducing the exact line and the exact missing marker.

## Also corrected

`AGENTS.md` said *"Read `SHIPPED.md` before every session. It states which phase
is active."* It does not, and should not. It now names both files, says which
question each answers, and points at `fid dash` for what is next.

`ARCHITECTURE.md` called `SHIPPED.md` the build order. That is `ROADMAP.md`.

## Addendum, same day: the first sweep was too narrow

`shipped_does_not_state_what_is_next` checks headings **inside** that file, and
passed while three other documents still sent readers to it for the plan:

| File | Said |
|---|---|
| `README.md` | "`SHIPPED.md` — Build order and current state" |
| `docs/guides/README.md` | the same line |
| `.claude/agents/fiducial-design.md` | "Read `SHIPPED.md` — current phase and what 'done' means" |

A file that holds only the record is not much use if three other files say it
holds the plan. The property is not "this file contains no forward-looking
heading"; it is **"nobody is sent here for what is next."**

`nothing_points_at_shipped_for_what_is_next` walks every tracked `.md` and
fails on a line naming `SHIPPED.md` alongside "build order", "current state",
"current phase" or "what is next" — unless the line also names `ROADMAP.md`,
which is what the correction itself reads like. `docs/specs/` is skipped: those
record what was true on the day they were written, and editing them is the edit
rule 4 forbids.

**Scaffolded products were never affected.** The product `ROADMAP.md` template
already pointed the right way, so this was the platform's own drift only.

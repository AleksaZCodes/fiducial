# `PHASES.md` is renamed `SHIPPED.md`

**Date:** 2026-09-14
**Status:** accepted
**Supersedes:** the file naming in
`docs/specs/2026-09-14-documentation-model.md` — that decision's model stands
unchanged; only the name of the record file is superseded.

---

## Context

The documentation model gave each file one job:

| File | Job |
|---|---|
| `ROADMAP.md` | what is **intended**, ordered |
| `PHASES.md` | what was **built** |

The split is right. The **name** was not: *phases* reads as a plan. A reader
landing on a repository with `ROADMAP.md` and `PHASES.md` side by side cannot
tell from the names which one looks forward, and "phases" arguably sounds more
like a schedule than "roadmap" does.

That was tolerable while `PHASES.md` genuinely held the schedule. It stopped
being true the same day: the roadmap took over ordering, and the next-items prose
was removed from `PHASES.md` precisely so it would hold only the record.

So the file's name described a job it no longer had.

## Decision

`PHASES.md` → **`SHIPPED.md`**.

Unambiguously past tense, and it pairs with `ROADMAP.md` so the pair reads
correctly from the names alone: **intended / shipped.**

Phase *numbering* stays. "Phase 18 found nine violations" is a useful handle for
a chunk of work, and the entries are organised that way. What changed is the name
of the file, not how the record is structured.

## Why not the alternatives

| Candidate | Rejected because |
|---|---|
| `CHANGELOG.md` | Changesets already generates `packages/*/CHANGELOG.md`. A root file with the same name holding something different — phase records rather than release notes — is a worse collision than the problem being fixed |
| `HISTORY.md` | Vague. History of what? |
| `BUILD-LOG.md` | Accurate and clunky, and invites being confused with CI logs |
| Keep `PHASES.md` | The name describes a job the file no longer has |

## Consequences

- `fid dash` reads `ROADMAP.md`, then `SHIPPED.md`, then `PHASES.md`, then
  `docs/ROADMAP.md`. **`PHASES.md` is retained as a fallback** so a repository
  that already has one keeps working — a rename in the platform should not
  silently stop reading a product's file.
- `SHIPPED.md` opens by stating its former name, so a reader following an old
  reference lands somewhere that explains itself.

## The append-only rule, applied to itself

Three older specs reference `PHASES.md`. **They were not edited.** They named the
file correctly at the time they were written, and rewriting them would make the
record claim a name that did not exist on those dates.

This document exists because the same applies to
`2026-09-14-documentation-model.md`, written hours earlier. Editing its table
would have been easier and would have quietly falsified what was decided this
morning. Appending is the rule (principle 1b), including when the old decision is
only hours old and the author is the one who made it.

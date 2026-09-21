# Foundations are additive: `fid new` scaffolds the minimum

**Date:** 2026-09-21
**Status:** accepted
**Supersedes:** nothing formally — the reasoning it revises lived in code
comments in `commands/new.rs`, not in a decision record. It is restated and
answered below rather than deleted.

---

## Context

`fid new` took 110ms and produced 34 files and 416KB before anyone had had a
single product thought. Measured on the scaffold as it stood:

| | |
|---|---|
| `fiducial.lock` | 151KB |
| `.fiducial/skills/design.md` | 23KB |
| `design-system.md` | 22KB |
| `apps/web/src/components/ui/doodle-arrows.tsx` | 21KB |
| `apps/web/src/app/marks.css` | 18KB |
| `scripts/build-design-system.mjs` | 18KB |

Almost all of it came from two capabilities installed unconditionally: `design`
always, and `i18n` via `--locales` defaulting to `sr,en` — the platform author's
own locales, applied to every product anyone creates.

The argument for installing them was written down in `commands/new.rs` and it is
a good one:

> the alternative to having one is not "no design", it is the default one —
> Inter, an indigo primary, `rounded-xl`, a gradient hero. Those are absent
> decisions, not neutral ones, and they are absent in the same direction in
> every product that never wrote anything down.

And for i18n: added later, localization is a refactoring pass over strings that
have already been missed.

Both are true. Neither is a reason to install at minute zero, and the cost was
being paid by the case the platform exists to serve: *"when I have a new idea,
build real small tools in as little tokens and as quickly as possible."*

## Decision

`fid new` scaffolds the minimum: 12 files, under 50KB. No design capability, no
locales, no roadmap.

`--locales <list>` installs i18n and names the locales explicitly; there is no
default locale set. `--full` creates the tree a product used to be born with —
the design capability plus the platform's seeded locales — in one step.

`ROADMAP.md` moves out of `SCAFFOLD_FILES` into a `FULL_ONLY_FILES` list. The
distinction is load-bearing: `fid upgrade` updates whatever a product's lock
already tracks and consults `SCAFFOLD_FILES` only to find templates the platform
has *added* since. So a `--full` product keeps receiving roadmap updates, and a
minimal product is never later handed a roadmap it declined.

`.github/workflows/ci.yml` stays in the core set even though it is on the same
list of things a small tool should not need. CI is what runs
`fid derive --check`, so it is the thing that keeps a product from drifting from
its own declarations — and there is no `fid add ci` to recover it with. A
default you cannot undo is not a default, so this one stays until that command
exists.

## Why not the alternatives

**`fid new --minimal`, keeping the heavy default.** The path everyone takes is
the default one, and a flag nobody types changes nothing. The measurement above
is what the default produces, so the default is what had to move.

**Keep `design`, drop only `i18n`.** Design was the larger half — four of the
five biggest files. Dropping the smaller half would have left the complaint
intact and made the rule harder to state.

**Make the foundations lazy rather than absent** — install them but leave their
gates advisory until the product declares itself ready. This is the more
sophisticated answer and it is a new concept (a per-capability "not yet"
lifecycle) to solve a problem that deleting two lines also solves. If deferred
capabilities earn their place later, they can be built on top of this; the
reverse is harder.

**Shrink `fiducial.lock`.** It is 151KB because it stores each template's full
`base_content` — a verbatim second copy of every scaffolded file. That looks
like a declare-once violation and is not: `fid upgrade`'s 3-way merge needs the
base version of a file the product has since modified, and a newer `fid` binary
carries only its own templates, never the old ones. So the base cannot be
recovered and must be stored eagerly. Dropping the two capabilities took the
lock from 151KB to 25KB on its own, which is most of what shrinking it would
have achieved, without giving up the merge.

## Consequences

`fid new` produces 12 files and 46KB in about 30ms, down from 34 files and
416KB. A product with no locales, no design system and no roadmap derives
cleanly and passes `fid derive --check` on the first commit, which was already
true and is now the path people actually take.

The tests that exercise capabilities now scaffold with `--full`, and say why.
That is the honest split: `dash.rs` is about rendering a product that *has*
roadmap, decisions and pipelines, and it should ask for one.

An agent or a person who wants the old behaviour has to know `--full` exists.
The closing checklist names each foundation and the command that installs it,
so the decision is presented at the moment it becomes real rather than made
silently beforehand.

## What this deliberately does not do

It does not make the foundations worse or lower-priority. `fid add design`
produces the identical tree it always did, and the absent-decision argument
above is still the reason to run it — just not the reason to run it before
there is anything to look at.

It does not touch existing products. Nothing is removed from a scaffolded
repository, and `fid upgrade` keeps maintaining every template a product's lock
already tracks, including `ROADMAP.md`.

It does not add `fid add ci` or `fid add roadmap`. Those are the commands that
would let CI leave the core set too, and until someone wants that, adding them
would be building for a need nobody has expressed.

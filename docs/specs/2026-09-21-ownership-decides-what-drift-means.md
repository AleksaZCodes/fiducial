# A scaffolded file is owned by the product or by the platform, and that decides what drift means

**Date:** 2026-09-21
**Status:** accepted
**Supersedes:** nothing

---

## Context

`fid doctor` reported 32 issues against fon. Twenty-six of them were files fon
had edited on purpose:

```
✗ MISSION.md: modified since scaffold … run `fid upgrade` to re-baseline.
✗ README.md: modified since scaffold …
✗ messages/en.json: modified since scaffold …
✗ content.toml: modified since scaffold …
✗ apps/web/src/app/page.tsx: modified since scaffold …
```

`MISSION.md`'s own scaffolded footer says:

> It is **product-owned** — edit it freely.

So the platform wrote a file, told the product to edit it, and then reported the
edit as drift. fon's CI runs `fid doctor` with `continue-on-error` as a result,
which does not cost only those 26 lines — it costs every real finding too.
A check that fires on the normal state of a real product is a check people
switch off.

The remaining question, recorded as open: *what re-baselining should mean is an
unmade decision.* It could not be answered while "drift" meant two different
things at once.

## Decision

Every scaffolded path is classified as **product-owned** or **platform-owned**,
once, in `crate::ownership`.

**Product-owned** — `MISSION.md`, `README.md`, `thesis.toml`, `fiducial.toml`,
`AGENTS.md`, `.gitignore`, `design-system.md`, `content.toml`, and everything
under `messages/`, `content/`, `press/`, `brand/`, `migrations/`, `pipelines/`
and `apps/`. Editing these is the intended use. `fid doctor` counts them and
prints one ✓ line; it reports none of them.

**Platform-owned** — everything else, including two exceptions carved out of
`apps/`: `apps/web/src/generated/` (derived, and guarded better by
`fid derive --check`, which knows which pipeline made the file) and
`apps/web/src/components/ui/` (shadcn copies that `fid upgrade` replaces). An
edit here is reported, and the message says what is at stake: `fid upgrade` will
merge over it.

**An unclassified path defaults to platform-owned.** The other default fails
silently — a platform file misfiled as the product's stops warning about an edit
that is about to be overwritten, and the first anyone hears of it is their work
disappearing. A test fails when a template is added to `SCAFFOLD_FILES` without
being classified.

That settles re-baselining, which splits cleanly along the same line:

- **Product-owned: never.** The lock's `base_content` must stay at the original
  scaffold, because it is the merge base — the only thing that lets
  `fid upgrade` tell your paragraph from the platform's. Re-baselining one would
  destroy the ability to merge. `fid rebaseline` refuses, and explains.

- **Platform-owned: the whole point.** `fid rebaseline <path>` records your
  version as the new base, declaring the fork deliberate. Named paths only;
  there is no `--all`, because silencing a list of real divergences in one
  keystroke is the same failure as `continue-on-error`, reached faster.

The old message had it exactly backwards: it asked every product to re-baseline
the one category that must never be re-baselined, and said nothing about the
category where it is the right answer.

## Why not the alternatives

**A per-file marker in the template.** Ownership would live next to the content
it describes. But the templates already state it in prose — `MISSION.md` says
"product-owned" in its footer — and reading that back means parsing English to
decide program behaviour, which is the failure this platform exists to avoid.
A marker comment would work, but it would have to be added to forty files and
stripped before writing, for a fact that fits in one reviewable list.

**Default to product-owned, enumerate the platform's files.** Shorter list, and
it fails in the wrong direction: anything forgotten goes quiet instead of loud.

**A per-product override in `fiducial.toml`.** Lets a product disagree with the
classification. Nobody has needed it yet, and adding it now would mean shipping
a knob before anyone has been bothered by its absence — and a knob that silences
warnings is one people reach for instead of reading them.

**Leave `fid doctor` alone and fix fon's CI.** This was the status quo, and it
is why `continue-on-error` is there. The tool was wrong; making the product work
around it would have kept every future product working around it too.

## Consequences

fon goes from **32 issues to 6**, and all six are real: a `ci.yml` carrying its
own pin, a deleted `claude-review.yml`, a customised `doodle.tsx`, and three
forked derive scripts. Each is a file `fid upgrade` will merge over, which is
worth knowing. `continue-on-error` can come off once those six are resolved —
each with `fid rebaseline` or `fid upgrade`, per file, deliberately.

`fid doctor` gains a ✓ line reading `26 product-owned file(s) edited — expected,
not drift`. Counted rather than silent: it is true, occasionally useful, and not
a problem.

The classification is now a thing to maintain. A new scaffolded template fails
the test until someone decides which side it belongs on — which is the intended
cost, since that decision is exactly what was being skipped.

## What this deliberately does not do

It does not change `fid upgrade`'s merge behaviour. A product-owned file is
still 3-way-merged against its original base, which is what should happen and
what re-baselining would have broken.

It does not model partial ownership. `AGENTS.md` is a real hybrid — a product
writes most of it, and parts are regenerated — and it is classified wholly
product-owned rather than growing a region mechanism for one file. If a second
file needs it, that is the time to build it.

It does not offer a way to un-accept a re-baselined file. `fid upgrade` already
takes the platform's version, which is the same outcome by a better-tested path.

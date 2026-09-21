# A product's thesis is a Fact with a Decision's lifecycle

**Date:** 2026-09-21
**Status:** accepted
**Supersedes:** nothing

---

## Context

fon's `MISSION.md` is thirty-three lines of prose. The sentence that actually
decides things is in the middle of its second paragraph:

> The design commitment that shapes everything else: **no unverified alert ever
> reaches a responder.**

That is a good thesis. It is falsifiable, it states a tension between speed and
trust, and it is the sentence you would sell with. The problem is not the
writing — it is that nothing downstream can address it.

Count the restatements. `messages/en.json` carries `meta.description`: *"A solar
LoRa sensor network for early wildfire detection in Serbia. A person confirms
every event. In development."* `messages/sr.json` carries a Serbian
restatement. `README.md` states it a third way. Four wordings of one claim, none
of them marked canonical, and no mechanism that could notice a fifth appearing
or any of them drifting. An editor judging a draft has nothing to judge against.

So the failure is structural. The claim was already sharp; it was unaddressable.
Everything the platform does about hand-copied facts — declare once, derive the
rest, gate the derivation — applies here and was not being applied.

The scaffold made it worse by asking for exactly the wrong thing.
`MISSION.md.tmpl` said: *"Replace this paragraph with one sentence: what is this
product for, and who does it serve?"* A paragraph, in prose, with no structure —
which is how fon's mission came to be an AI summary that was pasted in, that its
author says "kinda sucked", and that then propagated into content and editorial
judgment because it was the only thing there.

## Decision

A product declares its thesis in `thesis.toml`: one claim, with its parts
separable so each can be interrogated on its own.

`claim` is the only required field. The rest is optional and fillable months
later, in three groups — the **arc** (`what`, `for_whom`, `problem`, `why_now`,
`why_us`, `how`, `where`), the **test** (`falsified_by`, `disagrees`) and the
**evidence** (`proven`, `not_yet`).

It is a **Fact**: hand-authored, never generated, and things derive from it. It
carries a **Decision's lifecycle**: the file is append-only, a sharper claim
supersedes an older one and must say `because`, and no entry is ever edited.
This is the only place in the platform where the two lifecycles meet, and it is
deliberate — a thesis is not corrected like `[brand] primary_color`, it is
sharpened, and how it sharpened is most of what a reader wants.

`fid thesis` shows the current claim and the questions its empty fields answer.
`fid thesis log` shows the evolution. `fid thesis set` appends. The `fid-thesis`
pipeline derives `PITCH.md` — the arc assembled in the order you would say it —
and `thesis.ts`, so an app imports the sentence instead of retyping it.

Nothing about this is a gate. A product with no thesis derives nothing and
passes `fid derive --check`; a scaffold declares no thesis at all.

## Why not the alternatives

**Keep it in `MISSION.md` and parse the prose.** This is what exists, and it is
the thing that failed. Extracting a claim from a paragraph means a parser
guessing which sentence is load-bearing, and it would have guessed wrong about
fon — the first paragraph is longer and reads more like a mission.

**One `[thesis]` section in `fiducial.toml`.** Fewer moving parts, and every
tool already reads that file. But it is the wrong lifecycle: facts there are
edited in place, so the sharpening would exist only in `git log`, nothing could
reference "the thesis before this one", and `fid add` rewrites that file, which
appending history into it would fight.

**Markdown decision records in `docs/theses/`.** Matches a convention already in
the repo and leaves the most room to argue with yourself on the page. But it is
prose again, so deriving a pitch or a meta description means parsing paragraphs
— precisely how the claim got buried the first time.

**Require `falsified_by`.** Falsifiability is the test, so making it mandatory
is tempting. It is also a blocker by construction: on day one, with the idea
half-formed, you either stall or write a throwaway falsifier — and a fake one is
worse than an empty field, because it reads as finished and never gets revisited.
`fid thesis` asks about it first among the gaps instead.

**Store only the claim and its test.** Tightest declaration, sharpest sentence.
Rejected because the thesis is used for pitching and communication, and a pitch
needs the narrative: what, for whom, why now, why us, how, where. Those are not
decoration around the claim, they are the slides.

**Derive `README.md`, the hero and every locale's meta description.** Full
declare-once enforcement, and the reason not to is `messages/sr.json`: deriving
a Serbian description from an English claim needs a translator, and the pipeline
has none. Generating the source locale and leaving Serbian hand-written invents
a staleness concept; generating both produces machine-translated copy in the
place you sell with. So those stay hand-written and `fid advise` checks them
against the claim — advice, not a gate.

## Consequences

`fid new` ships `thesis.toml` and `pipelines/thesis.toml`, and its closing
checklist now leads with `fid thesis set` instead of "edit MISSION.md".
`MISSION.md`'s template stops asking for the paragraph and says the claim lives
in `thesis.toml`, leaving the file for context a structured claim cannot hold.

Two bugs were found wiring this up and are fixed here. The `fid-thesis` executor
wrote every declared output regardless of applicability, conjuring
`apps/web/src/generated/` in a product with no app; and `fid derive`'s note for
a skipped output stated one hardcoded cause — `[identity] storage = "none"` —
for every executor, which was already wrong for `fid-design`.

A product now has two files that describe why it exists, and the boundary
between them has to be kept: the claim in `thesis.toml`, everything else in
`MISSION.md`. That is a real cost and the templates state it explicitly.

## What this deliberately does not do

It does not judge the claim. Whether a thesis is falsifiable, whether a
reasonable person could disagree, whether it states a tension or merely
describes — those vary by reader and belong to `fid advise`, never to
`fid derive --check` (`2026-09-20-two-kinds-of-model.md`). A gate whose verdict
varies is not a gate.

It does not rewrite existing prose. fon's `MISSION.md` keeps its wording; it
gains a thesis beside it, and the drift between them becomes visible rather than
being resolved by a codemod.

It does not translate. The advisory-only boundary above rests on the pipeline
having no translator, and that is a gap worth closing rather than a permanent
property — a locale-to-locale translation contract, with a key-distribution
story that works for cloud agents, would let the source locale's meta
description derive honestly. Until that exists, this is where the line sits.

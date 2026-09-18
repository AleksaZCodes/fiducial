# fiducial:design — Design Skill

This product has the `design` capability. **One declaration, many derivations**
— `MISSION.md` principle 1, applied to taste.

The declaration is `design-system.md` at the product root. Everything visual in
this product is either written there or is a bug.

## Why this capability exists

An interface generated without a design system is not neutral. It has a look,
and the look is always the same one:

| Tell | What it actually is |
|---|---|
| Inter or Geist | the typeface nobody chose |
| A blue or indigo primary with a subtle hover | the accent nobody chose |
| `rounded-xl` on every card and button | the silhouette nobody chose |
| Gradient hero → headline → subheadline → CTA | the layout nobody chose |
| Icon + label sidebar | the navigation nobody chose |
| 64px of padding everywhere | the rhythm nobody chose |

None of these are bad decisions. They are *absent* decisions. Which gives the
one rule this whole capability exists to enforce:

> **Silence in the design system is a decision to use the default.**
> Every constraint you do not write, something else will write for you, and it
> will write the list above.

So `design-system.md` is not documentation of what was built. It is the input.
Write it first, build second.

## The loop

```
design-system.md              ← you edit this: the product's taste, in prose and hex
  ↓  fid derive                (pipeline: design — executor fid-design)
apps/web/src/app/tokens.css   ← generated: never hand-edit
  ↓  imported by globals.css, which is yours
components                    ← read tokens and named steps; never literals
  ↓  node scripts/build-design-system.mjs
design-system/*.html          ← generated gallery: what the system actually looks like
  ↓
fid derive --check            ← fails if tokens.css is missing or stale
```

Prose in `design-system.md` is for people. The fenced ` ```toml fid:<section> `
blocks are what the pipeline parses — `fid:type`, `fid:scale`, `fid:color`,
`fid:contrast`, `fid:shape`. Untagged fences are ignored, so an example snippet
in the prose is never mistaken for a declaration.

Both halves are in one file on purpose. Put the values in `fiducial.toml` and
the reasons in a document, and you have two declarations of the same taste that
disagree the first time either moves. Here the reason sits directly above the
value it explains, and the pipeline reads the same file the person does.

**`tokens.css` is derived; `globals.css` is yours.** The generator owns colours,
the four type roles, the shape scale, the `@theme inline` mapping and one class
per named type step. It does not own your base layer, your imports or your
commentary — a generator that owned the whole stylesheet would delete them on
every run.

The gallery is generated from the app's own stylesheet rather than hand-built,
so it cannot drift into showing a system the product does not have. If a swatch
in the gallery looks wrong, the token is wrong.

## Rule 1 · Name the font, never the category

`"a clean sans-serif"` resolves to Inter. `"a serif"` resolves to Georgia. A
category is not a decision.

This capability ships **four type roles**, not two:

| Role | Var | For |
|---|---|---|
| display | `--type-display` | headlines, wordmarks, numerals meant to be looked at |
| body | `--type-body` | running text — everything a person actually reads |
| script | `--type-script` | annotation, margin notes. **Never body copy, never a heading** |
| mono | `--type-mono` | code, identifiers, tabular figures |

Four rather than the usual sans + mono, because the two-role default cannot
express the thing that most separates a designed page from a generated one:
**the display face is not the body face at a larger size.** If your headline
and your paragraph are the same family, you have a document, not a design.

The script role is the second half of that. It gives the page a voice beside
itself — see Rule 5.

Declare the families in `design-system.md` with exact names and the weights to
load. Bind them once in the app shell; every consumer reads the role.

## Rule 2 · The scale is named, and there is nothing outside it

Ten steps ship: `display h1 h2 h3 subhead body small eyebrow note mono`, each a
class in the marks layer and an entry in `typeScale` from `@fiducial/tokens`.

Use the class. An inline `text-[2.375rem] leading-[1.06]` is a scale step that
exists on one page and nowhere else, and the next page will invent a different
one. If a design genuinely needs an eleventh step, **add it to the scale** —
both places — and then use it.

## Rule 3 · Hex with a role, not a description

`--color-primary: #B85207` — *ember orange, primary actions and links*.
Not "a warm rust". Not "brand orange". The value and what it is for.

Then declare the pairs, in `fid:contrast`. **`fid derive` computes every one of
them and fails if any falls under its minimum.** This is the rule that stopped
being advice: a palette that has never been measured is a palette that fails on
one of its pairs, and it is almost always `muted-foreground`. If a pair fails,
change the colour — not the minimum. The minimum is the requirement.

State the prohibitions too. `"No blues or purples anywhere"` is a real
constraint that does real work; "warm palette" is not.

## Rule 4 · Corners are a strategy, not a number

The shadcn slot set has exactly one shape decision in it — `--radius` — which
is why every product built on it has the same silhouette. One variable can say
*how much*; it can never say *what kind*.

So shape is declared as a strategy plus a three-step scale:

```
strategy   round | chamfer | square
panel      cards, dialogs, sections      (taller than ~5rem)
control    buttons, inputs, nav items    (~2.25rem and up)
chip       badges, pills, tags           (~1.75rem and up)
```

Three steps and not one, because under `chamfer` there is a hard geometric
constraint a radius does not have: **a chamfer must stay under half the
element's height**, or the two cuts on one edge meet and the box degenerates
into a lozenge. A chip given the panel corner is not slightly wrong, it is a
different shape.

`.cham` / `.cham-sm` / `.cham-xs` / `.cham-b` in the marks layer carry the
chamfer. They are inert under `round`.

## Rule 5 · Annotation, with a budget

The marks — `Arrow`, `Circle`, `Underline`, `Bracket`, `Burst`, `Check`,
`Cross`, `Note` — are the one device here that a layout algorithm cannot
imitate. A mark says *someone looked at this and pointed*.

Which is exactly why they are rationed:

> **At most two marks per viewport, and at most one of those `tone="accent"`.**

Three arrows on one screen is a clip-art page, and the device stops meaning
anything the moment it is decoration. The rules that go with it:

1. **Marks are `aria-hidden`.** A mark that points at text is emphasis, not
   information — the text has to carry the meaning alone. `label` exists for
   the rare mark that is genuinely the only thing saying something, and if you
   are reaching for it, ask why the page does not say it in words.
2. **Marks stroke in `--doodle-ink`, never `--foreground`.** An annotation at
   full text contrast is a second headline.
3. **The script face appears in `<Note>` and nowhere else.** Not in a heading,
   not in a button, not in a paragraph. It is a hand in the margin.
4. **Marks overshoot.** A circle that closes exactly is a shape; one that
   overshoots its own start is a gesture. The paths are drawn that way on
   purpose — do not "fix" them.

`fid add component doodle` installs the set.

## Rule 5a · The page is a component too

A library that gives you a button and leaves the page to you stops being
applied exactly where the defaults are strongest. So the page shapes ship too:

```
fid add component sections
```

`Section` `Band` `Eyebrow` `SectionHead` `Hero` `LiveDot` `StepList`
`StatusCard` `CardGrid` `Feature` `FeatureList` `Cta` — each one a shape this
design system has already decided (rhythm, measure, which named step, where the
rule goes), and none of them offering a gradient, a shadow, a hover lift or a
carousel as a prop. A list of negative constraints that the components can
still violate is not a list.

Assemble a landing page from these and it is consistent by construction. If a
page needs a shape that is not here, add it to `design-system.md` first and
then to `sections.tsx` — in that order.

## Rule 5b · The design system governs decks and documents too

Claude Design, Docs and Slides became one interface in September 2026. Claude
now picks the surface from the task rather than the user picking the tool, which
means the same request can come back as a page, a deck or a document — and a
design system that only describes a web page governs one of the three.

The failure this produces is specific and familiar: the website is exactly on
brand and the pitch deck looks like every other pitch deck. That is the original
problem wearing a different hat, and it is worse here, because a deck is the
artifact that goes to people who have never seen the site.

So `design-system.md` declares `fid:surfaces`, and `fid derive` validates it —
a surface that maps to a type step or a colour the declaration does not have is
an error, not a silent fallback.

```toml
[slides]
title       = "display"
body        = "subhead"        # not "body" — see below
canvas      = "1280x720"
max_points  = 4
chart_order = ["chart-1", "chart-2", "chart-3", "chart-4", "chart-5"]
never       = ["a gradient title slide", "an icon beside every bullet"]
```

Three things this gets right that a deck otherwise gets wrong:

1. **Slide body is a larger step than page body.** A slide is read from three
   metres away, a page from forty centimetres. Reusing one step for both is the
   single most common way a deck built from a web design system comes out
   unreadable — and it is invisible on the laptop it was built on.
2. **`max_points` is a number.** "One idea per slide" is advice, and advice
   loses to a bullet list that felt like it needed six items.
3. **`chart_order` may only name declared colours.** A chart needing one more
   series than the palette has is exactly where a sixth colour gets invented.
   Declaring the order means the sixth series is a conversation, not an
   accident.

The global **Not this** list applies to every surface. `never` on a surface is
what is *additionally* forbidden there.

### Publishing the system to Claude Design

The gallery is not only a page to look at. Uploaded to a claude.ai design-system
project, it is what Claude Design builds *from*: it reads the components,
checks its output against them, and corrects before you see it. The `@dsCard`
marker on each preview's first line is what puts it in the Design System pane.

```
fid design           list the cards, check both failure modes
fid design --check   the CI gate
fid design --plan    the write set, for /design-sync to publish
/design-sync         the upload itself
```

`fid design` does not upload — that needs a claude.ai authorization a CLI cannot
hold, and a `push` that does not push is worse than no command. What it owns is
the half that fails silently:

- **The gallery is rebuilt separately from `fid derive`.** Run the derive and
  skip the rebuild, and the published swatches describe a palette the product no
  longer has. Nothing else catches this: the app builds fine either way.
- **A preview with no card marker uploads and then is not there.** That reads
  as a dropped file rather than a malformed one, so it gets debugged in the
  wrong place.

Uploading a new `tokens.css` also re-derives the project's adherence config —
the rules that flag a raw hex or a raw `px` in generated output as "use a
design-system token". Leave a stale `tokens.css` up there and that linter goes
on enforcing token names the product has renamed.

### Brand as a skill

This capability installs `SKILL.md` into `.claude/skills/`, which is the same
mechanism Claude uses to keep presentations and documents on brand — a brand
skill activates automatically whenever a presentation, document or spreadsheet
is created, without being asked for. That is why the declaration lives next to
the skill rather than only in the repository's CSS: the CSS governs the app, and
the skill governs everything else the product ever has to hand someone.

## Rule 5c · The tells are in the copy, not only the CSS

A page can pass every rule above — exact families, named steps, measured
contrast — and still read as generated, because the giveaway is usually the
*writing* and the section furniture around it.

The eyebrow is the clearest case. `PRINCIPLE` in tracked-out caps above a
heading is a pattern that appears on essentially every generated landing page,
and it almost never carries information: the heading underneath already says
what the section is. It survives because it looks like design. **Do not put a
label above a heading unless the label says something the heading does not.**

This capability shipped `type-eyebrow` as a scale step and used it on six
sections of a real product. That was wrong, and the step stays only because a
genuine category label — a status, a date, a document class — is a real thing;
it is no longer the default furniture of a section.

The rest of the list, all of which are copy rather than CSS:

- **A heading that names the section instead of making a claim.** "How it
  works" is a table-of-contents entry. "Two channels must agree before anyone
  is called" is the section.
- **The three-part hero**: headline, one-sentence restatement, two buttons. If
  the subtitle only rephrases the headline, delete one of them.
- **Sentences built from "seamlessly", "robust", "powerful", "cutting-edge",
  "leverage", "empower", "revolutionize".** None of them survive the question
  *what would be different if the opposite were true?*
- **Tricolon everywhere** — "faster, simpler, safer". One adjective that is
  true beats three that rhyme.
- **A number with no unit or date.** "99.9% accurate" is a claim; "99.9%
  accurate, measured on 1,400 events over six weeks" is a fact.
- **Symmetry that the content does not have.** Four features because four fits
  the grid, when there are three.

The test: read a section aloud and ask what a competitor could not also say. If
the answer is nothing, the section is decoration no matter how well it is set.

## Rule 6 · Write the negative constraints

The most useful half of a design system is the half that says what the product
does *not* do, because that is the half a default cannot fill in.

`design-system.md` has a **Not this** section. Fill it. Real examples:

```
This product does not use:
  · full-bleed hero with centred text over a gradient
  · icon-grid feature sections
  · testimonial carousels
  · three-column link-list footers
  · pill-shaped anything
  · multi-layer shadows — one flat shadow or none
Instead: editorial column, long measure, pull quotes, inline figures,
         section bands separated by a rule rather than by whitespace alone.
```

Also name the anti-references. "Not Stripe-lookalike, not startup-modern, not
glassmorphism" is a constraint an agent and a person can both act on.

## Rule 7 · Test on one component before a page

Before generating a screen, generate a single component in all its states and
check it against the declaration: exact hex, exact corner, exact type step, no
invented gradient, no invented shadow. A design system that has only ever been
applied to a whole page has not been tested — the errors are hiding in the
composition.

## Rule 8 · The changelog is part of the declaration

`design-system.md` ends with a dated changelog. A visual decision that changed
without a line there is a decision the next person will change back.

```
## Changelog
- 2026-09-17 · Split type into four roles; body moved off the display face.
- 2026-09-17 · Corner strategy chamfer; --radius forced to 0.
```

## What this capability installs

| Path | Is |
|---|---|
| `design-system.md` | **the declaration** — product-owned, edit freely |
| `pipelines/design.toml` | the derivation: `design-system.md` → `tokens.css`, gated by `fid derive --check` |
| `apps/web/src/app/marks.css` | the mechanism: chamfer, marks, outline and frosted surfaces. Reads tokens, declares none |
| `scripts/build-design-system.mjs` | generates the gallery from the app's own stylesheet |

And after the first `fid derive`:

| Path | Is |
|---|---|
| `apps/web/src/app/tokens.css` | **generated** — guard-blocked, rewritten every derive |

`marks.css` assumes the Next.js layout `web-next` scaffolds. A SvelteKit
product moves it to `apps/web/src/marks.css` and imports it from there — the
declaration does not move, only where the mechanism lands.

## What this capability does not do

It does not derive your components. `fid:contrast` catches a palette that fails
its own rule, and `fid derive --check` catches a stylesheet that has drifted
from the declaration — but nothing here can catch a component that hardcodes
`#B85207` instead of reading `--primary`, or one that sets
`text-[2.375rem]` instead of using a named step. That is what Rule 7 is for:
test on one component, and look at what it actually emitted.

It does not derive the font loading. `fid:type` names the families and the
weights; wiring them into `next/font` or `@font-face` and binding the
`--font-*` variables is still yours, and the `var` field on a role is how you
tell the generator what you called them.

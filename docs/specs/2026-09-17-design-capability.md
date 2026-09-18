# Design capability

**Date:** 2026-09-17
**Status:** accepted

---

## Context

Every product this platform scaffolds had a design system already. It was the
default one: Inter, an indigo primary, `rounded-xl`, a gradient hero, an
icon-and-label sidebar, 64px of padding. Nobody chose it. It arrived because the
questions were never asked, and an unasked visual question resolves the same way
in every product that never asked it.

That is the same failure the platform exists to delete, in a domain it had not
reached: a fact typed a second time is drift, and a fact never declared at all
is drift with no first copy to diff against.

Three copies of the palette were already in the repository — `@fiducial/tokens`,
`web-next`'s `globals.css` and `web-svelte`'s `app.css` — and the latter two
carried a header saying "generated from @fiducial/tokens. Run `fid derive` to
regenerate." Nothing generated them. The claim had been false since it was
written.

---

## Declaration

`design-system.md` at the product root. Prose everywhere, and fenced
` ```toml fid:<section> ` blocks carrying what a machine can act on:
`fid:type`, `fid:scale`, `fid:color`, `fid:contrast`, `fid:shape`.

### Judgment: one file, not a block plus a document

The obvious shape was `[design]` in `fiducial.toml` for the values and a
document for the reasons. It was rejected.

A design system's reasons are not commentary on its values — they are the part
that decides the next value. "Chakra Petch stops resolving below ~1.1rem, and
that is a feature: it cannot leak into body copy" is what stops the next person
setting body copy in it. Split across two files, the reason is not read at the
moment the value is changed, and the two drift in the way this platform exists
to prevent — except that here only one of them is checkable, so the drift is
silent.

Untagged fences are ignored, so a design system full of example snippets cannot
have one of them read as a declaration. A tag declared twice is an error rather
than last-wins: a document that says a thing twice will have one copy edited.

### Judgment: four type roles, not two

The shadcn slot set ships sans and mono. Two roles cannot express the property
that most separates a designed page from a generated one — **the display face is
not the body face at a larger size**. A product with one family has a document,
not a design.

The fourth role, `script`, exists so a page can have a voice beside itself. It
is constrained to `<Note>` and nothing else, because a handwriting face used for
a heading is a greetings card.

`parse` refuses a declaration where `display` and `body` name the same family.

### Judgment: corners are a strategy, not a scalar

`--radius` can say how much and never what kind, which is why every product on
the shadcn slot set has one silhouette. Shape is declared as
`round | chamfer | square` plus a three-step scale (`panel`/`control`/`chip`).

Three steps rather than one because `chamfer` carries a geometric constraint a
radius does not: a chamfer must stay under half the element's height, or the two
cuts on one edge meet and the box degenerates into a lozenge. The scale is tied
to element classes rather than to a size ramp for that reason.

`--radius` is forced to 0 under a non-round strategy — the corner comes from the
`.cham-*` classes and a live radius would round the chamfer's own clip — while
the `--radius-*` scale is still emitted, so a component copied in from any
shadcn-shaped registry resolves.

---

## Derivation

`pipelines/design.toml`, executor `fid-design`, one output:
`apps/web/src/app/tokens.css`.

### Judgment: the generator owns tokens.css, not globals.css

`globals.css` stays product-owned and imports the derived file. A generator that
owned the whole stylesheet would delete the base layer, the imports and the
commentary on every run — and those are prose, which is exactly what this
capability spent its design budget arguing should live next to its subject.

The split is: values are derived, mechanism is `marks.css` (the chamfer system,
the marks, the surfaces — it reads tokens and declares none), and everything
else is the product's.

### Judgment: the contrast check is a gate, not a report

`fid:contrast` declares pairs and minimums; `fid-design` computes them from the
palette and **fails the derive** when one misses.

"Every text/background pair carries its measured contrast ratio, and changing a
colour means re-measuring — not eyeballing it" was in the skill as advice. It is
the kind of advice that is followed until the day it is not, and the pair that
fails is almost always `muted-foreground`, which nobody re-checks because it
looked fine. Deriving it makes the rule hold.

A pair that cannot be measured — an unknown token, a value that is not
`oklch()` or hex — is a **failure**, not a pass. The declaration asked for a
number.

Alpha is dropped when parsing `oklch(… / α)`: a ratio against a translucent
colour depends on what is behind it, which the file cannot know. Measuring the
opaque colour is the conservative approximation.

### Judgment: an output with nowhere to land is skipped, not demanded

`design` is installed by `fid new` into every product, including firmware-only
ones. Those have no web app and no way to load a stylesheet.

`outputs_not_applicable` — already the mechanism for identity's migrations under
`storage = "none"` — is extended to `fid-design`: the declaration is still
parsed and the palette still measured, but the stylesheet is not written and
`fid derive --check` does not demand it. Adding `fid add app next` starts the
derivation with no further ceremony.

The applicability test is a **manifest** (`package.json` / `Cargo.toml`), not a
directory. The capability installs its own `apps/web/src/app/marks.css`, so the
directory exists from the moment the capability does; checking for it would mean
the guard never fires.

`dash` shares the same rule rather than restating it. Two answers to "does this
product produce this artifact" is how two commands start disagreeing about one
file, and `dash` had already started: it would have shown a permanent freshness
problem on a correctly configured product.

---

## Installed by default

`fid new` installs `design` alongside `i18n`.

The argument is the one in *Context*: the alternative to having a design system
is having the default one. A capability that must be remembered is a capability
most products do not have, and the cost to a product that never wanted it is one
document and two CSS files.

`design-system.md` ships **filled in with a real pairing rather than TODOs** —
a system from the first commit beats a checklist nobody completes — and opens
with the method rather than only the rules: anti-references first (easier to
answer than "what is this", and it constrains more), then adjectives, then type,
then colour, then the negative constraints, then test one component before a
page.

A test asserts the shipped default parses, passes its own contrast check, and
names no family from the slop list. Shipping a palette that fails the rule the
capability exists to enforce would put that failure in every product.

---

## The page is a component

`fid add component sections` ships Hero, Band, SectionHead, StepList,
StatusCard, Feature, Cta and the rest.

A library that gives you a button and leaves the page to you stops being applied
exactly where the defaults are strongest — the page is where the gradient hero
and the icon grid live. A test asserts the sections contain no gradient, shadow,
hover-scale or carousel: a list of negative constraints that the components can
still violate is not a list.

`Feature` is what replaces the icon grid — a rule and a real sentence, because
an icon beside a four-word label is decoration standing in for a reason.

`rounded-full` has one carve-out, written down in the declaration and enforced
in the test: a six-pixel status dot. A circle that small is a circle, not a pill.

---

## Registry consolidation

Not part of the design capability, but found while extending the registry and
fixed here because it made the extension unsafe.

`packages/ui-react` and `packages/ui-svelte` held a second copy of every
component. The copies had drifted, and the drift was in the direction that
matters: the **shipped** copy imported `cn` from `../lib/utils`, which does not
resolve from the `apps/web/src/components/ui/` directory `fid add component`
installs into, while the copy being typechecked imported `./lib/utils` and
passed. A typecheck that only ever ran against a file nobody ships is worse than
none, because it reports confidence.

Both registries now live where they ship from — inside the crate that embeds
them — and carry their own `package.json` so the files that ship are the files
that get checked. The first run surfaced a real a11y defect in `Dialog.svelte`.

---

## Consequences

- Platform version 0.2.0. `fid-design` is a new capability surface, and a
  product that declares the pipeline needs a binary that can run it. `fid derive
  --check` never invokes executors, so an older binary still validates a
  product correctly; only re-deriving requires the new one.
- `@fiducial/tokens` gains `fontRoles`, `fontDefaults`, `typeScale`, `shape.ts`
  and two theme slots. `generateThemeCss()` takes an options object; every
  option is optional and the previous output is a subset of the new one.
- `web-next` and `web-svelte` no longer carry a palette. Their stylesheets
  import the derived tokens, and the "generated by `fid derive`" header on them
  is true for the first time.

---

## Not done

The pipeline does not police components. `fid:contrast` catches a palette that
fails its own rule and `fid derive --check` catches a stylesheet that has
drifted from its declaration, but nothing catches a component that hardcodes
`#B85207` instead of reading `--primary`, or one that writes `text-[2.375rem]`
instead of using a named step. The registry tests do this for the components
this repository ships; a product's own components are covered by Rule 7 —
test one component and look at what it emitted.

Font loading is not derived. `fid:type` names the families and the weights, and
emits the weights into `tokens.css` as a comment beside each role, but wiring
`next/font` or `@font-face` and binding the variables is the product's.

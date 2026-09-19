# {{name}} — Design System

**This file is the input, not the record.** It is written before the UI and it
is authoritative over it. When `globals.css` and this file disagree, this file
is right and the stylesheet is a bug — and since `fid derive` generates the
token stylesheet from the blocks below, they mostly cannot.

---

## 0 · How to write this file

Read this section once, do what it says, then delete it.

**Spend twenty minutes on prose before you generate a single screen.** Not
because the prose is the deliverable, but because of the one rule the rest of
this document exists to serve:

> **Silence is a decision to use the default.**
> Every constraint you do not write, something else writes for you — and what
> it writes is Inter, an indigo primary, `rounded-xl`, a gradient hero, an
> icon-and-label sidebar, and 64px of padding. Those are not bad decisions.
> They are *absent* ones, and they are absent in the same direction every time.

The method, in order:

1. **Name the anti-references first.** What this product must not be mistaken
   for is easier to answer than what it is, and it constrains more. "Not
   startup-modern, not a dashboard, not an NGO brochure" rules out more bad
   screens than any adjective rules in.
2. **Then the adjectives** — three to six, of the kind you could disagree with.
   "Clean" and "modern" are not decisions. "Slightly formal" is.
3. **Then type, because it is the loudest.** Name exact families, never
   categories: a category resolves to the default. Pick a display face and a
   body face that are *different voices*, not different sizes.
4. **Then colour, as hex plus a role.** Then measure the pairs — the pipeline
   will make you.
5. **Then shape**, as a strategy, not a number.
6. **Then the negative constraints.** This is the half a default cannot fill in
   and the half most people skip. Be specific and be unreasonable.
7. **Test on one component before a page.** Ask for a single component in all
   its states and check it against this file: exact value, exact corner, exact
   step, no invented gradient, no invented shadow. Errors hide in composition;
   a system only ever applied to a whole page has not been tested.

**Everything below ships filled in with real defaults, not TODOs** — a system
from the first commit beats a checklist nobody completes. But a default you
kept on purpose is a decision and a default you never read is the problem this
file exists to prevent. Go through it once.

### How this file is read

Prose is for people. The fenced ` ```toml fid:<section> ` blocks are what
`fid derive` parses. Untagged fences — like an example in the prose — are
ignored, so you can write code in here safely.

```
design-system.md              ← you edit this
  ↓  fid derive                (pipeline: design — executor fid-design)
apps/web/src/app/tokens.css   ← generated: never hand-edit
  ↓  imported by globals.css, which is yours
components                    ← read tokens and named steps, never literals
```

---

## 1 · Personality

> Three to six adjectives. They are the tiebreaker for every decision this
> document does not cover, so make them the kind you could disagree with.

`editorial` · `warm` · `considered` · `slightly formal`

**Anti-references** — what this product is explicitly not:

not corporate SaaS · not Stripe-lookalike · not startup-modern · not
glassmorphism · not a dashboard

**References** — what it is closer to:

_(Name two or three real sites or objects — "Stripe Press", "A24", "a field
manual", "a Swiss rail timetable". Specific enough to argue with.)_

---

## 2 · Type

Four roles, not two. The pair that shadcn ships — sans plus mono — cannot
express the thing that most separates a designed page from a generated one:
**the display face is not the body face at a larger size.** If your headline
and your paragraph are the same family, you have a document, not a design.

The script role is the other half. It gives the page a voice beside itself —
the hand in the margin. It belongs in `<Note>` and nowhere else: not a heading,
not a button, not a paragraph.

`display` is the slot you are expected to replace once you have a brand. The
other three are the floor.

> **Check `latin-ext` before you swap a family.** A face that cannot set `ž` is
> not a candidate for a product that ships in more than one language.

```toml fid:type
display = { family = "Chakra Petch",   weights = [500, 600, 700], fallback = ["ui-sans-serif", "system-ui", "sans-serif"] }
body    = { family = "Source Serif 4", weights = [400, 600],      fallback = ["ui-serif", "Georgia", "serif"] }
script  = { family = "Caveat",         weights = [500, 600],      fallback = ["ui-serif", "cursive"] }
mono    = { family = "JetBrains Mono", weights = [400, 500],      fallback = ["ui-monospace", "SFMono-Regular", "Menlo", "monospace"] }
```

Loading the family yourself (next/font, `@font-face`)? Add `var = "--font-x"`
to a role and it goes first in the stack, ahead of the family name — so the
self-hosted file is used when it has loaded and the same family still resolves
by name if it has not.

### The scale

Ten steps, one class each, and **nothing outside them**. An inline
`text-[2.375rem] leading-[1.06]` is a scale step that exists on one page and
nowhere else, and the next page invents a different one. Need an eleventh? Add
it here.

```toml fid:scale
display = { role = "display", size = "3.75rem",   leading = "1.04", weight = "600", tracking = "-0.025em", wrap = "balance" }
h1      = { role = "display", size = "2.75rem",   leading = "1.1",  weight = "600", tracking = "-0.02em",  wrap = "balance" }
h2      = { role = "display", size = "2rem",      leading = "1.18", weight = "600", tracking = "-0.015em", wrap = "balance" }
h3      = { role = "display", size = "1.25rem",   leading = "1.3",  weight = "600", tracking = "-0.01em" }
subhead = { role = "body",    size = "1.3125rem", leading = "1.55", weight = "400", wrap = "pretty" }
body    = { role = "body",    size = "1.0625rem", leading = "1.65", weight = "400", wrap = "pretty" }
small   = { role = "body",    size = "0.9375rem", leading = "1.6",  weight = "400", wrap = "pretty" }
eyebrow = { role = "display", size = "0.75rem",   leading = "1",    weight = "600", tracking = "0.18em", uppercase = true }
note    = { role = "script",  size = "1.25rem",   leading = "1.25", weight = "500" }
mono    = { role = "mono",    size = "0.8125rem", leading = "1.5",  weight = "400" }

# Display type is the one part of a scale that cannot be responsive by
# accident: 3.75rem on a 375px screen is six lines, and in a language with
# longer words it is eight.
clamp = { below = "40rem", display = "2.5rem", h1 = "2rem", h2 = "1.625rem" }
```

Rules that go with it:

- **No weight above 600** except in the hero. 700 exists for one element.
- One `display` step per page, at most.
- Body measure caps at **68 characters**. Wider is not a longer line, it is a
  line nobody finishes.
- **`body` is 17px, not 16px.** 16px is the browser default, which is why it is
  the size every generated page comes out at, and it is a size chosen in 1996
  for a 96dpi CRT. Running prose on a modern display wants 17 to 19, and a
  serif at a text optical size wants the upper half of that. Set the measure
  and the leading with it: a bigger size at the same line length is worse, not
  better.

---

## 3 · Colour

Hex — or OKLCH, which `fid derive` annotates with the hex — plus **what the
colour is for**. Not a description of the hue. "A warm red" is not a value and
"brand orange" is not a role.

```toml fid:color
[light]
background           = "oklch(1 0 0)"           # page ground
foreground           = "oklch(0.205 0 0)"       # body text
card                 = "oklch(1 0 0)"
card-foreground      = "oklch(0.205 0 0)"
popover              = "oklch(1 0 0)"
popover-foreground   = "oklch(0.205 0 0)"
primary              = "oklch(0.32 0 0)"        # primary actions, links
primary-foreground   = "oklch(0.985 0 0)"
secondary            = "oklch(0.97 0 0)"
secondary-foreground = "oklch(0.205 0 0)"
muted                = "oklch(0.97 0 0)"        # section bands, inset surfaces
muted-foreground     = "oklch(0.47 0 0)"        # secondary text, captions
accent               = "oklch(0.97 0 0)"
accent-foreground    = "oklch(0.205 0 0)"
destructive          = "oklch(0.577 0.245 27.325)"  # errors only
border               = "oklch(0.922 0 0)"
input                = "oklch(0.922 0 0)"
ring                 = "oklch(0.6 0 0)"
doodle-ink           = "oklch(0.55 0 0)"        # annotation marks
doodle-accent        = "oklch(0.32 0 0)"        # the one loud mark per viewport

# Data-visualisation series, in the order a chart spends them. Declared here
# because a deck that needs a sixth line is otherwise where a sixth colour gets
# invented. Replace these first when you replace the palette: a neutral ramp is
# a placeholder, and five greys is not a categorical scale.
chart-1              = "oklch(0.87 0 0)"
chart-2              = "oklch(0.556 0 0)"
chart-3              = "oklch(0.439 0 0)"
chart-4              = "oklch(0.371 0 0)"
chart-5              = "oklch(0.269 0 0)"

[dark]
background           = "oklch(0.16 0 0)"
foreground           = "oklch(0.97 0 0)"
card                 = "oklch(0.21 0 0)"
card-foreground      = "oklch(0.97 0 0)"
popover              = "oklch(0.21 0 0)"
popover-foreground   = "oklch(0.97 0 0)"
primary              = "oklch(0.87 0 0)"
primary-foreground   = "oklch(0.21 0 0)"
secondary            = "oklch(0.27 0 0)"
secondary-foreground = "oklch(0.97 0 0)"
muted                = "oklch(0.27 0 0)"
muted-foreground     = "oklch(0.72 0 0)"
accent               = "oklch(0.27 0 0)"
accent-foreground    = "oklch(0.97 0 0)"
destructive          = "oklch(0.704 0.191 22.216)"
border               = "oklch(1 0 0 / 12%)"
input                = "oklch(1 0 0 / 16%)"
ring                 = "oklch(0.56 0 0)"
doodle-ink           = "oklch(0.62 0 0)"
doodle-accent        = "oklch(0.87 0 0)"
chart-1              = "oklch(0.87 0 0)"
chart-2              = "oklch(0.72 0 0)"
chart-3              = "oklch(0.6 0 0)"
chart-4              = "oklch(0.47 0 0)"
chart-5              = "oklch(0.36 0 0)"
```

### Measured pairs

These are not documentation. `fid derive` computes every one of them from the
values above and **fails if any falls under its minimum** — which is what turns
"do not eyeball it" from advice into a rule. If a pair fails, change the
colour, not the minimum.

A pair with no `theme` is measured in both. A slot a theme does not override
falls back to light's.

```toml fid:contrast
pairs = [
  { fg = "foreground",         bg = "background", min = 4.5, note = "body text — the one that must never fail" },
  { fg = "muted-foreground",   bg = "background", min = 4.5, note = "secondary text; the pair that fails most often" },
  { fg = "muted-foreground",   bg = "muted",      min = 4.5, note = "the same text on a section band" },
  { fg = "primary",            bg = "background", min = 4.5, note = "primary used as a link colour" },
  { fg = "primary-foreground", bg = "primary",    min = 4.5, note = "label on a primary button" },
  { fg = "doodle-ink",         bg = "background", min = 3.0, note = "annotation is non-text; 3.0 is the graphics floor" },
]
```

### Prohibitions

_(State them. "No blues or purples anywhere" does real work; "warm palette"
does not.)_

- `--destructive` is for errors. Not for urgency, not for emphasis.
- No colour is introduced at a call site. If it is not in the block above, it
  does not exist.

---

## 4 · Shape and space

Corners are a **strategy**, not a number. The shadcn slot set has one shape
decision in it — `--radius` — which is why every product built on it has the
same silhouette. One variable can say *how much*; it can never say *what kind*.

Three sizes rather than one scalar, because under `chamfer` there is a hard
geometric constraint a radius does not have: **a chamfer must stay under half
the element's height**, or the two cuts on one edge meet and the box
degenerates into a lozenge. A chip given the panel corner is not slightly
wrong, it is a different shape.

```toml fid:shape
strategy      = "chamfer"   # round | chamfer | square
panel         = "1rem"      # cards, dialogs, sections (taller than ~5rem)
control       = "0.5rem"    # buttons, inputs, nav items (~2.25rem and up)
chip          = "0.375rem"  # badges, pills, tags (~1.75rem and up)
border        = "2px"       # structural edges
hair          = "1px"       # rules and dividers
doodle_stroke = "2.25px"    # mark weight — fixed, never scaled
edge_weight   = 1.5         # the weighted bottom/right edge; 1.0 flattens it
# radius = "0.625rem"       # only read under strategy = "round"
```

`edge_weight` is one of two sides of the depth cue, not the whole of it — see
"Texture and depth" below. Under `chamfer` a clipped element
cannot cast an outer `box-shadow`, so elevation is carried by weighting the
bottom and right edges instead, and the two corners between grade from light to
heavy the way a bevel under one light source does. It was briefly fixed at `3`,
which does not read as depth: at three times the base edge the bottom of a card
reads as a border someone got wrong. Keep it close to `1` — the cue works at
the threshold where you notice it without looking at it.

A cut is a detail, not a silhouette. These are smaller than they were: at
1.5rem a button stopped reading as a rectangle with cut corners and started
reading as an octagon, which is a shape with an opinion of its own that
competes with the label inside it. The corner should be noticed second.

**Depth: `.cham-lift`.** A `clip-path` element cannot cast an outer
`box-shadow` — the clip removes it — so elevation has to come from the edge.
`.cham-lift` weights the bottom and right, and the two corners between grade
from light to heavy, which is what a bevel under a single light source does.
Use it on what is genuinely raised: a primary action, a floating panel. A page
where everything lifts has nothing raised.

Under `chamfer` and `square`, `--radius` is forced to 0 and the corner comes
from `.cham-*` in the marks layer. The `--radius-*` scale is still emitted, so
a component copied in from any shadcn-shaped registry resolves.

**Spacing** — base 4px. Scale: `4 8 12 16 24 32 48 64 96`. Nothing between.

**Shadows** — one flat shadow or none. Never multi-layer, never coloured.
_(Under `chamfer`, note that a `clip-path` element cannot cast an outer
box-shadow at all: the clip cuts it off. Elevation has to be the edge.)_

---

## 4a · Texture and depth

A page of flat fills reads as a rendering of a design rather than as a made
thing. Two treatments fix that, and under `chamfer` both are shaped by one
fact: **a `clip-path` element cannot cast an outer `box-shadow`.** The clip
removes it. Every conventional depth cue is therefore unavailable, which is why
the two below are unusual.

**The bevel has two sides.** Bottom and right edges are weighted by
`edge_weight` — the dark side. The top inside edge of each fill carries a
hairline `inset` highlight (`--surface-highlight`) — the lit side. Inset
shadows paint inside the clip and survive it. Together they describe one light
source, above and to the left. Nothing else on the page may imply a second one.

A product that ships only the weighted edge has drawn half a bevel, and it
reads as a border that is wrong rather than as a surface that is raised.

**The grain.** One fixed, `pointer-events: none` layer over the viewport at
`--grain-opacity`, generated by an SVG `feTurbulence` filter rather than
shipped as an image — no request, no decode, resolution independent. Multiply
on light, screen on dark, so the speckle sits *in* the surface, not on it.

One fixed layer, never a texture per surface: noise attached to anything that
scrolls repaints every frame. Remove it entirely under
`prefers-reduced-motion` — a static dither across a full viewport is a known
migraine trigger, and nothing is lost when it goes, because texture is not
information.

Neither may become visible as an effect. If you can point at the grain, it is
too strong.

Both live in `marks.css` (mechanism) reading two values you set in your own
stylesheet (composition):

```css
:root {
  --surface-highlight: oklch(1 0 0 / 0.55);
  --grain-opacity: 0.035;
}
:root.dark {
  /* a white highlight at 55% on a dark surface is a scratch, not a bevel */
  --surface-highlight: oklch(1 0 0 / 0.07);
  --grain-opacity: 0.055;
}
```

---

## 5 · Components

**Start from shadcn/ui and adapt.** `npx shadcn@latest add <name>`, then change
the corner and the surface; change nothing else. Focus rings, `aria-invalid`,
disabled semantics, icon sizing and `asChild` come from upstream and stay as
upstream ships them — they are the parts that are invisible when wrong, and
nobody re-derives the whole list from memory. `button.tsx` and `card.tsx`
arrive already adapted; see the capability's SKILL.md for exactly which three
things "adapt" changes.


> Say how each one looks, in the terms above. Silence here is the gap a default
> fills.

- **Button** — flat, no gradient. `.cham-sm`, `type-small` at weight 500.
  One primary action per view; everything else is `.cham-outline`.
- **Input** — card fill, `--border-w` edge, focus ring in `--ring`. No inner
  shadow.
- **Card** — `.cham`, 2px edge, 1.4rem padding. No shadow, no hover lift. A
  card that reacts to the pointer is a button.
- **Chip / status pill** — `.cham-xs`, `type-small`, muted foreground. Says
  what is actually true; never decorative.
- **Nav** — text links at `type-small`, no icons. Sticky, frosted, `.cham-b`.
- **Table** — zebra striping in `--muted`, `--hair-w` rules, no outer border.

### Page shapes

`fid add component sections` — `Section` `Band` `Eyebrow` `SectionHead` `Hero`
`LiveDot` `StepList` `StatusCard` `CardGrid` `Feature` `FeatureList` `Cta`.

Build pages out of these rather than out of divs. They read named type steps
and tokens, they carry the rhythm and the measure, and none of them will let
you pass a gradient. The icon grid is replaced by `Feature` — a rule and a real
sentence, because an icon beside a four-word label is decoration standing in
for a reason.

### Annotation, with a budget

`Arrow` · `Circle` · `Underline` · `Bracket` · `Burst` · `Check` · `Cross` ·
`Note` — installed by `fid add component doodle`.

> **At most two marks per viewport, one of them `tone="accent"`.**

Three arrows on one screen is a clip-art page, and the device stops meaning
anything the moment it is decoration. Marks are `aria-hidden` — one that points
at text is emphasis, not information, and the text has to carry the meaning
alone. They stroke in `--doodle-ink`, never `--foreground`.

---

## 5a · Surfaces beyond the web page

Claude Design, Docs and Slides became one interface in September 2026. A design
system that only describes a web page is now an incomplete one: the same
declaration has to govern a deck and a document, or those come out generically
styled while the site is exactly on brand — which is the original problem
wearing a different hat.

This block is **declared and validated but not generated from**. There is no
stylesheet for a slide. The consumer is the skill an agent reads; the pipeline's
job is to refuse a surface that maps to a type step or a colour the declaration
does not have, so the mapping cannot quietly point at nothing.

```toml fid:surfaces
[slides]
title       = "display"   # the one line on a title slide
heading     = "h2"        # a heading inside a content slide
body        = "subhead"   # slide body copy is read from further away than page copy
small       = "small"     # slide numbers, sources, footnotes
canvas      = "1280x720"
max_points  = 4           # "one idea per slide" is advice until it is a number
chart_order = ["chart-1", "chart-2", "chart-3", "chart-4", "chart-5"]
never = [
  "a gradient title slide",
  "an icon beside every bullet",
  "a slide that is only a chart with no sentence saying what it shows",
  "stock photography",
]

[document]
title   = "h1"
heading = "h2"
body    = "body"
small   = "small"
never = [
  "a cover page with a full-bleed image",
  "a three-column layout for running prose",
]
```

Notes worth keeping:

- **Slide body is `subhead`, not `body`.** A slide is read from three metres
  away and a page from forty centimetres. Using the same step for both is the
  most common way a deck built from a web design system comes out unreadable.
- **`chart_order` may only name declared colours.** A deck cannot introduce a
  sixth series colour by inventing one, which is exactly what happens when a
  chart needs one more line than the palette has.
- The global **Not this** list below applies to every surface. `never` here is
  what is *additionally* forbidden on that one.

---

## 6 · Not this

> The half a default cannot fill in. Most people skip this section. It is the
> one that does the most work.

This product does not use:

- full-bleed hero with centred text over a gradient
- icon-grid feature sections
- testimonial carousels, logo walls, "trusted by" strips
- three-column link-list footers
- pill / `rounded-full` anything
- multi-layer or coloured shadows
- an icon beside every navigation label
- hover lifts, scale transforms, scroll-reveal fades on content
- more than two annotation marks in one viewport

Instead: editorial column, long measure, section bands separated by a rule,
figures inline with the text that explains them, and annotation used sparingly
enough that it still means something.

---

## 7 · Changelog

> A visual decision that changed without a line here is one the next person
> will change back.

- _(date)_ · Scaffolded from the Fiducial `design` capability defaults.

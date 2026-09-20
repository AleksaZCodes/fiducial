# fiducial:press — Press Skill

This product has the `press` capability. A press room is three things, and most
products ship one of them badly.

## What a press room actually is

| Part | What it is for | Where it lives |
|---|---|---|
| **Brand guidelines** | so other people use your mark correctly | derived from `brand` + `design` |
| **Press kit** | so a journalist can write about you without calling you | `press/boilerplate.md`, `press/facts.md`, the brand primitives |
| **Stories** | so a journalist can find an *angle* | `press/stories/*.md` |

The third one is the one almost nobody has, and it is the one that decides
whether you get written about.

## Why stories exist

A journalist does not need your announcement. They need **an angle their editor
will buy**, and they will find one whether or not you help. If your press room
is a logo zip and a boilerplate paragraph, the angle gets constructed from
whatever else is reachable — a competitor's framing, an old funding round, a
GitHub issue someone screenshotted.

A story is raw material for someone else's article: the sequence of what
happened, the numbers, a quote you would actually say, and the tension that
makes it worth a column. Not a press release. A press release is written to be
printed; a story is written to be *taken from*.

Each carries an `angle` in its frontmatter — `origin`, `technical`, `customer`,
`failure`, `milestone`, `people`, `market` — so a reporter scanning the index
finds the one that fits the section they write for.

### Include the failures

A press room containing only wins reads as marketing, and marketing does not
get quoted — it gets ignored, or worse, it gets used as the setup for a
sceptical piece.

The story about what did not work is the one that gets a call back, because it
is the only evidence on the page that you will answer a hard question honestly.
A reporter deciding whether to invest a day in you is reading for exactly that.

## What is derived, and why that matters here more than elsewhere

**A press kit exists to be copied.** That is its entire function, and it is why
a stale fact in one is worse than a stale fact anywhere else on a site: it does
not sit there being wrong, it propagates into articles you do not control and
cannot correct.

So everything another declaration already owns is read from that declaration:

- the legal and trading name, and the domain, from `[brand]`
- the logo from `brand/icon.svg` and `brand/wordmark.svg`, the two primitives —
  **never a third copy.** A press page that ships its own logo file is how a
  journalist ends up publishing the mark you stopped using. This has happened
  in a product in this repository, twice, with every check green.
- colours and type from the `design` capability's `design-system.md`

Only the prose a person must write lives in `press/`, and `fid derive` turns it
into `src/generated/press.ts`.

## The page, in the order a journalist needs it

`PressKit` lays the room out as: **assets** (the logo files, then a photo and
diagram gallery), **fast facts**, **boilerplate**, **stories**, **contact**.
Most visitors arrive with a story already and want a picture for it. Prose
first puts the thing they came for three screens down.

Every image carries a caption and a download button (`MediaFigure`), and every
story card is one link covering the whole card, with the story's cover on it.

## Media is keys, not files

Stories may carry `cover` and `cover_alt` in their frontmatter. The gallery is
a list of `{ key, alt, caption }`, and a content collection is the natural
source for it:

```toml
# content.toml
[collections.press-gallery]
kind   = "collection"
schema = { key = "string", alt = "string", caption = "string", order = "number" }
body   = "none"
route  = ""
```

`cover` and `key` are **object keys in the product's storage**
(`press/stories/origin.png`), never URLs and never files in git. `mediaUrl()`
in `src/lib/media.ts` turns a key into `/media/<key>`. The product serves that
path from its `[adapters] storage` backend, with `?download` sending
`Content-Disposition: attachment`, which is what the download buttons use. On
Cloudflare that is a route handler reading the R2 binding. Changing storage
vendor changes that route, not the content.

An image with text in it needs one object per locale (`system.en.png`,
`system.sr.png`), each named by its own locale's entry. A logo render or a
photo is shared.

Renders of the mark (a PNG of the wordmark for someone's slides) are media, but
they are renders of the two primitives, not sources. Re-render them whenever the
mark changes. The bucket is outside `fid derive`, so nothing will do it for you.

## The asset set, which is the standard

What an outsider is handed is not "the logo". It is four files per mark, and
the set is derived rather than exported by hand:

|  | Light background | Dark background |
|---|---|---|
| **Wordmark** | `brand/wordmark-light.svg` + `.png` | `brand/wordmark-dark.svg` + `.png` |
| **Mark** | `brand/icon-light.svg` + `.png` | `brand/icon-dark.svg` + `.png` |

Three properties, each for a failure somebody hit:

- **Transparent.** An asset with a baked background can only be used on that
  background, and the first thing anyone does is put the mark on their own
  surface.
- **A variant per surface, not one file plus an instruction.** "Use the other
  colour on dark" is a rule nobody reads; two files is not.
- **SVG and PNG.** SVG is better and PNG is what a slide deck, a print shop
  and half the CMSes in the world accept.

**The dark variant should be one colour** unless the product has a reason
otherwise: it is then also the one-ink asset for print and engraving, and it
sidesteps the question of what the accent looks like on near black.

A gallery entry declares the surface its asset is for (`tone`). A transparent
cream wordmark previewed on a cream card is an empty rectangle with a caption
under it, which looks like a broken image and is not one.

The SVGs come from `scripts/derive-logo.mjs`, with their colours read from the
design tokens, so the palette owns them. The PNGs are rendered from those SVGs
by `scripts/render-brand.mjs` into the media staging directory — they are
media, not artifacts, so they live in the bucket and not in git.

## Writing the boilerplate

Three lengths — one sentence, two-to-three, a paragraph — because three are
what get asked for, and writing a fourth on deadline is how an inaccurate one
enters circulation. Once a wrong number is in one article it is in every
article that cites it.

Write plain fact. No adjective that cannot be checked; no "leading", no
"revolutionary". A journalist who strips your adjectives takes your meaning
with them, and an unsupportable claim is the one a sceptical editor picks at.

If the product has a claims policy, these sentences are subject to it. A press
kit is the easiest place in a company to start saying more than is true,
because nobody reviewing it is the engineer who knows better.

## Localization

Boilerplate and stories are user-visible strings, so principle 1c applies:
declared once, one derivation per locale, and a missing translation is a
missing artifact rather than a fallback. A product with `i18n` installed should
carry a locale directory under each collection and fail the build when a
locale is short a story it advertises:

```
press/docs/<locale>/boilerplate.md
press/docs/<locale>/facts.md
press/stories/<locale>/<slug>.md
```

**The locale sits under the collection, not above it.** `press/<locale>/stories/`
reads fine on disk and is the one arrangement an editor cannot show two
languages of side by side — a content editor groups locales only for a folder
collection nested this way. The layout above is the same one `content/` uses,
so both are edited the same way.

## Setup

```toml
# fiducial.toml
[press]
founded      = "2026"
hq           = "Niš, Serbia"
press_email  = "press@example.com"   # an inbox someone reads
spokesperson = "Name, Role"
```

A press contact nobody answers is worse than none: a journalist on deadline
moves on and writes the piece without you.

Then write `press/boilerplate.md`, `press/facts.md`, and at least two stories —
one of which is not a win.

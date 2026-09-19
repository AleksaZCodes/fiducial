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
carry `press/<locale>/` rather than a single directory, and fail the build when
a locale is short a story it advertises.

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

# Content is one layer, and everything user-visible is content

**Date:** 2026-09-19
**Status:** proposed

---

## Context

This platform already derives user-visible material from four different places,
each invented separately, each with its own shape:

| What | Declared in | Derived to | Localized |
|---|---|---|---|
| UI strings | `messages/<locale>.json` | `generated/messages.ts` | yes, enforced |
| Legal prose | `[legal]` + jurisdiction | `generated/legal.ts` | yes, generated |
| Brand facts | `[brand]` | `generated/brand.ts`, `public/*` | no |
| Press room | `press/<locale>/*.md` | `generated/press.ts` | yes, enforced |

Four mechanisms for one idea. Add a blog and it is five. Add a dashboard that
edits any of it and there is no single surface to edit — the dashboard would
need four adapters to four bespoke shapes.

They are the same thing. A UI string, a boilerplate paragraph, a fast fact, a
legal clause, a blog post and the landing page copy are all **content**: prose
or values a person reads, authored by a human, localized, and rendered by a
framework this platform does not want to be coupled to.

The differences between them are real but small: cardinality (one string vs a
collection of posts), whether the body is rich, and who may edit. None of those
justify four loaders, four generated shapes and four sets of freshness rules.

## Decision

**One content layer.** A `content` capability that owns the model, and two thin
renderer bindings — one for Next.js, one for SvelteKit — that consume the same
derived artifact.

### The model

A content **collection** is a directory of entries with a declared schema. A
**singleton** is a collection with exactly one entry. Both live under a locale:

```
content/
  <collection>/
    <locale>/
      <slug>.md          # frontmatter + body
```

`content.toml` declares the collections:

```toml
[collections.posts]
kind    = "collection"
schema  = { title = "string", date = "date", summary = "string", tags = "string[]" }
body    = "rich"          # markdown body is part of the entry
route   = "/blog/{slug}"  # what the renderer mounts; "" means not routed

[collections.landing]
kind    = "singleton"
schema  = { headline = "string", subhead = "string" }
body    = "none"
```

### What it derives

One artifact, `src/generated/content.ts`, exporting every collection keyed by
locale, plus the schema as types. Same shape for a blog post, a landing
headline and a press story — so anything reading content reads one shape.

### Localized by construction

The rules `press` already enforces become the layer's rules, applied to every
collection:

- a locale declared in `[i18n]` and missing an entry is a **failed build**, not
  a fallback
- every locale carries the same slugs, or the build fails naming the difference

This is `MISSION.md` 1c, and the reason `press` declares
`requires_capabilities = ["i18n"]` — a capability deriving copy without the
locale machinery underneath it produces a monolingual artifact that reports
success.

### Framework bindings are the thin part

`generated/content.ts` is plain data: no React, no Svelte, no framework import.
The bindings are small and mechanical:

- `@fiducial/content-next` — route generators, `generateStaticParams`, metadata
- `@fiducial/content-svelte` — `+page.server.ts` loaders, the same data

**This is what makes the port mechanical.** Today a Next.js page holds both the
content access and the markup. Split them and converting to SvelteKit is
rewriting markup only, against a data module that does not change. The test for
whether the split is real: `generated/content.ts` must not import anything from
`react` or `svelte`, and a lint rule should say so.

### Media lives in object storage, not the repository

Images and downloadable assets are content too, but they are not text and they
do not belong in git. The `adapters` capability already declares a `storage`
contract with an `r2` implementation, and `deploy` already derives
`wrangler.toml` from `[adapters]`. So:

- one R2 bucket per product for site media
- a `press/` prefix within it for the press kit — a prefix rather than a second
  bucket, because a second bucket needs a second binding, a second lifecycle
  policy and a second set of credentials to carry the same objects

An entry references media by key; the renderer resolves the key through the
storage adapter. That keeps the reference framework-agnostic and the bucket
swappable.

**The logo is not media.** It stays a brand primitive in the repository,
because it is derived into the favicon, the manifest, the components and the
press page. A mark in a bucket is a mark that can be replaced without a commit,
which is the opposite of what a brand primitive is for.

### The dashboard comes last, and is a view

A dashboard is worth building only once the model above exists, because it is a
*view over the model* rather than a thing with its own storage. It reads
`content.toml` for the schema, reads and writes the entry files, and commits.
Git remains the store; the dashboard is an editor with a nicer surface.

This is also the argument against reaching for Payload or a hosted CMS first.
Those bring their own database and become the source of truth, which breaks the
one property this platform is built on: that the declaration is a file in the
repository, reviewable and revertible, and that `fid derive --check` can prove
the artifact matches it. A CMS with its own database cannot be checked by CI.

## Consequences

- `press` becomes a **consumer** of `content`, not a parallel mechanism. Its
  boilerplate and facts are singletons; its stories are a collection with an
  `angle` field. The doctrine in its skill stays; the loader goes.
- `i18n` keeps owning short UI strings. A message catalog is the right shape
  for a button label and the wrong one for a blog post, and merging them would
  put prose in JSON.
- `legal` keeps generating from jurisdiction — it is derived from a *fact*, not
  authored, which makes it a different thing wearing the same clothes.
- Ordering matters: the model, then the bindings, then R2, then the blog, then
  the dashboard. Building the dashboard before the model means writing it twice.

## Status

Proposed, not built. `press` is the working prototype of the rules; the
generalization is the work this spec authorizes.

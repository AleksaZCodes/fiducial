# fiducial:content — Content Skill

Everything a person reads is content, and content is one layer.

## What this replaces

Before this capability, a product derived user-visible material through four
unrelated mechanisms, each invented separately:

| What | Declared in | Shape |
|---|---|---|
| UI strings | `messages/<locale>.json` | flat key/value |
| Legal prose | `[legal]` + jurisdiction | generated from a fact |
| Brand facts | `[brand]` | flat settings |
| Press room | `press/<locale>/*.md` | frontmatter + markdown |

A blog made five. Anything that wanted to read or edit content — a dashboard,
an export, a search index — had to learn all of them.

They are the same idea: prose or values, authored by a human, localized, and
rendered by a framework the platform does not want to be coupled to. The
differences are cardinality, whether the body is rich, and who may edit. None
of those justifies four loaders.

See `docs/specs/2026-09-19-content-is-one-layer.md`.

## The model

A **collection** is a directory of entries. A **singleton** is a collection
with exactly one. Both live under a locale:

```
content/posts/en/hello.md
content/posts/sr/hello.md
content/landing/en/landing.md
content/landing/sr/landing.md
```

**Same filenames under every locale.** That is the whole reason the directory is
shaped this way: a missing translation becomes a missing *file*, which a build
can see. A language suffix on each filename hides the gap in a listing, and a
per-entry fallback hides it completely.

`content.toml` declares the collections and their schema:

```toml
[collections.posts]
kind   = "collection"
schema = { title = "string", date = "date", summary = "string", tags = "string[]", cover = "media" }
body   = "rich"
route  = "/blog/{slug}"
```

## What fails the build, and why each one

- **A locale with no directory for a collection.** `MISSION.md` 1c: a missing
  translation is a missing artifact, not a fallback. Serving the default
  locale's copy is tempting and wrong — the reader is never told, and they
  quote it.
- **Locales carrying different entries.** A collection with a post in one
  language and not the other is not translated; it is two collections, and the
  gap is invisible from whichever page you are on.
Types: `string`, `date`, `number`, `boolean`, `string[]`, and `media` — a
storage key like `press/gallery/icon.png`, which is a string at runtime and an
image picker in the editor. A key is not a URL and not a path in git; see the
product's `docs/media.md`.

Entries are read by `scripts/frontmatter.mjs`, which reads the YAML a writer
actually emits — wrapped scalars and block sequences included, because the
editor writes those. A line-wise `key: value` reader loses a wrapped summary
and turns a list into an empty string, silently.

- **A frontmatter field not in the schema.** Almost always a typo, and a typo
  that silently disappears is how a paragraph goes missing with nobody able to
  find it.
- **A body where `body = "none"`, or none where `body = "rich"`.**
- **A generated module that imports a framework.** See below.

## Framework-agnostic, and checked

`generated/content.ts` is plain data. It imports nothing — not React, not
Svelte, not `next/`. The pipeline asserts this and fails if it ever stops being
true.

That single property is what makes a framework port mechanical. A page normally
holds both the content access and the markup; split them, and moving from
Next.js to SvelteKit is rewriting markup against a data module that does not
change. Keep the split by putting queries in `lib/content.ts` — also
framework-free — and letting components receive data as props.

The test: if a component file is the only thing that has to change to move
frameworks, the split is real.

## Media is content, but it does not live here

Images and downloadable assets belong in object storage, behind the `storage`
adapter contract — `r2` today, swappable by declaration. An entry references
media by key and the renderer resolves it, so the reference survives a change
of vendor.

**The logo is the exception.** It stays a brand primitive in the repository,
because it is derived into the favicon, the manifest, the components and the
press page. A mark in a bucket can be replaced without a commit, which is the
opposite of what a primitive is for.

## Consumers, not parallel mechanisms

`press` is the worked example. Its boilerplate and facts are singletons, its
stories a collection with an `angle` field. A capability that finds itself
writing a second markdown loader should be declaring a collection instead.

`i18n` keeps short UI strings — a message catalog is the right shape for a
button label and the wrong one for a blog post. `legal` keeps generating from
jurisdiction, because it is derived from a fact rather than authored.

## The editor

Built, as the `cms` capability, and built the way this section said to: a *view
over this model*. It reads `content.toml` for the schema, reads and writes
entry files, and stops — you commit. Git stays the store. `fid add cms`, then
`pnpm cms`.

Its config is derived from this declaration, so the editor's fields follow the
schema. Do not hand-edit `tools/cms/config.yml`.

A hosted CMS with its own database would become the source of truth instead,
which breaks the property everything here rests on — that the declaration is a file
in the repository and `fid derive --check` can prove the artifact matches it.
A database cannot be checked by CI.

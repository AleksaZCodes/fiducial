# fiducial:cms — Content Editor Skill

This product has the `cms` capability: a **local** content editor over the
repository's own files.

`pnpm dev` in `apps/cms` (or from the root, where turbo runs it with the site)
serves **Keystatic** at `http://localhost:4321/keystatic`. Every save writes
plain `.md` and `.json` into `content/`, `press/`, `messages/` and the root, and
a watcher re-derives. You review the diff and commit it like any other change.

## What this is, in platform terms

The design spec's §10 says the workbench is **a view, never a database**: the
repository holds the data, and the workbench reads it and writes back through
ordinary commits. This is the **authoring** slice, for the one kind of fact a
person edits most often and least enjoys editing in a text editor: prose.

It holds no store of its own, has no login, and is not deployed. `apps/cms` is
its own small workspace, so the editor never enters the site's bundle and the
same editor serves a Next.js product and a SvelteKit one. That is not a missing
feature: a hosted editor needs auth, a session, a backend and a second copy of
the content, and the second copy is the thing this platform exists to delete.

## Two editors, one set of files

`[cms] editor` in `fiducial.toml` picks one:

| `editor` | Use it when | Notes |
|---|---|---|
| `keystatic` (default) | Writers who edit forms: collections, fields, a settings page | Real image picker, relationship pickers, slug field. Reads and writes the files directly |
| `tina` | Editing on the page itself, visual editing, a hosted editor later | Local mode needs no service; hosting it is a Tina Cloud or self-hosted dependency |

Both configs are derived from one model every time, and both edit the same
files, so switching is `editor = "tina"`, `fid derive`, `pnpm install` — never a
content migration. Two honest differences, because Tina stores different values:
a picture is a text field holding the media **key**, and a relation is the
entry's slug as text.

## The config is derived — do not edit it

`apps/cms/keystatic.config.ts`, `apps/cms/tina/config.ts` and
`apps/cms/package.json` are written by `scripts/derive-cms.mjs` from:

- `content.toml` — every collection, its schema and whether it has a body
- `press/<kind>/<locale>/` — boilerplate, facts and stories, if present
- `messages/<locale>.json` and `settings.json` — fields read from the files
- `[i18n] locales` — one editable collection per language

`fid derive --check` fails when they are stale, so the editor cannot drift from
the content model. **Change the declaration, not the config.** A hand-edited
config is a second declaration of the model, and it is the copy that goes
wrong: an editor offering a field the schema lacks writes entries `fid derive`
then rejects, and an editor missing a field leaves it empty on everything made
through it — which shows up as a blank page long after the editing session.

### Widgets follow schema types

| `content.toml` type | Keystatic |
|---|---|
| `string` | text field (the title is the slug field) |
| `date` | date picker |
| `partial-date` | text, `YYYY`, `YYYY-MM` or `YYYY-MM-DD` |
| `number` | integer |
| `boolean` | checkbox |
| `string[]` | list of text |
| `media` | image picker; stores a **key**, never a URL |
| `object[]` | list of small records |
| `body = "rich"` | Markdoc editor, written as `.md` |

`[cms.selects]`, `[cms.hints]` and `[cms.relations]` refine a field: a fixed
list of choices, help text, and a searchable pick from another collection (a
field named `related<Collection>` points at that collection with no
declaration).

## Localization

`content/<collection>/<locale>/<slug>.md` gives one editable collection per
language (`Posts (English)`, `Posts (Serbian)`), grouped under one heading.
Keystatic has no built-in translation model, so two things are yours to keep:

- **The file name is the entry's identity and must match in every language.**
  The slug field is editable when you create an entry precisely so a translated
  title does not rename the file. `fid derive` fails on a locale missing an
  entry, which is the check.
- A picture is attached per entry, so a translation re-attaches it (the same
  key works; the picker lists what is staged).

A **singleton** is one named file per language, not a one-entry folder.

## Settings and interface text

Two kinds of file are not collections of entries, so the editor offers them as a
**Settings** group of forms, with fields read from the files themselves:

- `settings.json`: links and facts that are the same in every language (typed
  once; exported as `settings` from the generated content module). List more JSON
  files in `[cms] settings = [...]` and add them to the pipeline's `inputs`.
- `messages/<locale>.json`: every string the interface prints, one form per
  language.

A new key appears in its form on the next derive and nothing lists a message by
name. A key an editor cannot address (anything but letters, digits, `_`, `-`)
is left out of the form and stays in the file.

What stays in code: `fiducial.toml` and `design-system.md`. They carry comments
and structure a form would flatten, and they change how the product is built
rather than what it says.

## Media

Images dropped into the editor are staged in `media/` and stored as **keys**
(`site/blog/hello/cover.png`), never URLs and never files in git.

- `pnpm media:push` uploads what changed, against a machine-local hash record;
  `--all` ignores it.
- `pnpm media:pull` stages what the content already refers to. Run it on a
  fresh checkout, or the editor shows an empty thumbnail for an image that is
  perfectly fine in production.
- `pnpm media:push --prune` also deletes what nothing refers to any more.

**Deleting an entry deletes its text and leaves its picture.** The record is
gone from the site and the object is still in the bucket, paid for and
reachable by anyone who kept the URL. Push reports those orphans every time and
deletes them only when asked, because the bucket is the one place here where a
mistake is not a `git revert`. Both directions read one definition of
"referenced" (`scripts/media-keys.mjs`).

A collection declares its key prefix in `content.toml`:

```toml
[collections.posts]
media = "site/blog"    # defaults to the collection's own name
```

## Not here yet

Hosted editing for people with no checkout (Keystatic's GitHub mode, Tina
Cloud) needs the media pipeline to stop assuming a local staging directory. It
is on the roadmap, not half-built here.

## What an agent should do with this

- Editing prose directly in `content/` or `press/` is fine and often faster.
  The editor is for the person, not for you.
- If a field is missing from the editor, fix `content.toml` and re-derive.
  Never patch the generated configs.
- Frontmatter written by the editor is normal YAML: wrapped scalars, quoted
  colons, block sequences. `scripts/frontmatter.mjs` reads those. A line-wise
  `key: value` parser silently loses a wrapped summary and turns a list into an
  empty string.

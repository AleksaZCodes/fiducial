# fiducial:cms — Content Editor Skill

This product has the `cms` capability: a content editor over the repository's
own files, local (Keystatic, Tina) or hosted (Pages CMS).

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

## Three editors, one set of files

`[cms] editor` in `fiducial.toml` picks one:

| `editor` | Use it when | Notes |
|---|---|---|
| `keystatic` (default) | Writers who edit forms: collections, fields, a settings page | Real image picker, relationship pickers, slug field. Reads and writes the files directly |
| `tina` | Editing on the page itself, visual editing, a hosted editor later | Local mode needs no service; hosting it is a Tina Cloud or self-hosted dependency |
| `pages` | People with no checkout and nothing to run: a hosted editor that commits to GitHub | Reads `.pages.yml` at the repository root; sign in at pagescms.org with GitHub. Needs no `apps/cms` install |

All three configs are derived from one model every time, and all edit the same
files, so switching is `editor = "tina"`, `fid derive`, `pnpm install` — never a
content migration. Honest differences, because Tina and Pages CMS store
different values: a picture is a text field holding the media **key**, and a
relation is the entry's slug as text.

### Opting out of the local workspace

`fid add capability cms` installs the Keystatic workspace under `apps/cms/`,
because Keystatic is the default. With `editor = "pages"` nothing runs it, so
set the editor and **delete `apps/cms/`**: `fid derive` then writes only
`.pages.yml`, `fid doctor` does not ask for the folder back, and `fid upgrade`
does not restore it. Switching to Keystatic or Tina later is `editor = "..."`,
`fid upgrade` (which brings the workspace back) and `fid derive`.

### Pages CMS

`.pages.yml` is held to Pages CMS's own strict schema by
`scripts/verify-cms-configs.sh` in this platform's CI, because Pages CMS ignores a
malformed config without saying so. Three things it cannot do, and so does not:

- **A bare media key.** Its image field stores a public path (`/images/x.png`),
  never `site/x.png`. Set `[cms] public_base = "/images"` and
  `media_input = "public/images"` (the repository directory it commits uploads
  to) and the picture becomes a picker; without both it stays a text field.
- **A relation as a picker.** Its `reference` stores the file name *with* its
  extension (`ada.md`), not the slug the content layer reads, so a relation is a
  text field. Partial dates ("2026-11") are text for the same kind of reason.
- **One file name in every language.** Each locale is its own collection; the
  editor asks for the file name when an entry is created, and keeping it the
  same across languages is on the person.

## The config is derived — do not edit it

`apps/cms/keystatic.config.ts`, `apps/cms/tina/config.ts`, `.pages.yml` and
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
| `body = "rich"` | rich-text editor, written as plain Markdown `.md` |
| `body = "none"` | no editor; the entry is still a frontmatter-only `.md` |

Tables are off in the body editor: Keystatic writes them in Markdoc's own
syntax, which a Markdown renderer prints as text. An image dropped into a body
is staged under the collection's media prefix, like a picture field's.

`[cms.selects]`, `[cms.hints]` and `[cms.relations]` refine a field: a fixed
list of choices, help text, and a searchable pick from another collection (a
field named `related<Collection>` points at that collection with no
declaration). Keys are quoted, `"collection.field" = …`.

- A select on a `string[]` field is a multi-select.
- A select among numbers (`[1, 2, 3]`) is a number field that names the
  choices, because a select stores text and the schema says number.
- **A value already in the content that is not one of the choices fails the
  derive.** The editor would show the default instead and write it over the
  value on the next save.

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
name.

An editor saves a JSON file *from its form*, so whatever the form cannot hold is
deleted on the first save. The derive therefore refuses, naming the file and
key: a key that is not letters, digits, `_` and `-` (so `"nav.home"`, which
should be nested), a `null`, and a list whose records differ in shape.

What stays in code: `fiducial.toml` and `design-system.md`. They carry comments
and structure a form would flatten, and they change how the product is built
rather than what it says.

## Media

Images dropped into the editor are staged in `media/` and stored as **keys**
(`<prefix>/<slug>/<file>`, e.g. `posts/hello/cover.png`), never files in git.
Set `[cms] public_base = "/images"` only if the site stores URL paths instead
of bare keys; the stored value is then `/images/posts/hello/cover.png`.

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

A picture inside a list of records (a gallery) counts as referenced too.

`[cms] media_field = "text"` edits a picture as its stored text instead of
through the picker. Use it when a value carries more than a key (a focus point
as `?focus=30,40`, a URL): the picker re-homes an image under the entry's own
folder when it saves and keeps no query, so it would rewrite every such value.

A collection declares its key prefix in `content.toml`:

```toml
[collections.posts]
media = "site/blog"    # defaults to the collection's own name
```

## Upgrading from Decap (0.14 and earlier)

`fid upgrade` converts `settings.toml` to `settings.json` (keeping the old file
as `settings.toml.bak`, since JSON cannot hold its comments) and removes
`tools/cms/`'s editor files. Delete the derived `tools/cms/config.yml` by hand,
add `editor = "keystatic"` to `[cms]` if you like it stated, and run
`pnpm install`. Decap-only keys (`media_prefix`, `[cms.images]` crops) are
ignored.

## Hosted Keystatic (GitHub storage)

`[cms] storage = "github"` and `repo = "owner/name"` make Keystatic commit
through the GitHub API instead of writing this working tree. The derived config
is type-checked against the real `@keystatic/core` in this platform's CI. What
the config cannot do for you:

- **Run the editor.** `apps/cms` is a Next app; for people with no checkout it
  has to be deployed somewhere, and the deploy is not wired here.
- **Authenticate.** Keystatic's GitHub mode needs a GitHub App and four values
  in the editor's environment (`KEYSTATIC_GITHUB_CLIENT_ID`,
  `KEYSTATIC_GITHUB_CLIENT_SECRET`, `KEYSTATIC_SECRET`,
  `NEXT_PUBLIC_KEYSTATIC_GITHUB_APP_SLUG`). Keystatic's own docs say how to
  create the app.
- **Keep media where it was.** Uploads are committed under `media_dir`, so that
  directory must be tracked, not gitignored; `pnpm media:push` still syncs it to
  the bucket, and the entries still store keys.

Tina Cloud is configured in Tina's own dashboard, not here, and
`storage = "github"` with `editor = "tina"` is refused. Pages CMS is already
hosted and needs none of this.

## What an agent should do with this

- Editing prose directly in `content/` or `press/` is fine and often faster.
  The editor is for the person, not for you.
- If a field is missing from the editor, fix `content.toml` and re-derive.
  Never patch the generated configs.
- Frontmatter written by the editor is normal YAML: wrapped scalars, quoted
  colons, block sequences. `scripts/frontmatter.mjs` reads those. A line-wise
  `key: value` parser silently loses a wrapped summary and turns a list into an
  empty string.

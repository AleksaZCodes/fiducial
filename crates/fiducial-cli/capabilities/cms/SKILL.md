# fiducial:cms — Content Editor Skill

This product has the `cms` capability: a **local** content editor over the
repository's own files.

`pnpm cms` serves Decap CMS at `http://localhost:8080` against the working
tree. Every save writes markdown into `content/` and `press/`, re-derives, and
stops. You review the diff and commit it like any other change.

## What this is, in platform terms

The design spec's §10 says the workbench is **a view, never a database**: the
repository holds the data, and the workbench reads it and writes back through
ordinary commits. `fid dash` is v0 of that, read-only. This is the **authoring**
slice — v2's first piece, for the one kind of fact a person edits most often
and least enjoys editing in a text editor: prose.

It holds no store of its own. It has no login and no identity provider. It is
not deployed, and there is no `/admin` route on the live site, because the way
in is to have the repository checked out. That is not a missing feature; a
hosted editor would need auth, a session, a backend and a second copy of the
content, and the second copy is the thing this platform exists to delete.

## The config is derived — do not edit it

`tools/cms/config.yml` is written by `scripts/derive-cms.mjs` from:

- `content.toml` — every collection, its schema and whether it has a body
- `press/<locale>/` — boilerplate, facts, and the stories folder, if present
- `[i18n] locales` / `default` — the languages shown side by side

`fid derive --check` fails when it is stale, so the editor cannot drift from
the content model. **Change the declaration, not the config.** A hand-edited
config is a second declaration of the model, and it is the copy that goes
wrong: an editor offering a field the schema lacks writes entries `fid derive`
then rejects, and an editor missing a field leaves it empty on everything made
through it — which shows up as a blank page long after the editing session.

### Widgets follow schema types

| `content.toml` type | In the editor |
|---|---|
| `string` | text field |
| `date` | date picker, UTC, no time — duplicated across locales |
| `number` | number field |
| `boolean` | toggle |
| `string[]` | list |
| `media` | image picker; stores a **key**, never a URL |
| `body = "rich"` | markdown editor |

## Localization

`content/<collection>/<locale>/<slug>.md` is exactly Decap's
`multiple_folders` structure, so collections are edited in every language side
by side and a missing translation is visible while you write rather than at
the next build.

The press room is the exception: `press/<locale>/stories/` puts the locale
*above* the collection, which is the opposite nesting, so each language's
stories are their own collection in the editor. The alternative was rearranging
the press layout to suit the editor, which is the tail wagging the dog.

## Media

Images dropped into the editor are staged in `media/` and stored as **keys**
(`site/uploads/photo.jpg`), never URLs and never files in git. `pnpm
media:push` uploads what changed to the product's bucket, comparing hashes
against a machine-local record; `--all` ignores that record.

Anything already in the bucket but not staged locally shows no thumbnail in
the editor. The value is still correct, and the page renders it.

## What an agent should do with this

- Editing prose directly in `content/` or `press/` is fine and often faster.
  The editor is for the person, not for you.
- If a field is missing from the editor, fix `content.toml` and re-derive.
  Never patch `tools/cms/config.yml`.
- Frontmatter written by the editor is normal YAML: wrapped scalars and block
  sequences. `scripts/frontmatter.mjs` reads those. A line-wise `key: value`
  parser will silently lose a wrapped summary and turn a list into an empty
  string, which is how this was first found.

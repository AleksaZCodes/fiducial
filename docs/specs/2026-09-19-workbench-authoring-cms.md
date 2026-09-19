# Workbench authoring — the `cms` capability

**Date:** 2026-09-19
**Status:** Accepted
**Implements:** part of §10 of `2026-09-06-fiducial-design.md` (workbench v2, authoring)
**Follows:** `2026-09-11-workbench-v0-fid-dash.md`

§10 sets the workbench out in three versions: v0 `fid dash` (read-only), v1
assets and multi-repo, v2 **authoring — edit facts, trigger derivations, open
agent tasks; every action a commit.** This is the first slice of v2, for the
kind of fact that is edited most often and enjoyed least in a text editor:
prose.

---

## 1. Authoring arrived before v1, and not as a Tauri app

§10 delivers the workbench as a Tauri app. This slice is a local web tool
instead, and that is a deliberate reordering rather than a change of plan.

**Why now.** The pressure was real and immediate: a product with two locales, a
blog, a press room and a gallery has four content shapes a person edits weekly.
Editing them as markdown by hand is the part of the system that felt like
maintenance, which §10 exists to remove.

**Why not Tauri yet.** A desktop shell buys filesystem and git access. A local
tool already has both, because it runs from the checkout. What a Tauri app adds
— one window over several repositories, the 3D viewer, spec sheets — belongs to
v1 and is not what editing a post needs. Building the editor inside a shell
that does not exist yet would have meant building the shell first.

**What this commits to.** The editing surface is a browser page served locally.
If v1 ships as Tauri it embeds this, because the contract underneath it is
files, not a UI.

## 2. The view still owns no store

Unchanged from v0, and the reason the capability is thin: the editor reads the
working tree through `decap-server` and writes markdown back into it. There is
no database, no draft store, no sync step. A save is a file change you review
with `git diff` and commit.

**Consequence accepted.** No collaborative editing, no drafts shared between
machines, no publish workflow. Anyone who needs those is asking for a store,
and the store is the thing §10 rules out.

## 3. The config is derived, not written

Decap needs a config listing every collection, field and widget. That is what
`content.toml` already declares and what `press/` already lays out.

**A hand-written config would be a second declaration of the content model —
and the copy that drifts silently**, because nothing renders from it. The two
failures are asymmetric and both bad: an editor offering a field the schema
does not have writes entries `fid derive` then rejects, and an editor missing a
field leaves it empty on everything created through it, which surfaces as a
blank page much later.

So `scripts/derive-cms.mjs` emits the config from the declarations, `fid derive
--check` holds them together, and the editor follows a schema change on the
next derive. This is §3.9 applied to a tool rather than to an artifact.

## 4. The editor writes YAML a hand-rolled parser could not read

The first entry saved through it came back as a wrapped scalar and a block
sequence — ordinary YAML, and unreadable by the line-wise `key: value` reader
the derivers had. The summary lost everything after its first line and the tag
list parsed as an empty string, which failed the schema check naming a field
the author never touched.

**The fix is the parser, not the editor.** `scripts/frontmatter.mjs` now reads
the subset a YAML writer emits: quoted and plain scalars, folded scalars, block
and flow sequences. It reports what it cannot read rather than skipping it,
because silence is what made the first version look correct.

**This is why `press` now requires `content`.** Both read entries, so both need
that reader, and two copies of it would be two things to fix next time. The
dependency is also honest: a press room is the content layer with a fixed
shape — boilerplate and facts are singletons, stories are a collection.

## 5. Media stays keys

Uploads are staged in the gitignored `media/` and stored as keys, never URLs
and never files in git. `pnpm media:push` sends what changed to the product's
bucket.

**The cost, stated.** An asset already in the bucket but not staged locally
shows no thumbnail in the editor. The stored value is still right and the page
renders it. Fixing that would mean either committing binaries or giving the
editor bucket credentials, and neither is worth a thumbnail.

## 6. What is deliberately absent

- **No hosted `/admin`.** Nothing is deployed. A hosted editor needs auth, a
  session and a backend, and it would be a second way into the content.
- **No editorial workflow.** Git branches and PRs are the review mechanism this
  platform already has.
- **No agent tasks yet.** §10's v2 also lists "open agent tasks". That belongs
  with `fid dash --json`, which already serialises the state an agent needs.

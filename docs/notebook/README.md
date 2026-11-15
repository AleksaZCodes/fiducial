# Notebook

The notes *about* Fiducial: where it is going, what it has shipped, the ideas
and questions that are not decisions yet. Decisions themselves are specs
(`docs/specs/`), and the build order is `ROADMAP.md`; this is the layer before
and around those.

## One home per kind of fact

| Kind | Home | Why |
|---|---|---|
| Vision, ideas, sparks, open questions, progress | **here** | They are about the product, change with it, and an agent working in this repository needs them without a second tool |
| Due dates, status, the next action, reviews | **the planning tool** (Notion today) | Planning spans more than this repository |
| Decisions | `docs/specs/` | Versioned with the code they decide |
| What ships when | `ROADMAP.md`, `SHIPPED.md` | Checked by `fid docs` |

The planning tool links here; nothing here links into it, because it is
private. Edit the substance here; edit dates and status there.

## Files

- [`vision.md`](vision.md): what Fiducial is for, and the principles it holds to
- [`ideas.md`](ideas.md): sparks not yet promoted to a spec or a task
- [`questions.md`](questions.md): open questions and current thinking
- [`progress.md`](progress.md): what each release moved, against the open tasks

Last synced: 2026-11-15. The sync is by hand until
`fid notebook` exists (see `progress.md`, Next).

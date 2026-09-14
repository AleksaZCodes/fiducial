# Harvesting an existing codebase

> Getting the good parts out of something you already built, without dragging the
> rest of it along.

---

## The problem

You have built things before. A web app, a prototype, a monorepo that got away
from you. Somewhere in there is work that took real effort and came out right:
the business rules with all their edge cases, the theme you tuned for a week, the
landing page that actually converted, the conventions you arrived at after
getting it wrong twice.

Starting the next product means either rebuilding that, or copying it. Both are
bad, in different ways:

- **Rebuilding** throws away the edge cases — which is the whole value, because
  the happy path was never the hard part.
- **Copying** brings the assumptions with it. Now two products share code that
  belongs to neither, and keeping them in sync is a job nobody signed up for.

[`MISSION.md`](../../MISSION.md) names the first one as an anti-goal: *we do not
rebuild what already works.* The second is the duplication the whole platform
exists to delete. Harvesting is the third option.

## The shape of the answer

Two halves, split along a line that matters:

| Half | Done by | Because |
|---|---|---|
| **Survey** — walk, classify, measure, detect, stage | `fid harvest` | Deterministic, boring, fast. An agent doing it by hand burns its context on `ls` output. |
| **Extraction** — decide what is worth keeping, and generalize it | the `/fiducial:harvest` skill | Judgment. Whether a function is a business rule or incidental framing is not a heuristic. |

And one rule holding it together:

> **`harvest/` is a staging area. It is never the product.**

Nothing is wired in. Nothing is overwritten. The donor stays reference material
until a person or an agent has read it and decided.

## Step 1 — survey

```sh
cd my-product
fid harvest ~/dev/old-web-app --name oldapp
```

```
✦ fid harvest — /home/you/dev/old-web-app

  Donor stack
    · Cloudflare Workers
    · Static site

  Inventory
    logic         6 files     3345 lines
    ui            4 files      537 lines
    theme         8 files      545 lines
    art          22 files        1 lines
    principle     4 files      319 lines
    ops           4 files      421 lines

  Staged   27 of 48 files into
           harvest/oldapp/assets/

  Wrote    harvest/oldapp/harvest.toml   (machine-readable inventory)
           harvest/oldapp/SURVEY.md      (read this first)

  ⚠ Nothing was wired into this product.
```

You get three things:

| Path | Is |
|---|---|
| `harvest/oldapp/SURVEY.md` | The human survey. Read this first. |
| `harvest/oldapp/harvest.toml` | The machine-readable inventory. What an agent reads. |
| `harvest/oldapp/assets/` | Readable copies of the donor's text files. |

### What the classifier is doing

Every file is sorted into one of eight kinds:

| Kind | Is | Typical value |
|---|---|---|
| `theme` | tokens, CSS custom properties, Tailwind config | **highest** — declarative, no dependencies |
| `principle` | decision records, conventions, READMEs | high — encoded judgment, free to keep |
| `contract` | schemas, migrations, shared types | high — usually improvable |
| `logic` | rules with no framework import | highest value, highest care |
| `ui` | components, pages, markup | medium — generalize, don't copy |
| `ops` | scripts, workflows, deploy config | medium — **check for secrets** |
| `test` | the executable statement of what logic does | port it *with* the logic |
| `art` | images, fonts, media | **check licensing first** |

The classification is a heuristic, and the survey says so: every row carries a
`reason` column stating the evidence. A wrong guess is visible and correctable
rather than silently authoritative.

`--json` gives the same facts for tooling:

```sh
fid harvest ../donor --json | jq '.summary'
```

## Step 2 — extract

```
/fiducial:harvest oldapp
```

The skill reads the survey and works kind by kind, in the order the survey
suggests — lowest risk first. For each kind it presents what it found, what is
worth lifting, **what is not worth lifting and why**, and what adopting it would
cost. Then it waits.

That last part is the point. A harvest where the agent says "these four things
are worth taking and these nine are not" is a better outcome than one where
everything gets imported.

### Why the order is what it is

Not by size, and not by how interesting the code looks:

1. **theme** — declarative, dependency-free, and carries what took longest to
   get right. Nearly always worth it.
2. **principle** — costs nothing, and prevents re-litigating decisions someone
   already made carefully.
3. **contract** — a donor usually has two hand-written copies of a shape. Port
   it once and derive the other, which is an improvement on what they had.
4. **logic** — the value is in the edge cases, and the edge cases live in the
   tests. Port the tests or port nothing.
5. **ui** — keep the structure, drop the product's nouns.
6. **ops** — often reusable verbatim, which is exactly the danger. Read every
   line for secrets and account IDs.
7. **art** — licensing before anything else.

## Worked example: reusing a landing page

The commonest real case. You have a site whose design you like and want the next
product's landing page to feel like it.

```sh
fid harvest ~/dev/old-site --name oldsite
```

What actually transfers:

- **The token set.** Colors, type scale, spacing rhythm, radii. This is most of
  the "feel", and it is pure data.
- **The section order and density.** How the hero resolves, how much proof comes
  before the first ask, where things breathe.
- **The structural markup.** A hero with an eyebrow, a headline, a subhead and
  two actions is a shape. The shape transfers.

What does not:

- **The copy.** It is about a different product.
- **The art.** Usually product-specific, and always a licensing question.
- **Anything wired to the donor's data layer.** That is a page, not a component.

The skill will tell you which of your donor's CSS is a real token system and
which is the pile of one-off values that drifted away from it — and it will
propose lifting the system while reporting the drift, rather than importing both.

## What harvest deliberately does not do

**It does not copy files into your source tree.** A tool that imported directly
would produce exactly what this platform exists to prevent.

**It does not decide.** The classifier reports evidence and the skill presents
options. Both are designed so a wrong call is visible.

**It does not read `node_modules`, `target`, `dist`, `.git` or twenty other
build directories.** Those are megabytes that classify as nothing.

**It does not stage large binaries.** Anything over 512 KB is inventoried but
left where it is — the point of staging is to give an agent something to read.

## Safety

- Harvest refuses to run when the destination is inside the source, because the
  walk would consume its own output.
- Lockfiles, `.env` files and logs are never inventoried or staged.
- `ops` files are the highest-risk category and the skill is explicit about it:
  assume secrets are present until you have checked, never copy one across, and
  never print one into a conversation.

## See also

- [`crates/fiducial-cli/README.md`](../../crates/fiducial-cli/README.md) — every `fid` command
- [`MISSION.md`](../../MISSION.md) — why "do not rebuild what works" is a rule
- `fid harvest --help` — the full flag reference

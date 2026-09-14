# Harvest catalogue — Ring of Pursuit

**Surveyed:** 2026-09-14 · **Donor:** `/home/aleksa/dev/ring-of-pursuit`
**Method:** `fid harvest` + `/fiducial:harvest` ([how](../guides/harvesting.md))

> **Status: catalogued, not extracted.** This is an inventory with a recipe per
> item. Nothing here has been lifted into the platform except where the
> **Decision** column says it has. That restraint is deliberate — see
> [Why most of this stays put](#why-most-of-this-stays-put).

---

## What is in the donor

Ring of Pursuit is a Turborepo monorepo: seven apps, eight packages, four
domain modules, on Supabase + Cloudflare.

| Kind | Files | Lines | Notes |
|---|---:|---:|---|
| ui | 287 | 29,241 | `packages/ui` (70 files) is a full shadcn-style registry with Storybook |
| principle | 425 | 43,770 | Unusually high — the donor documents its decisions heavily |
| logic | 220 | 16,887 | `modules/game-engine` (118 files) is the bulk |
| test | 77 | 9,741 | Vitest + Playwright |
| contract | 67 | 9,514 | Supabase migrations + declarative schemas |
| ops | 78 | 6,772 | 12 deploy/env scripts, GitHub workflows |
| theme | 10 | 757 | `packages/ui/src/styles/tokens.css` is the real one |
| art | 35 | 0 | binary |

> The first survey reported 61,560 lines of "logic" and 142,764 of "contract".
> Both were wrong: `.vitepress/cache`, `.wrangler/tmp` and `supabase/.temp` were
> being walked, and a 13,206-line VitePress dependency chunk ranked as the
> donor's single most valuable business logic. Fixed in `fid harvest`; the
> numbers above are the corrected run. **Running the tool on a real monorepo
> found a real defect in the tool** — which is the argument for doing this on
> something big rather than a fixture.

---

## The catalogue

Ordered by value per unit of risk. **Decision** is the recommendation; nothing
marked *Catalogued* has been moved.

### 1 · `OfflineQueue` — durability ⭐ EXTRACT

| | |
|---|---|
| **Source** | `packages/core/src/offline-queue/index.ts` (104 lines + 82 of tests) |
| **Target** | `@fiducial/headless` — `queue.ts` |
| **Decision** | **Extract.** This is the clearest second use in the donor. |

`@fiducial/headless` already has `OfflineQueue<T>`, and its doc comment already
credits this donor: *"Key invariant (from Ring of Pursuit): actions replay at
their original `enqueuedAt` timestamp, not at sync time."* The invariant was
lifted; the **durability was not**.

What the platform's version is missing:

| Donor has | Platform has | Worth lifting |
|---|---|---|
| IndexedDB persistence (via `idb`) | in-memory array | **yes** — an in-memory offline queue loses the queue on the reload that offline causes |
| `DEFAULT_BACKOFF_SCHEDULE_MS = [1000, 2000, 4000, 8000, 16000]` (~31 s) | `maxRetries: 3`, no delays | **yes** — a retry count without a schedule is not a backoff |
| `countPendingActions(type)` for an "N pending sync" indicator | `size` | yes, as a typed variant |

**Recipe.** Keep the platform's generic `OfflineQueue<T>` API. Add a
`StorageAdapter` interface with two implementations — in-memory (the current
behaviour, and what tests use) and IndexedDB. Port the backoff schedule as a
declared constant with the donor's rationale in the comment. Do **not** port
`DB_NAME = "ring-of-pursuit-offline-queue"`; the store name is a product fact.

**Why this one is unambiguous:** MISSION's rule is that a capability is
generalized when a *second* product needs it. The platform already declared it
needed this. It just stopped half way.

---

### 2 · Design tokens ⭐ HIGH VALUE

| | |
|---|---|
| **Source** | `packages/ui/src/styles/tokens.css` (236 lines) |
| **Target** | `@fiducial/tokens` |
| **Decision** | **Catalogued** — extract when a product needs a richer palette than the platform ships. |

The donor's token set is materially more complete than `@fiducial/tokens`:
semantic colour roles, a full type scale, elevation, and a dark theme that is
actually a second declaration rather than an inversion.

**Recipe.** Diff `tokens.css` against `@fiducial/tokens`. Lift the *structure*
(which roles exist, how dark mode is declared) rather than the values — the
values are Ring of Pursuit's brand. Ignore `apps/*/globals.css`; those are
per-app drift from this file and lifting them would import the drift.

**Watch:** `apps/marketing/src/app/globals.css` is 331 lines against the
canonical 236. That gap is the donor's own drift, and it is a finding about the
donor, not material to lift.

---

### 3 · Timezone-aware datetime ⭐ HIGH VALUE

| | |
|---|---|
| **Source** | `apps/marketing/src/lib/i18n/datetime.ts` (159), `packages/i18n/src/timezone.ts` |
| **Target** | `@fiducial/headless` or a new `@fiducial/temporal` |
| **Decision** | **Catalogued** — extract on second use. |

Solves a problem every product with scheduled events hits and nearly every one
gets wrong: translating between the naive `<input type="datetime-local">` value
an admin sees, the absolute UTC instant the database stores, and the wall-clock
string a reader needs — in a named IANA zone.

It depends on **nothing but native `Intl.DateTimeFormat`**, so it runs on the
server, the client and inside Server Actions. That makes it near-free to lift.

**Recipe.** Lift `timezone.ts` and the pure half of `datetime.ts` verbatim in
shape, renaming the domain nouns (`events.date` → a caller-supplied field). It
ships with tests (`timezone.test.ts`) — port those *first*.

---

### 4 · Supabase client factories

| | |
|---|---|
| **Source** | `packages/core/src/supabase/{browser,server}-client.ts`, `types.ts` (99 lines total) |
| **Target** | a `supabase` capability, if one is ever added |
| **Decision** | **Catalogued.** Do not extract yet — no second product needs it. |

Notable for being written *already generalized*. The donor's own comment: *"This
module has no idea what a Player or an Organizer is — that belongs entirely to
`modules/accounts`. See `AGENTS.md §2`: packages/ are technical adapters, not
domain logic."*

That discipline is the reason this would be a ten-minute lift. It is catalogued
rather than extracted because the platform has **no Supabase capability at all**,
and adding one speculatively is the anti-goal MISSION names: *the platform must
never become the project.*

---

### 5 · Geodesic point-in-shape geometry

| | |
|---|---|
| **Source** | `modules/game-engine/src/marker/geometry.ts` (822 lines) |
| **Target** | `fiducial-geometry` — **as a separate module**, not merged |
| **Decision** | **Catalogued.** Genuinely reusable, but read the warning. |

Hand-rolled point-in-polygon, circle/rectangle/regular-polygon → GeoJSON
conversion, and random-point-in-shape sampling, with **no Turf.js dependency** —
deliberately, per the donor's research notes. That makes it portable in a way
most geo code is not, and it is a credible candidate for `no_std` Rust.

**The warning, and it is the important part:** this uses an *equirectangular
flat-plane approximation*, accurate only at single-terrain scale. The donor
documents that explicitly and ties it to a GPS accuracy threshold it already
assumes. Lift the code without lifting that constraint and you get silently
wrong answers at continental scale.

**Recipe.** If lifted, it goes in its own `geodesic` module and carries the
approximation's validity range as a **declared fact**, per principle 1 — *a
physical fact is never a bare number; it carries its unit and its tolerance.*
Do not merge it into `fiducial-geometry`'s existing primitives: those are
Euclidean millimetres for enclosures, and conflating the two coordinate systems
is precisely the drift the platform exists to prevent.

---

### 6 · Map layer components

| | |
|---|---|
| **Source** | `packages/maps/src` — `BaseMap` (207), `MarkerLayer` (132), `PlayerLayer` (119), `TrackLayer` (115), `TerrainLayer` (108), `MapControls` (94) |
| **Target** | a `maps` capability |
| **Decision** | **Catalogued.** Lift the *shape*, not the files. |

A clean Mapbox GL layer decomposition. `BaseMap` + `MapControls` + `lib/bounds.ts`
+ `lib/color.ts` are genuinely product-agnostic. `PlayerLayer`, `MarkerLayer` and
`TerrainLayer` are Ring of Pursuit's domain wearing a component's clothes.

**Recipe.** Take `BaseMap`, `MapControls`, `bounds.ts`, `style.ts`, `color.ts`.
Leave the three domain layers — but keep their *pattern* (a layer is a
declarative component over a shared map instance) as the documented convention.

---

### 7 · Ops scripts ⚠️ READ EVERY LINE

| | |
|---|---|
| **Source** | `scripts/` — 12 files: `sync-env.mjs`, `push-supabase-config.mjs`, `setup-cloudflare.sh`, `setup-github-secrets.sh`, `setup-vercel-env.sh`, `deploy-*.mjs`, `check-migrations.mjs`, `status.mjs` |
| **Decision** | **Catalogued as patterns only. Do not copy.** |

The most reusable-looking and most dangerous category. These encode a real
answer to "how does one person keep env vars consistent across Vercel,
Cloudflare, Supabase and GitHub without a secrets manager" — which is a genuine
problem the platform has not solved.

**But:** assume every one of them contains account IDs, project refs and
possibly tokens until proven otherwise. `fid harvest` never stages `.env`, but
these scripts *read* it and may hardcode what it holds.

**Recipe.** Read them for the *pattern* — one declared source of truth for env,
fanned out to each provider — and re-implement. Do not copy a line. If this ever
becomes a capability, the declaration belongs in `fiducial.toml` and the fan-out
is a `fid derive` executor.

Two are more directly liftable: `check-migrations.mjs` (a drift check between
declarative schemas and applied migrations — the same shape as
`fid derive --check`) and `status.mjs` (which `fid dash` already supersedes).

---

### 8 · The decision-record habit ⭐ ALREADY PAID OFF

| | |
|---|---|
| **Source** | `docs/briefs/`, `docs/archive/specs/`, `ARCHITECTURE.md`, `AGENTS.md` (460 lines) |
| **Decision** | **Extracted as a practice, and it already was.** |

425 principle files, 43,770 lines — the largest category in the donor. Most is
stale status reporting. But the habit underneath is the reason this catalogue
could be written at all: nearly every non-obvious decision in the donor carries
an inline comment saying *why*, pointing at the brief that decided it.

That is what made it possible to tell a deliberate approximation from a bug, and
a generalized adapter from an accident. **The platform already inherited this**
as `docs/specs/` and principle 1b. Recording it here because the value is easy
to underrate: the donor's documentation is what made the donor harvestable.

---

## Explicitly NOT worth lifting

As valuable as the list above, and shorter to act on.

| Item | Why not |
|---|---|
| `modules/game-engine` (118 files, most of the logic) | Encodes one game's rules. Clean, well-tested, and completely specific. |
| `apps/round-runtime/round-server.ts` (1,522 lines) | A Durable Object for this game's live round. The *pattern* is in `@fiducial/realtime` already. |
| `packages/ui` components (70 files) | The platform's registries already cover the primitives, and the donor's are Base UI + shadcn like the platform's. Duplication, not reuse. **Exception:** `entity-list`, `lifecycle-stepper` and `empty` are genuinely good generic patterns — revisit on demand. |
| Supabase migrations (67 files, 9,514 lines) | One product's schema. The *declarative-schemas-plus-migrations* structure is the transferable idea, and it is a convention, not code. |
| `apps/*/globals.css` | Per-app drift from the canonical token file. Lifting these imports the drift. |
| All 35 art files | Product-specific, and licensing is unverified. |
| `packages/test-fixtures` | Fixtures for this domain. |

---

## Why most of this stays put

Nine catalogued, one extracted. That ratio is the point, not a shortfall.

MISSION names two anti-goals that pull against each other here, and both matter:

> **We do not rebuild what already works.** … **The platform must never become
> the project.** Nothing is added speculatively. A capability enters the platform
> when a real product needs it, and is generalized when a **second** one does.

A harvest that imported everything reusable would satisfy the first and violate
the second — leaving the platform carrying a Supabase adapter, a Mapbox
integration and a geodesic library that no product had asked for, each now
needing maintenance and none validated by a second use.

So the catalogue is the deliverable. Each entry has a recipe, a target, and the
warnings someone would otherwise rediscover. When a second product needs the
Supabase factories, the work is ten minutes and the thinking is already done.

**The one extraction is the one that had already met the bar.** `@fiducial/headless`
had declared it needed this queue and shipped half of it.

---

## Re-running this

```sh
fid harvest ~/dev/ring-of-pursuit --name rop
/fiducial:harvest rop
```

The survey is regenerated, not edited — it is a view over another repository.
This catalogue is the judgment layer on top, and it is the part worth keeping.

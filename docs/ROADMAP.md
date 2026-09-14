# Fiducial — Roadmap

> Captured 2026-09-14 from a founder-level scoping session. This file is the
> **anti-amnesia artifact**: everything that was decided or raised, written down
> once, so no part of it has to be re-derived from memory or a chat log.
>
> `PHASES.md` records what is *built*. This records what is *intended*, and why.
> `fid dash` reads the ⬜ 🟡 ✅ markers below.

---

## The thesis, restated at product scale

The platform's rule is *declare each fact once, derive every artifact from it*.
Phases 0–21 applied that to engineering facts: a pin, a board dimension, a
protocol message.

Everything below applies it to the facts a **product** has: its name, its
locales, its legal entity, its brand, where it deploys. Those are declared in
exactly the same way, and the things downstream of them — favicons, legal pages,
translated copy, OG images, deploy config — become derivations that go stale
loudly instead of silently.

> **Unifying the things you have to do that are otherwise in a thousand places.**

---

## The taxonomy

One overloaded word — "capability" — is split into three, because the three
behave differently. See `docs/specs/2026-09-14-capability-taxonomy.md`.

| Concept | Is | Example |
|---|---|---|
| **Declaration** | typed facts, written once, inert | `[brand]`, `[locales]`, `board.interface.json` |
| **Pipeline** | reads declarations → produces artifacts, gated by `fid derive --check` | brand → favicons, OG images, press kit |
| **Adapter** | a swappable implementation behind a fixed contract | `database = "supabase" \| "d1" \| "neon"` |
| **Capability** | the shipping unit that bundles the above + a skill + guard rules | `fid add i18n` |

**Granularity rule:** a capability is as big as one decision you would actually
make. "I want legal pages" is a decision. "I want a button" is a component.

---

## Order of work

Sequenced by **what unblocks what**, then by what a real product needs on day
one. Two rules drove this ordering:

**1 · Build two concrete cases before generalising.** The taxonomy
(declaration / pipeline / adapter) is not designed abstractly and then filled in.
i18n and brand are built as real capabilities first; the taxonomy is then
extracted from what they actually needed. That is the platform's own second-use
rule applied to its own architecture.

**2 · "Add it later, easily" has a prerequisite.** Every capability today is
compiled into the `fid` binary, so shipping one requires releasing the CLI.
Until capabilities resolve from *outside* the binary, "we can add that when we
need it" is not true. That makes item 25 load-bearing for the whole strategy,
not a mid-list nicety.

### Tier 0 — prove the pattern (two concrete cases)

| # | Item | State | Unlocks |
|---|---|---|---|
| 22 | **i18n — localized by construction** | 🟡 | The first declaration → pipeline → gate, on the hardest case. Legal text depends on it. |
| 23 | **Brand** | ⬜ | The second case, and the widest surface: favicons, app icons, OG images, press kit, social templates, in-theme email. Legal pages depend on it. |

### Tier 1 — make extension real

| # | Item | State | Unlocks |
|---|---|---|---|
| 24 | **Taxonomy made real** | ⬜ | Declarations, pipelines and adapters become first-class, generalised from 22 and 23 rather than guessed. |
| 25 | **External capabilities** | ⬜ | **The keystone.** Capabilities resolve from npm / crates / git instead of the binary. Everything below becomes addable on demand, by anyone, without a platform release. |
| 26 | **`fid capability extract`** | ⬜ | "It works in my product, now lift it." Mechanizes the second-use rule. |

### Tier 2 — everything a real product needs on day one

After these, `fid new` yields a product that is localized, branded, legally
furnished and deployable. That is the kickstart.

| # | Item | State | Depends on |
|---|---|---|---|
| 27 | **Cloudflare adapter set** | ⬜ | 24 — first real adapters: D1, R2, Workers, Access, Turnstile, Queues |
| 28 | **Legal & compliance** | ⬜ | 22 + 23 — legal text is localized copy about a declared entity |

### Tier 3 — velocity and correctness

| # | Item | State | Why not earlier |
|---|---|---|---|
| 29 | **Context sync** | ⬜ | Improves every later phase, but nothing is blocked on it |
| 30 | **Fast path** | ⬜ | Matters once there is production to hotfix |
| 31 | **Claude Design bridge** | ⬜ | Registry source → design-system previews, as a derivation |
| 31b | **Interactive seeding** | ⬜ | `fid new` asks; or seeds brand, theme, logos and components from an existing Claude Design project |

### Tier 4 — on demand, via item 25

None of these block a product. Each is added when wanted.

| # | Item | State |
|---|---|---|
| 32 | Research & authoring — papers, references, DOIs, templated documents | ⬜ |
| 33 | Demo & showcase — interactive landing-page demo, Storybook, feature toggles | ⬜ |
| 34 | Diagnostics — error tracking as an adapter with a no-op default | ⬜ |
| 35 | Small tools — backlinks, browser-compat banners | ⬜ |

### Out of band — urgent regardless of priority

Not ranked by value, ranked by **risk accruing while we wait**.

| Item | State | Why now |
|---|---|---|
| **CLA** | ⬜ | The repository is public and can accept pull requests. One contribution from a stranger permanently constrains re-licensing. Zero contributions today is the best moment, and it is a small file. |
| `CITATION.cff`, `CONTRIBUTING.md`, `SECURITY.md` | ⬜ | Small, expected, and prerequisites for being cited correctly |
| Zenodo DOI on a tagged release | ⬜ | Needed before the paper, and Fiducial is its own first customer for item 32 |

## 22 · i18n — *localized by construction* 🟡

**The principle.** Proposed as `MISSION.md` **1c**, alongside the existing 1b:

> A user-visible string is a fact. It is declared once and every locale is a
> derivation that must exist. A missing translation is a missing artifact, not a
> fallback — and like any stale artifact, it fails the build. Monolingual is a
> state you pass through before the first commit, not a state you ship.

**Why it goes first.** Dark mode is easy; language is hard. It is normally added
as a refactoring pass, at which point strings have already been missed. Done from
the bottom up it is free; done later it is never quite finished.

**The evidence.** Ring of Pursuit maintains 1377 keys in `en.json` and `sr.json`
with zero key drift — genuine discipline. And exactly one string slipped
through untranslated, invisibly:

```
home.organizer.benefit3.title = "Live dashboard"   ← identical in sr.json
```

The mechanism permits it: `messages[key] ?? key` renders the key itself when a
translation is missing, and `key: string` is untyped so a typo is undetectable.
Discipline caught 1376 of 1377. The argument is not that ROP was careless — it is
that **care is the wrong mechanism**.

| # | Mechanism | Kills |
|---|---|---|
| 1 | Catalog is the declaration; locales are generated | two hand-written copies drifting |
| 2 | Typed keys — a bad key is a compile error | silent key typos |
| 3 | `fid derive --check` fails on a missing translation | the invisible `"Live dashboard"` |
| 4 | No fallback-to-key; dev throws, prod cannot happen | untranslated text reaching users |
| 5 | Typed interpolation — `{count}` requires `count` | `{count}` rendering literally |
| 6 | Hardcoded-string detection | *"you miss strings"* — the actual complaint |

Item 6 is what makes it a **default** rather than a discipline.

**TypeScript first; Rust later, as a derivation.** An earlier draft argued this
catalog should be `no_std` Rust immediately, partly on the grounds of device-side
strings. `fon` is **headless**, so that argument was weak and is withdrawn.

The honest remaining case is error codes crossing the wire — a device reports a
code, and the app renders it in the reader's language from the same declared
keys. Real, but no product needs it yet.

So the JSON catalog is the declaration and TypeScript is the first derivation.
Adding a Rust derivation later is then a *new derivation* rather than a second
declaration — the second-use rule applied to our own design.

**Catalogs are JSON**, one file per locale (`messages/en.json`, `messages/sr.json`).
Decided so a translator can edit them without touching a build system — the
format has to be reachable by someone who is not a developer, which rules out
Rust consts and TOML embedded in `fiducial.toml`.

**`fid new` takes locales up front**, so there is never a monolingual moment.
Default locales: **`sr` and `en`**.

**Hardcoded-string detection fails the build**, with an explicit opt-out marker
for genuine exceptions — a brand name, a code sample. Decided over a warning,
because a warning is a thing you scroll past.

**Harvest from ROP:** `resolve-locale.ts`, `config.ts`, `timezone.ts` + datetime
helpers (native `Intl` only — server, client and edge). **Not** `translate.ts`;
its fallback is the defect.

---

## 26 · Brand

One declaration — legal name, trading name, contact email, domain, palette,
typography, logo — derives:

- favicons, app icons (every platform size)
- OG / Twitter card images
- press kit (logos, palette, boilerplate, screenshots)
- social post templates (Instagram and similar)
- email templates rendered in the product's own theme
- `sitemap.xml`, `robots.txt`, JSON-LD, `security.txt`

Feeds `@fiducial/tokens`, so brand and design system are one source, not two.

---

## 27 · Cloudflare — the default

**Decided:** Cloudflare is the default target, not one option among equals.
Vercel, Supabase, Neon and others remain adapters. See
`docs/specs/2026-09-14-cloudflare-default.md`.

Rationale: it covers database (D1), storage (R2), compute (Workers), auth
(Access), bot protection (Turnstile), queues, AI and networking under one
generous free tier — and it is the only vendor on the list that plausibly
reaches **IoT/LoRa and firmware OTA delivery**, which `fiducial-ota` will need.

The contract is still designed vendor-neutrally. A default is a default, not a
lock-in; principle 6 is not suspended for a vendor we happen to like.

---

## 28 · Legal & compliance

**GDPR compliance is a legal state, not a code state. No tool grants it.**

What is generated: consent record, cookie categories, data export, account
deletion, privacy/terms/cookie/imprint/accessibility pages from the brand and
jurisdiction declarations, bilingual by construction.

What is *not* generated, and ships as a reviewed checklist with reasoning: the
"don't forget" list for legal text, and every decision a human must actually make.

Harvest ROP's `LegalSection[]` pattern — legal text as structured, localized
data with `{email}` substitution, rendered by one component. Legal text is
long-form localized copy, so it falls out of i18n rather than being a second
system.

---

## 29 · Context sync

**The problem, stated by the founder:** code, documentation, agent context and
tooling drift apart, and keeping them together should be the platform's job.

Evidence it is real: `CLAUDE.md` and `AGENTS.md` both carried repository trees
that had gone stale past three crates, *and both carried a disclaimer telling
the reader to run `ls` instead of trusting them.* Phase 18 fixed them by hand and
added a test. **A test that fails after the fact is detection, not sync.**

Target: the parts of agent context that are derivable — the layout tree, the
command list, the capability list, the available skills — are **generated**, so
they cannot drift. The parts that are judgment stay hand-written.

---

## 30 · Fast path

**The problem:** an urgent production fix cannot wait on a ten-minute
end-to-end suite, but skipping CI by hand is how a bad fix ships.

Target: a declared *fast path* — the subset of checks that must never be skipped
(compile, unit tests, the freshness gate) separated from the slow, optional ones
(full e2e, multi-target matrices, wasm-pack). Correct by construction, not by
someone deciding under pressure which checks to bypass.

Constraint: whatever is skipped is **recorded**, and the full suite runs after
the fact. A fast path that hides what it skipped is just a broken CI.

---

## 31 · Open-source bootstrap

One command or skill turns any repository into a properly published
open-source project:

`LICENSE` · `CITATION.cff` · `CLA.md` · `CONTRIBUTING.md` · `SECURITY.md` ·
`CODE_OF_CONDUCT.md` · release + Zenodo DOI wiring · authorship and trademark
boundaries stated.

**Meta-requirement, stated explicitly:** everything built for Fiducial must be
available *to* the tools Fiducial builds. Fiducial is the first consumer of its
own open-source bootstrap — it currently lacks every file in that list.

---

## 32 · Research & authoring

A Fiducial repository is intended to encapsulate **every aspect of building** —
not only code, but recording, writing, and the artifacts a venture, a research
project or a paper actually needs.

Near term:
- Research: sources, references, citation management, DOI resolution
- Authoring: edit Markdown, derive the submittable artifact (IEEE templates and
  similar) — the author never touches LaTeX boilerplate
- Automated DOI for repositories, including Fiducial's own
- The build itself is recorded, because a Fiducial repo already records decisions

Later, and noted so it is not forgotten: **video editing**, and the general goal
of *supercharging one person to work at polymath level across every area of human
endeavour.*

---

## 33 · Demo & showcase

Not simulation in the numerical sense (`fiducial-sim` covers that). **Optional**,
and primarily an *interactive demo section on a landing page* — a real frontend
with real behaviour that a visitor can actually use, with features individually
toggleable.

The secondary benefit is internal: a surface that can be seen and analysed
deterministically, rather than having edge cases discovered in production.
Storybook is part of this and is currently underused.

---

## 31 · Claude Design bridge

Claude Design (`claude.ai/design`) holds design-system projects that sync with a
local component library **incrementally, one component at a time** — never a
wholesale replace. Its format:

| | |
|---|---|
| Unit | a preview HTML file, e.g. `components/button/index.html` |
| Card marker | `<!-- @dsCard group="…" -->` on the **first line** |
| Index | `_ds_manifest.json`, compiled from those markers by the app's self-check |
| Card fields | name, subtitle, viewport (width/height), group |
| Groups | free-form: `Type`, `Colors`, `Spacing`, `Components`, `Brand` |
| Validation | `.render-check.json` — `total / bad / thin / variantsIdentical` |
| Protocol | `list → finalize_plan → write`; paths are locked and shown before any write |

**The design insight:** that preview HTML is a *second representation* of a
component this repository already holds as `packages/ui-react/src/button.tsx`.
Two representations of one thing is the relationship this platform exists to
manage — so the registry source is the **declaration** and the Claude Design
preview is a **derivation**, generated by a pipeline and kept honest by
`fid derive --check`.

Which means "keeping the projects in sync" stops being a sync problem.

**Why consistency across projects already half-works:** the React and Svelte
registries both read the CSS custom properties published by `@fiducial/tokens`.
Adding Claude Design as a third consumer of the same tokens is the natural move,
rather than a fourth place for design values to live. Brand (item 23) feeds the
same tokens, so brand → tokens → every registry → Claude Design is one chain.

**Requires** `/design-login` in the session to authorize the `DesignSync` tool
against the user's claude.ai account.

## 31b · Interactive seeding

**The cherry on top**, and deliberately last: it is polish over machinery that
must exist first.

Two ways into a new product:

**Ask.** `fid new` becomes interactive — product name, locales, brand basics,
adapters, capabilities — instead of scaffolding a stub the author then edits by
hand. Non-interactive flags remain, because CI and agents must still be able to
run it unattended.

**Or seed from a Claude Design project.** Pull an existing design-system project
and derive `[brand]`, the token set, the theme, logos and the component registry
from it — with **strict data structures and stated opinions** about what a proper
shadcn registry and theme are. Opinionated on purpose: the point is that every
product looks like it came from the same studio, and that only holds if the
structure is not negotiable per product.

The intended loop, which is worth stating because it is the actual workflow:

```
fid new  →  components exist as registry source
         →  open the repo in Claude Design
         →  design back and forth there
         →  fid derive keeps both sides honest
```

**Open question — is that loop already integrated?** The `DesignSync` tool
references a `/design-sync` skill for exactly this, incremental and
component-at-a-time. That skill is not present in the current session's skill
list, so either it is gated behind `/design-login` or it is not installed here.
Resolve before building this item: if the round trip already exists, item 31 is
a thin adapter over it rather than a pipeline to write.

## 34 · Diagnostics

Error tracking as an **adapter with a no-op default** — wired in from the first
commit, costing nothing until pointed at a vendor.

Constraint: no self-hosted database for error tracking. The job is outsourced or
it is not done.

---

## 35 · Small tools

Failproof, extensible, customizable, opinionated, working out of the box:

- **Backlinks** — interconnect the sites you have built, for domain authority
- **Browser / API compatibility** — declare required web APIs; derive the banner.
  Graceful ("Chrome recommended") or blocking, per declaration
- Others as they earn their place

---

## Two tensions worth deciding consciously

**1 · "Everything before `fon`" versus the second-use rule.**

`MISSION.md` anti-goal 2: *"The platform must never become the project. Nothing
is added speculatively. A capability enters the platform when a real product
needs it, and is generalized when a second one does."*

The list above is a deliberate, stated exception — *"I can afford to make a
general thing."* Recorded here so it is a **choice** rather than an erosion, and
so the rule is still the rule afterwards.

The risk, named plainly: this list is large enough that "everything before `fon`"
could mean `fon` never gets built, and the mission says shipping products is the
point. Mitigation: each item above should be validated by *some* real consumer,
even a small one, before the next begins.

**2 · Fiducial as its own first customer.**

Several items — open-source bootstrap, research and authoring, context sync,
DOI — have Fiducial itself as the obvious first consumer. That is the healthiest
possible version of dogfooding and it partly resolves tension 1: the platform
is a real product with real needs, so building these is not speculative.

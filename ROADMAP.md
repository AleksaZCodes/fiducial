# Fiducial — Roadmap

> Captured 2026-09-14 from a founder-level scoping session. This file is the
> **anti-amnesia artifact**: everything that was decided or raised, written down
> once, so no part of it has to be re-derived from memory or a chat log.
>
> `SHIPPED.md` records what is *built*. This records what is *intended*, and why.
> `fid dash` reads the ⬜ 🟡 ✅ markers below — so **every item carries one**. An
> unmarked item is invisible to the dashboard, which then reports the roadmap as
> finished; that is exactly what happened to items 4–9 until 2026-09-15.
>
> **Nothing here says "next".** The order of work below plus the markers already
> say it, and `fid dash` derives it. A sentence naming the next item is a second
> declaration of what these markers hold, and it is the copy that goes stale.

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

## How this file relates to `SHIPPED.md`

| File | Holds | Numbering |
|---|---|---|
| `ROADMAP.md` (this) | what is **intended**, and why | items have **names**, no numbers at all |
| `SHIPPED.md` | what was **built**, phase by phase | phases are numbered, in the order they shipped |

**One numbering, and it lives in `SHIPPED.md`.** Items here are named, and their
order is their position under *Order of work* — nothing more. A phase entry in
`SHIPPED.md` names the item it implements, and that sentence is the only link
between the two files.

They were briefly coupled — roadmap items numbered 22–35 to shadow phase numbers
— which is a fact declared twice, and breaks the first time one item spans two
phases or one phase closes two items. The ordinals lingered as `### 1 ·`,
`### 2 ·` even after that was decided, which put a second set of numbers next to
the phase numbers and made "item 3" and "Phase 24" two names for one thing.
Removed 2026-09-15.

`fid dash` reads the first of `ROADMAP.md`, `SHIPPED.md`, `PHASES.md`,
`docs/ROADMAP.md` that exists — one file, so there is no double count.

---

## Order of work

Ordered by **cost of delay**, not by value. An item belongs early when *waiting
makes it more expensive*, which is not the same as wanting it most.

Three kinds of item, and the order follows from the kind:

| Kind | Cost of waiting | Goes |
|---|---|---|
| **Debt-accruing** | grows with every phase you wait | first |
| **Multiplying** | makes every later phase cheaper | second |
| **Terminal** | the same whenever you do it | last, by product value |

### i18n — *localized by construction* ✅

Complete. The runtime (`@fiducial/i18n`), the declaration, the pipeline, the
gate, the hardcoded-string detector and the capability have all shipped, and
`fid new` takes locales up front.

Also: the **second** declaration→pipeline case. `eda` was the first — it already
installs `board/board.interface.json` as a declaration plus `eda.toml` and
`enclosure.toml` as pipelines. That matters for what comes next.

### Capability taxonomy, made real — *multiplying* ✅

Complete. Declarations, pipelines and adapters are first-class in the CLI; see
`docs/specs/2026-09-15-capability-taxonomy-made-real.md`.

**The adapter half shipped as format only** — five contracts, each implementing
`none` and nothing else, with intended vendors recorded as `candidates` that
cannot be selected. Async trait definitions and TypeScript interfaces followed
in *Cross-platform adapter architecture* (shipped). Vendor implementations
follow in *Cloudflare adapter set*.

**Why now and not later:** the second-use rule is satisfied. `eda` and i18n are
two real declaration→pipeline capabilities, so the taxonomy is *generalized from
working cases* rather than designed speculatively. Waiting for a third adds
nothing but delay.

**Why now and not after publishing:** this settles the capability **format**.
Changing a format after third parties have published against it is a breaking
change for other people's work. Format first, publishing second — the order is
not interchangeable.

### External capabilities — *debt-accruing, and the keystone* ✅

Complete. Capabilities resolve from a directory or a git repository instead of
`include_str!` in the `fid` binary; see
`docs/specs/2026-09-15-external-capabilities.md`.

**npm and crates are deferred** rather than built: a registry adds a packaging
format — what a published capability contains, how a version range resolves,
which registry is authoritative — and nothing needs that yet. Design spec §3.3
names git as the source model, and git is the one with a consumer.

**This is the item whose cost of delay is highest and most measurable.** Every
capability built before it exists is one more compiled into the binary that later
has to be migrated out. The debt is linear in the number of capabilities built
while waiting — so each phase spent elsewhere makes this one strictly more
expensive, and nothing else on this list has that property.

It is also the item the whole strategy rests on. "We can add that later, easily"
is only true once shipping a capability does not require releasing the CLI.

### Context sync — *multiplying* ✅

Complete. The derivable parts of agent context are generated by `fid context`
and gated in CI; see `docs/specs/2026-09-15-context-sync.md`.

Every phase after this one adds a crate, a package or a capability, and today
each of those means a hand edit to `CLAUDE.md` and `AGENTS.md` with a test
catching the omission *after the fact*. Doing it here makes every later phase
cheaper and removes a recurring drift risk, rather than paying the tax six more
times first.

### Brand — *terminal, high value* ✅

**Complete for the text/data half.** One declaration — `[brand]`: legal name,
trading name, domain, contact email, two colours — derives a favicon (SVG,
vector, no rasterizer required), `site.webmanifest`, `robots.txt`,
`sitemap.xml` and a `schema.org` JSON-LD record, gated by `fid derive --check`
exactly like every other pipeline.

First capability built entirely on the finished system, which makes it the test
of whether steps 2–4 actually worked. It is: `[brand]` is a declaration exactly
like `[i18n]`, seeded the same way, validated field-by-field the same way, and
`fid-brand` is a fifth executor next to `fid-i18n` and `fid-mesh` with nothing
new invented.

**Raster favicons, OG/Twitter card images, a press kit, social templates and
in-theme email are deferred, not built.** Each needs a rendering step — font
shaping, rasterization — this pipeline does not carry, and none has a consumer
yet. Adding one is a new output name in `pipelines/brand.toml` and a new
branch in `fid-brand`'s match, not a redesign of `[brand]` — the same shape of
extension `fid-mesh` and `fid-i18n` already went through.

Unblocks **legal** and the **Claude Design bridge**. Feeds `@fiducial/tokens`,
so the chain is brand → tokens → every registry → Claude Design, one source
throughout — the token feed itself remains future work.

### Cross-platform adapter architecture — *multiplying, must precede real adapters* ✅

Complete. `crates/fiducial-adapters` defines `Database`, `Storage`, `Email`
and `Diagnostics` as Rust async trait contracts (using `BoxFuture` for
object-safe async without the `async-trait` crate); `packages/adapters` mirrors
each as a TypeScript interface. `None*` no-op implementations on both sides
work end-to-end. `fid add adapters` installs the capability; `fid derive` runs
the `fid-adapters` executor and generates `src/adapters.generated.ts` from
`[adapters]` in `fiducial.toml`, gated by `fid derive --check`.

**Firmware boundary stated and documented:** cloud adapter contracts do not
apply to `no_std` targets. Firmware uses `embedded-hal` / Embassy HAL and
`fiducial-ota` for OTA. Tauri bridges both worlds: this crate in the Rust
backend, `packages/adapters` in the TS frontend. `src/firmware.rs` holds the
full mapping.

**Vendor implementations** — `d1` and `r2` shipped as the first two; Supabase,
Neon/Postgres, S3, Resend, Sentry remain — under *Cloudflare adapter set*.

### Cloudflare adapter set — *terminal, product-critical* 🟡

D1, R2, Workers, Access, Turnstile, Queues, Workers AI. The first real adapters,
and the proof that the adapter contract is vendor-neutral rather than a
Cloudflare-shaped hole.

**Shipped:** `d1` (database), `r2` (storage), `turnstile` (botProtection),
`cloudflare-queues` (queue) and `cloudflare` (deploy) — five of seven. Each
landed on a contract that either already existed (`d1`, `r2`, `deploy`) or
was designed narrowly for it (`botProtection`, `queue`). Specs:
`docs/specs/2026-09-15-cloudflare-adapter-set.md`,
`docs/specs/2026-09-15-turnstile-and-queues.md`,
`docs/specs/2026-09-15-deploy-config-is-derived.md`.

**Workers shipped as `deploy`, and it found the platform's own founding
defect inside the platform.** `wrangler.toml` is now *derived* from
`[adapters]` + `[deploy]`: selecting `database = "d1"` is what puts a
`[[d1_databases]]` block there, under the binding name `D1Database`
actually reads. It had been hand-copied — and the `worker-cloudflare`
template's own commented example bound `MY_DB` where every adapter reads
`DB`, so a product following its scaffold got a Worker that threw on its
first query. One fact, two places, and the copy was wrong.

**Not yet:** Access and Workers AI — and neither stays here. Access folded
into the **Auth** item below, where Supabase shipped as the priority vendor
rather than Access. Workers AI folds into the **AI** item below, reframed as
one candidate implementation of a vendor-neutral contract rather than the
contract's namesake. What remains under this item is therefore nothing
Cloudflare-specific; it closes when those two do.

**Depends on:** cross-platform adapter architecture above.

Low compounding, high product value — correctly placed after the infrastructure
rather than before it.

### Auth — *terminal, product-critical* ✅

Users and authentication — named directly by the founder as a priority,
**Supabase-first**. Scoped 2026-09-15 before building
(`docs/specs/2026-09-15-turnstile-and-queues.md` §Context), then shipped the
same day: `docs/specs/2026-09-15-auth-contract.md`.

- **Full flows, not session-verification-only.** The `Auth` contract covers
  sign-up, sign-in (password + OAuth), sign-out, password reset, and session
  read — the product never touches a vendor SDK directly for auth, matching
  how `database`/`storage` already work.
- **Both session delivery models shipped:** `CookieKeyValueStore` (web-next/
  web-svelte, real PKCE support across the OAuth redirect) and
  `BearerKeyValueStore` (Tauri or a future mobile client — cannot carry PKCE
  state across a redirect without a cookie, so OAuth methods throw clearly
  under it rather than silently losing the verifier; a bearer client does
  OAuth directly against Supabase instead).
- **`auth` is not a field on `AdapterSet`.** Every other contract's real
  vendor is env-scoped; a real `Auth` needs a request-scoped session store.
  `createAuth(env, store)` is a second factory in the same generated file
  rather than a seventh `AdapterSet` field that would corrupt the uniform
  `new {Class}(env)` shape the other six rely on.
- **`supabase` real, TypeScript-only** — same non-structural reason
  `turnstile` is: a plain HTTPS API, but no Rust-reachable consumer yet.
  `clerk`, `auth.js` remain `candidates`.

A real bug surfaced in the process, unrelated to Auth itself: `NoneEmail`
and `NoneDiagnostics` had no explicit constructor, so `new NoneEmail(env)`
in the generated factory failed to typecheck — unnoticed since Phase 27
because nothing had ever run `tsc` against a generated
`adapters.generated.ts`. Fixed, and the gap it exposed is **now closed**:
`pnpm --filter @fiducial/adapters test:generated` derives the factory for
every vendor selection and compiles it, in its own CI job. Deleting the
constructor again fails it with `TS2554`. Spec:
`docs/specs/2026-09-15-generated-code-is-compiled-not-matched.md`.

**Hardened immediately afterward**, and two further defects in the same
day's work found and fixed — `getSession()` verified nothing, and bearer
delivery never worked at all. See **Identity at every level** below; spec:
`docs/specs/2026-09-15-identity-at-every-level.md`.

### Identity at every level — *multiplying* ✅

Auth answers *who*. This answers *may they* — and does it once, for users,
devices and services alike. Spec:
`docs/specs/2026-09-15-identity-at-every-level.md`.

`crates/fiducial-identity` (`no_std`) defines one `Principal` spanning
`User`, `Device`, `Service` and `Anonymous`, **reusing
`fiducial_core::DeviceId`** rather than minting a second device identity.
`can(principal, action, resource, grants)` is the whole authorization rule,
in a crate firmware can run — so a device deciding "is this command from my
owner?" offline reaches the same verdict as the Worker deciding it at the
edge. `Grant` is the user↔device link that had nowhere to live before.

**The rule is a conformance artifact, not two implementations trusted to
agree.** `can()` also exists in TypeScript (`@fiducial/identity`) for the
Worker and the app; an authorization divergence does not look like a bug, it
looks like access. So the decision table is generated from the Rust crate
into `docs/identity/vectors.json` and replayed by both suites, with CI
running the generator without the write flag — the same mechanism
`docs/protocol/vectors.json` uses for the wire format. Verified
adversarially: changing one rule in the TypeScript copy fails 10 tests.

**Also hardened `auth` in the same pass**, after an audit found two real
defects in it shipped hours earlier: `getSession()` verified no signature at
all (a forged cookie was a valid session with any user id), and bearer
delivery silently never worked (its token was written under a storage key
the SDK never reads). Both fixed and pinned by tests that fail if either
regresses.

**Grant storage shipped** (`docs/specs/2026-09-15-grant-storage.md`):
`fid add identity` derives `migrations/0001_grants.sql` from the identity
model — its `CHECK` lists *are* the `Principal`/`Resource`/`Role` variants,
so adding a role and forgetting the migration fails `fid derive --check`
rather than producing a table that silently rejects it. `SqlGrantStore` is a
consumer of the `database` contract, not a ninth contract beside it, so it
works on D1 today and any future vendor free. Tested against real SQLite
(what D1 runs) on the real generated schema.

**Postgres and RLS shipped**
(`docs/specs/2026-09-15-postgres-dialect-and-rls.md`), correcting the above:
the migration was SQLite-only and said so nowhere, so `GLOB '*[^0]*'` — a
constraint that spec was pleased with — is a syntax error on Postgres and a
Supabase product could not apply its own derived schema. The dialect is now
derived from `[adapters] database`, the store renumbers its placeholders
(`?` → `$1`), and `src/identity.generated.ts` carries both facts into
TypeScript so nothing is retyped. `rls = true` adds Postgres policies as
defence in depth — `can()` is still the rule, because it is the one both
languages share and the only one firmware can run.

Three defects, none of which a test that reads generated text can see: the
`GLOB` migration, the store's `?` placeholders, and a policy on `grants`
that reads `grants` and therefore refuses every query it guards
(*"infinite recursion detected in policy"*). `scripts/verify-postgres.sh`
runs the real thing against a real server — 14 checks, in CI on a
`postgres:16` service — and reproduces all three when the fixes are reverted.

**The migrations system shipped 2026-09-15**
(`docs/specs/2026-09-15-schema-migrations.md`): `fid add migrations` derives an
ordered manifest from `migrations/NNNN_slug.sql`, and `Migrator` in
`@fiducial/adapters/migrate` applies it through the `database` contract — so it
works on D1 today and any future vendor free. Ordering is numeric, the ledger
makes re-application a no-op, and a migration edited after it was applied stops
`apply()` entirely rather than compounding a schema nobody has reconciled.
Tested against real SQLite.

**Deliberately not done:** No Postgres `database` adapter either: `supabase`/`neon`/`postgres`
resolve the dialect correctly but remain `candidates`, so such a product
supplies its own `{ query, execute }`. **Device and service token issuance shipped 2026-09-15**
(`docs/specs/2026-09-15-token-issuance.md`): a fixed-layout,
domain-separated ed25519 bearer token, 105 bytes, verified `no_std` on every
device target. Not a JWT — a header that names its own algorithm is how
`alg: none` happens. Expiry is mandatory and verification fails closed without
a clock, the same rule grant expiry follows. The format is pinned by
conformance vectors the TypeScript verifier replays; changing one character of
the domain separator in TypeScript alone fails 23 checks.

**Grant expiry, delegation, audit and caching shipped 2026-09-15**
(`docs/specs/2026-09-15-grant-lifecycle.md`). A grant carries an expiry and a
delegator; `can()` takes the current time, and **fails closed without one** —
a device whose RTC has not synced cannot honour an expiry, and treating "no
clock" as "not expired" would make expiry evaporate in the one environment
least able to notice. A delegated grant is worth what its delegator's
authority is worth *at evaluation time*, so revoking a manager revokes
everything they issued. `explain()` returns the reason, which is the audit
record — reconstructing it afterwards is guesswork, because the grants table
has moved on by the time anyone reads the log. All of it mirrored in
TypeScript and pinned by 8 new conformance scenarios covering verdict,
effective role *and* reason.

**Closed 2026-09-15: `fid add identity` was one opt-in doing two jobs.**
`[identity] storage = "none"` installs the rule without the table, exactly as
sketched below. `can()` is unaffected — it takes grants as an argument and does
not care where they came from, which is what makes `none` a real configuration
rather than a disabled one. The generated TypeScript module still appears and
names its storage kind, so a product importing it gets a clear error rather
than a module-not-found.

Building it needed one mechanism that did not exist: a pipeline's `outputs` are
written by the capability author, who cannot know which of them a given product
wants, so `fid derive --check` demanded a migration the declaration said must
not exist. Outputs a configuration does not produce are now skipped by both the
record and the check, and switching to `none` stops tracking the old migration
and says so rather than deleting a file that may already have been applied.

The original note follows.

**`fid add identity` is one opt-in doing two jobs.** Everything here is
already opt-in — `fid new` generates none of it, the way all twelve
capabilities work, every adapter contract defaults to `none`, and
`[spine] enabled = false`. But installing the capability installs the *rule*
and the *storage* together. A product that wants `can()` and seeds its grants
from config or `MemoryGrantStore` still gets a migration it will never apply
and a generated module it will never import.

The fix that fits the platform is `[identity] storage = "none"`, because
`none` is already this platform's word for "wired in, reported, does nothing"
— see the `NONE` adapter, which is a real implementation rather than a
placeholder. Splitting identity into two capabilities is the alternative, and
is more surface for the same result.

Not built, deliberately: no product has yet wanted the rule without the
table, and a contract shaped for a consumer that does not exist is the
speculative-contract mistake this platform has already corrected once. The
first product that hits it is the signal to build it.

### AI — *terminal, product-critical* ⬜

"Some way to run AI on the edge or connect to an internet API" — named
correcting an earlier framing of this as "Workers AI": the founder was
explicit that Workers AI specifically might not be the right vendor, and
the actual need is thinking about **AI apps**, vendor-neutral. Scoped
2026-09-15, not yet built:

- **Conversational/agentic first.** Streaming chat completion, tool calls,
  system prompts — shaped like the Anthropic/OpenAI messages API. This is
  the founder's stated priority use case.
- **Embeddings/RAG named as a real second use, not this round's.** Turning
  product data into vectors for retrieval needs its own method and pairs
  with a vector store this platform does not have either — a second
  contract or a second method, decided when it has a consumer.
- **Workers AI is a candidate, not the contract's namesake.** Naming the
  contract after one vendor's product would repeat the mistake this
  platform's own adapter system exists to avoid.
- **Gateway-first, decided 2026-09-15.** An earlier sketch here was "write
  one adapter per model vendor and find their narrowest shared interface."
  The founder's correction: target an **AI gateway** — OpenRouter, or a
  comparable vendor-independent one — rather than N vendors.

  That is the better answer, and it dissolves the objection that made this
  item risky. The hard part of an LLM contract is that no neutral
  intersection exists: Anthropic puts `system` top-level where OpenAI makes
  it a message role; Anthropic has content blocks where OpenAI has a string
  or parts array; streaming event shapes differ substantially; and the
  surface moves fast (thinking config, `tool_choice` and prefill all changed
  shape within a year). Intersect those by hand and the contract is too thin
  for agentic work; superset them and you have picked a vendor without
  admitting it.

  A gateway already does that normalization and maintains it. So the
  contract targets **one** shape — the gateway's — and model choice becomes
  a *string in a declaration* rather than an adapter per vendor. Vendor drift
  becomes the gateway's problem, which is what you are paying it for.

  Open before building: whether `model` belongs in `[ai]` as a declaration
  (derivable into the factory, gated like everything else) or per-call; and
  whether a direct-vendor adapter is ever worth having as an escape hatch for
  someone who does not want a gateway in the path.

**Why it is not built yet:** highest design risk of everything discussed in
this round — model APIs vary more across vendors than storage or database
APIs do (chat completion vs. embeddings vs. image generation are genuinely
different shapes), so a first attempt is the likeliest of all these items to
need a redesign. No product has a concrete AI feature yet to generalize the
contract from.

### Legal & compliance — *terminal* ⬜

Depends on **i18n** (legal text is long-form localized copy) and **brand** (it is
copy about a declared entity). A genuine dependency, not a preference: built
earlier, it gets built twice.

### `fid capability extract` — *terminal* ⬜

"It works in my product, now lift it." Mechanizes the second-use rule. Needs the
taxonomy and external capabilities to exist first, or there is nothing to extract
*into*.

### On demand ⬜

None of these block a product; each is added when wanted, through the system
step 3 delivers.

| Item | Note |
|---|---|
| **Claude Design bridge** | Registry source → design-system previews as a derivation. **Resolve first:** `DesignSync` references a `/design-sync` skill for this round trip; if it already exists, this is a thin adapter, not a pipeline |
| **Fast path** | Matters once there is production to hotfix |
| **Interactive seeding** | The cherry on top — `fid new` asks, or seeds brand and registry from a Claude Design project |
| **Research & authoring** | Papers, references, DOIs, templated documents |
| **Demo & showcase** | Interactive landing-page demo, Storybook, feature toggles |
| **Diagnostics** | Error tracking as an adapter with a no-op default |
| **Small tools** | Backlinks, browser-compat banners |
| **Framework currency** | Capability templates must track current majors — Next.js 16, SvelteKit, Tauri. Pinned versions in a scaffold rot silently and a product starts a major behind |
| **Agent portability** | Skills and guard wiring are Claude-Code-only; author once, generate per vendor — see below |
| **Rust release versioning** | Changesets drives npm; the thirteen crates move in lockstep at 0.1.0 with nothing driving a bump |
| **Tagged releases + Zenodo DOI** | No release exists, so there is nothing to archive or cite |
| **Claude chat plugin** | The making philosophy, as a skill for claude.ai — see below |
| ~~**`fid dash` freshness detection**~~ ✅ | Shipped 2026-09-15. Dash equated "gated" with "a workflow runs `fid derive --check`" and reported this repository as ungated while nine gates ran on every commit. `[freshness] gates` declares the others — which command gates an artifact is a judgment, not something to pattern-match — and a gate declared but run by nothing is now reported too |

### Out of band — risk, not priority ⬜

Not ranked by value. Ranked by **what accrues while we wait.**

| Item | Why it cannot queue |
|---|---|
| **`CLA.md`** | The repository is public and can accept pull requests. One contribution from a stranger permanently constrains re-licensing — you would need their permission to ever dual-license. Zero contributions today is the best moment, and it is one small file. |
| `CITATION.cff`, `CONTRIBUTING.md`, `SECURITY.md` | Small, expected of a public project, and prerequisites for being cited correctly |
| Zenodo DOI on a tagged release | Needed before the paper; Fiducial is its own first customer for the research tooling |

## i18n — *localized by construction*

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

**Hardcoded-string detection warns; it does not fail the build.** Opportunistic,
not enforced — but surfaced where it cannot be scrolled past.

The problem with a warning is that it is a log line, and log lines are ignored by
humans and agents alike. So the finding is not printed and forgotten: it is
**reported** — a count and a file:line list in `fid doctor`, a section in
`fid dash`, and a machine-readable form under `--json` so an agent consumes it as
data rather than as terminal noise. The i18n `SKILL.md` instructs agents to clear
them opportunistically.

Warning rather than error was chosen deliberately: with failure, every false
positive blocks a developer, and the detector cannot be perfect about what counts
as user-visible text versus a CSS class, a `data-testid`, an aria role or a URL.
A warning that is genuinely *seen* beats an error that gets suppressed.

### Localization is more than translation

Strings are the visible half. The rest is formatting, and it is where correctness
actually bites:

| Concern | Why it belongs here |
|---|---|
| **Dates & times** | Format, order and separators are all locale-dependent |
| **Timezones** | An absolute instant plus an IANA zone. ROP already solves this well |
| **Numbers** | Decimal separator and grouping differ — `1,234.56` vs `1.234,56` |
| **Currency** | See below. Not implemented in ROP; wanted here |
| **Plurals** | Serbian has three plural forms; English has two |
| **Relative time** | "2 days ago" is not a string you can interpolate |

**Money is a fact that carries its unit — principle 1, exactly.**

> A physical fact is never a bare number. It carries its **unit** and its
> **tolerance**.

An amount without a currency is the same class of bug as a length without a unit,
and it fails the same way: silently, until two of them are added together. So
money is a **type**, never a number:

- amount held in **integer minor units** (cents), because binary floating point
  cannot represent `0.10` and money arithmetic must be exact
- currency is part of the value, not context
- **currency is independent of locale** — a Serbian reader may pay in EUR, and a
  system that infers one from the other is wrong for every cross-border product

**Harvest from ROP:** `resolve-locale.ts`, `config.ts`, `timezone.ts` + datetime
helpers (native `Intl` only — server, client and edge). **Not** `translate.ts`;
its silent fallback is the defect.

Currency is **new work**, not harvested — ROP has none.

---

## Brand

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

## Cross-platform adapter architecture

**The problem, stated by the founder:** "I feel like the adapters should be
cross-platform themselves for any real platform — be it Rust, WebAssembly,
web, Svelte, React, whatever."

**What shipped.** `crates/fiducial-adapters` defines four contracts as Rust
async traits: `Database`, `Storage`, `Email`, `Diagnostics`. Each uses
`BoxFuture<'a, T>` — `Pin<Box<dyn Future<...> + Send + 'a>>` — to stay
object-safe without the `async-trait` crate. `packages/adapters` mirrors each
as a TypeScript interface with the same method signatures. Both sides ship a
`None*` no-op implementation (e.g. `NoneDatabase`, `NoneStorage`) so the
default is always a valid, inert adapter rather than a missing dependency.

`fid add adapters` installs the capability. `fid derive` (the `fid-adapters`
executor) reads `[adapters]` from `fiducial.toml` and generates
`src/adapters.generated.ts` — a `createAdapters(env)` factory that returns
the no-op set by default and is wired to real implementations once vendors are
selected. The file is checked in and gated by `fid derive --check`, the same
as every other derived artifact.

**Contract → toml key mapping:**

| Rust trait | TS interface | `[adapters]` key | No-op |
|---|---|---|---|
| `Database` | `Database` | `database` | `NoneDatabase` |
| `Storage` | `Storage` | `storage` | `NoneStorage` |
| `Email` | `Email` | `email` | `NoneEmail` |
| `Diagnostics` | `Diagnostics` | `diagnostics` | `NoneDiagnostics` |

**Firmware boundary.** Cloud adapter contracts do not apply to `no_std`
firmware. Firmware uses `embedded-hal` / Embassy HAL for peripherals and
`fiducial-ota` for OTA. `crates/fiducial-adapters/src/firmware.rs` holds the
full mapping. Tauri bridges both worlds: this crate in the Rust backend,
`packages/adapters` in the TS frontend, `fiducial-tauri` for serial transport.

**Vendor implementations.** `d1` (database) and `r2` (storage) shipped —
see *Cloudflare adapter set*. Supabase, Neon/Postgres, S3, Resend and Sentry
remain candidates.

---

## Cloudflare — the default

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

## Legal & compliance

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

## Context sync

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

## Fast path

**The problem:** an urgent production fix cannot wait on a ten-minute
end-to-end suite, but skipping CI by hand is how a bad fix ships.

Target: a declared *fast path* — the subset of checks that must never be skipped
(compile, unit tests, the freshness gate) separated from the slow, optional ones
(full e2e, multi-target matrices, wasm-pack). Correct by construction, not by
someone deciding under pressure which checks to bypass.

Constraint: whatever is skipped is **recorded**, and the full suite runs after
the fact. A fast path that hides what it skipped is just a broken CI.

---

## Open-source bootstrap

One command or skill turns any repository into a properly published
open-source project:

`LICENSE` · `CITATION.cff` · `CLA.md` · `CONTRIBUTING.md` · `SECURITY.md` ·
`CODE_OF_CONDUCT.md` · release + Zenodo DOI wiring · authorship and trademark
boundaries stated.

**Meta-requirement, stated explicitly:** everything built for Fiducial must be
available *to* the tools Fiducial builds. Fiducial is the first consumer of its
own open-source bootstrap — it currently lacks every file in that list.

---

## Research & authoring

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

## Demo & showcase

Not simulation in the numerical sense (`fiducial-sim` covers that). **Optional**,
and primarily an *interactive demo section on a landing page* — a real frontend
with real behaviour that a visitor can actually use, with features individually
toggleable.

The secondary benefit is internal: a surface that can be seen and analysed
deterministically, rather than having edge cases discovered in production.
Storybook is part of this and is currently underused.

---

## Claude Design bridge

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

## Interactive seeding

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

## Claude chat plugin — the philosophy, outside Claude Code

Today the platform's way of thinking reaches an agent only inside a scaffolded
repository: the plugin contributes the guard and `/fiducial:*` commands, and
`AGENTS.md` carries the principles. **All of that requires a repository.**

Wanted: the same thing usable in an ordinary claude.ai project or chat — where
much of the actual thinking happens, long before there is a repo. Something that
knows how the making philosophy goes and uses its vocabulary: *declaration*,
*derivation*, *staleness*, *cost of delay*, *localized by construction*,
*capability* in the three-way sense.

**What it should carry:** the principles including 1c and 5c, the
declaration/pipeline/adapter taxonomy, the anti-goals (especially "the platform
must never become the project"), the decision-record habit, and the ordering
rule. Not the commands — there is no `fid` in a chat — but the **reasoning**.

**Open, to resolve before building:** the exact packaging for claude.ai skills
versus Claude Code plugins, and how much is shared. The two formats are close
(`SKILL.md` with frontmatter) but the distribution is different, and the honest
answer is that this has not been verified rather than assumed.

**The thing to avoid:** a second copy of the principles. They are already
declared in `MISSION.md` and restated in the scaffolded `AGENTS.md`, and that
duplication is already gated by a test precisely because it was a violation to
introduce. A third copy for chat makes it three. This should be **generated**
from `MISSION.md` — which makes it a natural consumer of **context sync**, and a
reason to keep that item where it is in the order rather than later.

## Agent portability — author once, generate per vendor

`AGENTS.md` is the cross-vendor convention and now carries the full command
reference, so a Codex, Copilot or Cursor session in a Fiducial repository knows
what `fid derive --check` is. That part is done.

**What is still Claude-Code-only:**

| Thing | Portable? |
|---|---|
| `AGENTS.md` | ✅ every agent reads it |
| The `fid` CLI | ✅ any agent can run it |
| `commands/*.md` (`/fiducial:harvest`, `/fiducial:platform`) | ❌ Claude Code packaging |
| `.claude-plugin/plugin.json` | ❌ Claude Code only |
| `PreToolUse` guard **wiring** | ❌ — though `fid guard-check` itself is a CLI any agent can call |
| 7 capability `SKILL.md` files | ❌ a Codex session installing `eda` gets no instructions |

**The shape of the fix:** make the vendor-neutral thing the declaration and
generate the wiring. A skill is authored once; `fid` emits the Claude Code
command file, the `AGENTS.md` section, and whatever another vendor needs. Same
move as the principles — one author, many derivations — which makes this a
consumer of **context sync** rather than a separate mechanism.

**Watch for:** the guard is the interesting case. The *rule* is portable because
`fid guard-check` is a CLI; only the hook that calls it before every shell
command is Claude-specific. An agent without hooks can still be told to run it,
which is weaker but not nothing.

## Rust release versioning

Changesets drives the JS side properly — `.changeset/*.md` declares
minor/patch/major per package and the release workflow opens a Release PR.
`@fiducial/headless` went 0.2.0 → 0.3.0 that way.

**The thirteen Rust crates have no equivalent.** All inherit
`version = "0.1.0"` from `[workspace.package]`, so they move in lockstep and
nothing drives a bump. `release.yml` publishes to crates.io, but the version
*decision* is manual where the npm one is automated.

Note that `WIRE_VERSION` is **deliberately not SemVer** and is not this gap: it
is a single integer with a declared minimum-compatible floor in
`docs/compat/matrix.toml`, gated by `fid release check`, because a protocol
version answers a different question than a package version. That mechanism is
already stricter than SemVer and should stay as it is.

## Tagged releases and the DOI

No git tag or GitHub release exists. That blocks the Zenodo DOI, which blocks
citing the work in the paper. Small, and on the critical path for
`docs/specs/2026-09-14-disclosure-authorship-and-citation.md`.

## Diagnostics

Error tracking as an **adapter with a no-op default** — wired in from the first
commit, costing nothing until pointed at a vendor.

Constraint: no self-hosted database for error tracking. The job is outsourced or
it is not done.

---

## Small tools

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

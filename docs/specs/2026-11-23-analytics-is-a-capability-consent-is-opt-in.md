# Analytics is a capability with many providers, and consent is opt-in

**Date:** 2026-11-23
**Status:** accepted
**Supersedes:** `docs/specs/2026-11-22-payload-is-the-cms-and-commerce-capability.md`
in part — **D7's second sentence** (traffic analytics is Cloudflare Web
Analytics "loaded only after consent"), **D14 in full** (consent and legal go
through the `legal` capability, with `analytics`/`marketing` categories and
scripts behind a recorded yes), its **open question 4** (which consent category
UTM capture falls under), and its **commit 9**. Everything else in that spec
stands: Payload is still the CMS, `commerce` is still the domain capability,
and D7's *first* sentence — business metrics come from the commerce rows, not
an event store — is reaffirmed, not weakened.

It also replaces the `Traffic analytics` row in `STACK.md` and adds two rows,
and it rewrites the consent bullet of the roadmap entry *Client business sites*.

---

## Context

Commits 0–5 of the Payload series are merged (`5a73264`): the `payload` and
`commerce` capabilities exist, `[deploy] crons`/`secrets` are declarable, and
the blocks, revalidation and commerce model are in. What remained was forms,
email, the dashboard, legal, backups and docs.

Two things changed that the earlier spec cannot absorb by addition.

**1 · The default site is to set no non-essential cookies at all.** The
2026-11-22 plan put Cloudflare Web Analytics, the Meta Pixel and the
Conversions API behind a consent banner shipped by the `legal` capability. A
banner is the price of non-essential storage; a site that performs none owes
nobody a banner, and every banner is a client component, a stored record, a
second legal surface and a measurable conversion cost. The decision is
therefore not "a nicer banner" but *delete the reason for one in the default
template*.

**2 · Analytics has four plausible vendors and a product may want several at
once.** The earlier plan named one vendor inside a legal component. That is the
shape this platform spent a phase removing: a vendor's model leaking into the
caller. Analytics is a contract.

**Why it cannot simply join `[adapters]`.** Verified by reading the code, not
recalled:

| Verified in this repository | Consequence |
|---|---|
| `Adapters` is `BTreeMap<String, String>` and `Adapters::get` returns **one** `&str` (`crates/fiducial-cli/src/config.rs:1006`) | `[adapters] analytics = [...]` is not expressible without changing the shape of every contract |
| `adapter::problem(contract, vendor)` reports on a single vendor string (`crates/fiducial-cli/src/adapter.rs:165`) | one-vendor reporting is what `fid doctor` and `fid dash` both consume (`doctor.rs:174`, `dash.rs:997`) |
| `render_adapters_factory` builds exactly one class per slot, lazily (`commands/derive.rs:2648`) | fan-out to several providers has no place in that factory's uniform `new {Class}(env)` shape |
| `botProtection`/`email` carry no per-vendor metadata beyond the name | `requiresConsent` per provider has nowhere to live in `[adapters]` |

So the contract, the `none`, the generated factory, the vendor-is-a-promise
rule and the `fid doctor` reporting all follow the adapter pattern exactly; the
**cardinality** is the one difference, and it is a declaration difference, not
a pattern difference.

**3 · `payload generate:types` and `generate:importmap` do not run.** The
2026-11-22 Status section records them as crashing "on Node 22 or later because
undici 7 requires a browser-only `CacheStorage` global", and
`scripts/verify-payload.sh` skips both, so `payload-types.ts` and
`importMap.ts` are **hand-kept placeholders** — hand-written derived artifacts,
which principle 1 forbids and which `fid derive --check` cannot gate.

That diagnosis was wrong, and the real one was reproduced in this session
(Node v22.23.5, `payload@3.94.1`, `tsx@4.23.15`, `undici@7.30.1`, installed
from the registry in the sandbox):

| Evidence | Finding |
|---|---|
| `node -e "typeof caches"` → `undefined`, yet `require("undici")` and `import("undici")` both **succeed** on bare Node 22 | the missing-global story cannot be the cause |
| `undici/index.js:177` is `new CacheStorage(kConstruct)`; `cachestorage.js:17` throws unless `arguments[0] === kConstruct` | undici guards its own constructor with a module-local `Symbol` |
| instrumenting `undici/lib/core/symbols.js` with a stack trace: it is **evaluated twice**, both times through `tsx/dist/require-*.mjs` (two different hook entry points, offsets `:9:2034` and `:9:2716`) | two `Symbol('constructable')` instances exist, so `kConstruct !== kConstruct` and undici throws `TypeError: Illegal constructor` |
| patching `kConstruct` to `Symbol.for(...)` in `node_modules` | gets past undici, and the **next** failure is `payload/dist/bin/loadEnv.js:3` — `Cannot destructure property 'loadEnvConfig' of 'import_env.default'`, i.e. `@next/env`'s ESM default under the same tsx transform |
| `payload generate:types --use-swc` with `@swc-node/register` | resolution works, then `SyntaxError: The requested module './src/pages' does not provide an export named 'Pages'` |
| `payload generate:types --disable-transpile` with a config whose every specifier is Node-resolvable | **works** — "Types written to ./payload-types.ts" |
| the same with the template's extensionless relative imports (`./src/payload/collections/Users`) | `ERR_MODULE_NOT_FOUND` — Node's ESM resolver needs the extension |
| `payload`'s `package.json` pins `"tsx": "4.23.15"` **exactly**; `tsx@4.23.15` and `payload@3.94.1` are both the current `latest` | pinning around it is a fork of a pinned transitive dependency, and there is no newer release to wait for |
| `node --import ./scripts/ts-resolve.mjs node_modules/payload/bin.js generate:types --disable-transpile`, where the hook is 20 lines registering one `module.registerHooks({ resolve })` that retries a relative specifier with `.ts`, `.tsx`, `/index.ts` | **both generators work**: "Types written…" and "Writing import map to …/src/app/(payload)/admin/importMap.js" |

Two byproducts of the last line matter. Payload writes the import map to
`src/app/(payload)/admin/importMap.js` — not the `src/app/(payload)/importMap.ts`
the capability ships as a placeholder. And the working path uses **Node's own
type stripping**, unflagged since Node 22.18, while `.nvmrc` in this repository
still says `20` — a Node that cannot run it, and that went end-of-life in
April 2026.

## Decision

**D15 · The default template ships no consent banner and no consent module,
because it sets nothing that needs consent.** Superseding D14. A default site
sets **no non-essential cookies and loads no third-party scripts**. Strictly
necessary client storage is enumerated, and the list is short: the **cart** in
`localStorage`/`sessionStorage` (only where `[commerce] cart = true`), and the
**auth cookie** for a staff or client-editor login. Embedded maps and videos
are **click-to-load**: a placeholder with the provider named, and no request to
that provider until the visitor asks for one.

**D16 · `analytics` is a capability whose contract takes many providers at
once.** The declaration is a list, not a vendor string:

```toml
[analytics]
providers = ["first-party", "cloudflare-web-analytics"]
retention_days = 396                    # 13 months

[analytics.cloudflare-web-analytics]
token_env = "CF_BEACON_TOKEN"
```

Site code calls the contract and only the contract — `track(event)` and
`pageview(ctx)`. Adding, removing or reordering providers is an edit to
`fiducial.toml` and a `fid derive`; no site file changes. Several providers run
together, and the generated factory fans out: one `Analytics` to the caller,
*n* providers behind it, each failure isolated so a broken vendor cannot lose
an event for the others.

**D17 · The contract lives in TypeScript; the provider *registry* lives in
Rust.** `packages/adapters/src/analytics.ts` holds the `Analytics` interface,
the typed event union, `NoneAnalytics` and the provider classes.
`crates/fiducial-cli/src/analytics.rs` holds `PROVIDERS` — id,
`requires_consent`, default on/off, the env/secret names each reads, and one
line saying what it stores — mirroring `adapter::CONTRACTS`.

The split follows the two questions each side answers. Every consumer of the
*contract* is a browser or an edge server, which is D11's own rule for staying
in TypeScript, and there is no Rust caller to justify a
`fiducial-adapters::analytics` yet. But `fid doctor`, config validation, the
derived factory and the **generated legal wording** all have to read
`requires_consent` with no `node_modules` in sight, so the registry is Rust.
`requires_consent` is declared **once**, in that table: the TypeScript provider
classes never restate it, the generated factory wraps the ones that need it,
and a test asserts every provider id in TypeScript appears in the Rust registry
— the same pairing `derive.rs` already requires between `implementations` and
the import table.

**D18 · The event schema is a declaration, and five of its events are the
funnel.** `[analytics] events` is seeded with `page_view`, `cta_click`,
`add_to_cart`, `checkout_start`, `quote_request`, `order_submitted`, and the
funnel's ordered steps are declared alongside. One declaration derives: the TS
event union and payload types, the first-party table's columns, the funnel
query, and the sentence in the privacy notice that lists what is recorded. A
chemistry or billing product adds its own event names in the same block —
new facts, no core change.

**D19 · The first-party provider writes nothing to the visitor's device, and
the visitor hash is derived rather than stored.** A beacon `POST` to `/api/e`
writes one row: event type, landing path, UTM source/medium/campaign, referrer
host, timestamp, and
`visitor_hash = HMAC-SHA-256(secret, "YYYY-MM-DD" (UTC) + ip + user-agent)`
truncated to 128 bits, computed in the Worker with WebCrypto. The **raw IP is
never stored and never logged**. The "daily-rotating salt" is therefore a
*derivation* of one declared secret and the date — no rotation job, no KV, no
stored salt, and yesterday's hashes become unlinkable by arithmetic rather than
by a deletion that has to be remembered. The secret's name is declared in
`[deploy] secrets` (commit 1 of the earlier series made that possible) and its
value is set with `wrangler secret put`. `requiresConsent` is false.

**D20 · UTM capture is strictly necessary session storage, and lead source is
one declaration with two readers.** Superseding open question 4. Landing path,
UTM parameters and referrer host are captured at landing into
`sessionStorage` — session-scoped, not a cookie, gone when the tab closes —
attached to every event and to the order or quote row at submit. `[commerce]
lead_sources` stays the declared vocabulary; the hidden field that carries it
into an order is filled by the analytics capture module through a scoped
template (`for/commerce/`), so there is **one** capture mechanism rather than
one for statistics and another for the CRM. A product without `analytics`
installed still gets the field, filled from the referrer alone.

**D21 · `consent` is a separate, opt-in capability that requires `legal`, and
`legal` stops shipping a banner.** The banner, the footer preferences link,
`apps/web/src/generated/consent.ts` and the `preticked_consent` advisory check
move out of `legal` into `consent`. `legal` keeps what it is for: words.
Installing `consent` is what makes a `requires_consent` provider legal to
enable, and nothing else changes: `first-party` and
`cloudflare-web-analytics` keep running whether it is installed or not, and
they are never gated, because gating a cookieless beacon behind a banner buys
no compliance and loses the data that tells you the banner is costing you
conversions.

**D22 · Enabling a `requires_consent` provider without the `consent` capability
is a configuration error.** `Analytics::validate` reads `[capabilities]
enabled` and the Rust registry and fails the `analytics` pipeline; `fid doctor`
reports the same finding through a `problem()`-shaped function rather than
throwing, because that is why `adapter::problem` returns instead of printing.
`meta-pixel-capi` is the first provider the rule bites.

**D23 · The funnel view reads the first-party table and nothing else.** Weekly
buckets (ISO week, UTC), the declared funnel steps as columns, grouped by
landing page and by UTM source, in the Payload admin dashboard and through the
existing read-only API-key role (D6's `service` principal at `viewer`). It
never mixes a provider's numbers with Cloudflare's or PostHog's: two sources
summed into one figure is the second-source-of-truth error D7 already named,
and each vendor's own dashboard remains the place to read that vendor.

**D24 · Retention is declared once and enforced by a cron.** `[analytics]
retention_days` (seeded 396 = 13 months) derives both the scheduled delete —
a `[deploy] crons` entry and a handler removing rows older than the window —
and the retention sentence in the privacy notice. Changing the number changes
the job and the page together or `fid derive --check` fails.

**D25 · The generated legal wording is derived from `[analytics]` and the
registry, and marked for human review.** The privacy and cookie pages state
plainly, in every declared locale: that **no cookies are used for analytics**;
that first-party statistics are cookieless and identify a visit through a
hash that changes every day; that Cloudflare Web Analytics is in use and is
cookieless; the list of optional providers, with the ones actually enabled
named; the retention window; and that the only cookie the site sets is the
staff login cookie, with the cart in local storage. Enabling PostHog therefore
edits the privacy page on the next `fid derive`, and failing to re-derive fails
CI. Every page keeps the review marker: **this wording is generated, not
lawyered.**

**D26 · The Payload CLI runs on native Node, not through its bundled tsx, and
its two generators become gated pipeline outputs.** The npm scripts become
`node --import ./scripts/payload-ts-resolve.mjs node_modules/payload/bin.js
generate:types --disable-transpile` (and the same for `generate:importmap`),
where the platform-owned hook is the 20 lines above. Consequences, each
deliberate:

- `pipelines/payload-codegen.toml` declares
  `apps/web/src/payload/payload-types.ts` and
  `apps/web/src/app/(payload)/admin/importMap.js` as **outputs**, so they are
  gated. They are *not applicable* until `apps/web/node_modules` exists, which
  is the condition `outputs_not_applicable` already encodes for three other
  pipelines.
- the hand-kept `importMap.ts` placeholder is **deleted**, and the admin route
  imports the generated `(payload)/admin/importMap.js` — the path Payload
  actually writes.
- the Node floor rises to **22.18** (`.nvmrc`, `engines`, the CI matrix),
  because native type stripping is what removes tsx from the path. Node 20 is
  already end-of-life, and `.nvmrc` saying `20` while CI and every sandbox run
  22 was fiction.
- the product `tsconfig.json` gains **`erasableSyntaxOnly: true`**, so `tsc
  --noEmit` gates the single constraint native stripping imposes (no enums, no
  namespaces, no parameter properties) instead of letting a contributor
  discover it as a codegen crash.
- `scripts/verify-payload.sh` runs both generators and its misdiagnosis comment
  goes with them, and an upstream issue is filed on each side: tsx (a module
  evaluated twice through its require hook on Node ≥ 22.15) and Payload (its
  CLI unusable on current Node with its pinned tsx).

## Why not the alternatives

- **Keep the banner and gate everything behind it.** It is the 2026-11-22 plan,
  and its cost is paid on every visit by every visitor for data two cookieless
  providers already give. A banner is only honest when something actually needs
  consent; shipping one by default teaches a client that a banner *is*
  compliance.
- **`[adapters] analytics = "cloudflare-web-analytics"`.** One vendor per
  contract is the whole shape of `Adapters`, and the brief requires several at
  once — first-party plus Cloudflare is the *default*. Widening every contract
  to a list to serve one contract's cardinality would make ten single-vendor
  slots read as if they were lists.
- **A provider list inside the component, with `if` per vendor.** That is what
  the superseded plan had, and it is the vendor's model in the caller: the site
  would import PostHog's SDK to decide not to use it, and removing a provider
  would be a code edit in a file the product owns.
- **Declare `requiresConsent` on the TypeScript class.** It is the natural home
  for it right until something without `node_modules` needs to read it — and
  three things do: `fid doctor`, the config validator, and the generated legal
  page. A flag in both places is the duplicated fact this platform exists to
  delete.
- **A Rust `Analytics` trait in `fiducial-adapters` now.** Nothing in Rust
  calls it. The platform has already corrected one speculative contract (the AI
  contract); the second consumer — most likely the self-hosted `axum` target —
  is what makes it real.
- **Google Analytics 4 as a provider.** It needs consent in the EU and RS, it
  is the provider most likely to make a client's site a transfer-mechanism
  question, and the funnel the owner asks for is five numbers a `GROUP BY`
  answers. Not rejected forever; not in the default set, and not the first one
  written.
- **An event store for the business metrics too.** Unchanged from D7: order
  status timestamps answer "is this paid". The events table answers "how did
  they arrive", which the order row cannot, and the two are never joined into
  one number.
- **Patch undici (or pin tsx) so the Payload CLI's tsx path works.** Both were
  tried in this session. The undici patch succeeds and immediately exposes a
  *second* tsx-transform failure in `@next/env`, so the workaround would be two
  patches deep in transitive dependencies of a pinned dependency — and the
  thing being worked around is a transform this product does not need, because
  Node runs TypeScript itself now.
- **`--use-swc`.** It adds a dependency to every product and fails on named
  exports from a stripped `.ts` module. A dependency that adds a failure mode
  is worse than no dependency.
- **Leave `payload-types.ts` hand-maintained.** It is a derived artifact
  written by hand, which is the one thing principle 1 calls an error rather
  than a shortcut, and it silently drifts the moment a field is added to a
  collection — which commits 7 through 9 of this series do repeatedly.

## Consequences

- `legal` becomes smaller and `consent` exists: a product that wants a banner
  installs one capability, and a product that does not ships no consent code at
  all. The `legal` capability's `for/web-next/.../cookie-consent.tsx` and its
  second pipeline output move; `fid doctor` stays clean because both are
  platform-owned files being retired, not forked.
- `[legal] cookie_categories` keeps its role, with the same rule as a provider:
  declaring any category but `necessary` requires the `consent` capability.
- A new unauthenticated write endpoint exists (`/api/e`). It is same-origin
  checked, size-capped and schema-validated, and it is a cost surface on a paid
  Workers plan; D19's row is 9 short columns, which is what keeps D1 writes
  affordable, and rate limiting stays an open question rather than a claim.
- The platform gains a second **list-shaped** vendor declaration. `[adapters]`
  stays one-vendor-per-contract, and the difference is now visible in
  `fiducial.toml` rather than hidden in a factory.
- Products must run Node ≥ 22.18. CI images, `scripts/verify-*.sh` and every
  scaffolded `engines` field change in one commit.
- `payload generate:types` output becomes a real gated artifact, so a collection
  edited without re-deriving fails CI instead of drifting from its types.
- The generated privacy wording now depends on `[analytics]`, which means a
  vendor change is a legal-page change. That is the intended coupling, and it
  is also the one that will fail `fid derive --check` most often.

## What this deliberately does not do

- No `identify()`, no user profiles, no cross-site or cross-device linking, no
  session recording, no heatmaps, no A/B framework.
- No consent banner in the default template, and no "cookie wall".
- No raw IP stored, anywhere, for any provider.
- No Google Analytics, no Google Tag Manager, no ads pixels beyond the
  explicitly opt-in `meta-pixel-capi`.
- No analytics data in the agency app, and no cross-client aggregation: one
  Payload instance, one D1, one client (D9 unchanged).
- No Rust `analytics` contract, and no event pipeline into DuckDB/R2 — that row
  stays what 2026-11-22 narrowed it to, high-volume telemetry.
- No attempt to fix tsx or undici in this repository beyond filing the issues:
  the workaround is a script this platform owns, removed when upstream lands.

## Open questions

1. **Where config validation belongs.** The recommendation is the existing
   split: `Analytics::validate` bails inside the `analytics` pipeline, and
   `fid doctor`/`fid dash` report through a non-throwing `problem()`. The
   alternative — validating in `Config::load` so every command fails — would
   stop `fid doctor` from being able to *describe* the problem, which is the
   reason `adapter::problem` returns a `String`. Not settled: whether
   `fid derive` on an unrelated pipeline should fail too.
2. **Rust-derived TS, or TS-side contract.** D17 says TS contract plus Rust
   registry. The alternative is a full `fiducial-adapters::analytics` with the
   TS derived from it, which is where `identity` ended up. The trigger to
   revisit is the first Rust caller (self-hosted `axum`).
3. **The daily salt's secret.** Declared by name in `[deploy] secrets`, value
   set with `wrangler secret put`. Open: rotation cadence for the underlying
   secret, and whether a late-arriving beacon crossing midnight UTC deserves a
   one-day grace window (recommendation: no — the double-count is smaller than
   the complexity).
4. **Where the legal wording lives.** The analytics paragraphs could be page
   copy in `legal.rs` next to the existing per-locale text (one home for legal
   prose, but prose in Rust), or message keys in `messages/<locale>.json` via
   `seed_messages` (principle 1c's home for user-visible strings, but legal
   copy a product can silently edit). Recommendation: `legal.rs`, consistent
   with every other generated legal page, and revisit if a client needs to
   reword it.
5. **Who owns the events table's DDL.** The `migrations` capability, or SQL the
   `analytics` capability derives and `wrangler d1 migrations apply` applies
   next to Payload's own migration table. Either way the table is prefixed and
   Payload never sees it.
6. **`requires_paths` on a pipeline.** The codegen pipeline needs
   "not applicable until `apps/web/node_modules` exists".
   `outputs_not_applicable` already hand-codes three such conditions in Rust;
   this is the fourth, and the rule of two says the declared form is overdue.
   Proposed but not decided here.
7. **Is Cloudflare Web Analytics consent-free in RS and the EU?** Carried from
   the brief as "cookieless beacon, no consent". It is the claim a reviewer
   should check first, because it is the one the default template relies on.
8. **Rate limiting `/api/e`.** Cloudflare's rate-limiting rules are account
   configuration, and this repository deliberately calls no Cloudflare API.
   Open: whether the limit is documented, declared in `[deploy]`, or enforced
   in the handler.
9. **Does `errors`/`monitoring` eventually fold into this?** Workers Analytics
   Engine is a candidate for both. Rule of two: not now.

## The plan

### Commit order

Appended to the 2026-11-22 series, whose commits 0–5 are merged. Commits 6–11
of that list are **withdrawn as written**: 9 is replaced by commits 3 and 6
below, and 6, 7, 10 and 11 return as 7, 8, 9 and 10. Each commit is
Conventional Commits, bumps `[workspace.package] version` when anything under
`crates/` changes, adds its `SHIPPED.md` row, and keeps its subject ≤ 72
characters.

| # | Commit | Files | Why here |
|---|---|---|---|
| 1 | `fix(payload): run the Payload CLI on native Node, not tsx` | `capabilities/payload/apps/web/scripts/payload-ts-resolve.mjs` (new), `capabilities/payload/capability.toml` (`npm_scripts`), `capabilities/payload/pipelines/payload-codegen.toml` (new), `capabilities/payload/apps/web/tsconfig.json` (`erasableSyntaxOnly`), delete `apps/web/src/app/(payload)/importMap.ts` + fix the admin import, `src/commands/derive.rs` (`outputs_not_applicable`), `src/pipeline.rs` (`requires_paths`, if adopted), `.nvmrc`, root + template `engines`, `.github/workflows/*` Node version, `scripts/verify-payload.sh`, `capabilities/payload/SKILL.md`, `tests/payload_capability.rs` | debt-accruing in the strict sense of 5c: every later commit adds collections and fields, and each one that lands first adds another hand-written line to a file that is supposed to be generated |
| 2 | `feat(analytics): one contract, many providers, cookieless by default` | `packages/adapters/src/analytics.ts` + `index.ts` + tests + changeset, `crates/fiducial-cli/src/analytics.rs` (registry), `src/config.rs` (`[analytics]`, `validate`), `src/commands/derive.rs` (`fid-analytics` → `apps/web/src/generated/analytics.ts`), `src/commands/doctor.rs` + `dash.rs` (report `problem`), `capabilities/analytics/**` (`SKILL.md`, `capability.toml`, `pipelines/analytics.toml`, `for/web-next/` `/api/e` route, capture module, click-to-load embed, events DDL), `packages/studio/src/schemas/fiducial.schema.json`, `tests/analytics_pipeline.rs` | nothing downstream can be written against a contract that does not exist, and the four providers are what make the contract real rather than a guess |
| 3 | `feat(consent): consent is an opt-in capability, off by default` | `capabilities/consent/**` (banner, footer link, `generated/consent.ts` pipeline), `capabilities/legal/` (banner + second output removed), `src/legal.rs` (`render_consent_ts`, `preticked_consent` move), `src/analytics.rs` (the `requires_consent` rule bites), `tests/consent_capability.rs`, `tests/legal_pipeline.rs` | the rule in D22 cannot be tested until the capability it names exists, and the default-template tests in item 7 need both sides |
| 4 | `feat(commerce): the weekly funnel and order dashboard` | `capabilities/commerce/apps/web/src/payload/views/Dashboard.tsx`, `…/metrics.ts` (orders and quotes), `capabilities/analytics/for/commerce/…/funnel.ts` (events), the API-key branch in `payload/access.ts`, `tests/commerce_pipeline.rs` | one dashboard file, written once: the funnel panel reads commit 2's table and the order panel reads the collections commit 5 already shipped. The numbers are zero until commit 7 fills them, and the tests seed rows directly |
| 5 | `feat(analytics): delete first-party events after 13 months` | `[analytics] retention_days` already declared in 2; the scheduled handler, its `[deploy] crons` entry, `capabilities/analytics/SKILL.md` (the restore/retention runbook), `tests/analytics_pipeline.rs` | small, and it must exist before the first real traffic, not after |
| 6 | `feat(legal): cookieless wording, and distance selling in Serbia` | `src/legal.rs` (privacy + cookie copy derived from `[analytics]`; RS distance-selling page; the made-to-measure withdrawal exception; review markers), `profiles/legal.toml` (RS gains `distance-selling`), `tests/legal_pipeline.rs`, `docs/prose.lock` where a described paragraph moved | replaces the withdrawn commit 9. It reads the registry, so it follows 2 and 3 |
| 7 | `feat(commerce): the order and made-to-measure forms` | variant picker, both forms, Turnstile through `createAdapters(env).botProtection`, the lead-source hidden field fed by D20's capture, `tests/commerce_pipeline.rs` | the withdrawn commit 6, now downstream of the capture it shares with analytics |
| 8 | `feat(commerce): owner and customer email, and the jobs queue` | `payload/hooks/notifyOwner.ts`, `confirmToCustomer.ts`, message keys in both locales, `payload/jobs/*`, the queue's cron | the withdrawn commit 7, unchanged |
| 9 | `feat(payload): nightly D1 export to R2, and a documented restore` | the scheduled handler, `[deploy] crons` + `secrets`, the runbook in `payload/SKILL.md` | the withdrawn commit 10, unchanged |
| 10 | `docs: analytics, consent and the Payload series in the record` | `fid context`, `README.md`, `SHIPPED.md`, `ROADMAP.md` markers, `fid docs --accept` where a paragraph moved | the derived capability tables go stale the moment commit 2 lands; closing them once at the end costs one commit instead of six |

### The rules this series is held to

Unchanged from 2026-11-22, plus `scripts/verify-payload.sh` now running both
generators, and `pnpm --filter @fiducial/adapters test:generated` covering the
analytics factory for every provider selection — because a generated factory
asserted as text is not tested.

### What the tests must prove

Item 7 of the brief, as four assertions on a **default** scaffold:

1. no `Set-Cookie` in any response but the staff login route's;
2. no `<script src>` pointing outside the product's own origin, and no
   `cookie-consent` component in the tree at all;
3. nothing written to `localStorage`/`sessionStorage` except the cart key and
   the UTM capture key, both declared;
4. `[analytics] providers = ["meta-pixel-capi"]` without `consent` in
   `[capabilities] enabled` fails `fid derive` **and** is reported by
   `fid doctor`.

## Status

**2026-11-23** — All ten commits merged on branch `ccr-c766b4d5-4mp7o3`
(0.47.0–0.55.0). The four test-plan assertions above hold for items 2, 3 and
4 (verified via `cargo test`). Item 1 (no Set-Cookie except staff login) is
not an automated test — it requires a live Workers response. Lighthouse and
live D1 bindings are unverified.

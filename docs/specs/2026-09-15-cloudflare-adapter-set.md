# Cloudflare adapter set: `d1` and `r2` ship; the rest is scoped, not built

**Date:** 2026-09-15
**Status:** accepted

---

## Context

`ROADMAP.md` names the Cloudflare adapter set as the first real vendors,
"the proof that the adapter contract is vendor-neutral rather than a
Cloudflare-shaped hole," and lists its scope as **D1, R2, Workers, Access,
Turnstile, Queues, Workers AI** — seven Cloudflare products.

*Cross-platform adapter architecture* (shipped, Phase 27) had already done
the work this phase depends on: `Database`, `Storage`, `Email`, `Diagnostics`
exist as Rust async traits and TypeScript interfaces, each with a `None*`
no-op, and `fid derive` generates `src/adapters.generated.ts` from
`[adapters]` in `fiducial.toml`. `crates/fiducial-cli/src/adapter.rs`
documents the exact seam: a vendor moves from `candidates` to
`implementations`, and `vendor_ts_class_and_path` in `derive.rs` gets a new
match arm. Nothing else was supposed to change — and for `d1` and `r2`,
nothing else did.

Of the seven items named, only two — **D1** and **R2** — land on a contract
that already exists (`database`, `storage`). The other five do not:

| Item | What it is | Existing contract? |
|---|---|---|
| Workers | compute | `deploy` exists as a contract name but carries no trait — "a build-time/pipeline concern, not a runtime call" per `fiducial-adapters::lib` |
| Access | identity / auth gate | none |
| Turnstile | bot / CAPTCHA challenge | none |
| Queues | async job queue | none |
| Workers AI | inference | none |

## Decision

Ship `d1` and `r2` as real, working vendor implementations of the
`database` and `storage` contracts. Defer Workers-as-`deploy`, Access,
Turnstile, Queues and Workers AI, each named individually below with the
reason it does not enter this phase.

**What shipped:**

- `packages/adapters/src/database.ts` — `D1Database implements Database`,
  reached through a Workers binding (`env.DB`). `execute`/`query`/`queryOne`
  wrap D1's `prepare().bind().run()/all()/first()`; `batch` wraps `db.batch()`.
- `packages/adapters/src/storage.ts` — `R2Storage implements Storage`,
  reached through `env.BUCKET`. `put`/`get`/`delete` map directly; `list`
  follows R2's cursor across pages so a bucket over 1000 keys is not silently
  truncated.
- `crates/fiducial-cli/src/adapter.rs` — `d1` and `r2` moved from
  `candidates` to `implementations` for their contracts.
- `crates/fiducial-cli/src/commands/derive.rs` — `vendor_ts_class_and_path`
  gained the two match arms; selecting `database = "d1"` or `storage = "r2"`
  now generates a real import instead of falling back to the no-op.
- 20 TypeScript tests (`packages/adapters/src/adapters.test.js`) against
  fake bindings; 4 new end-to-end tests through the real `fid` binary
  proving the declared vendor reaches the generated factory.

## Why not the alternatives

**Building all seven at once.** The five without a contract each need one
designed first — and Phase 23's own adapter format was explicit that a
contract designed against zero consumers repeats the mistake it had just
corrected ("an adapter registry offering `supabase` before anything speaks
Supabase is that bug with a different noun"). `fon` has not asked for
identity, bot protection, a job queue, or inference yet. Designing five new
contracts speculatively, in the same phase that is supposed to be proving
the *existing* two are vendor-neutral, would be the anti-goal the platform
states in `MISSION.md`: nothing added before a real product needs it.

**A Rust-side `D1Database`/`R2Storage`, for symmetry with the TypeScript
side.** Considered, because `fiducial-adapters`' own doc comment says the
Rust and TypeScript sides are "kept in sync by design," and a reader could
reasonably expect a real vendor to appear on both. It does not, and the
reason is not an oversight: a D1 or R2 **binding** is a Workers runtime
object — it exists only inside a Worker, the same way `env.DB` in a Worker
is not a value a Tauri desktop process can obtain. Reaching D1 or R2 from
Rust would mean a different client entirely — Cloudflare's general HTTP API
(D1's `/accounts/{id}/d1/database/{id}/query` endpoint; R2's S3-compatible
API) authenticated by an API token instead of a binding. Nothing in this
repository has a Tauri product that needs cloud database or storage from its
desktop backend, so that second client is not built. `lib.rs` and the
capability's `SKILL.md` now say this explicitly, so "mirrors" is read as
"mirrors for the contract and `none`," not "mirrors for every vendor."

**`R2Storage.signedUrl` returning a working URL.** A presigned R2 URL needs
AWS SigV4 signing against R2's S3-compatible API, which needs an R2 API
token (access key + secret) — credentials a Workers binding does not carry
and that exist only outside the Workers runtime. Implementing it would mean
a second client (an S3-shaped one, inside a Worker, carrying its own
secrets) that nothing here needs yet. It throws a `StorageError` naming
exactly this, instead of silently returning `""` the way `NoneStorage` does
— a *selected* vendor returning an empty URL would read as an R2 bug, not
an unimplemented method.

## Consequences

- `fid capability list --all` now shows `database` and `storage` as
  `selectable: none, d1` / `none, r2` — the first two contracts with more
  than one real option.
- A `worker-cloudflare` product can declare `database = "d1"` and
  `storage = "r2"` in `fiducial.toml`, bind `DB` and `BUCKET` in
  `wrangler.toml`, run `fid derive`, and get a working adapter with no
  further wiring — `createAdapters(env)` returns the real classes.
- The binding-name convention (`DB`, `BUCKET`) is new surface: `fid derive`
  cannot see a hand-edited `wrangler.toml`'s binding names, so the adapter
  reads fixed ones. Documented in both class doc comments and the
  capability's `SKILL.md`; a product binding under a different name passes
  a differently-shaped `env` or wraps the class.
- `ROADMAP.md`'s Cloudflare adapter set item is marked 🟡, not ✅ — two of
  seven named pieces are real, and the item's own name still promises five
  more. Marking it done would repeat exactly the failure Phase 16's review
  found and fixed: reporting a gate as satisfied when it was not.

## What this deliberately does not do

- No Workers-as-`deploy` adapter. `deploy` is documented as build-time/
  pipeline, not a runtime trait — what it would derive (a `wrangler deploy`
  invocation, environment promotion) is closer to a `fid release` extension
  than an object behind `Box<dyn Deploy>`, and needs its own design pass.
- No Access, Turnstile, Queues, or Workers AI adapter, and no new contract
  for any of them — see the table above. Each is real future work with no
  contract to attach to yet.
- No Rust-side D1/R2 client (Cloudflare's HTTP/S3 APIs from a Tauri
  backend) — no consumer.
- No `R2Storage.signedUrl` implementation — no consumer, and the honest
  answer is that it needs different credentials than the binding carries.
- No change to `Email` or `Diagnostics` — Resend and Sentry remain
  candidates, untouched by this phase.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

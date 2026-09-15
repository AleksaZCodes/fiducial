# Turnstile and Cloudflare Queues ship; Auth and AI are scoped, not built

**Date:** 2026-09-15
**Status:** accepted

---

## Context

`docs/specs/2026-09-15-cloudflare-adapter-set.md` shipped `d1` and `r2` and
deferred the rest of the roadmap's Cloudflare adapter set — Workers as
`deploy`, Access, Turnstile, Queues, Workers AI — because none of the last
four had a contract to attach to.

Asked directly which of those to build next, the founder named four
priorities in one message: **users and authentication, primarily Supabase**;
**Turnstile and Queues**; and **some way to run AI on the edge or connect to
an internet API**, explicitly correcting Workers AI as the wrong frame —
"we need to think about AI apps," not just Cloudflare's own inference
product.

Turnstile and Queues are two more items landing on the same "narrow
contract, real vendor" shape `d1`/`r2` already proved. Auth and AI are not —
each needs a contract designed from nothing, and each carries real risk of
being drawn wrong on the first attempt (Auth: leaking Supabase's session
model into every consumer; AI: collapsing chat completion and embeddings
into one contract because they were built at the same time). Scoped by
question to the founder before building:

- **Auth scope:** full flows (sign-up, sign-in incl. OAuth, sign-out,
  password reset, session read) — not session-verification-only. The
  product never touches a vendor SDK directly for auth, the same posture
  `database`/`storage` already have.
- **Session delivery:** both cookie-based SSR (web-next/web-svelte, matching
  Supabase's own SSR helper pattern) and bearer token (Tauri or a future
  mobile client).
- **AI use case:** conversational/agentic first — streaming chat completion,
  tool calls, system prompts. Embeddings/RAG is a real second use, not
  this round's.
- **Build order:** Turnstile and Queues first (this document); Auth after,
  as its own phase, because it is the larger design project and deserves
  design time separate from two small, low-risk items.

This document covers what shipped from that decision — Turnstile and
Queues — and records the Auth and AI scope decisions in `ROADMAP.md` so
they are not re-derived from this conversation later.

## Decision

Ship `botProtection` and `queue` as two new contracts, each with one real
vendor (`turnstile`, `cloudflare-queues`), following exactly the seam
`d1`/`r2` left: move from `candidates` to `implementations`, add a
`vendor_ts_class_and_path` match arm, nothing else changes for callers.

**`botProtection` / `Turnstile`:**

- Contract: `verify(token, remoteIp?) -> VerifyOutcome { success, challengeTs?,
  hostname? }` — the fields Turnstile, reCAPTCHA and hCaptcha's `siteverify`
  responses all share. No score field: reCAPTCHA v3 has one, Turnstile and
  hCaptcha do not, so it is not part of the narrow contract.
- `NoneBotProtection` **fails open** (`success: true` unconditionally) —
  documented loudly as a real security-relevant default, not a placeholder:
  choosing `none` means no protection, not "protection pending."
- `Turnstile` is a plain HTTPS POST to
  `https://challenges.cloudflare.com/turnstile/v0/siteverify`, authenticated
  by a secret (`env.TURNSTILE_SECRET_KEY`, set via `wrangler secret put`) —
  **not** a binding, unlike `d1`/`r2`. This is the first real vendor in the
  adapter system reached over plain HTTP rather than a Workers binding.

**`queue` / `CloudflareQueue`:**

- Contract: `send(body)` / `sendBatch(bodies)` — **producer side only, no
  consumer method.** Cloudflare Queues (like most managed queues fronting
  serverless compute) deliver by *invoking* the consumer — a Worker exports
  a `queue(batch, env)` handler — rather than the consumer polling. A
  `receive()` method would misdescribe delivery and have nothing to bind
  against on the one real vendor this contract has.
- `NoneQueue` discards silently, matching `NoneStorage`'s posture.
- `CloudflareQueue` reads `env.QUEUE` (binding-only, same boundary as
  `d1`/`r2`) and sends with `contentType: "bytes"` so a consumer gets the
  same `Uint8Array` back rather than Cloudflare's default JSON round-trip.

**Both are TypeScript-only, for two different reasons:**

| Vendor | Why Rust-side is unbuilt |
|---|---|
| `cloudflare-queues` | Structural — the producer binding exists only inside a Worker, same as `d1`/`r2`. |
| `turnstile` | Not structural — a plain HTTPS call is reachable from Rust. Unbuilt because every product shape here submits forms from TypeScript (a Next.js/SvelteKit server action or a Worker), so a Rust client has no consumer. |

The distinction matters for whoever adds the next contract: "binding-only"
and "no consumer yet" are different reasons to stop at TypeScript, and
conflating them would make a future genuinely-Rust-reachable vendor look
architecturally blocked when it is only unbuilt.

**`run_fid_adapters` refactored** from four hand-duplicated
import/class/field triples to a loop over `ADAPTER_SLOTS: &[(&str, &str)]`
— the `([adapters] key, AdapterSet field name)` pairs. Going from four
contracts to six by hand-adding two more copies of the same five-line block
was the signal to generalize; the table is the only place a seventh contract
would need a line added.

## Why not the alternatives

**A `receive`/pull method on `Queue`, for interface completeness.** Rejected
because it cannot be implemented against `cloudflare-queues` — the one real
vendor delivers by invocation, not polling — and a method nothing can
satisfy is worse than an honestly incomplete contract. `Database`'s `batch`
promises atomicity because every vendor behind it has a real primitive for
it; `Queue.sendBatch` explicitly does not, for the same reason in reverse.

**A `score` field on `VerifyOutcome`, since reCAPTCHA v3 has one.**
Rejected on the same "narrowest shared interface" rule the Rust module doc
already states for `Database`: a field only some vendors can fill is a
field that leaks one vendor's model into the contract. A reCAPTCHA v3
adapter can expose a score on its own type; the contract does not need to
carry a `None` for everyone else.

**Building Auth and AI in this same round**, since the founder named all
four. Rejected per the founder's own build-order choice — Auth is a larger
design project (full flows, two session models, a real vendor with real
security stakes) and deserves dedicated design attention rather than being
folded into a round whose actual proof point is "the small ones are cheap
when they land on an existing shape." Building it fast here risks exactly
the mistake Phase 23 already named: a contract drawn wrong because it was
designed under the schedule pressure of shipping three things at once.

## Consequences

- `fid capability list --all` now shows six adapter contracts with a real
  vendor path documented; `botProtection` and `queue` join `database` and
  `storage` as `selectable: none, <vendor>`.
- `ROADMAP.md`'s Cloudflare adapter set item stays 🟡 — four of seven named
  pieces are now real (`d1`, `r2`, `turnstile`, `cloudflare-queues`); Workers
  as `deploy`, Access, and Workers AI (reframed as a general AI contract,
  see below) remain.
- `ROADMAP.md` gains a new, explicitly-scoped **Auth** entry (Supabase-first,
  full flows, cookie + bearer sessions) and an **AI** entry (conversational/
  agentic first, Workers AI reframed as one candidate implementation of a
  vendor-neutral contract rather than the contract's namesake) — both ⬜,
  neither built, so the scoping conversation is not lost before the next
  session picks either up.
- 8 new Rust unit tests (`bot_protection.rs`, `queue.rs`), 11 new TypeScript
  tests, 2 new end-to-end CLI tests.

## What this deliberately does not do

- No Auth contract or Supabase Auth implementation — scoped in `ROADMAP.md`,
  built next as its own phase per the founder's stated order.
- No AI contract — scoped in `ROADMAP.md` as conversational/agentic-first;
  RAG/embeddings named as a real second use, not this round's.
- No Rust-side `Turnstile` — no consumer (see table above).
- No `receive`/consumer-side API for `Queue` — not implementable against the
  one real vendor; consuming stays a product's own exported handler.
- No `recaptcha`/`hcaptcha` implementation, no `sqs` implementation — both
  remain named `candidates`, untouched.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

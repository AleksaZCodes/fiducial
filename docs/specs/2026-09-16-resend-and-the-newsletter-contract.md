# Resend ships the `email` contract, and the list is a second contract

**Date:** 2026-09-16
**Status:** proposed

---

## Context

`email` has been a contract with one implementation — `NoneEmail`, which
returns an empty message ID and sends nothing — since the adapter system
landed. `docs/specs/2026-09-15-turnstile-and-queues.md` moved `botProtection`
and `queue` off that same starting line by giving each exactly one real
vendor. `email` is the next one, and the founder named the vendor: Resend.

**What made it urgent is not the contract, it is the newsletter.** The stated
requirement is "a newsletter of some sorts before the first product," and the
cost of delay is unusually legible: a landing page that ships without a
subscribe box does not collect zero addresses, it *discards* every visitor it
gets, permanently and silently. Every other item in the current round can be
built after the first product and lose only time. This one loses the audience
that existed while it was missing.

So this document covers two things that arrive together and are deliberately
kept apart in the design:

1. **`email = "resend"`** — a real vendor for the existing transactional
   contract, following the `turnstile` seam exactly.
2. **`newsletter`** — a *new* contract, because managing a list of people who
   asked to hear from you is not the same operation as sending one message to
   one person, and collapsing them would repeat the mistake the AI item in
   `ROADMAP.md` is explicitly holding back to avoid ("collapsing chat
   completion and embeddings into one contract because they were built at the
   same time").

**Evidence the separation is real, from the vendor itself.** Resend is
currently migrating from **Audiences** to **Segments** — a "Global Contacts"
model where a contact is team-global rather than scoped to one audience, with
`POST /contacts` at the top level and `segments`/`topics` replacing audience
membership. The audience-scoped routes (`GET /audiences/{audience_id}/contacts`,
`DELETE /audiences/{id}/contacts/{id}`) still work. Confirmed against current
documentation via context7 on 2026-09-16, not from training data — which would
have modelled the adapter around `audience_id` and been wrong within a year.

That is a list-shaped API changing shape while the transactional
`POST /emails` endpoint did not move at all. Two contracts, with independent
rates of change, is what that is.

## Decision

### 1 · `ResendEmail` implements `email`

- Plain HTTPS `POST https://api.resend.com/emails`, `Authorization: Bearer
  ${RESEND_API_KEY}`, JSON body. Returns the provider `id` as the contract's
  message ID.
- **Secret-reached, not binding-reached** — `env.RESEND_API_KEY`, set with
  `wrangler secret put`. Same boundary as `Turnstile`, unlike `d1`/`r2`.
- Constructor throws `EmailError` when the key is absent, with the
  `wrangler secret put` command in the message — matching `Turnstile`'s
  behaviour, so a misconfigured product fails at construction rather than at
  the first send.
- `fetchImpl` constructor parameter defaulting to global `fetch`, so tests
  stub rather than monkeypatch `globalThis`. Established by `Turnstile`.
- Maps Resend's `429` onto `EmailError` — the Rust contract already has a
  `RateLimited { retry_after_secs }` variant and Resend's documented default
  is 10 requests/second/team, so this is a real path, not a defensive one.

### 2 · `newsletter` is a new contract

```ts
interface Newsletter {
  subscribe(email: string, attrs?: SubscriberAttributes): Promise<Subscription>;
  unsubscribe(email: string): Promise<void>;
  status(email: string): Promise<Subscription | null>;
}
```

`Subscription` is `{ id: string; email: string; subscribed: boolean }` — the
intersection that Resend, Buttondown, Loops, Listmonk and Mailchimp all
report. `SubscriberAttributes` carries optional `firstName` / `lastName` only.

- **`subscribe` is idempotent.** Submitting the same address twice is the
  normal case for a landing page, not an error, and every vendor here treats
  re-adding an existing contact as an upsert or a benign conflict. A contract
  that surfaced "already subscribed" as a failure would push that handling
  into every caller.
- **`unsubscribe` sets the unsubscribed flag; it does not delete.** Deleting
  the record destroys the evidence that the person opted out, which is the
  one thing a suppression list exists to remember. Resend's `unsubscribed:
  true` is exactly this.
- **`NoneNewsletter` discards silently** and returns a plausible
  `Subscription` with an empty `id`. It matches `NoneQueue`'s posture, and
  the contrast with `NoneBotProtection` is deliberate: discarding a
  subscription is safe, whereas failing a bot check open is a real security
  position that had to be documented loudly.

### 3 · `ResendNewsletter` targets the current API, and is told which list

`env.RESEND_API_KEY` plus `env.RESEND_AUDIENCE_ID`, both read from `env` the
way `SupabaseAuth` reads `SUPABASE_URL` and `SUPABASE_ANON_KEY`. The list
identifier is **not** compiled into the adapter, which is what makes the
Audiences→Segments migration a configuration change rather than a code change.

### 4 · The mechanical seam

| Where | Change |
|---|---|
| `adapter::CONTRACTS` | `email`: `resend` moves `candidates` → `implementations`. New `newsletter` contract, `implementations: [none, resend]` |
| `ADAPTER_SLOTS` in `derive.rs` | one row: `("newsletter", "newsletter")` |
| `vendor_ts_class_and_path` | three arms: `email/resend`, `newsletter/none`, `newsletter/resend` |
| wrangler secret list | `RESEND_API_KEY` when `email = "resend"` **or** `newsletter = "resend"`; `RESEND_AUDIENCE_ID` when `newsletter = "resend"` |
| `AdapterSet` + `createNoneAdapters` | one field |
| `packages/adapters` | new `src/newsletter.ts`, new `./newsletter` export |
| `crates/fiducial-adapters` | new `src/newsletter.rs` — trait + `NoneNewsletter` only |

### 5 · Rust gets the contract and `none`, not the vendor

Exactly the `bot_protection` precedent, and for the same reason rather than
the other one: **not structural, no consumer.** A `POST` with a bearer token
is reachable from Rust; nothing in this repository would call it. Every
product shape that renders a subscribe form or sends a transactional message
does so from TypeScript. Recording *which* of the two reasons applies matters
— `cloudflare-queues` is Rust-unreachable by construction, `resend` is merely
unbuilt, and conflating those makes a future Rust consumer look blocked when
it is only unwritten.

## Why not the alternatives

**Put `subscribe` on the `email` contract.** It is one fewer contract and it
is wrong. `email` is designed against Resend, SES and Cloudflare Email
Routing; the latter two have no concept of a subscriber list at all, so the
method would be unimplementable on two of the three vendors the contract
exists to span. A contract method that most of its intended vendors cannot
satisfy is the "lock-in wearing a portability costume" the adapter module doc
warns about.

**Give `newsletter` a `send(broadcast)` method.** Rejected for this round.
Composing and sending a campaign differs across vendors in template model,
segmentation, scheduling and review — the parts that are genuinely
editorial — and there is no narrow intersection to draw yet. Subscribing is
the operation a product needs *in code on day one*; sending a broadcast is a
human act performed in a dashboard. Revisit when a second vendor and a real
scheduled-send requirement both exist, which is the second-use rule.

**A `[newsletter]` declaration block in `fiducial.toml` holding the audience
ID.** More correct in the long run and premature now. It would be the first
adapter whose configuration is a declaration rather than an env var, which is
a genuinely new shape for the derive pipeline to grow, and it buys nothing
until either a second vendor names its list differently or something other
than the adapter needs to read the ID. `env` matches `SupabaseAuth` today.

**Model the adapter on `audience_id` paths.** This is what training data
alone would have produced. Resend is actively moving off it.

**Build double opt-in here.** See below.

## Consequences

- A product gets a working subscribe path from `[adapters] newsletter =
  "resend"` and two secrets, with no vendor SDK in application code. The
  landing page can exist before the first product does, which is the point.
- `@fiducial/adapters` gains its seventh runtime contract. `ADAPTER_SLOTS`
  absorbing it as one table row is the payoff from the refactor recorded in
  the Turnstile spec; this is the first contract added since, and it is the
  test of that claim.
- **The secrets list in the generated `wrangler.toml` is now
  many-to-one.** `RESEND_API_KEY` is required by two different contracts, so
  the derivation must union rather than append, or a product selecting both
  `email = "resend"` and `newsletter = "resend"` gets the line twice.
- `fid doctor` starts accepting `email = "resend"`, which it previously
  rejected as an intended-but-unimplemented vendor.
- Resend's 10 req/s team-wide limit is now a shared budget between
  transactional sends and list writes. Nothing in this round coordinates
  them; a product that discovers the interaction is the signal to add
  something that does.

## What this deliberately does not do

- **No double opt-in, and no consent record.** Both belong to the **Legal &
  compliance** roadmap item, which owns consent records and cookie categories
  and depends on i18n and brand. Building a confirmation flow here means
  building it twice, which is the exact reason that item is sequenced after
  those. The contract does not preclude it: a confirmation step is a caller
  concern that ends in the same `subscribe` call. **`newsletter = "resend"`
  is not GDPR compliance, and nothing in this round claims to be** — that is
  the roadmap's own position on legal state versus code state, and it applies
  to the subscribe box more than to anything else the platform generates.
- **No broadcast send, no template rendering, no segment management.**
- **No newsletter *capability*.** `botProtection` and `queue` did not get one
  and neither does this. Contracts live in the `adapters` capability; a
  capability directory here would be surface with nothing in it.
- **No Rust `ResendEmail` / `ResendNewsletter`.**
- **No unsubscribe link generation.** Resend injects its own for broadcasts,
  and generating one for transactional mail requires a signed token scheme
  and a route to receive it — a real feature, not a line of adapter code.

---

<!--
Append-only (principle 1b). Superseding this means a new dated file, not an
edit to the reasoning above.
-->

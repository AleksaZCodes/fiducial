# Auth contract: full flows, Supabase-first, request-scoped by necessity

**Date:** 2026-09-15
**Status:** accepted

---

## Context

`docs/specs/2026-09-15-turnstile-and-queues.md` scoped, but deliberately did
not build, an Auth contract — the founder's stated top priority, named
directly: "we need users and authentication, primarily supabase." That
document recorded four decisions made before building anything:

- **Full flows**, not session-verification-only: sign-up, sign-in (password
  + OAuth), sign-out, password reset, session read. The product never
  touches the vendor SDK directly, matching `database`/`storage`'s posture.
- **Both session delivery models**: cookie-based SSR (web-next/web-svelte)
  and bearer token (Tauri or a future mobile client).
- **Supabase is the priority vendor.**
- **Built after Turnstile/Queues**, as its own phase, because it is the
  larger design project.

This document is that phase.

## Decision

Ship `Auth` as a seventh adapter contract, with `supabase` as its one real
vendor, TypeScript-only — and, unlike every contract before it, **not** a
field on `AdapterSet`/`createAdapters(env)`. It gets its own generated
factory, `createAuth(env, store)`, in the same file.

### Why `auth` cannot be `createAdapters(env)`'s seventh field

Every existing slot is **env-scoped**: a Workers binding or an API key lives
on `env` for the Worker's lifetime, and `new {Class}(env)` is enough to
build a working instance. A real `Auth` implementation cannot be: sign-in,
sign-out and session reads need somewhere to persist session material (and,
for OAuth, a PKCE verifier) **between requests** — an httpOnly cookie for a
server-rendered app, or nothing server-side at all for a bearer client that
owns its own token. That storage is request-scoped by nature, not
env-scoped, and forcing it into `new {Class}(env)` would mean either
silently dropping the store argument (breaking every real use) or corrupting
the uniform shape the other six slots rely on. `createAuth(env, store)` is
a second, honestly-different factory in the same generated file rather than
a seventh `AdapterSet` field — the file documents why inline, so a reader
does not have to guess it was an oversight.

### The contract

```ts
interface Auth {
  signUp(email, password): Promise<AuthSession>;
  signIn(email, password): Promise<AuthSession>;
  signInWithOAuth(provider, redirectTo): Promise<{ url: string }>;
  exchangeCodeForSession(code): Promise<AuthSession>;
  signOut(): Promise<void>;
  getSession(): Promise<AuthSession | null>;
  resetPasswordForEmail(email, redirectTo): Promise<void>;
  updatePassword(newPassword): Promise<void>;
}
```

No explicit `refreshSession` — `getSession()` verifies and refreshes as
needed internally, matching how Supabase's own SDK behaves when given a
working storage adapter, and avoiding an extra method whose only job would
be "do the thing `getSession` already does."

### `AuthKeyValueStore` — the storage extension point, not a Fiducial invention

`@supabase/supabase-js` itself defines `SupportedStorage`: a plain async
key/value interface (`getItem`/`setItem`/`removeItem`) it uses to persist
session and PKCE state, normally backed by `localStorage` or
`AsyncStorage`. `AuthKeyValueStore` mirrors that shape exactly. This is the
"narrowest shared interface" rule applied to *storage* rather than to the
auth operations — and it means `SupabaseAuth` needs no dependency on
`@supabase/ssr`'s own cookie adapter (which brings a specific multi-chunk
cookie serialization format built for a larger combined payload than this
contract's sessions need); it uses the same extension point `@supabase/ssr`
itself is built on, with Fiducial's own much simpler cookie encoding.

Two implementations ship:

- **`CookieKeyValueStore`** — one JSON-serialized cookie per key, against a
  framework-agnostic `{ get, set, delete }` the caller supplies. PKCE round
  trips correctly: the verifier written during `signInWithOAuth` survives to
  the callback request because it is in a cookie.
- **`BearerKeyValueStore`** — in-memory, one request's lifetime.
  `.fromAuthorizationHeader(header)` seeds it from an incoming bearer token.
  **Cannot** support the OAuth redirect flow — there is nowhere server-side
  to keep PKCE state between the redirect-out and callback requests without
  a cookie. `signInWithOAuth`/`exchangeCodeForSession` throw a clear
  `AuthError` under this store instead of silently losing the verifier
  (mid-flow data loss disguised as success would be the worse failure mode).
  A native/bearer client performs OAuth directly against Supabase itself and
  sends the resulting session to this API afterward — this server's role
  there is verification (`getSession`), not orchestrating a redirect it has
  no page to render.

### `SupabaseAuth`

Wraps `@supabase/supabase-js`'s plain `createClient` (not `@supabase/ssr`),
configured with `storage: store, persistSession: true, autoRefreshToken:
false, flowType: "pkce"` — the SDK's own documented extension point, not a
workaround. `autoRefreshToken: false` because refresh happens on-demand
inside `getSession()`'s call chain, not on a background timer a stateless
Worker cannot run anyway.

Every vendor error is mapped through `throwIfError`, which distinguishes
invalid credentials (HTTP 400 / `invalid_credentials`) and rate limiting
(HTTP 429) into named `AuthError`s with a clear message, falling through to
the raw vendor message for anything else — the same shape `Database`'s
`DatabaseError` and `Email`'s `EmailError` already use for provider errors.

### `NoneAuth` fails loudly, not silently

Unlike almost every other `None*` type in this system, `NoneAuth`'s write
methods **throw** rather than fabricating a fake session. `auth = "none"`
means "no auth vendor configured," and a product that has not chosen one
should find out immediately on the first sign-in attempt, not ship believing
it has working authentication. `signOut()` and `getSession()` still behave
like a real no-op (succeed / report signed-out) because there is nothing
false to claim there.

## Why not the alternatives

**Reusing `@supabase/ssr`'s cookie adapter directly**, instead of building
`AuthKeyValueStore`. Rejected: `@supabase/ssr` is built around chunked,
multi-cookie session serialization sized for a payload larger than this
contract's `AuthSession` needs, and depending on it would import that
package's specific cookie-naming convention as a permanent implementation
detail. Using the SDK's own lower-level `storage` extension point instead
gets the same correctness (real PKCE support, real refresh) with a much
smaller, self-owned serialization format.

**One `AuthSessionStore` interface shaped around `AuthSession` directly**
(read/write/clear a whole session object), considered before settling on
`AuthKeyValueStore`. Rejected because it cannot host PKCE-verifier storage
without inventing a second, parallel storage channel — the SDK needs to
stash an opaque string under its own key during `signInWithOAuth` and
retrieve it during `exchangeCodeForSession`, which a session-shaped store
has no slot for. The plain key/value shape handles both without inventing
anything.

**A single class handling both cookie and bearer delivery internally**
(branching on a "mode" flag). Rejected — `SupabaseAuth`'s actual behavior
already varies by store today (`signInWithOAuth`/`exchangeCodeForSession`
check `instanceof BearerKeyValueStore` and throw), so a mode flag would be a
second, redundant way of saying the same thing the store type already says.

**Supabase Auth via the general Cloudflare "Access" framing** the original
roadmap item used. Rejected per the founder's correction: Access is
Cloudflare's own zero-trust product, a different thing from a vendor-neutral
Auth contract with Supabase as its first vendor. The two are unrelated;
`ROADMAP.md` no longer conflates them.

## Consequences

- `fid capability list --all` shows `auth` as an eighth adapter contract,
  `selectable: none, supabase`.
- `fid derive` (with `adapters` installed) always emits both
  `createAdapters(env)` and `createAuth(env, store)` in
  `src/adapters.generated.ts`, regardless of whether `auth` is declared —
  same posture every other contract already has (undeclared defaults to
  `none`).
- **A real latent bug found and fixed in the process, unrelated to Auth
  itself:** `NoneEmail` and `NoneDiagnostics` had no explicit constructor
  (implicit zero-arg), so `new NoneEmail(env)` / `new NoneDiagnostics(env)`
  in the generated factory would fail to typecheck under `strict` — it had
  gone unnoticed since Phase 27 because nothing had ever actually run `tsc`
  against a generated `adapters.generated.ts`, only string-`contains()`
  checks in the Rust end-to-end tests. Manually verified this round (a
  scaffold with every real vendor selected, symlinked against the built
  package, `tsc --noEmit`) with zero errors after the fix. **Not** turned
  into an automated CI check in this phase — see below.
- `ROADMAP.md`'s Auth item moves from scoped-but-unbuilt to shipped.

## What this deliberately does not do

- **No automated typecheck of the generated factory in CI.**
  *(Closed 2026-09-15 by
  `docs/specs/2026-09-15-generated-code-is-compiled-not-matched.md`, which
  built exactly the suite sketched below.)* The bug above
  was caught by hand this round, not by a new test. Adding one properly
  (spinning up a scaffold, linking the built `@fiducial/adapters`, running
  `tsc --noEmit` from a Rust integration test) is real, valuable work in the
  same spirit as Phase 18's `ui-svelte` fix and Phase 21's generated
  captures, but it is a genuine new CI dependency edge (the JS package must
  be built before the Rust test suite runs) that deserves its own pass
  rather than being rushed in as a side effect of Auth. Recorded here so it
  is a known gap, not a silently accepted one.
- No Clerk or Auth.js implementation — both remain named `candidates`.
- No Rust-side `SupabaseAuth` — no Rust-reachable consumer, same reasoning
  as `turnstile`.
- No `refreshSession` method — folded into `getSession()`, see above.
- No multi-factor auth, no magic links, no organizations/teams — Supabase
  supports all three; none is in the founder's stated scope for this round,
  and each is additive to the existing contract shape when it is.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

# Identity at every level, and two real defects in the auth that shipped hours earlier

**Date:** 2026-09-15
**Status:** accepted

---

## Context

`docs/specs/2026-09-15-auth-contract.md` shipped the `Auth` contract with
`SupabaseAuth` behind it. Asked to make auth "robust and reusable, and go
hand in hand with identity on all levels," an audit of that same-day work
found two defects — both real, both shipped, one silent and one a security
hole:

**1. `getSession()` did not verify anything.** It called the SDK's own
`getSession()`, which reads the session out of the configured storage and
checks only `expires_at`. On a server, that storage is a *client-supplied
cookie*. A forged cookie containing a well-formed but unsigned session
object would have been returned as a valid session with whatever `user.id`
the caller wrote in it. Supabase's own documentation warns about exactly
this and says to use `getUser()` server-side; the shipped code did not.

**2. Bearer delivery never worked at all.** `BearerKeyValueStore`
`.fromAuthorizationHeader()` wrote the raw JWT under the key
`sb-access-token`. Reading `@supabase/auth-js`'s source: `__loadSession`
reads `getItemAsync(this.storage, this.storageKey)` where `storageKey`
defaults to `supabase.auth.token`, and expects a **JSON-serialized session
object**, not a bare token string. So the key was never read, and would have
been the wrong shape if it had been. Every bearer request's `getSession()`
returned `null` — silently, which is the worse kind: the feature appeared to
exist, had documentation and tests, and did nothing.

The second defect is instructive about the first: the shipped tests passed
because they asserted the adapter *called* the SDK, not that the call
produced a verified session. Both defects live in the gap between "the code
runs" and "the code is right."

Separately, the platform had three unrelated notions of *who*:
`fiducial_core::DeviceId` at L0 (firmware, `fiducial-protocol`,
`fiducial-ota`), a user UUID inside the `auth` adapter, and nothing at all
for a service calling another service. None could be compared to another, so
the question a real product actually asks — *may this actor do this to this
thing?* — had nowhere to live, and would have been answered separately, and
differently, at each level. That is what "identity on all levels" names.

## Decision

### Auth verifies, always

Every session `SupabaseAuth.getSession()` returns has had its access token's
signature checked, via the SDK's `getClaims()` — which verifies locally
against the project's JWKS when the signing key is asymmetric (no network
round trip at the edge) and falls back to a network `getUser()` validation
otherwise. Either way the signature is checked.

Cookie delivery additionally refuses a session whose **stored user id
disagrees with the verified token's subject**: the cookie carries both, and
trusting the user object without pinning it to the token it arrived with
would re-open the same hole one level down.

### Bearer delivery becomes a first-class thing, not a storage key

`AuthKeyValueStore` was right — it mirrors the SDK's own `SupportedStorage`
extension point, which is what PKCE and cookie persistence need. What was
wrong was pretending a bearer token is a *stored* value. The second
constructor argument is now an `AuthSessionContext`:

```ts
interface AuthSessionContext {
  readonly storage: AuthKeyValueStore;   // what the SDK persists into
  readonly bearerToken?: string | null;  // what this request supplied directly
}
```

`CookieSessionContext` and `BearerSessionContext` implement it.
`BearerSessionContext.fromAuthorizationHeader()` parses the token into a
field the adapter actually reads, and `getSession()` verifies *that* token
and builds a session from its verified claims — never consulting storage,
because under bearer delivery there is nothing in it.

`AuthUser.emailVerified` and `createdAt` became nullable, and
`AuthSession.refreshToken` too, because a session verified from a bearer JWT
genuinely does not carry them. `null` means "this delivery model cannot tell
you," which a caller can act on; a fabricated `false` or `""` would be a
claim about the user that is not true.

### `fiducial-identity` — one `Principal`, one rule, every level

A new `no_std` crate. `Principal` spans `User`, `Device`, `Service` and
`Anonymous`, and **reuses `fiducial_core::DeviceId`** rather than minting a
second device identity — principle 1, a fact declared once. `no_std` because
a device deciding "is this command from my owner?" offline must run the same
rule as the Worker deciding it at the edge.

The rule is one function:

```rust
can(principal, action, resource, grants) -> bool
```

Deny by default. Two rules hold before any grant is consulted:

1. **An unidentified principal is refused** — anonymous, or an all-zero
   sentinel id. This one matters concretely: a device that has not yet read
   its UID out of OTP would otherwise authenticate as "device zero," an
   identity *every* unprovisioned device shares.
2. **A device may read itself with no grant.** Requiring one would mean
   every device needs a provisioning round trip before it can say anything
   at all — including the "I am here, I am unclaimed" message provisioning
   itself depends on. It is a *read*: a device still may not write itself
   unguarded, or a compromised one could rewrite its own configuration and
   call it self-service.

`Grant { principal, resource, role }` is the user↔device link that had
nowhere to live before. `Role` is ordered `Viewer < Member < Admin < Owner`;
`Owner` is distinct from `Admin` because transfer and deletion are an
owner's to make and an installer holding `Admin` should not inherit them.

### The decision table is a conformance artifact

`can()` exists twice: Rust (firmware, desktop, CLI) and TypeScript
(`@fiducial/identity`, for the Worker and the app). Two implementations of
one rule is the drift this platform exists to prevent — and an authorization
rule is the worst possible place for it, because a divergence does not look
like a bug, **it looks like access**.

So the whole decision table is generated from the Rust crate into
`docs/identity/vectors.json` (2,130 lines, seven scenarios, every principal
probed against every action and resource) and replayed by *both* suites. CI
runs the generator without `FIDUCIAL_WRITE_VECTORS`, so a rule change that
skipped regeneration fails the build. Same mechanism
`docs/protocol/vectors.json` uses for the wire format.

**Verified adversarially, as that mechanism requires.** Changing one rule in
the TypeScript implementation (letting a device write itself, not just read
itself) fails 10 of the 167 TypeScript tests. Regressing `getSession()` to
its pre-fix unverified behavior fails 3 of the adapters tests. Both tables
discriminate; neither merely passes.

### The bridge is structural

`principalFromSession(session)` in TypeScript and `user_principal(&uuid)` in
Rust take the *id*, not the session type. So `@fiducial/identity` depends on
nothing and `fiducial-identity` stays `no_std` — and both work with any
vendor's session shape. An absent or unverified session becomes `ANONYMOUS`,
which `can()` refuses outright, so verification and authorization fail
closed together.

## Why not the alternatives

**Keeping `getSession()` as-is and telling callers to use `getUser()`.**
Rejected: the contract's job is to be right by default. A contract whose
correct use requires knowing which of two similar methods is the safe one
has moved the trap rather than removed it — and the trap here is invisible,
because the unsafe method returns exactly what the caller expected.

**Verifying with `getUser()` directly instead of `getClaims()`.** `getUser()`
is a network round trip on every request. `getClaims()` gets the same
guarantee locally when the project uses asymmetric signing keys, and falls
back to `getUser()` itself when it cannot — so it is never weaker and is
usually much faster at the edge. Choosing the slower one unconditionally
would be paying for a guarantee already available.

**A separate `UserIdentity` type in the auth adapter, rather than a shared
crate.** This is the choice that would have felt smaller and been wrong: it
puts the user's identity in the adapter layer, where firmware cannot reach
it, and leaves `DeviceId` unrelated to it forever. The first product feature
that asks "does this user own this device?" would then have to invent the
join, in one place, at one level, in a way the other levels could not check.

**A permissions string/scope model (`"device:write"`) instead of
`(Action, Resource)`.** Rejected because scope strings are parsed, and
parsing is where a `no_std` firmware implementation and a TypeScript one
diverge first. Two enums and a comparison have no parser to disagree about.

**A wildcard *principal* in `Grant`** ("everyone may read"). Rejected: that
is a policy decision, not an identity one, and allowing it would make every
grant table a potential blanket grant — one typo from public.

## Consequences

- Auth is fail-closed at both layers: an unverified token yields no session,
  and no session yields `ANONYMOUS`, which is refused.
- A rename landed one commit after the original: `BearerKeyValueStore` →
  `BearerSessionContext`, `CookieKeyValueStore` wrapped by
  `CookieSessionContext`, and `createAuth(env, store)` →
  `createAuth(env, ctx)`. Churn on a day-old API, accepted because the old
  shape could not express a working bearer path. Nothing had been published.
- `fiducial-identity` enters the 4-target `no_std` spine matrix
  automatically — that matrix derives its crate list by grepping for
  `#![no_std]`, so adding a crate needed no CI edit. The Phase 18
  derivation paying off.
- A new CI job, `identity`, runs the Rust rule and the TypeScript
  conformance replay.
- 19 Rust tests + 2 doctests, 167 TypeScript tests (most of them generated
  probes), 6 new adapter verification tests.

## What this deliberately does not do

- **No grant storage.** `can()` takes a grant table; where grants live —
  a D1 table, Postgres RLS policies, a device's flash — is a product's
  decision, and inventing a persistence layer here with no consumer is the
  speculative-contract mistake this platform has already corrected once.
- **No token issuance for devices or services.** `Principal::Device` and
  `Principal::Service` are identities the rule understands; how a device
  *proves* it is that device (a signed attestation, an mTLS cert, a
  provisioning secret) is real work with real cryptographic choices, and it
  belongs in its own pass with a product asking for it. `fiducial-ota`
  already signs images; that is the nearest existing precedent.
- **No automated typecheck of the generated factory in CI** — still the gap
  recorded in the auth spec, still caught by hand this round.
- **No roles beyond the four**, no per-field or row-level policies, no
  delegation or expiry on grants. Each is additive to the existing shape.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

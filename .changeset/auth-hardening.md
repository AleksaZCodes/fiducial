---
"@fiducial/adapters": patch
---

**Security fix.** `SupabaseAuth.getSession()` verified nothing — it returned
whatever the configured storage held, checking only `expires_at`. On a server
that storage is a client-supplied cookie, so a forged one was a valid session
with any `user.id` in it. Every path now verifies the access token's
signature via `getClaims()` (local JWKS check where the signing key is
asymmetric, network `getUser()` fallback otherwise), and cookie delivery also
refuses a session whose stored user disagrees with the verified token's
subject.

**Bearer delivery now works.** It previously never did: the token was written
under `sb-access-token`, a key `@supabase/auth-js` does not read (it reads a
JSON session from its own `storageKey`), so every bearer `getSession()`
silently returned `null`. `AuthSessionContext` replaces the bare store —
`CookieSessionContext` and `BearerSessionContext`, the latter carrying the
request's token in a field the adapter actually reads.

Breaking within this unreleased package: `BearerKeyValueStore` →
`BearerSessionContext`, `createAuth(env, store)` → `createAuth(env, ctx)`.
`AuthUser.emailVerified`/`createdAt` and `AuthSession.refreshToken` are now
nullable, because a session verified from a bearer JWT does not carry them.

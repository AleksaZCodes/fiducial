# @fiducial/adapters

## 0.2.0

### Minor Changes

- c42e400: `Auth` — a seventh adapter contract for users and authentication, with
  `SupabaseAuth` as its first real vendor (`@supabase/supabase-js`, new
  runtime dependency).
  
  Full flows: `signUp`, `signIn`, `signInWithOAuth`, `exchangeCodeForSession`,
  `signOut`, `getSession`, `resetPasswordForEmail`, `updatePassword`.
  `NoneAuth` fails loudly (throws) rather than fabricating a session —
  `auth = "none"` means no auth is configured.
  
  `Auth` is request-scoped, not env-scoped like every other contract here, so
  it is not part of `AdapterSet`. `fid derive` emits a second factory,
  `createAuth(env, store)`, alongside `createAdapters(env)`. `store` is an
  `AuthKeyValueStore` — mirrors `@supabase/supabase-js`'s own `SupportedStorage`
  extension point — with two implementations: `CookieKeyValueStore` (web-next/
  web-svelte, full OAuth/PKCE support) and `BearerKeyValueStore` (Tauri or a
  future mobile client; cannot carry PKCE state across an OAuth redirect
  without a cookie, so those two methods throw clearly under it instead).
  
  Select with `auth = "supabase"` in `[adapters]`.
- 17c5a9b: `D1Database` and `R2Storage` — the first real vendor implementations of the
  `database` and `storage` contracts, reached through Cloudflare Workers
  bindings (`env.DB`, `env.BUCKET`).
  
  `D1Database` wraps D1's `prepare().bind().run()/all()/first()` and
  `batch()`. `R2Storage` wraps `put`/`get`/`delete`/`list` (following R2's
  cursor across pages); `signedUrl` throws rather than silently returning an
  empty string, naming why: a presigned R2 URL needs SigV4 signing against the
  S3-compatible API with an R2 API token, which the binding does not carry.
  
  Select with `database = "d1"` / `storage = "r2"` in `[adapters]` —
  `fid derive` wires the real class in, no other change needed.
- fe70b81: Resend ships the `email` contract, and the subscriber list is a second one:
  
  - `ResendEmail` — the `email` contract's first real vendor. `POST /emails`
    over plain HTTPS with `env.RESEND_API_KEY`, the secret-reached boundary
    `Turnstile` established rather than the binding boundary `d1`/`r2` use.
  - `Newsletter` / `ResendNewsletter` — a **new contract**: `subscribe`,
    `unsubscribe`, `status`. Not a method on `email`, because SES and
    Cloudflare Email Routing have no subscriber list at all and `subscribe`
    would be unimplementable on two of the three vendors `email` spans.
  
  `subscribe` is idempotent — resubmitting an address is the normal case for a
  landing page, not an error. `unsubscribe` treats a contact the vendor does
  not have as success: the caller asked for "this person is not subscribed"
  and that state already holds. Every other failure still throws.
  
  No broadcast send, and no double opt-in or consent record — those belong to
  the Legal & compliance work, and `newsletter = "resend"` is not GDPR
  compliance.
  
  Select with `email = "resend"` / `newsletter = "resend"` in `[adapters]` —
  `fid derive` wires the real class in and names `RESEND_API_KEY` and
  `RESEND_AUDIENCE_ID` in the generated `wrangler.toml`.
- e0eed2b: Two new adapter contracts, each with a real Cloudflare vendor:
  
  - `BotProtection` / `Turnstile` — verifies a widget-solved token against
    Cloudflare's `siteverify` endpoint over plain HTTPS (`env.TURNSTILE_SECRET_KEY`).
    `NoneBotProtection` fails open — `botProtection = "none"` means no
    protection at all, not "protection pending."
  - `Queue` / `CloudflareQueue` — producer-side only (`send`/`sendBatch`),
    reached through `env.QUEUE`. No consumer method: Cloudflare Queues deliver
    by invoking an exported `queue(batch, env)` handler, not by polling.
  
  Select with `botProtection = "turnstile"` / `queue = "cloudflare-queues"` in
  `[adapters]` — `fid derive` wires the real class in, same as `d1`/`r2`.

### Patch Changes

- 614332f: **Security fix.** `SupabaseAuth.getSession()` verified nothing — it returned
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

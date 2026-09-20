# @fiducial/adapters

## 0.4.0

### Minor Changes

- b88d74f: A second kind of model, and an advisory layer that never gates.
  
  `SystemOne` is a new adapter contract for models that answer typed questions
  about a state and return calibrated probability distributions rather than text.
  TypeSafe's Jev is the first. It is a peer of `Ai`, not a method on it: `Ai` takes
  a turn history and returns prose or tool calls, and coercing typed decisions
  through that shape is the exact mismatch these models exist to remove.
  
  Two vendors, one wire shape. `OpenRouterSystemOne` posts to
  `/api/alpha/decisions` — not under `/api/v1`, not chat completions — which
  proxies to TypeSafe through a dedicated decisions router, so the typed-answer
  shape survives the gateway. `TypeSafeSystemOne` goes direct. They differ only in
  URL, secret, and whether the model id carries a `typesafe/` prefix. `openrouter`
  is the default because a product declaring `ai = "openrouter"` already holds the
  key, and one secret to rotate beats two.
  
  Three question types with differently-shaped answers: Noul (yes/no, returns a
  probability and deliberately no `confidence` — the value already is the
  distribution), Choice (one of N, with a distribution over all options), Score
  (a position on ordered levels, which can land between them). `noulOf`,
  `choiceOf` and `scoreOf` narrow an answer by key so a mixed-up question id is a
  caught error rather than a silent cast. Both vendors share one retry loop with
  exponential backoff, jitter, and `Retry-After`, because both APIs require
  backoff on 429 and 529 and neither ships that for free without its SDK.
  
  `@fiducial/advisor` is new: `fid-advise`, reached as `fid advise`. It reviews a
  diff for the rules an exact check cannot reach — a fact declared twice under two
  names, logic sitting above the layer it could reach, a comment explaining what
  instead of why. Rules that *are* decided exactly are deliberately not asked.
  Every check is a hybrid: code narrows candidates deterministically and the model
  judges only what survives.
  
  **`AdapterSet` gains a `systemOne` member.** Code that builds an `AdapterSet` by
  hand needs one more field; `createNoneAdapters()` and the factory `fid derive`
  generates are already updated.

## 0.3.0

### Minor Changes

- 9054bd8: Add the `ai` contract — conversational and agentic language-model calls —
  with `openrouter` as its first vendor.
  
  The contract targets an **AI gateway**, not one adapter per model vendor.
  There is no honest intersection of the Anthropic and OpenAI shapes: `system`
  is top-level in one and a message role in the other, content blocks meet a
  parts array, streaming events differ substantially, and three parameters
  changed shape within a year. Intersecting those by hand yields a contract too
  thin to write an agent against; supersetting them picks a vendor without
  admitting it.
  
  - `Ai` — `chat()` for a completed response, `stream()` for an
    `AsyncIterable<StreamEvent>`. Messages, a top-level system prompt, tool
    definitions, tool calls, tool results, stop reasons and usage.
  - `OpenRouterAi` — secret-reached (`env.OPENROUTER_API_KEY`), so it works
    from any runtime with `fetch`.
  - `NoneAi` **throws** rather than returning an empty completion, joining
    `NoneAuth`. The rule, now that there are two: a no-op send is
    indistinguishable from a real one at the call site, but a completion *is*
    the result — fabricating one turns "no vendor selected" into a blank answer
    in the UI, debugged as a model bug.
  
  Streamed tool calls are emitted whole. Gateways stream tool arguments as
  partial JSON, which a consumer can do nothing with but buffer, so the adapter
  buffers once.
  
  `AdapterSet` gains an `ai` field, and `createNoneAdapters()` fills it.
  
  Select with `ai = "openrouter"` in `[adapters]` and declare `[ai] model` —
  `fid derive` passes the model into the generated factory and names
  `OPENROUTER_API_KEY` in the derived `wrangler.toml`. A per-call
  `request.model` overrides the declared default.

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

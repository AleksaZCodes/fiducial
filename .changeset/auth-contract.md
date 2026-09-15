---
"@fiducial/adapters": minor
---

`Auth` — a seventh adapter contract for users and authentication, with
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

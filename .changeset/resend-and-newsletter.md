---
"@fiducial/adapters": minor
---

Resend ships the `email` contract, and the subscriber list is a second one:

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

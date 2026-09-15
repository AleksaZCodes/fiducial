---
"@fiducial/adapters": minor
---

Two new adapter contracts, each with a real Cloudflare vendor:

- `BotProtection` / `Turnstile` — verifies a widget-solved token against
  Cloudflare's `siteverify` endpoint over plain HTTPS (`env.TURNSTILE_SECRET_KEY`).
  `NoneBotProtection` fails open — `botProtection = "none"` means no
  protection at all, not "protection pending."
- `Queue` / `CloudflareQueue` — producer-side only (`send`/`sendBatch`),
  reached through `env.QUEUE`. No consumer method: Cloudflare Queues deliver
  by invoking an exported `queue(batch, env)` handler, not by polling.

Select with `botProtection = "turnstile"` / `queue = "cloudflare-queues"` in
`[adapters]` — `fid derive` wires the real class in, same as `d1`/`r2`.

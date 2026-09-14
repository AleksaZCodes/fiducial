# Cloudflare is the default target

**Date:** 2026-09-14
**Status:** accepted

---

## Decision

Cloudflare is Fiducial's **default** deployment and infrastructure target.
Vercel, Supabase, Neon and others remain fully supported as adapters.

## Why a default at all

Principle 6 says *commit to contracts, not to tools* — which is an argument for
adapters, not against defaults. A system with no default is a system that asks
you to make an infrastructure decision before you have a product, every time.
The default is what `fid new` assumes when you have not said otherwise.

## Why Cloudflare

It is the only vendor on the list that covers the whole surface under one
account and one generous free tier:

| Need | Cloudflare |
|---|---|
| Compute | Workers |
| Database | D1 |
| Storage | R2 (no egress fees) |
| Auth / access | Access |
| Bot protection | Turnstile |
| Queues, cron | Queues, Triggers |
| AI | Workers AI |
| Networking, firewall, CDN, DNS | included |

Two reasons beyond the feature matrix:

**It reaches the hardware.** Fiducial is a cross-domain platform with
`fiducial-ota` — signed, resumable firmware updates. Those images have to be
*served* from somewhere, to devices, possibly over LoRa gateways. Cloudflare is
the only candidate that plausibly spans the web app and the device fleet. No
other vendor on the list is even in that conversation.

**R2 has no egress fees.** For firmware distribution — the same few hundred
kilobytes fetched by every device in a fleet — egress is the cost that matters,
and it is the one most vendors charge hardest for.

## What "default" does and does not mean

**Does:** `fid new` assumes Cloudflare. The Cloudflare adapters are first-party,
best-tested, and documented as the happy path.

**Does not:** contracts are still designed vendor-neutrally, against the
narrowest plausible implementation. Principle 6 is not suspended for a vendor we
happen to like. If a contract can only be satisfied by Cloudflare, the contract
is wrong.

Stated plainly because it is the exact failure mode of a default: a default is a
starting point, not a lock-in, and the way you find out you got it wrong is that
the second adapter is impossible to write.

## Acknowledged trade-off

Next.js has first-class support on Vercel, and that is a real advantage for
Next-based products. The `vercel` deploy adapter is therefore first-party too,
not a second-class citizen — both are used in practice.

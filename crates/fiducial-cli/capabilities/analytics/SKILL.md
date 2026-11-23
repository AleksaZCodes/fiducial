---
name: fiducial-analytics
covers:
  - "fiducial.toml#analytics"
  - "apps/web/src/generated/analytics.ts"
  - "apps/web/src/app/api/e/route.ts"
  - "apps/web/src/lib/analytics-capture.ts"
---

# fiducial:analytics — One contract, many providers, cookieless by default

This product has the `analytics` capability.

## The contract

Site code calls only `track(event)` and `pageview(ctx)` from
`generated/analytics.ts`. The generated file fans out to every configured
provider. Adding or removing a provider is an edit to `[analytics] providers`
in `fiducial.toml` followed by `fid derive`; no site file changes.

## Default providers

| Provider | Requires consent | What it stores |
|---|---|---|
| `first-party` | No | One D1 row per event: visitor hash (daily-rotating HMAC, no raw IP), event type, path, UTM params, referrer, timestamp |
| `cloudflare-web-analytics` | No | Nothing — cookieless beacon, processed by Cloudflare |

## Optional providers

| Provider | Requires consent | What it stores |
|---|---|---|
| `posthog` | No | Nothing on the device; `cookieless_mode` always; EU host |
| `meta-pixel-capi` | Yes — requires `consent` capability | Hashed email/phone via Meta Conversions API |

A `requires_consent` provider in `[analytics] providers` without the `consent`
capability installed fails `fid derive` and is reported by `fid doctor`.

## First-party endpoint

`apps/web/src/app/api/e/route.ts` — an unauthenticated POST that writes one
analytics row to D1. The endpoint is same-origin checked and schema-validated.
The visitor hash is `HMAC-SHA256(ANALYTICS_HMAC_SECRET, date + ip + ua)`,
truncated to 128 bits and computed with WebCrypto. No raw IP is stored.

Set the secret: `wrangler secret put ANALYTICS_HMAC_SECRET`

## UTM capture

`apps/web/src/lib/analytics-capture.ts` — captures the landing path, UTM
parameters and referrer host into `sessionStorage` at page load. This is
strictly necessary (no consent needed): session storage is not a cookie, it
is scoped to the tab, and it is used only to attach attribution to events and
to the order/quote row at submit — not for tracking across sessions.

## Configuration

```toml
[analytics]
providers = ["first-party", "cloudflare-web-analytics"]
retention_days = 396

[analytics.cloudflare-web-analytics]
token_env = "CF_BEACON_TOKEN"
```

`retention_days` (default 396 = 13 months) derives the scheduled delete cron
and the retention sentence in the generated privacy notice.

## Retention

`[analytics] retention_days` (default 396 = 13 months) controls how long
first-party event rows are kept. `fid derive` generates
`generated/analytics-retention.ts` with `ANALYTICS_RETENTION_DAYS` and
`cleanupAnalyticsEvents(db)`. The cleanup route lives at
`apps/web/src/app/api/analytics-cleanup/route.ts`.

If `[deploy]` is not yet declared when analytics is added, the capability seeds
`[deploy] crons = ["0 0 * * *"]` (daily at midnight UTC) and
`[deploy] secrets` with `ANALYTICS_CRON_SECRET`. If `[deploy]` is already
declared, add `crons = ["0 0 * * *"]` to that block and add
`"ANALYTICS_CRON_SECRET"` to its `secrets` array manually.

**Calling the cleanup from a cron trigger.** Cloudflare scheduled triggers
run the Worker's `scheduled` export. In an OpenNext app, add a custom Worker
entrypoint that:
1. Delegates `fetch` to OpenNext's handler as normal.
2. In `scheduled`, calls:
   ```typescript
   await fetch(new URL('/api/analytics-cleanup', origin), {
     method: 'POST',
     headers: { 'x-cron-secret': env.ANALYTICS_CRON_SECRET },
   })
   ```

Set the secret: `wrangler secret put ANALYTICS_CRON_SECRET`

**Restore procedure.** There is no on-platform restore for first-party events.
The D1 database is backed up nightly (commit 9 in the Payload series). To
restore after accidental deletion: apply the latest R2 snapshot with
`wrangler d1 execute --file <snapshot.sql>`, then run a manual cleanup
pass to remove any events beyond the retention window.

## What an agent should do

- Use `createAnalytics(env)` from `generated/analytics.ts` in route handlers
  that want to record events. Never import a provider class directly.
- Call `initAnalyticsCapture()` from `analytics-capture.ts` in the root
  layout's client component.
- To add a provider: add its id to `[analytics] providers` and run `fid derive`.
  Never edit `generated/analytics.ts` by hand.
- To change the retention window: edit `[analytics] retention_days` and run
  `fid derive`. Both `analytics-retention.ts` and the privacy notice update
  together; `fid derive --check` catches a stale file.
- To add a custom event type: add it to `[analytics] events` (not built yet;
  see the spec open questions).

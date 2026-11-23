---
"@fiducial/adapters": minor
---

Add `Analytics` contract with typed event schema and four provider classes.

`Analytics` interface with `track(event)` and `pageview(ctx)`. Typed event
union: `page_view`, `cta_click`, `add_to_cart`, `checkout_start`,
`quote_request`, `order_submitted`. Providers: `NoneAnalytics`,
`FirstPartyAnalytics`, `CloudflareWebAnalytics`, `PostHogAnalytics`,
`MetaPixelCapi`. Exported from `@fiducial/adapters/analytics`.

# Products send preload-eligible HSTS, and submit per hostname — never the apex

**Date:** 2026-09-20
**Status:** accepted
**Supersedes:** nothing

---

## Context

`fire.outreachnet.work` was serving **no security headers at all**. Measured
against the live response on 2026-09-20:

```
$ curl -sSI https://fire.outreachnet.work
HTTP/2 200
server: cloudflare
x-powered-by: Next.js          ← announces framework and version
                               ← no strict-transport-security
                               ← no content-security-policy
                               ← no x-content-type-options
                               ← no referrer-policy
                               ← no permissions-policy
```

Nothing was misconfigured. Nothing had ever been configured: the `web-next`
capability's `next.config.ts` template carried `reactStrictMode` and nothing
else, so every product scaffolded from it shipped the same way. A missing header
is invisible — no build fails, no page breaks, and the gap is only visible to
someone who thinks to look at a response.

HSTS specifically has a second-order problem. Serving the header protects
returning visitors, but the *first* request to a host a browser has never seen
can still go out over plain HTTP, and that request is where a downgrade attack
lands. The preload list closes it by shipping the pin inside the browser. Getting
on that list requires `max-age` ≥ 1 year plus `includeSubDomains`, and getting
*off* it takes months.

## Decision

**The template serves a preload-eligible HSTS header:**
`max-age=63072000; includeSubDomains; preload`.

Two years, subdomains included. A shorter `max-age` is not a cautious version of
this — it is one that hstspreload.org rejects, so the header would carry the
`preload` directive as a lie.

**The `preload` directive is a declaration of willingness, not an act.** Nothing
is preloaded until a hostname is submitted at hstspreload.org. That submission is
a deliberate step taken per product, by a person, after reading this file.

**Submit the specific hostname. Never submit the apex.** `outreachnet.work` with
`includeSubDomains` would pin every present and future subdomain to HTTPS inside
every updated browser — including ones that do not exist yet, and including any
that turn out to need plain HTTP for a device, a redirect, or a local
integration. The blast radius of the apex is every name under it, forever, and
the recovery path is a removal request plus however long browser release cycles
take. Submitting `fire.outreachnet.work` costs one hostname's worth of the same
commitment.

**Before submitting a hostname, confirm every name under it terminates TLS.**
`includeSubDomains` is the part that breaks things, and it breaks them silently
for anyone whose browser already has the pin.

## Why not the alternatives

**Cloudflare Transform Rules in the dashboard.** Fastest, and it is what a
dashboard is for. Rejected on principle 4: a header set there is a fact living
outside the repository — not reviewable, not revertible, not diffable, invisible
to an agent, and absent in any second environment. It also drifts from the code
that assumes it, with nothing to notice.

**A `_headers` file.** Works for static assets on Cloudflare Pages, and does not
cover responses the Next.js server function produces — which is most of a page.
Splitting one fact across two mechanisms by which part of the response it lands
on is the kind of split this platform exists to remove.

**Per-product config instead of the template.** This is the whole argument for
the platform: a header set is a fact every web product needs and none should
have to rediscover. Declared once in the capability, derived into every product,
and `fid upgrade` carries a later correction to all of them. A product that needs
something different extends `securityHeaders` rather than forking the file.

**A nonce-based CSP now.** Correct, and deferred. Next.js emits an inline
bootstrap script per page, so a policy without `'unsafe-inline'` on `script-src`
serves a blank page; the fix is generating a nonce per request in middleware and
threading it through. Shipping the rest of the policy now — `object-src 'none'`,
`base-uri 'self'`, `form-action 'self'`, `frame-ancestors 'none'` — closes the
injected-`<base>`, off-site-form-post and clickjacking holes today. Waiting for
the nonce work would have shipped none of it.

**`max-age=0` first, as a staged rollout.** The usual advice, and it earns its
caution when a site has mixed-content or subdomain unknowns. Here the site is
already HTTPS-only behind Cloudflare with no plain-HTTP dependency, so the staged
version buys nothing and leaves the first-request window open for however long
the stage lasts.

## Consequences

- Every product scaffolded from `web-next` gets these headers, and every existing
  one gets them at its next `fid upgrade` — the template is tracked in
  `fiducial.lock`, so the change 3-way-merges into a product that edited the
  file.
- `X-Powered-By` stops being sent (`poweredByHeader: false`).
- A product serving media from object storage inherits `img-src … https:`, which
  is broader than ideal. Narrowing it needs the bucket hostname, which is a
  product-level fact this template does not have.
- **`script-src` keeps `'unsafe-inline'` until nonce middleware exists.** Written
  down here so it is a known debt rather than an assumed-solved problem.
- A submitted hostname is effectively permanent. That is the point, and it is why
  submission is not automated.

## What this deliberately does not do

- **Does not submit anything to hstspreload.org.** Serving the directive and
  joining the list are separate acts, and the second one is a person's to take.
- **Does not preload the apex**, and no product should — see above.
- **Does not add nonce middleware**, so the CSP is not yet strict against script
  injection. Stated plainly rather than implied by a green header grade.
- **Does not touch Cloudflare configuration.** Everything here is in the
  repository, which is the point.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

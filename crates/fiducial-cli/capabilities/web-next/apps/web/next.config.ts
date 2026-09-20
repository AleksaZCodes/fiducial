import type { NextConfig } from "next";

/**
 * Response headers every page carries.
 *
 * Declared here rather than in a Cloudflare dashboard rule, because a header set
 * in a dashboard is a fact that lives outside the repository: it cannot be
 * reviewed, reverted, diffed, or known about by an agent, and it silently does
 * not exist in a second environment. Principle 4.
 *
 * Exported so a product can extend rather than fork this file — principle 3, no
 * abstraction without an escape hatch:
 *
 *   headers: async () => [{ source: "/:path*", headers: [...securityHeaders, mine] }]
 */
export const securityHeaders = [
  /**
   * Two years, subdomains included, and preload-eligible.
   *
   * `max-age` must be at least a year and `includeSubDomains` must be present
   * for hstspreload.org to accept a submission, so a shorter value is not a
   * cautious version of this — it is an ineligible one.
   *
   * **The `preload` directive here does not preload anything.** It only declares
   * willingness; the site must be submitted at hstspreload.org, and that is a
   * deliberate, slow-to-undo step. Read
   * `docs/specs/2026-09-20-hsts-preload.md` before submitting — removal takes
   * months and browsers keep the pin until they next update, so a subdomain
   * that later needs plain HTTP is simply unreachable in the meantime.
   */
  {
    key: "Strict-Transport-Security",
    value: "max-age=63072000; includeSubDomains; preload",
  },

  /**
   * Stop the browser guessing a type the server did not send.
   *
   * The attack this closes is an upload served back as `text/html` because its
   * bytes looked like markup — sniffing turns a stored file into stored XSS.
   */
  { key: "X-Content-Type-Options", value: "nosniff" },

  /**
   * Send the origin cross-site, the full path same-site, and nothing over
   * plain HTTP.
   *
   * The default in most browsers is already this, but "already the default" is
   * not a guarantee — it is a default, and it has changed before.
   */
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },

  /**
   * Framing is denied twice, on purpose.
   *
   * CSP `frame-ancestors` is the modern rule and supersedes this one — but
   * `X-Frame-Options` is what an older browser understands, and clickjacking
   * does not care which header a victim's browser failed to implement.
   */
  { key: "X-Frame-Options", value: "DENY" },

  /**
   * Deny the capability set a content site has no use for.
   *
   * Written as an explicit deny-list rather than omitted: an omitted directive
   * means "whatever the browser's default is", and defaults move toward
   * permissive when a feature becomes popular.
   */
  {
    key: "Permissions-Policy",
    value: [
      "accelerometer=()",
      "camera=()",
      "geolocation=()",
      "gyroscope=()",
      "magnetometer=()",
      "microphone=()",
      "payment=()",
      "usb=()",
    ].join(", "),
  },

  /**
   * Sever the window reference a popup opener keeps.
   *
   * Without it, a page this site opens — or that opens it — can reach back
   * through `window.opener`.
   */
  { key: "Cross-Origin-Opener-Policy", value: "same-origin" },

  /**
   * Content Security Policy.
   *
   * **`'unsafe-inline'` on `script-src` is a real weakness and is here
   * knowingly.** Next.js emits an inline bootstrap script on every page, so a
   * policy without it serves a blank page. Removing it means generating a nonce
   * per request in middleware and threading it through — worth doing, and not
   * something to pretend is already done.
   *
   * The directives that are *not* compromised still close the common holes, and
   * they are the reason this ships now rather than after the nonce work:
   *
   *   object-src 'none'      — no Flash/Java/plugin embedding
   *   base-uri 'self'        — an injected <base> cannot re-root every relative URL
   *   form-action 'self'     — an injected form cannot post credentials off-site
   *   frame-ancestors 'none' — clickjacking, per the header above
   *
   * `img-src` allows any HTTPS origin because media is served from object
   * storage whose hostname is a product-level fact this template does not know.
   * A product that knows its bucket hostname should narrow this.
   */
  {
    key: "Content-Security-Policy",
    value: [
      "default-src 'self'",
      "script-src 'self' 'unsafe-inline'",
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob: https:",
      "font-src 'self' data:",
      "connect-src 'self'",
      "media-src 'self' https:",
      "object-src 'none'",
      "base-uri 'self'",
      "form-action 'self'",
      "frame-ancestors 'none'",
      "upgrade-insecure-requests",
    ].join("; "),
  },
];

const config: NextConfig = {
  // Strict mode catches common React mistakes early.
  reactStrictMode: true,

  /**
   * Do not announce the framework and version.
   *
   * `X-Powered-By: Next.js` is free reconnaissance: it tells a scanner which
   * CVE list to try. It is not a vulnerability on its own, which is exactly why
   * it survives — nobody files a bug for it.
   */
  poweredByHeader: false,

  async headers() {
    return [{ source: "/:path*", headers: securityHeaders }];
  },
};

export default config;

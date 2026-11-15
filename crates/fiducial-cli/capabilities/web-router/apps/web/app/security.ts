/**
 * Response headers every page carries, set by the Worker (`workers/app.ts`).
 *
 * Declared here rather than in a Cloudflare dashboard rule, because a header set
 * in a dashboard is a fact that lives outside the repository: it cannot be
 * reviewed, reverted, diffed, or known about by an agent. The same names, with
 * the same values, as the Next.js template and the self-hosted Caddyfile; a test
 * in the platform fails when the three drift.
 *
 * `Strict-Transport-Security` carries no `preload`: that is a deliberate,
 * slow-to-undo step. See `docs/specs/2026-09-20-hsts-preload.md`.
 */
export const responseHeaders: ReadonlyArray<readonly [string, string]> = [
  ["Strict-Transport-Security", "max-age=63072000; includeSubDomains"],
  ["X-Content-Type-Options", "nosniff"],
  ["Referrer-Policy", "strict-origin-when-cross-origin"],
  ["X-Frame-Options", "DENY"],
  [
    "Permissions-Policy",
    "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()",
  ],
  ["Cross-Origin-Opener-Policy", "same-origin"],
  [
    "Content-Security-Policy",
    "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https:; font-src 'self' data:; connect-src 'self'; media-src 'self' https:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests",
  ],
];

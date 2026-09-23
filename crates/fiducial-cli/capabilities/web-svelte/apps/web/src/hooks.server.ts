import type { Handle } from "@sveltejs/kit";

/**
 * Response headers for every request this app serves.
 *
 * `fid doctor` checks that a SvelteKit product declares the platform's
 * required set, and this is where a SvelteKit product declares it — one hook,
 * covering every route including `+server.ts` endpoints. A per-route or
 * per-layout `setHeaders` does not: a `load` function never runs for an API
 * route, so the endpoints end up as the only unprotected surface, which is
 * the one an attacker is most interested in.
 */
const SECURITY_HEADERS: Record<string, string> = {
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Permissions-Policy": "geolocation=(), microphone=(), camera=()",
  "Cross-Origin-Opener-Policy": "same-origin",
  "Strict-Transport-Security": "max-age=63072000; includeSubDomains; preload",

  // `Content-Security-Policy` is deliberately NOT here. It is declared in
  // `svelte.config.js` under `kit.csp`, and the reason is not organisational.
  //
  // SvelteKit boots the client from an *inline* `<script>` it generates into
  // the body. A CSP written by hand in this file cannot know that script's
  // hash, so `script-src 'self'` blocks it, the app never hydrates, and every
  // interactive component silently stops working while the server-rendered
  // HTML still looks perfect. That shipped on a live product: a countdown
  // frozen at 00:00:00 and a language picker that would not open, with the
  // build, the typecheck and `fid doctor` all green.
  //
  // `kit.csp` lets SvelteKit derive the hash from the bytes it actually
  // emitted. Adding CSP back here would re-break it even if the directives
  // looked identical: a browser intersects two CSP headers, so the stricter
  // one wins and the hash is lost.
};

export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);
  for (const [key, value] of Object.entries(SECURITY_HEADERS)) {
    response.headers.set(key, value);
  }
  return response;
};

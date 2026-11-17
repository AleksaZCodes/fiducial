/**
 * Studio's sign-in with GitHub: the one piece that cannot live in the browser.
 *
 * GitHub's OAuth web flow ends with the app exchanging a `code` for a token,
 * and that exchange needs the app's **client secret**. A static page must not
 * hold one, so this Worker does the exchange and nothing else: it keeps no
 * session, stores no token, and hands the token back to Studio in the URL
 * fragment (which a browser never sends to any server, this one included).
 *
 *   Studio ──► /login?return=<studio url> ──► github.com/login/oauth/authorize
 *   github ──► /callback?code&state ──► [exchange, secret stays here]
 *   Worker ──► <studio url>#gh_token=…
 *
 * Two defences, both small:
 *
 * - **`return` is an allowlist** (`ALLOWED_ORIGINS`). Without it the Worker would
 *   be an open redirect that appends a user's token to any URL.
 * - **`state` is bound to the browser** by a cookie. Without that, an attacker
 *   could start a flow, then get a victim's browser to finish it, signing the
 *   victim into the attacker's account (login CSRF).
 *
 * Prefer a **GitHub App** over an OAuth App: its user tokens are limited to the
 * repositories and permissions (Contents and Pull requests, read and write) the
 * installation grants, where an OAuth App's `repo` scope is every repository.
 */

export interface Env {
  GITHUB_CLIENT_ID: string;
  GITHUB_CLIENT_SECRET: string;
  /** Comma-separated origins Studio is served from, e.g. `https://studio.example.com`. */
  ALLOWED_ORIGINS: string;
  /** Scopes to request. Empty for a GitHub App, which ignores them. */
  OAUTH_SCOPE?: string;
}

const COOKIE = "__Host-studio_state";
const encoder = new TextEncoder();

function b64url(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromB64url(s: string): Uint8Array {
  const pad = s.length % 4 === 0 ? "" : "=".repeat(4 - (s.length % 4));
  const bin = atob(s.replace(/-/g, "+").replace(/_/g, "/") + pad);
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

async function sign(secret: string, data: string): Promise<string> {
  const key = await crypto.subtle.importKey("raw", encoder.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return b64url(new Uint8Array(await crypto.subtle.sign("HMAC", key, encoder.encode(data))));
}

function allowed(env: Env, returnUrl: string): URL | null {
  let url: URL;
  try {
    url = new URL(returnUrl);
  } catch {
    return null;
  }
  const origins = env.ALLOWED_ORIGINS.split(",").map((o) => o.trim()).filter(Boolean);
  return origins.includes(url.origin) ? url : null;
}

function text(status: number, body: string): Response {
  return new Response(body, { status, headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" } });
}

export async function handle(request: Request, env: Env, fetchImpl: typeof fetch = fetch): Promise<Response> {
  const url = new URL(request.url);

  if (url.pathname === "/login") {
    const ret = allowed(env, url.searchParams.get("return") ?? "");
    if (!ret) return text(400, "return URL is not an allowed origin");
    const nonce = b64url(crypto.getRandomValues(new Uint8Array(16)));
    const payload = b64url(encoder.encode(JSON.stringify({ r: ret.toString(), n: nonce, e: Date.now() + 10 * 60_000 })));
    const state = `${payload}.${await sign(env.GITHUB_CLIENT_SECRET, payload)}`;
    const to = new URL("https://github.com/login/oauth/authorize");
    to.searchParams.set("client_id", env.GITHUB_CLIENT_ID);
    to.searchParams.set("redirect_uri", `${url.origin}/callback`);
    to.searchParams.set("state", state);
    if (env.OAUTH_SCOPE) to.searchParams.set("scope", env.OAUTH_SCOPE);
    return new Response(null, {
      status: 302,
      headers: {
        location: to.toString(),
        "set-cookie": `${COOKIE}=${nonce}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=600`,
        "cache-control": "no-store",
      },
    });
  }

  if (url.pathname === "/callback") {
    const code = url.searchParams.get("code");
    const state = url.searchParams.get("state") ?? "";
    const [payload, mac] = state.split(".");
    if (!code || !payload || !mac) return text(400, "missing code or state");
    if ((await sign(env.GITHUB_CLIENT_SECRET, payload)) !== mac) return text(400, "state was not issued here");
    let parsed: { r: string; n: string; e: number };
    try {
      parsed = JSON.parse(new TextDecoder().decode(fromB64url(payload)));
    } catch {
      return text(400, "unreadable state");
    }
    if (Date.now() > parsed.e) return text(400, "sign-in took too long; start again");
    const cookie = /(?:^|;\s*)__Host-studio_state=([^;]+)/.exec(request.headers.get("cookie") ?? "")?.[1];
    if (cookie !== parsed.n) return text(400, "this browser did not start the sign-in");
    const ret = allowed(env, parsed.r);
    if (!ret) return text(400, "return URL is not an allowed origin");

    const res = await fetchImpl("https://github.com/login/oauth/access_token", {
      method: "POST",
      headers: { accept: "application/json", "content-type": "application/json" },
      body: JSON.stringify({ client_id: env.GITHUB_CLIENT_ID, client_secret: env.GITHUB_CLIENT_SECRET, code, redirect_uri: `${url.origin}/callback` }),
    });
    const data = (await res.json().catch(() => ({}))) as { access_token?: string; error_description?: string };
    if (!data.access_token) return text(502, `GitHub refused the code: ${data.error_description ?? "no token returned"}`);

    ret.hash = `gh_token=${encodeURIComponent(data.access_token)}`;
    return new Response(null, {
      status: 302,
      headers: {
        location: ret.toString(),
        // The nonce did its job; do not leave it lying around.
        "set-cookie": `${COOKIE}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0`,
        "cache-control": "no-store",
        "referrer-policy": "no-referrer",
      },
    });
  }

  return text(404, "Studio sign-in: /login and /callback only");
}

export default { fetch: (request: Request, env: Env) => handle(request, env) };

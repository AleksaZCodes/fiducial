import assert from "node:assert/strict";
import { test } from "node:test";
import { handle, type Env } from "./index.ts";

const env: Env = {
  GITHUB_CLIENT_ID: "cid",
  GITHUB_CLIENT_SECRET: "s3cret",
  ALLOWED_ORIGINS: "https://studio.example.com, http://127.0.0.1:7777",
};
const W = "https://auth.example.com";

async function login(ret: string) {
  return handle(new Request(`${W}/login?return=${encodeURIComponent(ret)}`), env);
}

function cookieOf(res: Response): string {
  return res.headers.get("set-cookie")!.split(";")[0];
}

const githubOk = (async () => new Response(JSON.stringify({ access_token: "gho_abc" }))) as typeof fetch;

test("login sends the browser to GitHub with a signed state and a cookie", async () => {
  const res = await login("https://studio.example.com/");
  assert.equal(res.status, 302);
  const to = new URL(res.headers.get("location")!);
  assert.equal(to.origin + to.pathname, "https://github.com/login/oauth/authorize");
  assert.equal(to.searchParams.get("client_id"), "cid");
  assert.equal(to.searchParams.get("redirect_uri"), `${W}/callback`);
  assert.match(res.headers.get("set-cookie")!, /^__Host-studio_state=.+HttpOnly; Secure; SameSite=Lax/);
  assert.ok(!to.toString().includes("s3cret"), "the secret never leaves the Worker");
});

test("a return URL outside the allowlist is refused (no open redirect)", async () => {
  for (const bad of ["https://evil.example.com/", "javascript:alert(1)", "not a url", ""]) {
    assert.equal((await login(bad)).status, 400, bad);
  }
});

test("the full round trip hands the token back in the fragment, never the query", async () => {
  const start = await login("https://studio.example.com/app/");
  const state = new URL(start.headers.get("location")!).searchParams.get("state")!;
  const back = await handle(
    new Request(`${W}/callback?code=c0de&state=${encodeURIComponent(state)}`, { headers: { cookie: cookieOf(start) } }),
    env,
    githubOk,
  );
  assert.equal(back.status, 302);
  const to = new URL(back.headers.get("location")!);
  assert.equal(to.origin + to.pathname, "https://studio.example.com/app/");
  assert.equal(to.search, "");
  assert.equal(to.hash, "#gh_token=gho_abc");
  assert.match(back.headers.get("set-cookie")!, /Max-Age=0/);
});

test("the exchange sends the secret to GitHub and nowhere else", async () => {
  const start = await login("https://studio.example.com/");
  const state = new URL(start.headers.get("location")!).searchParams.get("state")!;
  let sent: { url: string; body: string } | null = null;
  const spy = (async (u: string, init: RequestInit) => {
    sent = { url: String(u), body: String(init.body) };
    return new Response(JSON.stringify({ access_token: "t" }));
  }) as unknown as typeof fetch;
  await handle(new Request(`${W}/callback?code=x&state=${encodeURIComponent(state)}`, { headers: { cookie: cookieOf(start) } }), env, spy);
  assert.equal(sent!.url, "https://github.com/login/oauth/access_token");
  assert.match(sent!.body, /s3cret/);
});

test("a callback from a browser that did not start the sign-in is refused (login CSRF)", async () => {
  const start = await login("https://studio.example.com/");
  const state = new URL(start.headers.get("location")!).searchParams.get("state")!;
  const noCookie = await handle(new Request(`${W}/callback?code=x&state=${encodeURIComponent(state)}`), env, githubOk);
  assert.equal(noCookie.status, 400);
  const wrong = await handle(
    new Request(`${W}/callback?code=x&state=${encodeURIComponent(state)}`, { headers: { cookie: "__Host-studio_state=other" } }),
    env,
    githubOk,
  );
  assert.equal(wrong.status, 400);
});

test("a forged or altered state is refused", async () => {
  const start = await login("https://studio.example.com/");
  const state = new URL(start.headers.get("location")!).searchParams.get("state")!;
  const [payload] = state.split(".");
  for (const bad of ["x.y", `${payload}.AAAA`, "nodot", ""]) {
    const res = await handle(new Request(`${W}/callback?code=x&state=${encodeURIComponent(bad)}`, { headers: { cookie: cookieOf(start) } }), env, githubOk);
    assert.equal(res.status, 400, bad);
  }
});

test("when GitHub refuses the code, the error is shown and no token is sent anywhere", async () => {
  const start = await login("https://studio.example.com/");
  const state = new URL(start.headers.get("location")!).searchParams.get("state")!;
  const refuse = (async () => new Response(JSON.stringify({ error_description: "bad_verification_code" }))) as typeof fetch;
  const res = await handle(new Request(`${W}/callback?code=x&state=${encodeURIComponent(state)}`, { headers: { cookie: cookieOf(start) } }), env, refuse);
  assert.equal(res.status, 502);
  assert.match(await res.text(), /bad_verification_code/);
  assert.equal(res.headers.get("location"), null);
});

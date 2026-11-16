import assert from "node:assert/strict";
import { test } from "node:test";
import { b64decode, b64encode, githubSource } from "./github.ts";

interface Call { method: string; url: string; body?: Record<string, unknown> }

/** A GitHub that answers from a script and records what it was asked. */
function fake(answers: Record<string, unknown>) {
  const calls: Call[] = [];
  const f = (async (url: string, init?: RequestInit) => {
    const u = String(url).replace("https://api.github.com", "");
    const method = init?.method ?? "GET";
    calls.push({ method, url: u, body: init?.body ? JSON.parse(String(init.body)) : undefined });
    const key = Object.keys(answers).find((k) => `${method} ${u}`.startsWith(k));
    if (!key) return new Response(JSON.stringify({ message: `no answer for ${method} ${u}` }), { status: 404 });
    return new Response(JSON.stringify(answers[key]), { status: 200 });
  }) as typeof fetch;
  return { f, calls };
}

const NO_SCHEMAS = { fiducial: {}, topology: {} };
const base = { owner: "o", repo: "r", branch: "main", token: "t" };

test("base64 round-trips non-ASCII text", () => {
  assert.equal(b64decode(b64encode("šđčćž — ok")), "šđčćž — ok");
});

test("a commit-mode save writes straight to the branch, with the read sha as its lock", async () => {
  const { f, calls } = fake({
    "GET /repos/o/r/contents/a.toml": { content: b64encode("x = 1"), sha: "abc" },
    "PUT /repos/o/r/contents/a.toml": { content: { sha: "def" } },
  });
  const s = githubSource(base, NO_SCHEMAS, f);
  assert.equal(await s.read("a.toml"), "x = 1");
  await s.write("a.toml", "x = 2");
  const put = calls.find((c) => c.method === "PUT")!;
  assert.equal(put.body!.branch, "main");
  assert.equal(put.body!.sha, "abc");
  assert.equal(s.review, undefined);
});

test("pr mode makes one work branch from the base head and reuses it", async () => {
  const { f, calls } = fake({
    "GET /repos/o/r/git/ref/heads/main": { object: { sha: "head1" } },
    "POST /repos/o/r/git/refs": {},
    "PUT /repos/o/r/contents/": { content: { sha: "n" } },
    "POST /repos/o/r/pulls": { html_url: "https://github.com/o/r/pull/9" },
  });
  const s = githubSource({ ...base, mode: "pr" }, NO_SCHEMAS, f);
  assert.equal(s.review!.branch(), null);
  await s.write("a.toml", "1");
  await s.write("b.toml", "2");
  const refs = calls.filter((c) => c.url.endsWith("/git/refs"));
  assert.equal(refs.length, 1, "one branch, not one per save");
  const work = s.review!.branch()!;
  assert.match(work, /^studio\//);
  assert.equal(refs[0].body!.sha, "head1");
  for (const put of calls.filter((c) => c.method === "PUT")) assert.equal(put.body!.branch, work);
  const url = await s.review!.open("Edit declarations");
  assert.equal(url, "https://github.com/o/r/pull/9");
  const pull = calls.find((c) => c.url.endsWith("/pulls"))!;
  assert.deepEqual([pull.body!.head, pull.body!.base], [work, "main"]);
});

test("pr mode reads from the work branch once there is one, so a person sees their own edits", async () => {
  const { f, calls } = fake({
    "GET /repos/o/r/git/ref/heads/main": { object: { sha: "h" } },
    "POST /repos/o/r/git/refs": {},
    "PUT /repos/o/r/contents/": { content: { sha: "n" } },
    "GET /repos/o/r/contents/a.toml": { content: b64encode("v"), sha: "s" },
  });
  const s = githubSource({ ...base, mode: "pr" }, NO_SCHEMAS, f);
  await s.read("a.toml");
  assert.match(calls.at(-1)!.url, /ref=main$/);
  await s.write("a.toml", "v2");
  await s.read("a.toml");
  assert.match(decodeURIComponent(calls.at(-1)!.url), /ref=studio\//);
});

test("opening a pull request with nothing saved is refused", async () => {
  const s = githubSource({ ...base, mode: "pr" }, NO_SCHEMAS, fake({}).f);
  await assert.rejects(() => s.review!.open("x"), /nothing saved/);
});

test("secrets and bulk directories are never listed", async () => {
  const { f } = fake({
    "GET /repos/o/r/git/trees/main": {
      tree: ["a.toml", ".env", "x/.env.local", "node_modules/p/i.json", "apps/web/dist/a.js", "k.pem", "README.md"].map((path) => ({ path, type: "blob" })),
    },
  });
  assert.deepEqual(await githubSource(base, NO_SCHEMAS, f).files(), ["a.toml", "README.md"]);
});

test("a GitHub error surfaces as its own message", async () => {
  await assert.rejects(() => githubSource(base, NO_SCHEMAS, fake({}).f).read("zzz"), /no answer/);
});

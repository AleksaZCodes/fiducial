import assert from "node:assert/strict";
import { test } from "node:test";
import { app } from "./app";

// A Hono app is `fetch`, so a route is tested by calling it: no server, no port.

test("healthz answers ok", async () => {
  const res = await app.request("/healthz");
  assert.equal(res.status, 200);
  assert.deepEqual(await res.json(), { ok: true });
});

test("hello greets by name, and the world by default", async () => {
  const named = await app.request("/api/hello?name=Ada");
  assert.deepEqual(await named.json(), { message: "Hello, Ada" });
  const anon = await app.request("/api/hello");
  assert.deepEqual(await anon.json(), { message: "Hello, world" });
});

test("an unknown route is a 404, not a crash", async () => {
  const res = await app.request("/nope");
  assert.equal(res.status, 404);
});

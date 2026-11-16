import assert from "node:assert/strict";
import { test } from "node:test";
import { loadConfig, signInUrl } from "./config.ts";

const reply = (body: unknown, status = 200) => (async () => new Response(JSON.stringify(body), { status })) as unknown as typeof fetch;

test("a missing or broken config means no sign-in Worker, not a crash", async () => {
  assert.deepEqual(await loadConfig(reply({}, 404)), { authUrl: "" });
  assert.deepEqual(await loadConfig((async () => { throw new Error("offline"); }) as unknown as typeof fetch), { authUrl: "" });
  assert.deepEqual(await loadConfig(reply({ authUrl: 5 })), { authUrl: "" });
});

test("the Worker URL is trimmed and the return address is encoded", async () => {
  assert.equal((await loadConfig(reply({ authUrl: "https://a.example.com/" }))).authUrl, "https://a.example.com");
  assert.equal(signInUrl("https://a.example.com", "https://s.example.com/x?y=1"), "https://a.example.com/login?return=https%3A%2F%2Fs.example.com%2Fx%3Fy%3D1");
});

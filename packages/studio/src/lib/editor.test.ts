import assert from "node:assert/strict";
import { test } from "node:test";
import { deployedUrl, editorLink } from "./editor.ts";
import { parseToml } from "./toml.ts";

const cfg = (t: string) => parseToml(t);
const gh = { owner: "o", repo: "r", branch: "dev" };

test("no [cms] block means no editor, and says why", () => {
  assert.equal(editorLink({ config: cfg(""), canStartDev: true }).kind, "none");
});

test("Pages CMS opens on the connected repository and branch", () => {
  const l = editorLink({ config: cfg('[cms]\neditor = "pages"'), github: gh, canStartDev: false });
  assert.deepEqual(l, { kind: "open", label: "Open in Pages CMS", url: "https://app.pagescms.org/o/r/dev" });
});

test("Pages CMS with no repository known asks for one", () => {
  assert.equal(editorLink({ config: cfg('[cms]\neditor = "pages"'), canStartDev: true }).kind, "none");
  const l = editorLink({ config: cfg('[cms]\neditor = "pages"\nrepo = "a/b"'), canStartDev: true });
  assert.equal(l.kind === "open" && l.url, "https://app.pagescms.org/a/b/main");
});

test("hosted Keystatic is the deployed site's /keystatic", () => {
  const l = editorLink({ config: cfg('[brand]\ndomain = "example.com"\n[cms]\nstorage = "github"'), canStartDev: false });
  assert.equal(l.kind === "open" && l.url, "https://example.com/keystatic");
  assert.equal(editorLink({ config: cfg('[cms]\nstorage = "github"'), canStartDev: false }).kind, "none");
});

test("local Keystatic starts the dev server, then opens it", () => {
  const config = cfg("[cms]\n");
  assert.deepEqual(editorLink({ config, canStartDev: true }), { kind: "start", label: "Start Keystatic", path: "keystatic" });
  const open = editorLink({ config, canStartDev: true, devUrl: "http://localhost:4321/" });
  assert.equal(open.kind === "open" && open.url, "http://localhost:4321/keystatic");
});

test("Tina is /admin locally and Tina Cloud over GitHub", () => {
  const config = cfg('[cms]\neditor = "tina"');
  const local = editorLink({ config, canStartDev: true, devUrl: "http://localhost:4321/" });
  assert.equal(local.kind === "open" && local.url, "http://localhost:4321/admin");
  assert.equal(editorLink({ config, github: gh, canStartDev: false }).kind, "open");
});

test("without a machine, a local editor cannot be offered", () => {
  assert.equal(editorLink({ config: cfg("[cms]\n"), canStartDev: false }).kind, "none");
});

test("the deployed site is the brand domain, else the deploy route", () => {
  assert.equal(deployedUrl(cfg('[brand]\ndomain = "https://x.com/"')), "https://x.com");
  assert.equal(deployedUrl(cfg('[deploy]\nroute = "y.com/*"')), "https://y.com");
  assert.equal(deployedUrl(cfg("")), null);
});

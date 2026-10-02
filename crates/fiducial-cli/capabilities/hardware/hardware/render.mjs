#!/usr/bin/env node
// hardware/build/{scene.json, *.stl} -> the review pack, hardware/build/review/:
//
//   assembled.png  exploded.png  section.png  inside.png  underside.png  board.png   what changed, at a glance
//   viewer.html    one self-contained file: orbit, explode, section
//
// Platform-owned (installed by `fid add capability hardware`). Needs the
// `three` and `playwright-core` packages and a Chromium; set CHROMIUM_PATH if
// Playwright's own is not installed.
//
// This is the open loop the hardware capability is built around: a physical
// design is not reviewed from numbers. Every build ends here, and an agent that
// changed geometry shows these pictures to a person before it goes on.

import { createServer } from "node:http";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const build = resolve(here, "build");
const out = join(build, "review");
const threeRoot = resolve(dirname(fileURLToPath(import.meta.resolve("three"))), "..");
const viewer = readFileSync(join(here, "viewer.html"), "utf8");
const localViewer = viewer.replace(
  /<script type="importmap">[\s\S]*?<\/script>/,
  '<script type="importmap">{"imports":{"three":"/_three/build/three.module.js","three/addons/":"/_three/examples/jsm/"}}</script>',
);

const types = { ".html": "text/html", ".js": "text/javascript", ".json": "application/json", ".stl": "model/stl" };
const server = createServer((req, res) => {
  const path = decodeURIComponent(new URL(req.url, "http://x").pathname);
  if (path === "/" || path === "/viewer.html") {
    res.writeHead(200, { "content-type": "text/html" });
    return res.end(localViewer);
  }
  const file = path.startsWith("/_three/") ? join(threeRoot, path.slice(8)) : join(build, path);
  if (!existsSync(file)) {
    res.writeHead(404);
    return res.end();
  }
  res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
  res.end(readFileSync(file));
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = `http://127.0.0.1:${server.address().port}/viewer.html`;

async function launch() {
  const { chromium } = await import("playwright-core");
  const args = ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"];
  const candidates = [process.env.CHROMIUM_PATH, undefined];
  for (const dir of ["/opt/pw-browsers"]) {
    if (!existsSync(dir)) continue;
    for (const d of readdirSync(dir).filter((d) => /^chromium-\d+$/.test(d)).sort().reverse()) {
      candidates.push(join(dir, d, "chrome-linux", "chrome"));
    }
  }
  let last;
  for (const executablePath of candidates) {
    if (executablePath === null) continue;
    try {
      return await chromium.launch({ executablePath, args });
    } catch (e) {
      last = e;
    }
  }
  throw new Error(`no Chromium could be launched; set CHROMIUM_PATH. Last error: ${last?.message?.split("\n")[0]}`);
}

mkdirSync(out, { recursive: true });
const browser = await launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
// Loaded once; each view is a camera move, not a reload of every mesh.
await page.goto(`${base}?view=assembled&shot=1`);
await page.waitForFunction(() => window.__ready === true, null, { timeout: 120000 });
for (const view of ["assembled", "exploded", "section", "inside", "underside", "board"]) {
  await page.evaluate((v) => new Promise((done) => {
    window.setView(v);
    requestAnimationFrame(() => requestAnimationFrame(done));
  }), view);
  await page.screenshot({ path: join(out, `${view}.png`) });
  console.log(`wrote ${join("hardware/build/review", view + ".png")}`);
}
await browser.close();
server.close();
if (errors.length) {
  console.error(errors.join("\n"));
  process.exit(1);
}

// The standalone viewer: every mesh inlined, three from a CDN.
const scene = JSON.parse(readFileSync(join(build, "scene.json"), "utf8"));
const checks = existsSync(join(build, "checks.json")) ? readFileSync(join(build, "checks.json"), "utf8") : "null";
const stl = Object.fromEntries(scene.parts.map((p) => [p.stl, readFileSync(join(build, p.stl)).toString("base64")]));
const inline = `<script>window.__SCENE__=${JSON.stringify(scene)};window.__CHECKS__=${checks};window.__STL__=${JSON.stringify(stl)};</script>`;
writeFileSync(join(out, "viewer.html"), viewer.replace('<script type="importmap">', `${inline}\n<script type="importmap">`));
console.log("wrote hardware/build/review/viewer.html");

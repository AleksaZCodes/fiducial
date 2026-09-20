// `pnpm cms` — the content editor, against this working tree.
//
// Three things run together, because each on its own is a trap:
//
//   1. `decap-server`, the proxy that reads and writes the repository's files.
//      Without it the editor loads and every save fails.
//   2. A static server for this directory. `file://` will not do: the editor
//      fetches config.yml, which a file:// page may not read.
//   3. A watcher that re-runs `fid derive` when content changes. Saving an
//      entry writes markdown, and the site renders from `generated/*`, so an
//      edit without a derive shows nothing and leaves the repo stale — which
//      CI then fails on, long after the person editing has gone.
//
// Everything it writes is an ordinary file change: review it with `git diff`
// and commit it. There is no database and no sync step, which is the whole
// reason the editor can be this thin.
import { spawn } from "node:child_process";
import { createReadStream, existsSync, mkdirSync, readFileSync, watch } from "node:fs";
import { createServer } from "node:http";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const repo = fileURLToPath(new URL("../..", import.meta.url));
const PORT = Number(process.env.CMS_PORT ?? 8080);
const PROXY_PORT = Number(process.env.CMS_PROXY_PORT ?? 8081);

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".yml": "text/yaml; charset=utf-8",
  ".svg": "image/svg+xml",
};

// The upload folder has to exist before the editor asks for it: the media
// library lists a directory, and a directory that has never been written to
// makes the Media tab look broken rather than empty.
const uploads = join(repo, "media", "site/uploads");
mkdirSync(uploads, { recursive: true });

const proxy = spawn("npx", ["decap-server"], {
  cwd: repo,
  env: { ...process.env, PORT: String(PROXY_PORT) },
  stdio: "inherit",
  shell: process.platform === "win32",
});

// The origin the media bucket is served from, for previewing an image that is
// in the bucket but not staged locally. Read from the derived brand module, so
// the domain is still declared exactly once.
const brand = join(repo, "apps/web/src/generated/brand.ts");
const siteOrigin = existsSync(brand)
  ? `https://${readFileSync(brand, "utf8").match(/export const domain = "([^"]+)"/)?.[1]}`
  : null;

const send = (res, file) => {
  res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
  createReadStream(file).pipe(res);
};

const site = createServer((req, res) => {
  const path = new URL(req.url, "http://localhost").pathname;
  const rel = path.replace(/^\/+/, "");

  // The editor's own files.
  const file = join(here, path === "/" ? "index.html" : rel);
  if (file.startsWith(here) && existsSync(file)) {
    send(res, file);
    return;
  }

  // The site's icon, so the tab is not a broken square.
  if (path === "/favicon.svg") {
    const icon = join(repo, "apps/web/public/favicon.svg");
    if (existsSync(icon)) {
      send(res, icon);
      return;
    }
  }

  // A media key. The editor asks for a stored value as a path — `cover:
  // site/blog/flame-cover.png` becomes a request for `/site/blog/flame-cover.png`
  // — so serve it from the staging directory, and fall back to the live bucket
  // for anything not staged. Without the fallback, every image already
  // uploaded shows as a broken thumbnail while you edit.
  const staged = join(repo, "media", rel);
  if (staged.startsWith(join(repo, "media")) && existsSync(staged)) {
    send(res, staged);
    return;
  }
  if (siteOrigin && rel && !rel.startsWith("_")) {
    res.writeHead(302, { location: `${siteOrigin}/media/${rel}` }).end();
    return;
  }

  res.writeHead(404).end("Not found");
});

// Debounced: one save writes several files, and `fid derive` on each of them
// would queue a derive per file and report failures for half-written entries.
let timer;
let running = false;
function derive() {
  clearTimeout(timer);
  timer = setTimeout(() => {
    if (running) return;
    running = true;
    const p = spawn("fid", ["derive"], { cwd: repo, stdio: "inherit", shell: true });
    p.on("close", () => {
      running = false;
    });
  }, 600);
}

for (const dir of ["content", "press"]) {
  const abs = join(repo, dir);
  if (existsSync(abs)) watch(abs, { recursive: true }, derive);
}

site.listen(PORT, () => {
  console.log(`\n  content editor   http://localhost:${PORT}`);
  console.log(`  local backend    http://localhost:${PROXY_PORT}`);
  console.log("  saves write files in this repo; review them with `git diff`\n");
});

const stop = () => {
  proxy.kill();
  site.close();
  process.exit(0);
};
process.on("SIGINT", stop);
process.on("SIGTERM", stop);

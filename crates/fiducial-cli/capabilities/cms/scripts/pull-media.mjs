// `pnpm media:pull` — stage the media that content already refers to.
//
// The mirror of `media:push`. Push sends what is in `media/` to the bucket;
// this brings back what the repository's content points at and does not have
// locally.
//
// ## Why it exists
//
// The content editor's media library is the `media/` directory. Open an entry
// whose image was uploaded from another machine — or before `media/` was ever
// populated — and the editor asks its proxy for a file that is not there. The
// result is a 500 in the console and an empty thumbnail, on an entry that is
// perfectly correct and renders fine in production.
//
// Nothing is wrong in that state, which is exactly why it is worth fixing: the
// editor looks broken and the content is not.
//
// ## What it pulls
//
// The keys content actually uses: every `media` field in every entry, and
// every inline image in a body. Not the whole bucket — there is no listing
// call here, and a bucket may hold assets no page refers to.
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseToml } from "./derive-content.mjs";
import { frontmatter } from "./frontmatter.mjs";

const repo = fileURLToPath(new URL("..", import.meta.url));
const at = (p) => join(repo, p);
const read = (p) => readFileSync(at(p), "utf8");

const cfg = parseToml(read("fiducial.toml"));
const bucket = cfg.deploy?.r2_bucket_name;
if (!bucket) {
  console.error("fiducial.toml: [deploy] r2_bucket_name is not set — nothing to pull from.");
  process.exit(1);
}
const force = process.argv.includes("--all");

/** Every markdown file under a directory. */
function markdown(dir) {
  const abs = at(dir);
  if (!existsSync(abs)) return [];
  return readdirSync(abs).flatMap((name) => {
    const p = join(dir, name);
    return statSync(at(p)).isDirectory() ? markdown(p) : p.endsWith(".md") ? [p] : [];
  });
}

// Which frontmatter fields hold a key is a fact `content.toml` already states.
const collections = parseToml(read("content.toml")).collections ?? {};
const mediaFields = new Set(["cover", "key"]);
for (const spec of Object.values(collections)) {
  for (const [field, type] of Object.entries(spec.schema ?? {})) {
    if (type === "media") mediaFields.add(field);
  }
}

const keys = new Set();
for (const file of [...markdown("content"), ...markdown("press")]) {
  const src = read(file);
  const { meta, body } = frontmatter(src, () => {});
  for (const [field, value] of Object.entries(meta)) {
    if (mediaFields.has(field) && typeof value === "string" && value) keys.add(value);
  }
  // Inline images in a body: `![alt](key "caption")`.
  for (const m of body.matchAll(/!\[[^\]]*\]\(([^\s)]+)(?:\s+"[^"]*")?\)/g)) {
    if (!/^(https?:)?\/\//.test(m[1])) keys.add(m[1]);
  }
}

if (!keys.size) {
  console.log("no media keys in content — nothing to pull.");
  process.exit(0);
}

let pulled = 0;
let have = 0;
let missing = 0;
for (const key of [...keys].sort()) {
  const dest = join(repo, "media", key);
  if (existsSync(dest) && !force) {
    have++;
    continue;
  }
  mkdirSync(dirname(dest), { recursive: true });
  const r = spawnSync(
    "npx",
    ["wrangler", "r2", "object", "get", `${bucket}/${key}`, "--file", dest, "--remote"],
    { cwd: join(repo, "apps/web"), stdio: ["ignore", "ignore", "pipe"] },
  );
  if (r.status !== 0 || !existsSync(dest)) {
    // A key content refers to that the bucket does not have is worth saying
    // out loud: it renders as a broken image on the live site too.
    console.error(`  ×  ${key} — not in ${bucket}`);
    missing++;
    continue;
  }
  console.log(`  ✓  ${key}`);
  pulled++;
}

// Staged files match the bucket, so record them as pushed: otherwise the next
// `media:push` would upload everything it just downloaded.
if (pulled) {
  const recordPath = join(repo, ".media-pushed.json");
  const record = existsSync(recordPath) ? JSON.parse(readFileSync(recordPath, "utf8")) : {};
  const { createHash } = await import("node:crypto");
  for (const key of keys) {
    const file = join(repo, "media", key);
    if (existsSync(file)) {
      record[key] = createHash("sha256").update(readFileSync(file)).digest("hex");
    }
  }
  writeFileSync(recordPath, `${JSON.stringify(record, null, 2)}\n`);
}

console.log(`\n${pulled} pulled, ${have} already staged${missing ? `, ${missing} missing` : ""}`);

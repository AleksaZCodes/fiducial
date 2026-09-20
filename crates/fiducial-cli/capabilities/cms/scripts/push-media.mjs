// `pnpm media:push` — send the local media staging directory to the bucket.
//
// Media is not in git (see docs/media.md). `media/` is where files wait on
// their way to object storage: an image dropped in by the content editor, a
// re-rendered logo, a photograph. The key an entry stores is the path under
// `media/`, so the staging tree and the bucket have the same shape and there
// is no mapping table to keep.
//
// It uploads what changed. A file's SHA-256 is compared with the record of the
// last push in `.media-pushed.json`, which is machine-local and gitignored —
// the bucket is the truth, and this only avoids re-uploading the unchanged.
// `--all` ignores it, which is what to use after someone else uploads, or if
// you are unsure.
//
// Vendor: `[adapters] storage` names it and this speaks the one command that
// vendor takes. Moving storage changes `upload()` and nothing else.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { referencedKeys } from "./media-keys.mjs";

const repo = fileURLToPath(new URL("..", import.meta.url));
const MEDIA = join(repo, "media");
const RECORD = join(repo, ".media-pushed.json");
const all = process.argv.includes("--all");
const dryRun = process.argv.includes("--dry-run");
const prune = process.argv.includes("--prune");

const cfg = readFileSync(join(repo, "fiducial.toml"), "utf8");
const bucket = cfg.match(/r2_bucket_name\s*=\s*"([^"]+)"/)?.[1];
if (!bucket) {
  console.error("fiducial.toml: [deploy] r2_bucket_name is not set — nothing to push to.");
  process.exit(1);
}

const TYPES = {
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".svg": "image/svg+xml",
  ".pdf": "application/pdf",
  ".mp4": "video/mp4",
  ".zip": "application/zip",
};

function walk(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
}

const record = existsSync(RECORD) && !all ? JSON.parse(readFileSync(RECORD, "utf8")) : {};
const files = walk(MEDIA).filter((f) => !f.endsWith(".DS_Store"));
if (!files.length) {
  console.log("media/ is empty — nothing to push.");
  process.exit(0);
}

let pushed = 0;
let skipped = 0;
for (const file of files) {
  // The key IS the path under media/. Windows separators would make a key
  // nobody can fetch, so they are normalised.
  const key = relative(MEDIA, file).split("\\").join("/");
  const hash = createHash("sha256").update(readFileSync(file)).digest("hex");
  if (record[key] === hash) {
    skipped++;
    continue;
  }
  const ext = key.slice(key.lastIndexOf("."));
  const type = TYPES[ext];
  if (!type) {
    console.error(`  ?  ${key} — unknown type ${ext}, skipped. Add it to TYPES if it belongs.`);
    continue;
  }
  if (dryRun) {
    console.log(`  →  ${key} (${type})`);
    pushed++;
    continue;
  }
  // `--remote` is not optional: without it wrangler writes to a local
  // simulator and reports success, and the live site never sees the file.
  const r = spawnSync(
    "npx",
    [
      "wrangler",
      "r2",
      "object",
      "put",
      `${bucket}/${key}`,
      "--file",
      file,
      "--content-type",
      type,
      "--remote",
    ],
    { cwd: join(repo, "apps/web"), stdio: ["ignore", "ignore", "inherit"] },
  );
  if (r.status !== 0) {
    console.error(`  ×  ${key} — upload failed`);
    process.exit(1);
  }
  console.log(`  ✓  ${key}`);
  record[key] = hash;
  pushed++;
}

if (!dryRun) writeFileSync(RECORD, `${JSON.stringify(record, null, 2)}\n`);
console.log(`\n${pushed} uploaded, ${skipped} unchanged → ${bucket}`);

// ── Orphans ──────────────────────────────────────────────────────────────────
// Deleting an entry deletes its text and leaves its picture: the record is
// gone from the site and the object is still in the bucket, paid for and
// reachable by anyone who kept the URL. Nothing reports that, because nothing
// else knows what "referenced" means.
//
// Reported by default, deleted only with `--prune`. The bucket is the one
// place in this system where a mistake is not a `git revert`.
const referenced = referencedKeys();
const known = new Set([...Object.keys(record), ...files.map((f) => relative(MEDIA, f).split("\\").join("/"))]);
const orphans = [...known].filter((k) => !referenced.has(k)).sort();

if (!orphans.length) {
  console.log("no orphans: every object this repo knows about is referenced by content.");
} else if (!prune) {
  console.log(`\n${orphans.length} object(s) no entry refers to any more:`);
  for (const k of orphans) console.log(`  ·  ${k}`);
  console.log("\n`pnpm media:push --prune` deletes them from the bucket and the staging directory.");
} else {
  console.log(`\npruning ${orphans.length} unreferenced object(s):`);
  for (const key of orphans) {
    if (dryRun) {
      console.log(`  →  would delete ${key}`);
      continue;
    }
    const r = spawnSync("npx", ["wrangler", "r2", "object", "delete", `${bucket}/${key}`, "--remote"], {
      cwd: join(repo, "apps/web"),
      stdio: ["ignore", "ignore", "inherit"],
    });
    if (r.status !== 0) {
      console.error(`  ×  ${key} — delete failed`);
      continue;
    }
    rmSync(join(MEDIA, key), { force: true });
    delete record[key];
    console.log(`  ✓  deleted ${key}`);
  }
  if (!dryRun) writeFileSync(RECORD, `${JSON.stringify(record, null, 2)}\n`);
}

// Which media keys the content actually refers to.
//
// One answer, used by both directions of the sync: `media:pull` fetches these,
// and `media:push` treats everything else as an orphan. Two implementations of
// "what is referenced" would disagree the first time a field was added, and
// the disagreement would read as data loss — a pull that misses a cover, or a
// prune that deletes one.
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseToml } from "./derive-content.mjs";
import { frontmatter } from "./frontmatter.mjs";

const repo = fileURLToPath(new URL("..", import.meta.url));
const read = (p) => readFileSync(join(repo, p), "utf8");

/** Every markdown file under a directory, recursively. */
function markdown(dir) {
  const abs = join(repo, dir);
  if (!existsSync(abs)) return [];
  return readdirSync(abs).flatMap((name) => {
    const p = join(dir, name);
    return statSync(join(repo, p)).isDirectory() ? markdown(p) : p.endsWith(".md") ? [p] : [];
  });
}

/**
 * The keys referenced by every entry in the product.
 *
 * Which frontmatter fields hold a key is a fact `content.toml` already states
 * (`type = "media"`); `cover` and `key` are added because the press room
 * declares its story covers outside that file.
 */
export function referencedKeys() {
  const collections = parseToml(read("content.toml")).collections ?? {};
  const fields = new Set(["cover", "key"]);
  // A picture inside a list of records (a gallery) is referenced too. Missing it
  // here made `media:push --prune` treat every gallery image as an orphan.
  const recordFields = new Set();
  for (const spec of Object.values(collections)) {
    for (const [field, type] of Object.entries(spec.schema ?? {})) {
      if (type === "media") fields.add(field);
    }
    for (const shape of Object.values(spec.objects ?? {})) {
      for (const [k, type] of Object.entries(shape)) if (type === "media") recordFields.add(k);
    }
  }

  const keys = new Set();
  for (const file of [...markdown("content"), ...markdown("press")]) {
    const { meta, body } = frontmatter(read(file), () => {});
    for (const [field, value] of Object.entries(meta)) {
      if (fields.has(field) && typeof value === "string" && value) keys.add(value);
      if (Array.isArray(value)) {
        for (const item of value) {
          if (!item || typeof item !== "object") continue;
          for (const k of recordFields)
            if (typeof item[k] === "string" && item[k]) keys.add(item[k]);
        }
      }
    }
    // Inline images in a body: `![alt](key "caption")`.
    for (const m of body.matchAll(/!\[[^\]]*\]\(([^\s)]+)(?:\s+"[^"]*")?\)/g)) {
      if (!/^(https?:)?\/\//.test(m[1])) keys.add(m[1]);
    }
  }
  return keys;
}

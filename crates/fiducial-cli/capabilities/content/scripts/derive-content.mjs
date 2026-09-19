// content.toml + content/<collection>/<locale>/*.md  →  generated/content.ts
//
// One layer for everything a person reads. Before this, a product derived
// user-visible material through four unrelated mechanisms — message catalogs,
// legal prose, brand facts, a press room — each with its own shape. A blog
// made five. Anything wanting to read or edit content had to learn all of them.
//
// See docs/specs/2026-09-19-content-is-one-layer.md.
import { readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname } from "node:path";

const at = (p) => new URL(`../${p}`, import.meta.url);
const read = (p) => readFileSync(at(p), "utf8");
const fail = (msg) => {
  throw new Error(`derive-content: ${msg}`);
};

// ── TOML, only as far as we need it ──────────────────────────────────────────
// Handles `[a.b.c]` tables, quoted strings, inline tables and arrays, including
// arrays written across lines. Multi-line arrays matter: `[i18n] locales` is
// normally written that way, and a parser that stops at the newline reports no
// locales at all rather than erroring.
function parseToml(src) {
  const root = {};
  let table = root;
  let pending = null;
  for (const raw of src.split("\n")) {
    const line = raw.trim();
    if (pending) {
      pending.buf += ` ${line}`;
      if (balanced(pending.buf)) {
        table[pending.key] = parseValue(pending.buf);
        pending = null;
      }
      continue;
    }
    if (!line || line.startsWith("#")) continue;
    const t = line.match(/^\[([^\]]+)\]$/);
    if (t) {
      table = root;
      for (const part of t[1].split(".")) {
        table[part] ??= {};
        table = table[part];
      }
      continue;
    }
    const kv = raw.match(/^\s*([A-Za-z_][\w-]*)\s*=\s*(.*)$/);
    if (!kv) continue;
    const v = kv[2].trim();
    if (!balanced(v)) {
      pending = { key: kv[1], buf: v };
      continue;
    }
    table[kv[1]] = parseValue(v);
  }
  return root;
}
const balanced = (s) => {
  let n = 0;
  for (const c of s) {
    if (c === "[" || c === "{") n++;
    if (c === "]" || c === "}") n--;
  }
  return n === 0;
};
function splitTop(s) {
  const out = [];
  let depth = 0;
  let cur = "";
  let q = false;
  for (const c of s) {
    if (c === '"') q = !q;
    if (!q && (c === "[" || c === "{")) depth++;
    if (!q && (c === "]" || c === "}")) depth--;
    if (!q && c === "," && depth === 0) {
      out.push(cur);
      cur = "";
      continue;
    }
    cur += c;
  }
  if (cur.trim()) out.push(cur);
  return out.map((x) => x.trim()).filter(Boolean);
}
function parseValue(v) {
  v = v.replace(/\s+#.*$/, "").trim();
  if (v.startsWith('"') && v.endsWith('"')) return v.slice(1, -1);
  if (v.startsWith("[")) return splitTop(v.slice(1, -1)).map(parseValue);
  if (v.startsWith("{")) {
    const o = {};
    for (const part of splitTop(v.slice(1, -1))) {
      const m = part.match(/^([A-Za-z_][\w-]*)\s*=\s*([\s\S]+)$/);
      if (m) o[m[1]] = parseValue(m[2]);
    }
    return o;
  }
  if (v === "true" || v === "false") return v === "true";
  if (/^-?\d+(\.\d+)?$/.test(v)) return Number(v);
  return v;
}

// ── Frontmatter ──────────────────────────────────────────────────────────────
function frontmatter(md, where) {
  const m = md.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/);
  if (!m) fail(`${where}: no frontmatter. An entry starts with a --- block.`);
  const meta = {};
  for (const line of m[1].split("\n")) {
    const kv = line.match(/^([A-Za-z_][\w-]*):\s*(.*)$/);
    if (kv) meta[kv[1]] = parseValue(kv[2].trim());
  }
  return { meta, body: m[2].trim() };
}

// ── Schema check ─────────────────────────────────────────────────────────────
// A field outside the schema is an error rather than extra data. It is almost
// always a typo, and a typo that silently disappears from a page is the reason
// this is checked at all.
function checkSchema(meta, schema, where) {
  for (const [field, type] of Object.entries(schema)) {
    if (!(field in meta)) fail(`${where}: missing required field \`${field}\` (${type})`);
    const v = meta[field];
    const ok =
      type === "string[]"
        ? Array.isArray(v)
        : type === "number"
          ? typeof v === "number"
          : type === "boolean"
            ? typeof v === "boolean"
            : type === "date"
              ? typeof v === "string" && /^\d{4}-\d{2}-\d{2}/.test(v)
              : typeof v === "string";
    if (!ok) fail(`${where}: field \`${field}\` should be ${type}, got ${JSON.stringify(v)}`);
  }
  for (const field of Object.keys(meta)) {
    if (!(field in schema)) {
      fail(
        `${where}: field \`${field}\` is not in the collection's schema.\n` +
          `  Add it to content.toml, or fix the spelling. A field that is not ` +
          `declared is dropped silently otherwise, which is how a typo becomes ` +
          `a missing paragraph nobody can find.`,
      );
    }
  }
}

// ── Load ─────────────────────────────────────────────────────────────────────
const cfg = parseToml(read("fiducial.toml"));
const locales = cfg.i18n?.locales ?? [];
if (!Array.isArray(locales) || locales.length === 0) {
  fail("[i18n] declares no locales. This capability requires `i18n`.");
}

const decl = parseToml(read("content.toml"));
const collections = decl.collections ?? {};
if (Object.keys(collections).length === 0) fail("content.toml declares no collections");

const out = { locales, collections: {} };

for (const [name, spec] of Object.entries(collections)) {
  const kind = spec.kind ?? "collection";
  const schema = spec.schema ?? {};
  const wantsBody = (spec.body ?? "none") === "rich";
  const byLocale = {};

  for (const loc of locales) {
    const dir = `content/${name}/${loc}`;
    if (!existsSync(at(dir))) {
      // MISSION.md 1c: a missing translation is a missing artifact, not a
      // fallback. Serving the default locale's copy here is the tempting
      // behaviour and the wrong one — the reader is never told, and a
      // journalist or customer quotes text in the wrong language.
      fail(
        `collection \`${name}\` has nothing for locale \`${loc}\`.\n` +
          `  expected: ${dir}/\n` +
          `  A missing translation is a missing artifact, not a fallback ` +
          `(MISSION.md 1c).\n  Write it, or remove \`${loc}\` from [i18n] locales.`,
      );
    }
    const files = readdirSync(at(dir)).filter((f) => f.endsWith(".md")).sort();
    if (kind === "singleton" && files.length !== 1) {
      fail(`singleton \`${name}\` (${loc}) must hold exactly one entry, found ${files.length}`);
    }
    byLocale[loc] = files.map((f) => {
      const where = `${dir}/${f}`;
      const { meta, body } = frontmatter(read(where), where);
      checkSchema(meta, schema, where);
      if (body && !wantsBody) fail(`${where}: collection declares body = "none" but this entry has one`);
      if (!body && wantsBody) fail(`${where}: collection declares body = "rich" but this entry has none`);
      return { slug: f.replace(/\.md$/, ""), ...meta, ...(wantsBody ? { body } : {}) };
    });
  }

  // Set parity. A collection carrying an entry in one language and not another
  // is not translated — it is two different collections, and the gap is
  // invisible on whichever page you happen to be reading.
  const sets = locales.map((l) => byLocale[l].map((e) => e.slug).sort().join("|"));
  if (new Set(sets).size > 1) {
    const detail = locales
      .map((l) => `    ${l}: ${byLocale[l].map((e) => e.slug).join(", ") || "(none)"}`)
      .join("\n");
    fail(
      `collection \`${name}\` does not carry the same entries in every locale.\n\n${detail}\n\n` +
        `  An entry exists in every locale, or it exists in none.`,
    );
  }

  out.collections[name] = { kind, route: spec.route ?? "", byLocale };
}

// ── Emit ─────────────────────────────────────────────────────────────────────
const lines = [
  "// Generated by `fid derive` (pipeline: content) from content.toml and content/.",
  "// Do not edit — edit the declarations and re-derive.",
  "//",
  "// PLAIN DATA. This module imports nothing, and must not: no React, no Svelte,",
  "// no framework. That is what makes a port mechanical — moving between web",
  "// frameworks is rewriting markup against a data module that does not change.",
  "// The pipeline asserts it, so the property cannot quietly rot.",
  "",
  `export const content = ${JSON.stringify(out, null, 2)} as const`,
  "",
  "export type Locale = (typeof content.locales)[number]",
  "export type CollectionName = keyof typeof content.collections",
  "",
  "/** Every entry in a collection, for one locale. */",
  "export function entries<N extends CollectionName>(name: N, locale: Locale) {",
  "  return content.collections[name].byLocale[locale]",
  "}",
  "",
  "/**",
  " * The one entry of a singleton.",
  " *",
  " * `.at(0)` rather than `[0]`: `as const` makes an empty collection a",
  " * `readonly []`, and indexing that is a type error rather than `undefined`.",
  " * A declared-but-empty collection is a normal state — it is how a blog",
  " * exists before its first post — so the accessors tolerate it.",
  " */",
  "export function single<N extends CollectionName>(name: N, locale: Locale) {",
  "  return entries(name, locale).at(0)",
  "}",
  "",
  "/** One entry by slug, or undefined. */",
  "export function entry<N extends CollectionName>(name: N, locale: Locale, slug: string) {",
  "  const all = entries(name, locale) as readonly { slug: string }[]",
  "  return all.find((e) => e.slug === slug)",
  "}",
  "",
];
const rendered = lines.join("\n");

// The assertion that keeps the promise above true.
if (/\bfrom\s+["'](react|svelte|next\/|\$app\/)/.test(rendered)) {
  fail("the generated module imports a framework; it must stay plain data");
}

const target = "apps/web/src/generated/content.ts";
mkdirSync(dirname(at(target).pathname), { recursive: true });
writeFileSync(at(target), rendered);
const n = Object.keys(out.collections).length;
console.log(`wrote ${target} (${n} collection(s), ${locales.length} locale(s))`);

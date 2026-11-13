// content.toml + press/ + messages/ + settings.json + [i18n]  →  apps/cms/*
//
// The editor's shape is derived, never written by hand.
//
// A content editor needs every collection, field and widget listed. That is the
// information `content.toml` already declares and `press/`, `messages/` and
// `settings.json` already hold, so a hand-written editor config would be a
// second declaration of the content model — the copy that drifts silently,
// because nothing renders from it. An editor offering a field the schema lacks
// writes an entry `fid derive` then rejects; one missing a field leaves it empty
// on everything made through it.
//
// So this reads the declarations, builds ONE neutral model of what is editable,
// and emits it for the editors we support:
//
//   keystatic  apps/cms/keystatic.config.ts   (default)
//   tina       apps/cms/tina/config.ts
//
// Both configs are always written and both edit the same files — plain `.md`
// with YAML frontmatter, plain `.json` — so changing `[cms] editor` is a
// derive and an install, never a content migration. `apps/cms/package.json` is
// derived too, so only the chosen editor's packages are installed.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { parseToml } from "./derive-content.mjs";

const at = (p) => new URL(`../${p}`, import.meta.url);
const read = (p) => readFileSync(at(p), "utf8");
const has = (p) => existsSync(at(p));

const cfg = parseToml(read("fiducial.toml"));
const productName = cfg.product?.name ?? "site";
const locales = cfg.i18n?.locales ?? ["en"];
const cms = cfg.cms ?? {};
const EDITORS = ["keystatic", "tina"];
const editor = cms.editor ?? "keystatic";
if (!EDITORS.includes(editor)) {
  throw new Error(`[cms] editor = "${editor}" is not one of: ${EDITORS.join(", ")}`);
}
// Uploads are staged in a gitignored directory and stored in content as KEYS
// (`site/blog/photo.png`), never URLs. `pnpm media:push` syncs them to the bucket.
const mediaDir = cms.media_dir ?? "media";
const selects = cms.selects ?? {}; // "collection.field" → ["a", "b"]
const hints = cms.hints ?? {}; //     "collection.field" → "help text"

const collections = parseToml(read("content.toml")).collections ?? {};
const language = (l) => new Intl.DisplayNames(["en"], { type: "language" }).of(l) ?? l;
const label = (name) => {
  const words = name.replace(/[-_]/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2");
  return words.charAt(0).toUpperCase() + words.slice(1);
};

// ── The neutral model ────────────────────────────────────────────────────────
// A field is `{ name, label, type, ...}`; type is one of
//   text · long · date · partial · number · bool · list · media · select ·
//   relation · objects · object · body

// A field named `related<Collection>` points at that collection with no
// declaration; `[cms.relations]` says so for the rest.
const relations = { ...(cms.relations ?? {}) };
for (const [name, spec] of Object.entries(collections)) {
  for (const f of Object.keys(spec.schema ?? {})) {
    const m = /^related([A-Z]\w*)$/.exec(f);
    const target = m ? m[1].charAt(0).toLowerCase() + m[1].slice(1) : "";
    if (target && collections[target]) relations[`${name}.${f}`] ??= target;
  }
}

const TYPE = { string: "text", date: "date", "partial-date": "partial", number: "number", boolean: "bool", "string[]": "list", media: "media" };

function modelField(coll, spec, f, t) {
  const key = `${coll}.${f}`;
  const base = { name: f, label: label(f), hint: hints[key] };
  if (t === "object[]") {
    const shape = Object.entries(spec.objects?.[f] ?? {});
    return {
      ...base,
      type: "objects",
      fields: shape.map(([k, kt]) => ({
        name: k,
        label: label(k),
        type: TYPE[kt] ?? "text",
        prefix: spec.media ?? coll,
        hint: hints[`${key}.${k}`],
      })),
    };
  }
  if (relations[key]) return { ...base, type: "relation", target: relations[key], many: t === "string[]" };
  if (selects[key]) return { ...base, type: "select", options: selects[key] };
  return { ...base, type: TYPE[t] ?? "text", prefix: spec.media ?? coll };
}

const bodyField = { name: "body", label: "Body", type: "body" };
const entries = []; // { group, key, kind: "collection" | "file" | "data", label, path, fields, ... }
const keyOf = (name, locale) => `${name.replace(/-(\w)/g, (_, c) => c.toUpperCase())}_${locale}`;

for (const [name, spec] of Object.entries(collections)) {
  const schema = Object.entries(spec.schema ?? {});
  const fields = [
    ...schema.map(([f, t]) => modelField(name, spec, f, t)),
    ...(spec.body === "rich" ? [bodyField] : []),
  ];
  for (const locale of locales) {
    if (spec.kind === "singleton") {
      entries.push({ group: label(name), key: keyOf(name, locale), kind: "file", locale, label: `${label(name)} (${language(locale)})`, path: `content/${name}/${locale}/${name}`, fields });
    } else {
      // Entries are named by one text field, the first of title / name / headline.
      const title = ["title", "name", "headline"].find((k) => spec.schema?.[k] === "string") ?? schema.find(([, t]) => t === "string")?.[0];
      if (!title) throw new Error(`collection \`${name}\` has no string field to name entries by; add a \`title\``);
      entries.push({ group: label(name), key: keyOf(name, locale), kind: "collection", locale, label: `${label(name)} (${language(locale)})`, path: `content/${name}/${locale}`, title, fields });
    }
  }
}

// The press room nests the locale under each collection, like `content/`.
if (has("press/docs")) {
  for (const locale of locales) {
    entries.push({ group: "Press: boilerplate and facts", key: keyOf("pressDocs", locale), kind: "collection", locale, label: `Press documents (${language(locale)})`, path: `press/docs/${locale}`, title: "title", fields: [{ name: "title", label: "Name", type: "text" }, { name: "body", label: "Text", type: "body" }] });
  }
}
if (has("press/stories")) {
  // A free-text angle would not match `press.angle.*` in the message catalogue.
  const angles = ["origin", "technical", "customer", "failure", "milestone", "people", "market"];
  for (const locale of locales) {
    entries.push({
      group: "Press stories", key: keyOf("pressStories", locale), kind: "collection", locale, label: `Press stories (${language(locale)})`, path: `press/stories/${locale}`, title: "title",
      fields: [
        { name: "title", label: "Title", type: "text" },
        { name: "angle", label: "Angle", type: "select", options: angles },
        { name: "date", label: "Date", type: "date" },
        { name: "summary", label: "Summary", type: "text" },
        { name: "cover", label: "Cover", type: "media", prefix: "press/stories" },
        { name: "cover_alt", label: "Cover alt", type: "text" },
        { name: "body", label: "Story", type: "body" },
      ],
    });
  }
}

// Files that are not collections of entries: the fields come from the files
// themselves, so a new key appears in its form on the next derive and nothing
// lists a message by name.
const fromJson = (value) =>
  Object.entries(value).flatMap(([k, v]) => {
    if (!/^[\w-]+$/.test(k)) return []; // a key an editor cannot address
    const base = { name: k, label: label(k) };
    if (Array.isArray(v)) {
      const first = v.find((x) => x && typeof x === "object");
      return [first ? { ...base, type: "objects", fields: fromJson(first) } : { ...base, type: "list" }];
    }
    if (v && typeof v === "object") return [{ ...base, type: "object", fields: fromJson(v) }];
    if (typeof v === "boolean") return [{ ...base, type: "bool" }];
    if (typeof v === "number") return [{ ...base, type: "number" }];
    const long = typeof v === "string" && (/\n/.test(v) || v.length > 90);
    return [{ ...base, type: long ? "long" : "text" }];
  });

const dataFiles = [];
for (const file of ["settings.json", ...(cms.settings ?? [])]) {
  if (has(file)) dataFiles.push({ key: file.replace(/\.json$/, "").replace(/[^\w]/g, "_"), label: label(file.replace(/\.json$/, "")), path: file.replace(/\.json$/, ""), fields: fromJson(JSON.parse(read(file))) });
}
for (const locale of locales) {
  if (has(`messages/${locale}.json`)) {
    dataFiles.push({ key: `messages_${locale}`, label: `Interface text (${language(locale)})`, path: `messages/${locale}`, fields: fromJson(JSON.parse(read(`messages/${locale}.json`))) });
  }
}
for (const d of dataFiles) entries.push({ group: "Settings", kind: "data", ...d });

// ── Emit: Keystatic ──────────────────────────────────────────────────────────
// Keystatic rejects `..` in a path and resolves the rest against the directory
// the server runs in, so `dev.mjs` starts it from the repository root and every
// path here is repository-relative.
const J = JSON.stringify;

function ksField(f, e) {
  const o = `label: ${J(f.label)}${f.hint ? `, description: ${J(f.hint)}` : ""}`;
  switch (f.type) {
    case "long": return `fields.text({ ${o}, multiline: true })`;
    case "date": return `fields.date({ ${o} })`;
    case "partial": return `fields.text({ label: ${J(f.label)}, description: ${J(f.hint ?? "YYYY, YYYY-MM or YYYY-MM-DD")} })`;
    case "number": return `fields.integer({ ${o} })`;
    case "bool": return `fields.checkbox({ ${o} })`;
    case "list": return `fields.array(fields.text({ label: "Item" }), { label: ${J(f.label)}, itemLabel: (p) => p.value })`;
    // The directory is the gitignored staging area and `publicPath` the key
    // prefix, so what the picker stores is exactly a media key.
    case "media": return `fields.image({ ${o}, directory: ${J(`${mediaDir}/${f.prefix}`)}, publicPath: ${J(`${f.prefix}/`)} })`;
    case "select": return `fields.select({ ${o}, options: [${f.options.map((v) => `{ label: ${J(v)}, value: ${J(v)} }`).join(", ")}], defaultValue: ${J(f.options[0] ?? "")} })`;
    case "relation": {
      const target = keyOf(f.target, e.locale);
      return f.many
        ? `fields.array(fields.relationship({ label: "Entry", collection: ${J(target)} }), { label: ${J(f.label)}, itemLabel: (p) => p.value ?? "" })`
        : `fields.relationship({ ${o}, collection: ${J(target)} })`;
    }
    case "objects": {
      const first = f.fields[0]?.name;
      return `fields.array(fields.object({ ${f.fields.map((x) => `${J(x.name)}: ${ksField(x, e)}`).join(", ")} }), { label: ${J(f.label)}, itemLabel: (p) => ${first ? `String(p.fields[${J(first)}].value ?? "")` : '""'} })`;
    }
    case "object": return `fields.object({ ${f.fields.map((x) => `${J(x.name)}: ${ksField(x, e)}`).join(", ")} }, { label: ${J(f.label)} })`;
    case "body": return `fields.markdoc({ label: ${J(f.label)}, extension: "md" })`;
    default: return `fields.text({ ${o} })`;
  }
}

function ksEntry(e) {
  const schema = e.fields
    .map((f) => {
      // The file name is the entry's identity and must match in every language,
      // so the slug is editable at creation rather than derived from a translation.
      if (e.kind === "collection" && f.name === e.title) {
        return `    ${J(f.name)}: fields.slug({ name: { label: ${J(f.label)} }, slug: { label: "File name", description: "The same in every language." } }),`;
      }
      return `    ${J(f.name)}: ${ksField(f, e)},`;
    })
    .join("\n");
  const body = e.fields.some((f) => f.type === "body");
  const format = e.kind === "data" ? `format: "json"` : body ? `format: { contentField: "body" }` : `format: { data: "yaml" }`;
  if (e.kind === "collection") {
    return `    ${e.key}: collection({\n      label: ${J(e.label)},\n      path: ${J(`${e.path}/*`)},\n      slugField: ${J(e.title)},\n      ${format},\n      schema: {\n${schema}\n      },\n    }),`;
  }
  return `    ${e.key}: singleton({\n      label: ${J(e.label)},\n      path: ${J(`${e.path}`)},\n      ${format},\n      schema: {\n${schema}\n      },\n    }),`;
}

const groups = {};
for (const e of entries) (groups[e.group] ??= []).push(e.key);
const navigation = Object.fromEntries(Object.entries(groups));

const keystatic = `// Generated by \`fid derive\` (pipeline: cms) from content.toml, press/, messages/,
// settings.json and [i18n]. Do not edit — change the declaration and re-derive.
// \`fid derive --check\` fails if this file is stale.
//
// Paths are relative to the repository root, where the editor runs. Entries are plain
// \`.md\` with YAML frontmatter: the same files Tina edits, so switching
// \`[cms] editor\` never means converting content.
import { collection, config, fields, singleton } from "@keystatic/core";

export default config({
  storage: { kind: "local" },
  ui: {
    brand: { name: ${J(productName)} },
    navigation: ${J(navigation, null, 6).replace(/\n}$/, "\n    }")},
  },
  collections: {
${entries.filter((e) => e.kind === "collection").map(ksEntry).join("\n")}
  },
  singletons: {
${entries.filter((e) => e.kind !== "collection").map(ksEntry).join("\n")}
  },
});
`;

// ── Emit: Tina ───────────────────────────────────────────────────────────────
// Tina's strengths are visual editing and editing on the page itself; it costs a
// service dependency (Tina Cloud or self-hosted) once editing is hosted. Locally
// it needs neither. Two honest differences from Keystatic, both because Tina
// stores different values: a picture is a text field holding the media KEY
// (Tina's image picker would store a path), and a relation is the entry's slug
// as text (Tina's reference stores a file path).
function tinaField(f) {
  const base = `name: ${J(f.name)}, label: ${J(f.label)}${f.hint ? `, description: ${J(f.hint)}` : ""}`;
  switch (f.type) {
    case "long": return `{ type: "string", ${base}, ui: { component: "textarea" } }`;
    case "date": return `{ type: "datetime", ${base} }`;
    case "number": return `{ type: "number", ${base} }`;
    case "bool": return `{ type: "boolean", ${base} }`;
    case "list": return `{ type: "string", list: true, ${base} }`;
    case "select": return `{ type: "string", ${base}, options: ${J(f.options)} }`;
    case "objects": return `{ type: "object", list: true, ${base}, fields: [${f.fields.map(tinaField).join(", ")}] }`;
    case "object": return `{ type: "object", ${base}, fields: [${f.fields.map(tinaField).join(", ")}] }`;
    case "body": return `{ type: "rich-text", name: "body", label: ${J(f.label)}, isBody: true }`;
    case "media": return `{ type: "string", ${base.replace(/description: [^,]*,?/, "")}, description: ${J(`Media key under ${f.prefix}/ (staged in ${mediaDir}/, synced by pnpm media:push)`)} }`;
    case "relation": return `{ type: "string", ${base}, description: ${J(`Slug of an entry in ${f.target}`)}${f.many ? ", list: true" : ""} }`;
    default: return `{ type: "string", ${base} }`;
  }
}

function tinaCollection(e) {
  const title = e.kind === "collection" ? e.title : null;
  const fields = e.fields.map((f) => (f.name === title ? tinaField(f).replace(" }", ", isTitle: true, required: true }") : tinaField(f))).join(",\n      ");
  const file = e.kind === "collection" ? null : e.path.split("/").pop();
  const dir = e.kind === "collection" ? e.path : e.path.split("/").slice(0, -1).join("/");
  const format = e.kind === "data" ? "json" : "md";
  const only = file ? `\n    match: { include: ${J(file)} },\n    ui: { allowedActions: { create: false, delete: false } },` : "";
  return `  {\n    name: ${J(e.key)},\n    label: ${J(e.label)},\n    path: ${J(dir || ".")},\n    format: ${J(format)},${only}\n    fields: [\n      ${fields},\n    ],\n  }`;
}

const tina = `// Generated by \`fid derive\` (pipeline: cms) from content.toml, press/, messages/,
// settings.json and [i18n]. Do not edit — change the declaration and re-derive.
//
// Edits the same \`.md\` and \`.json\` files as Keystatic. Run with \`pnpm dev\` in
// apps/cms when \`[cms] editor = "tina"\`.
import { defineConfig } from "tinacms";

export default defineConfig({
  branch: "main",
  // Local mode: the repository root, three levels up from this file, is the content store.
  localContentPath: "../../../",
  build: { outputFolder: "admin", publicFolder: "public" },
  schema: {
    collections: [
${entries.map(tinaCollection).join(",\n")},
    ],
  },
});
`;

// ── Emit: the editor workspace's package.json ────────────────────────────────
const pkg = {
  name: `@${productName}/cms`,
  version: "0.1.0",
  private: true,
  scripts: {
    dev: editor === "tina" ? "node dev.mjs here tinacms dev" : "node dev.mjs root next dev apps/cms --port 4321",
  },
  dependencies:
    editor === "tina"
      ? { "@tinacms/cli": "^4.0.5", tinacms: "^3.15.2", react: "^19.0.0", "react-dom": "^19.0.0" }
      : { "@keystatic/core": "^0.6.9", "@keystatic/next": "^5.0.5", next: "^16.0.0", react: "^19.0.0", "react-dom": "^19.0.0" },
  devDependencies: { "@types/node": "^22.0.0", "@types/react": "^19.0.0", "@types/react-dom": "^19.0.0", typescript: "^7.0.0" },
};

const stamp = "Generated by `fid derive` (pipeline: cms). Do not edit — set [cms] editor in fiducial.toml.";
function emit(dest, text) {
  mkdirSync(dirname(at(dest).pathname), { recursive: true });
  writeFileSync(at(dest), text);
}
emit("apps/cms/keystatic.config.ts", keystatic);
emit("apps/cms/tina/config.ts", tina);
emit("apps/cms/package.json", `${JSON.stringify({ ...pkg, "//": stamp }, null, 2)}\n`);
console.log(`wrote apps/cms (${editor}; ${entries.length} editable item(s), ${locales.length} locale(s))`);

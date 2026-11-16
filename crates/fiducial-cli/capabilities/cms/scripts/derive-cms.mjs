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
//   pages      .pages.yml                      (Pages CMS: hosted, edits through GitHub)
//
// All three configs are always written and both edit the same files — plain `.md`
// with YAML frontmatter, plain `.json` — so changing `[cms] editor` is a
// derive and an install, never a content migration. `apps/cms/package.json` is
// derived too, so only the chosen editor's packages are installed.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { parseToml } from "./derive-content.mjs";
import { frontmatter } from "./frontmatter.mjs";

const at = (p) => new URL(`../${p}`, import.meta.url);
const read = (p) => readFileSync(at(p), "utf8");
const has = (p) => existsSync(at(p));

const cfg = parseToml(read("fiducial.toml"));
const productName = cfg.product?.name ?? "site";
const locales = cfg.i18n?.locales ?? ["en"];
const cms = cfg.cms ?? {};
const EDITORS = ["keystatic", "tina", "pages"];
const editor = cms.editor ?? "keystatic";
if (!EDITORS.includes(editor)) {
  throw new Error(`[cms] editor = "${editor}" is not one of: ${EDITORS.join(", ")}`);
}
// Uploads are staged in a gitignored directory and stored in content as KEYS
// (`site/blog/photo.png`), never URLs. `pnpm media:push` syncs them to the bucket.
const mediaDir = cms.media_dir ?? "media";
// Where the site serves that directory, when a stored value is a URL path
// (`/images/site/x.png`) rather than a bare key. Empty means a bare key.
const publicBase = cms.public_base ?? "";
const publicPath = (prefix) => `${publicBase}${publicBase ? "/" : ""}${prefix}/`;
// `picker` (default): Keystatic's image picker, which uploads into the staging
// directory and stores the key. `text`: the stored value is edited as text. A
// product whose values carry more than a key (`/images/x.webp?focus=30,40`)
// needs this: the picker re-homes an image under the entry's folder on save
// and keeps no query, so every saved entry would lose what the value said.
const mediaField = cms.media_field ?? "picker";
if (!["picker", "text"].includes(mediaField)) {
  throw new Error(`[cms] media_field = "${mediaField}" is not one of: picker, text`);
}
// Pages CMS commits an upload into a repository directory and stores the PUBLIC
// path it is served at. It can therefore hold `/images/x.png`, never a bare key
// like `site/x.png`: its write path always prefixes `/`. So an image field is
// offered only for a product that stores URL paths (`public_base`) and says where
// the repository keeps them (`media_input`); anything else is a text field.
const mediaInput = cms.media_input ?? "";
// Where Keystatic keeps what it saves. `local` writes this working tree, so the
// editor runs next to a checkout. `github` has Keystatic commit through the
// GitHub API, so people with no checkout can use a deployed editor; it needs a
// repository to commit to, and the GitHub App's keys in the editor's environment.
const storage = cms.storage ?? "local";
if (!["local", "github"].includes(storage)) {
  throw new Error(`[cms] storage = "${storage}" is not one of: local, github`);
}
const repoSlug = cms.repo ?? "";
if (storage === "github" && !/^[\w.-]+\/[\w.-]+$/.test(repoSlug)) {
  throw new Error(
    `[cms] storage = "github" needs repo = "owner/name" (got ${JSON.stringify(repoSlug)}): it is the repository the editor commits to`,
  );
}
if (storage === "github" && editor !== "keystatic") {
  throw new Error(
    `[cms] storage = "github" is Keystatic's GitHub mode, and the editor is "${editor}". Tina Cloud is configured in Tina's own dashboard; Pages CMS is already hosted.`,
  );
}
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
//   multiselect ·
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

const TYPE = {
  string: "text",
  date: "date",
  "partial-date": "partial",
  number: "number",
  boolean: "bool",
  "string[]": "list",
  media: "media",
};

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
  if (relations[key])
    return { ...base, type: "relation", target: relations[key], many: t === "string[]" };
  if (selects[key]) {
    // An editor's select stores text, so a choice among numbers is a number
    // field that says which ones; a select would write "1" where 1 is declared.
    if (selects[key].every((o) => typeof o === "number")) {
      return { ...base, type: "number", hint: base.hint ?? `One of: ${selects[key].join(", ")}` };
    }
    // On a list field the choice is several of them, stored as a list.
    return {
      ...base,
      type: t === "string[]" ? "multiselect" : "select",
      options: selects[key].map(String),
    };
  }
  return { ...base, type: TYPE[t] ?? "text", prefix: spec.media ?? coll };
}

const bodyField = { name: "body", label: "Body", type: "body" };
const entries = []; // { group, key, kind: "collection" | "file" | "data", label, path, fields, ... }
const keyOf = (name, locale) => `${name.replace(/-(\w)/g, (_, c) => c.toUpperCase())}_${locale}`;

for (const [name, spec] of Object.entries(collections)) {
  const schema = Object.entries(spec.schema ?? {});
  const prefix = spec.media ?? name;
  const fields = [
    ...schema.map(([f, t]) => modelField(name, spec, f, t)),
    ...(spec.body === "rich" ? [bodyField] : []),
  ];
  for (const locale of locales) {
    if (spec.kind === "singleton") {
      entries.push({
        group: label(name),
        key: keyOf(name, locale),
        kind: "file",
        locale,
        label: `${label(name)} (${language(locale)})`,
        path: `content/${name}/${locale}/${name}`,
        prefix,
        fields,
      });
    } else {
      // Entries are named by one text field, the first of title / name / headline.
      const title =
        ["title", "name", "headline"].find((k) => spec.schema?.[k] === "string") ??
        schema.find(([, t]) => t === "string")?.[0];
      if (!title)
        throw new Error(
          `collection \`${name}\` has no string field to name entries by; add a \`title\``,
        );
      entries.push({
        group: label(name),
        key: keyOf(name, locale),
        kind: "collection",
        locale,
        label: `${label(name)} (${language(locale)})`,
        path: `content/${name}/${locale}`,
        prefix,
        title,
        fields,
      });
    }
  }
}

// The press room nests the locale under each collection, like `content/`.
if (has("press/docs")) {
  for (const locale of locales) {
    entries.push({
      group: "Press: boilerplate and facts",
      key: keyOf("pressDocs", locale),
      kind: "collection",
      locale,
      label: `Press documents (${language(locale)})`,
      path: `press/docs/${locale}`,
      prefix: "press/docs",
      title: "title",
      fields: [
        { name: "title", label: "Name", type: "text" },
        { name: "body", label: "Text", type: "body" },
      ],
    });
  }
}
if (has("press/stories")) {
  // A free-text angle would not match `press.angle.*` in the message catalogue.
  const angles = ["origin", "technical", "customer", "failure", "milestone", "people", "market"];
  for (const locale of locales) {
    entries.push({
      group: "Press stories",
      key: keyOf("pressStories", locale),
      kind: "collection",
      locale,
      label: `Press stories (${language(locale)})`,
      path: `press/stories/${locale}`,
      prefix: "press/stories",
      title: "title",
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

// A select shows a value it does not offer as its default, and the next save
// writes that default over what the file said. So every value already in the
// content has to be one of the choices before the editor opens it.
for (const e of entries) {
  if (e.kind !== "collection" && e.kind !== "file") continue;
  const choices = e.fields.filter((f) => f.type === "select" || f.type === "multiselect");
  if (choices.length === 0) continue;
  const dir = e.kind === "file" ? dirname(e.path) : e.path;
  for (const file of has(dir) ? readdirSync(at(dir)).filter((n) => n.endsWith(".md")) : []) {
    const { meta } = frontmatter(read(`${dir}/${file}`), (m) => {
      throw new Error(`${dir}/${file}: ${m}`);
    });
    for (const f of choices) {
      const values = [meta[f.name]].flat().filter((v) => v !== undefined && v !== "");
      const v = values.find((x) => !f.options.includes(String(x)));
      if (v !== undefined) {
        throw new Error(
          `${dir}/${file}: \`${f.name}\` is ${JSON.stringify(v)}, not one of [cms.selects]: ${f.options.join(", ")}. Saving it in the editor would replace it.`,
        );
      }
    }
  }
}

// Files that are not collections of entries: the fields come from the files
// themselves, so a new key appears in its form on the next derive and nothing
// lists a message by name.
//
// An editor saves a JSON file from its form, so anything the form cannot hold
// is DELETED on the first save: a key it cannot address, a `null`, a list whose
// records differ in shape. That is refused here, at derive time, with the file
// and the key, instead of being discovered as a missing string in production.
function fromJson(value, where) {
  return Object.entries(value).map(([k, v]) => {
    const at = `${where}: \`${k}\``;
    if (!/^[A-Za-z_][\w-]*$/.test(k)) {
      throw new Error(
        `${at} is not a key a content editor can hold (letters, digits, _ and -; not first a digit). Rename it.`,
      );
    }
    const base = { name: k, label: label(k) };
    if (v === null)
      throw new Error(
        `${at} is null, which an editor saves as an empty string. Use "" or remove it.`,
      );
    if (Array.isArray(v)) {
      if (v.length === 0 || v.every((x) => typeof x === "string")) return { ...base, type: "list" };
      const shape = (x) =>
        x && typeof x === "object" && !Array.isArray(x) ? Object.keys(x).sort().join(",") : null;
      if (!v.every((x) => shape(x) !== null && shape(x) === shape(v[0]))) {
        throw new Error(
          `${at} must be a list of strings or of records with the same keys; an editor drops whatever does not fit the first one.`,
        );
      }
      return { ...base, type: "objects", fields: fromJson(v[0], `${where}.${k}[]`) };
    }
    if (typeof v === "object")
      return { ...base, type: "object", fields: fromJson(v, `${where}.${k}`) };
    if (typeof v === "boolean") return { ...base, type: "bool" };
    if (typeof v === "number") return { ...base, type: "number" };
    return { ...base, type: /\n/.test(v) || v.length > 90 ? "long" : "text" };
  });
}

const dataFiles = [];
for (const file of ["settings.json", ...(cms.settings ?? [])]) {
  if (has(file))
    dataFiles.push({
      key: file.replace(/\.json$/, "").replace(/[^\w]/g, "_"),
      label: label(file.replace(/\.json$/, "")),
      path: file.replace(/\.json$/, ""),
      fields: fromJson(JSON.parse(read(file)), file),
    });
}
for (const locale of locales) {
  if (has(`messages/${locale}.json`)) {
    dataFiles.push({
      key: `messages_${locale}`,
      label: `Interface text (${language(locale)})`,
      path: `messages/${locale}`,
      fields: fromJson(JSON.parse(read(`messages/${locale}.json`)), `messages/${locale}.json`),
    });
  }
}
for (const d of dataFiles) entries.push({ group: "Settings", kind: "data", ...d });

// ── Printing TypeScript ──────────────────────────────────────────────────────
// The emitters build plain data and this prints it, so the generated files are
// laid out like hand-written code (principle 3: every generated artifact is
// readable) and nothing is assembled by string surgery. `code("…")` is an
// expression printed as-is; `call(fn, …args)` a call whose arguments are data.
const CODE = Symbol("code");
const CALL = Symbol("call");
const code = (text) => ({ [CODE]: text });
const call = (fn, ...args) => ({ [CALL]: fn, args });
const IDENT = /^[A-Za-z_$][\w$]*$/;

function print(value, depth = 0) {
  if (value?.[CODE] !== undefined) return value[CODE];
  if (value?.[CALL] !== undefined)
    return `${value[CALL]}(${value.args.map((a) => print(a, depth)).join(", ")})`;
  if (typeof value !== "object" || value === null) return JSON.stringify(value);
  const items = Array.isArray(value)
    ? value.map((v) => print(v, depth + 1))
    : Object.entries(value)
        .filter(([, v]) => v !== undefined)
        .map(([k, v]) => `${IDENT.test(k) ? k : JSON.stringify(k)}: ${print(v, depth + 1)}`);
  const [open, close] = Array.isArray(value) ? ["[", "]"] : ["{ ", " }"];
  if (items.length === 0) return Array.isArray(value) ? "[]" : "{}";
  const flat = `${open}${items.join(", ")}${close}`;
  if (!flat.includes("\n") && flat.length + depth * 2 <= 72) {
    return flat;
  }
  const pad = "  ".repeat(depth + 1);
  return `${open.trim()}\n${items.map((i) => `${pad}${i},`).join("\n")}\n${"  ".repeat(depth)}${close.trim()}`;
}

const described = (f) => ({ label: f.label, description: f.hint });

// ── Emit: Keystatic ──────────────────────────────────────────────────────────
// Keystatic rejects `..` in a path and resolves the rest against the directory
// the server runs in, so `dev.mjs` starts it from the repository root and every
// path here is repository-relative.
//
// Bodies are written as plain `.md`. Tables are off because Keystatic writes
// them in Markdoc's own syntax, which a Markdown renderer prints as text; an
// image dropped into a body is staged under the collection's media prefix like
// any other, never committed next to the entry.
function ksField(f, e) {
  switch (f.type) {
    case "long":
      return call("fields.text", { ...described(f), multiline: true });
    case "date":
      return call("fields.date", described(f));
    case "partial":
      return call("fields.text", {
        label: f.label,
        description: f.hint ?? "YYYY, YYYY-MM or YYYY-MM-DD",
      });
    case "number":
      return call("fields.number", described(f));
    case "bool":
      return call("fields.checkbox", described(f));
    case "list":
      return call("fields.array", call("fields.text", { label: "Item" }), {
        label: f.label,
        itemLabel: code("(p) => p.value"),
      });
    // The directory is the gitignored staging area and `publicPath` the key
    // prefix, so what the picker stores is exactly a media key.
    case "media":
      if (mediaField === "text") {
        return call("fields.text", {
          label: f.label,
          description: f.hint ?? `Path or key under ${publicPath(f.prefix)}`,
        });
      }
      return call("fields.image", {
        ...described(f),
        directory: `${mediaDir}/${f.prefix}`,
        publicPath: publicPath(f.prefix),
      });
    case "select":
      return call("fields.select", {
        ...described(f),
        options: f.options.map((v) => ({ label: v, value: v })),
        defaultValue: f.options[0] ?? "",
      });
    case "multiselect":
      return call("fields.multiselect", {
        ...described(f),
        options: f.options.map((v) => ({ label: v, value: v })),
      });
    case "relation": {
      const collection = keyOf(f.target, e.locale);
      return f.many
        ? call("fields.array", call("fields.relationship", { label: "Entry", collection }), {
            label: f.label,
            itemLabel: code('(p) => p.value ?? ""'),
          })
        : call("fields.relationship", { ...described(f), collection });
    }
    case "objects": {
      // A record is listed by its first text field: a picture's key is no name.
      const first = (
        f.fields.find((x) => ["text", "long", "select"].includes(x.type)) ?? f.fields[0]
      )?.name;
      return call("fields.array", call("fields.object", ksSchema(f.fields, e)), {
        label: f.label,
        itemLabel: code(
          first ? `(p) => String(p.fields[${JSON.stringify(first)}].value ?? "")` : '() => ""',
        ),
      });
    }
    case "object":
      return call("fields.object", ksSchema(f.fields, e), { label: f.label });
    case "body":
      return call("fields.markdoc", {
        label: f.label,
        extension: "md",
        options: {
          table: false,
          image: { directory: `${mediaDir}/${e.prefix}`, publicPath: publicPath(e.prefix) },
        },
      });
    default:
      return call("fields.text", described(f));
  }
}

function ksSchema(fields, e) {
  return Object.fromEntries(
    fields.map((f) => [
      f.name,
      // The file name is the entry's identity and must match in every language,
      // so the slug is editable at creation rather than derived from a translation.
      e.kind === "collection" && f.name === e.title && fields === e.fields
        ? call("fields.slug", {
            name: described(f),
            slug: { label: "File name", description: "The same in every language." },
          })
        : ksField(f, e),
    ]),
  );
}

function ksEntry(e) {
  const hasBody = e.fields.some((f) => f.type === "body");
  // A frontmatter-only entry is still a `.md` file: `emptyContent` is the body
  // that holds nothing. Without it Keystatic writes `.yaml`, which no reader here
  // looks at, and the collection shows up empty.
  const schema = {
    ...ksSchema(e.fields, e),
    ...(e.kind !== "data" && !hasBody
      ? { body: call("fields.emptyContent", { extension: "md" }) }
      : {}),
  };
  const format = e.kind === "data" ? "json" : { contentField: "body" };
  return e.kind === "collection"
    ? call("collection", {
        label: e.label,
        path: `${e.path}/*`,
        slugField: e.title,
        format,
        schema,
      })
    : call("singleton", { label: e.label, path: e.path, format, schema });
}

const navigation = {};
for (const e of entries) {
  navigation[e.group] ??= [];
  navigation[e.group].push(e.key);
}
const pick = (pred) => Object.fromEntries(entries.filter(pred).map((e) => [e.key, ksEntry(e)]));

const keystatic = `// Generated by \`fid derive\` (pipeline: cms) from content.toml, press/, messages/,
// settings.json and [i18n]. Do not edit — change the declaration and re-derive.
// \`fid derive --check\` fails if this file is stale.
//
// Paths are relative to the repository root, where the editor runs. Entries are
// plain \`.md\` with YAML frontmatter: the same files Tina edits, so switching
// \`[cms] editor\` never means converting content.
import { collection, config, fields, singleton } from "@keystatic/core";

export default config(${print({
  storage: storage === "github" ? { kind: "github", repo: repoSlug } : { kind: "local" },
  ui: { brand: { name: productName }, navigation },
  collections: pick((e) => e.kind === "collection"),
  singletons: pick((e) => e.kind !== "collection"),
})});
`;

// ── Emit: Tina ───────────────────────────────────────────────────────────────
// Tina's strengths are visual editing and editing on the page itself; it costs a
// service dependency (Tina Cloud or self-hosted) once editing is hosted. Locally
// it needs neither. Two honest differences from Keystatic, both because Tina
// stores different values: a picture is a text field holding the media KEY
// (Tina's image picker would store a path), and a relation is the entry's slug
// as text (Tina's reference stores a file path). Its date picker saves midnight
// UTC as a full timestamp, which the content layer reads as that date.
function tinaField(f, title) {
  const base = { name: f.name, label: f.label, description: f.hint };
  switch (f.type) {
    case "long":
      return { type: "string", ...base, ui: { component: "textarea" } };
    case "date":
      return { type: "datetime", ...base, ui: { dateFormat: "YYYY-MM-DD", timeFormat: false } };
    case "number":
      return { type: "number", ...base };
    case "bool":
      return { type: "boolean", ...base };
    case "list":
      return { type: "string", list: true, ...base };
    case "select":
      return { type: "string", ...base, options: f.options };
    case "multiselect":
      return { type: "string", list: true, ...base, options: f.options };
    case "objects":
      return { type: "object", list: true, ...base, fields: f.fields.map((x) => tinaField(x)) };
    case "object":
      return { type: "object", ...base, fields: f.fields.map((x) => tinaField(x)) };
    case "body":
      return { type: "rich-text", name: "body", label: f.label, isBody: true };
    case "media":
      return {
        type: "string",
        ...base,
        description:
          f.hint ??
          `Media key under ${f.prefix}/ (staged in ${mediaDir}/, synced by pnpm media:push)`,
      };
    case "relation":
      return {
        type: "string",
        ...base,
        list: f.many || undefined,
        description: f.hint ?? `Slug of an entry in ${f.target}`,
      };
    default:
      return {
        type: "string",
        ...base,
        ...(f.name === title ? { isTitle: true, required: true } : {}),
      };
  }
}

function tinaCollection(e) {
  const parts = e.path.split("/");
  const single = e.kind !== "collection";
  return {
    name: e.key,
    label: e.label,
    path: single ? parts.slice(0, -1).join("/") || "." : e.path,
    format: e.kind === "data" ? "json" : "md",
    match: single ? { include: parts.at(-1) } : undefined,
    ui: single ? { allowedActions: { create: false, delete: false } } : undefined,
    fields: e.fields.map((f) => tinaField(f, e.title)),
  };
}

const tina = `// Generated by \`fid derive\` (pipeline: cms) from content.toml, press/, messages/,
// settings.json and [i18n]. Do not edit — change the declaration and re-derive.
//
// Edits the same \`.md\` and \`.json\` files as Keystatic. Run with \`pnpm dev\` in
// apps/cms when \`[cms] editor = "tina"\`.
import { defineConfig } from "tinacms";

export default defineConfig(${print({
  branch: "main",
  // Local mode: the repository root, three levels up from this file.
  localContentPath: "../../../",
  build: { outputFolder: "admin", publicFolder: "public" },
  schema: { collections: entries.map(tinaCollection) },
})});
`;

// ── Emit: Pages CMS ─────────────────────────────────────────────────────────
// Pages CMS is a hosted editor over a GitHub repository: it reads `.pages.yml`
// from the root and commits every save, so editing needs no local checkout and
// no server of ours. The same plain `.md` and `.json` files, again.
//
// The shape is the one in its own schema (`lib/config-schema.ts`, strict: an
// unknown key is an error, and a malformed file is silently tolerated by the
// app, which is why the derived output is held to that schema in
// `scripts/verify-cms-configs.sh`). Values of a few kinds differ from the others,
// for reasons Pages CMS dictates:
//  - a picture is a text field holding the media key, unless the product stores
//    URL paths and names where the repository keeps them (see `mediaInput`);
//  - a relation is text holding the entry's slug. Its `reference` field would
//    store `{name}`, which is the file name WITH its extension (`ada.md`), and
//    nothing it offers yields the slug the content layer reads;
//  - a partial date ("2026", "2026-11") is text, because its date field only
//    holds a full day.
const pagesFile = (e) =>
  e.kind === "collection" ? undefined : `${e.path}.${e.kind === "data" ? "json" : "md"}`;

function pagesField(f, e) {
  const base = { name: f.name, label: f.label, description: f.hint || undefined };
  switch (f.type) {
    case "long":
      return { ...base, type: "text" };
    case "date":
      return { ...base, type: "date" };
    case "partial":
      return { ...base, type: "string", description: f.hint ?? "A year, a month (2026-11) or a day" };
    case "number":
      return { ...base, type: "number" };
    case "bool":
      return { ...base, type: "boolean" };
    case "list":
      return { ...base, type: "string", list: true };
    case "select":
      return { ...base, type: "select", options: { values: f.options } };
    case "multiselect":
      return { ...base, type: "select", options: { multiple: true, values: f.options } };
    case "objects":
      return { ...base, type: "object", list: true, fields: f.fields.map((x) => pagesField(x, e)) };
    case "object":
      return { ...base, type: "object", fields: f.fields.map((x) => pagesField(x, e)) };
    case "body":
      return { name: "body", label: f.label, type: "rich-text" };
    case "media":
      return publicBase && mediaInput
        ? { ...base, type: "image", options: { media: "uploads" } }
        : {
            ...base,
            type: "string",
            description:
              f.hint ?? `Media key under ${f.prefix}/ (staged in ${mediaDir}/, synced by pnpm media:push)`,
          };
    case "relation":
      return {
        ...base,
        type: "string",
        list: f.many || undefined,
        description: f.hint ?? `Slug of an entry in ${f.target}`,
      };
    default:
      return { ...base, type: "string", ...(f.name === e.title ? { required: true } : {}) };
  }
}

function pagesEntry(e) {
  const fields = e.fields.map((f) => pagesField(f, e));
  if (e.kind === "collection") {
    return {
      name: e.key,
      label: e.label,
      type: "collection",
      path: e.path,
      format: "yaml-frontmatter",
      filename: { template: "{primary}.md", field: "create" },
      view: { primary: e.title, default: { sort: e.title, order: "asc" } },
      fields,
    };
  }
  return {
    name: e.key,
    label: e.label,
    type: "file",
    path: pagesFile(e),
    format: e.kind === "data" ? "json" : "yaml-frontmatter",
    fields,
  };
}

// Group by section the way the other editors' navigation does. Pages CMS finds an
// entry by its name, so a group's name is prefixed: a `Settings` group and the
// `settings.json` file would otherwise both be `settings`.
const pagesContent = [];
for (const e of entries) {
  let g = pagesContent.find((x) => x.label === e.group);
  if (!g) pagesContent.push((g = { name: `group-${e.group.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")}`, label: e.group, type: "group", items: [] }));
  g.items.push(pagesEntry(e));
}

const pagesConfig = {
  media: publicBase && mediaInput
    ? [{ name: "uploads", label: "Uploads", input: mediaInput, output: publicBase.replace(/\/$/, "") || "/" }]
    : undefined,
  content: pagesContent,
};

// A small YAML printer for plain data: strings are always double-quoted (JSON
// strings are valid YAML), so nothing is ever read as a number, a boolean or a
// date by accident.
function yaml(v, indent = 0) {
  const pad = "  ".repeat(indent);
  if (Array.isArray(v)) {
    if (v.length === 0) return " []";
    return `\n${v
      .map((x) => {
        if (x && typeof x === "object" && !Array.isArray(x)) {
          const body = yamlMap(x, indent + 1).replace(/^ +/, "");
          return `${pad}- ${body}`;
        }
        return `${pad}-${yaml(x, indent + 1)}`;
      })
      .join("\n")}`;
  }
  if (v && typeof v === "object") return `\n${yamlMap(v, indent)}`;
  return ` ${JSON.stringify(v)}`;
}
function yamlMap(o, indent) {
  const pad = "  ".repeat(indent);
  return Object.entries(o)
    .filter(([, v]) => v !== undefined)
    .map(([k, v]) => `${pad}${k}:${yaml(v, indent + 1)}`)
    .join("\n");
}

const pagesYml = `# Generated by \`fid derive\` (pipeline: cms) from content.toml, press/, messages/,
# settings.json and [i18n]. Do not edit — change the declaration and re-derive.
# \`fid derive --check\` fails if this file is stale.
#
# Read by Pages CMS (https://pagescms.org), which edits these same files through
# GitHub. It is the editor to choose when the people editing have no checkout.
${yamlMap(pagesConfig, 0)}
`;

// ── Emit: the editor workspace's package.json ────────────────────────────────
const pkg = {
  name: `@${productName}/cms`,
  version: "0.1.0",
  private: true,
  scripts: {
    dev:
      editor === "pages"
        ? 'node -e "console.error(\'Pages CMS is hosted: sign in at https://app.pagescms.org and open this repository.\')"'
        : editor === "tina"
          ? "node dev.mjs here tinacms dev"
          : "node dev.mjs root next dev apps/cms --port 4321",
  },
  dependencies:
    editor === "pages"
      ? {}
      : editor === "tina"
      ? { "@tinacms/cli": "^4.0.5", tinacms: "^3.15.2", react: "^19.0.0", "react-dom": "^19.0.0" }
      : {
          "@keystatic/core": "^0.6.9",
          "@keystatic/next": "^5.0.5",
          next: "^16.0.0",
          react: "^19.0.0",
          "react-dom": "^19.0.0",
        },
  devDependencies: {
    "@types/node": "^22.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    typescript: "^7.0.0",
  },
};

const stamp =
  "Generated by `fid derive` (pipeline: cms). Do not edit — set [cms] editor in fiducial.toml.";
function emit(dest, text) {
  mkdirSync(dirname(at(dest).pathname), { recursive: true });
  writeFileSync(at(dest), text);
}
emit("apps/cms/keystatic.config.ts", keystatic);
emit(".pages.yml", pagesYml);
emit("apps/cms/tina/config.ts", tina);
emit("apps/cms/package.json", `${JSON.stringify({ ...pkg, "//": stamp }, null, 2)}\n`);
console.log(
  `wrote apps/cms (${editor}; ${entries.length} editable item(s), ${locales.length} locale(s))`,
);

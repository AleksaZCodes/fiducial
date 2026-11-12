// content.toml + press/ + [i18n]  →  tools/cms/config.yml
//
// The editor's shape is derived, never written by hand.
//
// Decap CMS needs a config listing every collection, every field and every
// widget. That is the same information `content.toml` already declares and the
// same layout `press/` already has, so writing it out again would be a second
// declaration of the content model — and the one that silently drifts, because
// nothing renders from it. An editor offering a field the schema does not have
// writes a file `fid derive` then rejects; an editor missing a field leaves it
// blank on every entry created through it.
//
// So this reads the declarations and emits the config, and `fid derive --check`
// holds the two together. Change a collection's schema and the editor changes
// with it on the next derive.
//
// What it does NOT do: invent a content model. Every collection here exists
// because a declaration says so.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { parseToml } from "./derive-content.mjs";

const at = (p) => new URL(`../${p}`, import.meta.url);
const read = (p) => readFileSync(at(p), "utf8");
const has = (p) => existsSync(at(p));

const cfg = parseToml(read("fiducial.toml"));
const locales = cfg.i18n?.locales ?? ["en"];
const defaultLocale = cfg.i18n?.default ?? locales[0];
const cms = cfg.cms ?? {};
// Uploads land in the gitignored staging directory, keyed the way content
// refers to media everywhere else. `pnpm media:push` syncs them to the bucket.
const mediaDir = cms.media_dir ?? "media";
// The URL the site serves that directory at. A stored image value is this plus
// the file's path, so what the picker saves is what the page can load.
const publicBase = cms.public_base ?? "";
// Where a loose upload goes: one whose entry has no collection of its own,
// or one made from the Media tab rather than from a field.
//
// The Media tab shows exactly this folder and no other. Decap's media library
// is FLAT — the proxy lists the files directly inside a folder and does not
// walk into subdirectories — so there is no "everything" view to point it at.
// Each collection's own folder is what its image picker shows, which is where
// an entry's images are actually found.
const mediaPrefix = cms.media_prefix ?? "site/uploads";
// select: `[cms.selects]` maps "collection.field" → ["opt1", "opt2", ...]
// hint:   `[cms.hints]`   maps "collection.field" → "hint text"
const selects = cms.selects ?? {};
const hints = cms.hints ?? {};

const language = (l) => new Intl.DisplayNames(["en"], { type: "language" }).of(l) ?? l;

const collections = parseToml(read("content.toml")).collections ?? {};

/** Schema type → Decap widget. Unknown types fall back to a plain string. */
const WIDGET = {
  string: { widget: "string" },
  date: { widget: "datetime", date_format: "YYYY-MM-DD", time_format: false, picker_utc: true },
  "partial-date": {
    widget: "datetime",
    date_format: "YYYY-MM-DD",
    time_format: false,
    picker_utc: true,
  },
  number: { widget: "number", value_type: "int" },
  boolean: { widget: "boolean" },
  "string[]": { widget: "list", field: { name: "item", label: "Item", widget: "string" } },
  media: { widget: "image", allow_multiple: false },
};

const MAY_BE_EMPTY = {
  string: { required: false, default: "" },
  "string[]": { required: false, default: [] },
  media: { required: false, default: "" },
  "partial-date": { required: false, default: "" },
};

/**
 * Where a collection's images live, on disk and in the key space.
 *
 * These two have to agree with how keys are actually written, because of how
 * the editor resolves an existing one: it takes the **file name** out of the
 * stored value and looks for it under the field's `media_folder`. So a key of
 * `<prefix>/<file>` resolves, and `<prefix>/<extra>/<file>` does not — it asks
 * the proxy for a path that was never there, which is a 500 in the console and
 * a broken thumbnail on the page.
 *
 * `media` on the collection declares the prefix; the default is the
 * collection's own name, which is the shape a new collection should use.
 */
const mediaFolders = (prefix) => ({
  media_folder: `/${mediaDir}/${prefix}`,
  public_folder: `${publicBase}/${prefix}`,
});

// The folders are set on the COLLECTION, not on each media field: an image
// dropped into a markdown body is the collection's too, and a field-level
// folder leaves the body editor on the global default — which is how a post's
// inline images ended up being looked for under the uploads prefix.
const field = (name, type, extra = {}) => ({
  name,
  label: label(name),
  ...(WIDGET[type] ?? WIDGET.string),
  ...extra,
});

function label(name) {
  const words = name.replace(/[-_]/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

// ── Collections from content.toml ────────────────────────────────────────────
// `content/<collection>/<locale>/<slug>.md` is exactly Decap's
// `multiple_folders` i18n structure, so these are editable in every language
// side by side, which is the point: a missing translation is visible while you
// are writing rather than at the next build.
const out = [];

// `[cms.relations]` maps "collection.field" to the collection it points at, so the
// field is a searchable dropdown of that collection's entries, not a slug typed by hand.
const relations = { ...(cms.relations ?? {}) };
// Any field named `related<Collection>` points at that collection, so a new link
// kind is a field in content.toml and nothing else.
for (const [name, spec] of Object.entries(collections)) {
  for (const f of Object.keys(spec.schema ?? {})) {
    const m = /^related([A-Z]\w*)$/.exec(f);
    const target = m ? m[1].charAt(0).toLowerCase() + m[1].slice(1) : "";
    if (target && collections[target]) relations[`${name}.${f}`] ??= target;
  }
}
const titleKey = (target) =>
  ["title", "name", "headline"].find((k) => k in (collections[target]?.schema ?? {})) ?? "slug";

function buildField(name, spec, f, t, inLocale) {
  const key = `${name}.${f}`;
  const opts = selects[key];
  const hint = hints[key];
  // Facts that do not change with the language (dates, numbers, switches, choices, pictures,
  // links to other entries, addresses) are written once in the default language and copied
  // to the others. Text is written per language.
  const sameEverywhere =
    ["date", "partial-date", "number", "boolean", "media"].includes(t) ||
    relations[key] ||
    selects[key] ||
    /^(href|url|link)$/.test(f);
  const i18n = inLocale ? { i18n: sameEverywhere ? "duplicate" : true } : {};
  if (t === "object[]") {
    const shape = Object.entries(spec.objects?.[f] ?? {});
    return {
      name: f,
      label: label(f),
      widget: "list",
      label_singular: label(f).replace(/s$/, ""),
      summary: `{{fields.${(shape.find(([, kt]) => kt !== "media") ?? shape[0] ?? ["label"])[0]}}}`,
      ...(inLocale ? { i18n: true } : {}),
      required: false,
      default: [],
      ...(hint ? { hint } : {}),
      fields: shape.map(([k, kt]) =>
        field(k, kt, {
          ...MAY_BE_EMPTY[kt],
          ...(hints[`${key}.${k}`] ? { hint: hints[`${key}.${k}`] } : {}),
        }),
      ),
    };
  }
  if (relations[key]) {
    const target = relations[key];
    const many = t === "string[]";
    return {
      name: f,
      label: label(f),
      widget: "relation",
      collection: target,
      search_fields: [titleKey(target)],
      value_field: "{{slug}}",
      display_fields: [titleKey(target)],
      multiple: many,
      ...i18n,
      required: false,
      default: many ? [] : "",
      ...(hint ? { hint } : {}),
    };
  }
  if (opts) {
    return {
      name: f,
      label: label(f),
      widget: "select",
      options: opts,
      ...i18n,
      required: false,
      default: opts[0] ?? "",
      ...(hint ? { hint } : {}),
    };
  }
  return field(f, t, {
    ...i18n,
    ...MAY_BE_EMPTY[t],
    ...(hint ? { hint } : {}),
  });
}

const bodyField = (inLocale) => ({
  name: "body",
  label: "Body",
  widget: "markdown",
  required: false,
  ...(inLocale ? { i18n: true } : {}),
});

for (const [name, spec] of Object.entries(collections)) {
  const prefix = spec.media ?? name;
  const schema = Object.entries(spec.schema ?? {});

  if (spec.kind === "singleton") {
    // A singleton is one named file per language, not a one-entry folder. A folder
    // collection of one lists an unnamed record and loads blank when the title field
    // is empty; a file entry is always found by its path and always has a name.
    out.push({
      name,
      label: label(name),
      files: locales.map((l) => ({
        name: `${name}-${l}`,
        label: `${label(name)}: ${language(l)}`,
        file: `content/${name}/${l}/${name}.md`,
        format: "frontmatter",
        ...mediaFolders(prefix),
        fields: [
          ...schema.map(([f, t]) => buildField(name, spec, f, t, false)),
          ...(spec.body === "rich" ? [bodyField(false)] : []),
        ],
      })),
    });
    continue;
  }

  out.push({
    name,
    label: label(name),
    folder: `content/${name}`,
    ...mediaFolders(prefix),
    create: true,
    delete: true,
    slug: "{{slug}}",
    extension: "md",
    format: "frontmatter",
    i18n: true,
    fields: [
      ...schema.map(([f, t]) => buildField(name, spec, f, t, true)),
      ...(spec.body === "rich" ? [bodyField(true)] : []),
    ],
  });
}

// ── The press room ───────────────────────────────────────────────────────────
// `press/<locale>/stories/*.md` puts the locale ABOVE the collection, which is
// the opposite of Decap's `multiple_folders` (`<folder>/<locale>/<slug>`). So
// the stories of each language are their own collection rather than one
// collection edited in two languages. That is a real loss — the two are not
// side by side — and the alternative was moving the press layout to suit the
// editor, which is the tail wagging the dog.
const STORY_FIELDS = [
  ["title", "string"],
  ["angle", "string"],
  ["date", "date"],
  ["summary", "string"],
  ["cover", "media"],
  ["cover_alt", "string"],
];
// The angles a story can take, from the capability's own list. A free-text
// angle would not match `press.angle.*` in the message catalogue, and the label
// on the press page would come out blank.
const ANGLES = ["origin", "technical", "customer", "failure", "milestone", "people", "market"];

// The press room, whose layout nests the locale under each collection exactly
// as `content/` does — press/docs/<locale>/ and press/stories/<locale>/ — so
// both are folder collections the editor shows in every language side by side.
// A file collection cannot do that: Decap supports i18n on one only as a
// single file holding every language, which is not how these are written.
if (has("press/docs") || has("press/stories")) {
  if (has("press/docs")) {
    out.push({
      name: "press-docs",
      label: "Press: boilerplate and facts",
      folder: "press/docs",
      // Two fixed documents per locale, not a list someone adds to.
      create: false,
      delete: false,
      extension: "md",
      format: "frontmatter",
      i18n: true,
      // These two documents are body only — there is no title to infer an
      // entry name from, and Decap says so loudly. The file name is the name:
      // `boilerplate` and `facts` are what they are called everywhere else.
      fields: [
        // A title, because an entry list with nothing to name entries by
        // renders the whole document as its own label. It is the document's
        // name in that language, not a heading the press page prints.
        { name: "title", label: "Name", widget: "string", i18n: true },
        { name: "body", label: "Text", widget: "markdown", i18n: true },
      ],
    });
  }

  if (has("press/stories")) {
    out.push({
      name: "press-stories",
      label: "Press stories",
      folder: "press/stories",
      ...mediaFolders("press/stories"),
      create: true,
      delete: true,
      slug: "{{slug}}",
      extension: "md",
      format: "frontmatter",
      i18n: true,
      fields: [
        ...STORY_FIELDS.map(([f, t]) =>
          f === "angle"
            ? {
                name: "angle",
                label: "Angle",
                widget: "select",
                options: ANGLES,
                i18n: "duplicate",
              }
            : field(f, t, { i18n: t === "date" ? "duplicate" : true }),
        ),
        // The story itself. It was missing, which made the editor a form for
        // everything about a story except the story.
        { name: "body", label: "Story", widget: "markdown", i18n: true },
      ],
    });
  }
}

// ── Settings: the facts that are not entries ─────────────────────────────────
// `settings.toml` (links, ids, a form's address: the same in every language) and
// `messages/<locale>.json` (every string the interface prints) are files, not
// collections of entries, so they are file collections. Their fields are read
// from the files themselves, so a new key appears in the form on the next
// derive and nothing is declared twice.
//
// `[cms] settings` lists the TOML files to offer; it defaults to `settings.toml`.
const walkFields = (value) =>
  Object.entries(value).map(([k, v]) => {
    if (Array.isArray(v)) return { name: k, label: label(k), widget: "list", required: false };
    if (v && typeof v === "object") {
      return { name: k, label: label(k), widget: "object", collapsed: true, fields: walkFields(v) };
    }
    if (typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v)) {
      return { name: k, label: label(k), ...WIDGET.date, required: false };
    }
    // A line break, or a long string, gets a text area; the rest one line.
    // `{placeholders}` and `[highlight]` marks are part of the text.
    const long = typeof v === "string" && (/\n/.test(v) || v.length > 90);
    return { name: k, label: label(k), widget: long ? "text" : "string", required: false };
  });

const settings = [];
for (const file of cms.settings ?? ["settings.toml"]) {
  if (!has(file)) continue;
  settings.push({
    name: file.replace(/\.toml$/, "").replace(/[^\w-]/g, "-"),
    label: label(file.replace(/\.toml$/, "")),
    file,
    format: "toml",
    fields: walkFields(parseToml(read(file))),
  });
}
for (const l of locales) {
  if (has(`messages/${l}.json`)) {
    settings.push({
      name: `messages-${l}`,
      label: `Interface text: ${language(l)}`,
      file: `messages/${l}.json`,
      format: "json",
      fields: walkFields(JSON.parse(read(`messages/${l}.json`))),
    });
  }
}
if (settings.length > 0) out.push({ name: "settings", label: "Settings", files: settings });

// ── The config ───────────────────────────────────────────────────────────────
// Written as YAML by hand rather than through a dependency: this is a fixed,
// shallow shape that this file alone produces, and a serialiser would be a
// package to keep current for thirty lines of output.
function yaml(value, indent = 0) {
  const pad = "  ".repeat(indent);
  if (Array.isArray(value)) {
    return value
      .map((v) => {
        const body = yaml(v, indent + 1);
        return typeof v === "object" && v !== null
          ? `${pad}- ${body.trimStart()}`
          : `${pad}- ${body.trim()}`;
      })
      .join("\n");
  }
  if (value && typeof value === "object") {
    return Object.entries(value)
      .map(([k, v]) => {
        if (Array.isArray(v) || (v && typeof v === "object")) {
          return `${pad}${k}:\n${yaml(v, indent + 1)}`;
        }
        return `${pad}${k}: ${scalar(v)}`;
      })
      .join("\n");
  }
  return `${pad}${scalar(value)}`;
}
const scalar = (v) => {
  if (typeof v === "boolean" || typeof v === "number") return String(v);
  const s = String(v);
  // Quote anything that is not plainly a word: `{{slug}}` unquoted is a YAML
  // flow mapping, not a template.
  return /^[\w./\- ]+$/.test(s) && !/^\s|\s$/.test(s) ? s : JSON.stringify(s);
};

const config = {
  // `git-gateway` is a placeholder the local proxy never uses. `pnpm cms`
  // serves this editor against the working tree, so edits are ordinary file
  // changes you review and commit like any other.
  backend: { name: "git-gateway" },
  local_backend: true,
  locale: defaultLocale,
  media_folder: `${mediaDir}/${mediaPrefix}`,
  public_folder: `${publicBase}/${mediaPrefix}`,
  i18n: { structure: "multiple_folders", locales, default_locale: defaultLocale },
  collections: out,
};

const header = [
  "# Generated by `node scripts/derive-cms.mjs` from content.toml, press/, settings.toml, messages/ and",
  "# `[i18n]` in fiducial.toml. Do not edit — change the declaration and",
  "# re-derive. `fid derive --check` fails if this file is stale.",
  "",
].join("\n");

const dest = "tools/cms/config.yml";
mkdirSync(dirname(at(dest).pathname), { recursive: true });
writeFileSync(at(dest), `${header}${yaml(config)}\n`);
console.log(`wrote ${dest} (${out.length} collection(s), ${locales.length} locale(s))`);

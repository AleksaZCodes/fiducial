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
import { mkdirSync, readFileSync, readdirSync, existsSync, writeFileSync } from "node:fs";
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
// Where a loose upload goes: one whose entry has no collection of its own,
// or one made from the Media tab rather than from a field.
//
// The Media tab shows exactly this folder and no other. Decap's media library
// is FLAT — the proxy lists the files directly inside a folder and does not
// walk into subdirectories — so there is no "everything" view to point it at.
// Each collection's own folder is what its image picker shows, which is where
// an entry's images are actually found.
const mediaPrefix = cms.media_prefix ?? "site/uploads";

const collections = parseToml(read("content.toml")).collections ?? {};

/** Schema type → Decap widget. Unknown types fall back to a plain string. */
const WIDGET = {
  string: { widget: "string" },
  date: { widget: "datetime", date_format: "YYYY-MM-DD", time_format: false, picker_utc: true },
  number: { widget: "number", value_type: "int" },
  boolean: { widget: "boolean" },
  "string[]": { widget: "list" },
  media: { widget: "image", allow_multiple: false },
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
  public_folder: prefix,
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
for (const [name, spec] of Object.entries(collections)) {
  const prefix = spec.media ?? name;
  const fields = Object.entries(spec.schema ?? {}).map(([f, t]) =>
    // A date is the same instant in every language; duplicating it keeps the
    // locales from drifting apart by a day.
    field(f, t, { i18n: t === "date" ? "duplicate" : true }),
  );
  if (spec.body === "rich") {
    fields.push({ name: "body", label: "Body", widget: "markdown", i18n: true });
  }
  out.push({
    name,
    label: label(name),
    folder: `content/${name}`,
    ...mediaFolders(prefix),
    create: spec.kind !== "singleton",
    delete: spec.kind !== "singleton",
    slug: "{{slug}}",
    extension: "md",
    format: "frontmatter",
    i18n: true,
    fields,
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
  public_folder: mediaPrefix,
  i18n: { structure: "multiple_folders", locales, default_locale: defaultLocale },
  collections: out,
};

const header = [
  "# Generated by `node scripts/derive-cms.mjs` from content.toml, press/ and",
  "# `[i18n]` in fiducial.toml. Do not edit — change the declaration and",
  "# re-derive. `fid derive --check` fails if this file is stale.",
  "",
].join("\n");

const dest = "tools/cms/config.yml";
mkdirSync(dirname(at(dest).pathname), { recursive: true });
writeFileSync(at(dest), `${header}${yaml(config)}\n`);
console.log(`wrote ${dest} (${out.length} collection(s), ${locales.length} locale(s))`);

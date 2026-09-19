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
const mediaPrefix = cms.media_prefix ?? "site/uploads";

const collections = parseToml(read("content.toml")).collections ?? {};

/** Schema type → Decap widget. Unknown types fall back to a plain string. */
const WIDGET = {
  string: { widget: "string" },
  date: { widget: "datetime", date_format: "YYYY-MM-DD", time_format: false, picker_utc: true },
  number: { widget: "number", value_type: "int" },
  boolean: { widget: "boolean" },
  "string[]": { widget: "list" },
  // A media key, not a URL: `media_folder` stages the file locally and
  // `public_folder` is the prefix that ends up in the frontmatter.
  media: {
    widget: "image",
    media_folder: `/${mediaDir}/${mediaPrefix}`,
    public_folder: mediaPrefix,
    allow_multiple: false,
  },
};

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

if (has("press")) {
  const files = [];
  for (const loc of locales) {
    for (const doc of ["boilerplate", "facts"]) {
      if (!has(`press/${loc}/${doc}.md`)) continue;
      files.push({
        name: `${doc}-${loc}`,
        label: `${label(doc)} (${loc})`,
        file: `press/${loc}/${doc}.md`,
        fields: [{ name: "body", label: label(doc), widget: "markdown" }],
      });
    }
  }
  if (files.length) {
    out.push({ name: "press-docs", label: "Press: boilerplate and facts", files });
  }

  for (const loc of locales) {
    if (!has(`press/${loc}/stories`)) continue;
    out.push({
      name: `press-stories-${loc}`,
      label: `Press stories (${loc})`,
      folder: `press/${loc}/stories`,
      create: true,
      delete: true,
      slug: "{{slug}}",
      extension: "md",
      format: "frontmatter",
      fields: STORY_FIELDS.map(([f, t]) =>
        f === "angle"
          ? { name: "angle", label: "Angle", widget: "select", options: ANGLES }
          : field(f, t),
      ),
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

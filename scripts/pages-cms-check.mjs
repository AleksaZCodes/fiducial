// Holds a derived `.pages.yml` to Pages CMS's own schema.
//
// Pages CMS tolerates a malformed config without a word, so an emitter that is
// wrong shows up as a missing field in someone's editor, not as an error. This
// runs the file through the application's own strict zod schema
// (`lib/config-schema.ts`), with its field-type registry stubbed to the names it
// registers.
//
// Usage: node pages-cms-check.mjs <schema.ts, already stubbed> <.pages.yml>
// (scripts/verify-cms-configs.sh prepares the schema and the dependencies.)
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { resolve } from "node:path";

const [schemaPath, file] = process.argv.slice(2);
const { ConfigSchema } = await import(pathToFileURL(resolve(schemaPath)).href);
const YAML = (await import("yaml")).default;

const doc = YAML.parse(readFileSync(file, "utf8"));
const r = ConfigSchema.safeParse(doc);
if (!r.success) {
  for (const i of r.error.issues) console.error(`${i.path.join(".")}: ${i.message}`);
  process.exit(1);
}

// What the schema does not say: every `reference` names a collection that exists,
// every `image` names a media entry that exists, and names are unique.
const names = [];
const collections = new Set();
const refs = [];
const images = [];
const walkFields = (fields) => {
  for (const f of fields ?? []) {
    if (f.type === "reference") refs.push(f.options?.collection);
    if (f.type === "image" && typeof f.options?.media === "string") images.push(f.options.media);
    walkFields(f.fields);
  }
};
const walk = (items) => {
  for (const i of items) {
    names.push(i.name);
    if (i.type === "group") walk(i.items);
    else {
      if (i.type === "collection") collections.add(i.name);
      walkFields(i.fields);
    }
  }
};
walk(doc.content ?? []);
const dupes = names.filter((n, k) => names.indexOf(n) !== k);
if (dupes.length) throw new Error(`duplicate names: ${[...new Set(dupes)].join(", ")}`);
for (const c of refs) if (!collections.has(c)) throw new Error(`reference to a collection that does not exist: ${c}`);
const media = new Set((Array.isArray(doc.media) ? doc.media : []).map((m) => m.name));
for (const m of images) if (!media.has(m)) throw new Error(`image names media "${m}", which is not declared`);
console.log(`valid: ${names.length} names, ${refs.length} reference(s), ${images.length} image(s)`);

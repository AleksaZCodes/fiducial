// The route list → generated/seo.ts + public/sitemap.xml
//
// ## Routes were the missing declaration
//
// `fid-brand` writes a sitemap with one URL in it — the domain root — and says
// why in its own comment: a product's route list is "a router's declaration,
// not the brand's", left to be extended by hand "until routes become their own
// declaration". By hand meant: the site grew a press room, a blog, stories,
// posts, legal pages and a credits page, and every one of them was missing
// from the sitemap, unadvertised to any crawler, with nothing failing.
//
// This is that declaration, and it is not a new list to keep. Every route here
// is computed from something that already declares it:
//
//   · `[i18n] locales`          — every page exists once per language
//   · `content.toml` `route`    — a collection already says where it mounts
//   · `press/<locale>/stories/` — the stories that exist
//   · `generated/legal.ts`      — the pages the jurisdiction requires
//   · `[seo] pages`             — the handful of routes that are just pages
//
// Add a post and it is in the sitemap. Add a locale and every route doubles.
//
// ## Two outputs, one source
//
// `sitemap.xml` is for crawlers. `generated/seo.ts` is for the application:
// the title, description, canonical and alternates each page renders, and the
// key of its social image. They come from one pass so a page cannot advertise
// one canonical to a crawler and another to a reader.
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
// `toml-lite.mjs`, not `derive-content.mjs`, for `parseToml`.
//
// `derive-content.mjs` is a pipeline executor with no main-module guard: its
// body runs on import. Importing it for one helper therefore re-derived the
// whole content model as a side effect of running THIS pipeline — an
// undeclared write to `generated/content.ts` from a pipeline that does not
// list it as an output, which is the exact class of thing
// `fid derive --check` exists to catch. It was invisible because content's own
// pipeline writes the same bytes, so the artifact was never stale; it would
// have surfaced the first time the two disagreed, as a file changing under a
// pipeline that never claimed it.
//
// `toml-lite.mjs` is a parser and nothing else, and it already handles the
// arrays-across-several-lines that an `outputs` list is always written as.
import { parseToml } from "./toml-lite.mjs";
import { frontmatter } from "./frontmatter.mjs";

const at = (p) => new URL(`../${p}`, import.meta.url);
const read = (p) => readFileSync(at(p), "utf8");
const readJson = (p) => JSON.parse(read(p));

const fail = (msg) => {
  console.error(`derive-seo: ${msg}`);
  process.exit(1);
};

const cfg = parseToml(read("fiducial.toml"));
const locales = cfg.i18n?.locales ?? ["en"];
const defaultLocale = cfg.i18n?.default ?? locales[0];
const domain = cfg.brand?.domain ?? fail("[brand] domain is not set");
const origin = `https://${domain}`;
const seo = cfg.seo ?? {};

// Routes that are pages rather than entries. Declared, because a page with no
// collection behind it has nothing else to derive it from.
//
// Each entry is either a path, or a table naming the catalogue keys its title
// and description live under:
//
//   pages = [
//     { path = "/",      title = "meta.title",  description = "meta.description" },
//     { path = "/blog",  title = "blog.title",  description = "blog.metaDescription" },
//   ]
//
// A bare path keeps working and resolves through `CONVENTIONAL_KEYS` below.
// The keys are declarable because the script used to hold a table of four
// paths — `/`, `/press`, `/blog`, `/credits` — with one product's key
// spellings baked in, and every other product's page set hit
// "no title is declared for it". That message named the wrong problem: the
// page HAD a title, in a key this script had never heard of. A product also
// legitimately declares that copy somewhere other than the catalogue, and then
// the only way to satisfy a hard-coded key is to write the string a second
// time — a duplicate of a declared fact, which is the one thing this platform
// is for preventing.
const CONVENTIONAL_KEYS = {
  "/": ["meta.title", "meta.description"],
  "/press": ["press.title", "press.metaDescription"],
  "/blog": ["blog.title", "blog.metaDescription"],
  "/credits": ["credits.title", "credits.intro"],
};

const staticPages = (seo.pages ?? ["/"]).map((entry) => {
  if (typeof entry === "string") {
    const keys = CONVENTIONAL_KEYS[entry];
    if (!keys) {
      fail(
        `[seo] pages lists ${entry}, and this script has no conventional title key for it.\n` +
          "  Declare the keys with the path instead of listing the path alone:\n" +
          `    { path = "${entry}", title = "<key>", description = "<key>" }\n` +
          "  They are message keys, resolved in each locale's catalogue.",
      );
    }
    return { path: entry, title: keys[0], description: keys[1] };
  }
  if (!entry.path || !entry.title || !entry.description) {
    fail(
      `[seo] pages has an entry missing path, title or description: ${JSON.stringify(entry)}`,
    );
  }
  return entry;
});

// ── Where the outputs go ─────────────────────────────────────────────────────
//
// Read from `pipelines/seo.toml` rather than written here. The pipeline's
// `outputs` is already the declaration of where these three artifacts land —
// it is what `fid derive --check` hashes — so a second copy of the paths in
// this script is a copy that can disagree with it.
//
// It did disagree. Both were written for Next.js, where static assets live in
// `public/`. A SvelteKit product's live at `static/`, and adopting this
// capability there meant editing the pipeline (the documented one-line change)
// while the script kept writing `public/sitemap.xml`: the file the pipeline
// declared was never produced, so `fid derive --check` failed on a missing
// artifact and pointed at neither the cause nor the fix. Reading the
// declaration makes that edit sufficient, which is what it was supposed to be.
const pipeline = existsSync(at("pipelines/seo.toml"))
  ? parseToml(read("pipelines/seo.toml"))
  : fail("pipelines/seo.toml is missing — this script is the `seo` pipeline's executor");
const declaredOutputs = pipeline.outputs ?? [];

/** The declared output ending in `name`, or a failure naming what to add. */
const output = (name) =>
  declaredOutputs.find((o) => o.endsWith(`/${name}`) || o === name) ??
  fail(
    `pipelines/seo.toml declares no output ending in \`${name}\`.\n` +
      `  Its outputs are ${JSON.stringify(declaredOutputs)}.\n` +
      "  This script derives seo.ts, seo.json and sitemap.xml; each one needs a\n" +
      "  declared path, because that path is what `fid derive --check` guards.",
  );

const tsOut = output("seo.ts");
const jsonOut = output("seo.json");
const xmlOut = output("sitemap.xml");

// The brand pipeline must not also own the sitemap: two pipelines writing one
// file means the last one to run wins, silently, and which one that is depends
// on pipeline names. Removing it there is a one-line edit, so this says so
// rather than fighting over the file.
//
// Matched on the file name, not on a full path. The hard-coded
// `apps/web/public/sitemap.xml` this replaces matched nothing in a SvelteKit
// product — whose brand pipeline says `apps/web/static/sitemap.xml` — so the
// race this guard exists to prevent was left in place precisely where the
// paths differ, which is where it was most likely to happen.
const brandPipeline = existsSync(at("pipelines/brand.toml")) ? read("pipelines/brand.toml") : "";
const brandSitemap = (parseToml(brandPipeline || "outputs = []").outputs ?? []).find((o) =>
  o.endsWith("/sitemap.xml"),
);
if (brandSitemap) {
  fail(
    `pipelines/brand.toml still lists ${brandSitemap} as an output.\n` +
      "  Remove that line: `seo` owns the sitemap now, because it is the one that\n" +
      "  knows the routes. Two pipelines writing one artifact is a race decided by\n" +
      "  pipeline name order.",
  );
}

const prefix = (locale) => (locale === defaultLocale ? "" : `/${locale}`);
const localized = (locale, path) => `${prefix(locale)}${path === "/" ? "" : path}` || "/";

const messages = Object.fromEntries(locales.map((l) => [l, readJson(`messages/${l}.json`)]));
const msg = (locale, path) =>
  path.split(".").reduce((o, k) => (o == null ? undefined : o[k]), messages[locale]);

// ── The routes ───────────────────────────────────────────────────────────────
/** @type {{path: string, locale: string, key: string, title: string, description: string, kind: string, date?: string}[]} */
const routes = [];
const add = (r) => routes.push(r);

// `key` identifies the same page across languages, so the alternates of a page
// are the other routes with the same key. A page whose translation does not
// exist simply has no alternate, rather than pointing at a 404.
for (const locale of locales) {
  for (const { path: page, title: titleKey, description: descKey } of staticPages) {
    const title = msg(locale, titleKey);
    const description = msg(locale, descKey);
    if (!title || !description) {
      fail(`messages/${locale}.json is missing \`${!title ? titleKey : descKey}\` for ${page}`);
    }
    add({
      path: localized(locale, page),
      locale,
      key: `page${page}`,
      title,
      description,
      kind: page === "/" ? "home" : "page",
    });
  }
}

// Legal pages: the set the jurisdiction produced, read from the derived
// catalogue rather than re-listed here.
const legalTs = existsSync(at("apps/web/src/generated/legal.ts"))
  ? read("apps/web/src/generated/legal.ts")
  : "";
const legalPages =
  legalTs.match(/export type LegalPage = ([^;]+);/)?.[1].match(/"([^"]+)"/g)?.map((s) =>
    s.slice(1, -1),
  ) ?? [];
for (const locale of locales) {
  for (const page of legalPages) {
    // An authored override owns the title when it exists; otherwise the
    // generated catalogue's heading is the page's real title.
    const overridePath = `content/legal-pages/${locale}/${page}.md`;
    const catalog = legalTs.split(`export const ${locale}: LegalCatalog`)[1] ?? "";
    const title = existsSync(at(overridePath))
      ? (frontmatter(read(overridePath), () => {}).meta.title ??
        page.charAt(0).toUpperCase() + page.slice(1))
      : (catalog.split(`${page}:`)[1]?.match(/title:\s*"([^"]+)"/)?.[1] ??
        page.charAt(0).toUpperCase() + page.slice(1));
    add({
      path: `${prefix(locale)}/legal/${page}`,
      locale,
      key: `legal/${page}`,
      title,
      description: msg(locale, "meta.description"),
      kind: "page",
    });
  }
}

// Collections that mount somewhere: `content.toml` already declares the route.
const collections = parseToml(read("content.toml")).collections ?? {};
for (const [name, spec] of Object.entries(collections)) {
  if (!spec.route) continue;
  for (const locale of locales) {
    const dir = `content/${name}/${locale}`;
    if (!existsSync(at(dir))) continue;
    for (const file of readdirSync(at(dir)).filter((f) => f.endsWith(".md"))) {
      const slug = file.replace(/\.md$/, "");
      const { meta } = frontmatter(read(`${dir}/${file}`), (m) => fail(`${dir}/${file}: ${m}`));
      add({
        path: `${prefix(locale)}${spec.route.replace("{slug}", slug)}`,
        locale,
        key: `${name}/${slug}`,
        title: meta.title ?? slug,
        description: meta.summary ?? "",
        kind: "article",
        date: meta.date ?? undefined,
      });
    }
  }
}

// Press stories.
for (const locale of locales) {
  const dir = `press/stories/${locale}`;
  if (!existsSync(at(dir))) continue;
  for (const file of readdirSync(at(dir)).filter((f) => f.endsWith(".md"))) {
    const slug = file.replace(/\.md$/, "");
    const { meta } = frontmatter(read(`${dir}/${file}`), (m) => fail(`${dir}/${file}: ${m}`));
    add({
      path: `${prefix(locale)}/press/${slug}`,
      locale,
      key: `press/${slug}`,
      title: meta.title ?? slug,
      description: meta.summary ?? "",
      kind: "article",
      date: meta.date ?? undefined,
    });
  }
}

routes.sort((a, b) => a.path.localeCompare(b.path));

// ── generated/seo.ts ─────────────────────────────────────────────────────────
// The social image lives at a path per route, rendered at build time by
// `scripts/render-og.mjs`. It is not committed: it is a picture of facts the
// repository already holds, so it is rebuilt rather than stored.
const ogPath = (r) => `/og/${r.locale}/${r.key.replace(/^page\//, "").replace(/\//g, "-") || "home"}.png`;

const byKey = new Map();
for (const r of routes) {
  if (!byKey.has(r.key)) byKey.set(r.key, []);
  byKey.get(r.key).push(r);
}

const entries = routes.map((r) => ({
  path: r.path,
  locale: r.locale,
  key: r.key,
  kind: r.kind,
  title: r.title,
  description: r.description,
  og: ogPath(r),
  ...(r.date ? { date: r.date } : {}),
  alternates: Object.fromEntries(byKey.get(r.key).map((o) => [o.locale, o.path])),
}));

const ts = `// Generated by \`node scripts/derive-seo.mjs\`. Do not edit.
//
// One entry per route per locale: what the page tells a crawler and a reader.
// \`alternates\` is every language this page exists in, which is what hreflang
// needs and what the language picker uses to stay on the page.
//
// \`og\` is where the page's social image is, rendered at build time from these
// same fields by scripts/render-og.mjs.

export interface SeoRoute {
  readonly path: string;
  readonly locale: string;
  /** The same page in another language shares this key. */
  readonly key: string;
  readonly kind: "home" | "page" | "article";
  readonly title: string;
  readonly description: string;
  readonly og: string;
  readonly date?: string;
  readonly alternates: Readonly<Record<string, string>>;
}

export const seoRoutes = ${JSON.stringify(entries, null, 2)} as const satisfies readonly SeoRoute[];

/** The entry for a path, or undefined if the path is not a declared route. */
export function seoFor(path: string): SeoRoute | undefined {
  return seoRoutes.find((r) => r.path === path);
}
`;

mkdirSync(dirname(at(tsOut).pathname), { recursive: true });
writeFileSync(at(tsOut), ts);
console.log(`wrote ${tsOut} (${entries.length} route(s), ${locales.length} locale(s))`);

// The same routes as JSON, for the node scripts that also need them — the
// social-image renderer runs before the app is built, so it cannot import a
// TypeScript module. Same rule as the logo pipeline: two readers, one source.
writeFileSync(at(jsonOut), `${JSON.stringify({ origin, defaultLocale, routes: entries }, null, 2)}\n`);
console.log(`wrote ${jsonOut}`);

// ── sitemap.xml ──────────────────────────────────────────────────────────────
// With `xhtml:link` alternates, which is what tells a search engine that two
// URLs are the same page in two languages rather than duplicates competing
// with each other.
const xml = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">',
];
for (const r of entries) {
  xml.push("  <url>");
  // The root is `https://host`, not `https://host/`: that is the form the page
  // itself declares as canonical, and a sitemap that names a different string
  // for the same page is a discrepancy a crawler has to resolve on its own.
  xml.push(`    <loc>${origin}${r.path === "/" ? "" : r.path}</loc>`);
  for (const [loc, path] of Object.entries(r.alternates)) {
    xml.push(
      `    <xhtml:link rel="alternate" hreflang="${loc}" href="${origin}${path === "/" ? "" : path}"/>`,
    );
  }
  if (r.alternates[defaultLocale]) {
    xml.push(
      `    <xhtml:link rel="alternate" hreflang="x-default" href="${origin}${r.alternates[defaultLocale] === "/" ? "" : r.alternates[defaultLocale]}"/>`,
    );
  }
  if (r.date) xml.push(`    <lastmod>${r.date}</lastmod>`);
  xml.push("  </url>");
}
xml.push("</urlset>", "");

mkdirSync(dirname(at(xmlOut).pathname), { recursive: true });
writeFileSync(at(xmlOut), xml.join("\n"));
console.log(`wrote ${xmlOut} (${entries.length} url(s))`);

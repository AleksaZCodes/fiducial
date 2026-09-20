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
import { parseToml } from "./derive-content.mjs";
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
const staticPages = seo.pages ?? ["/"];

// The brand pipeline must not also own the sitemap: two pipelines writing one
// file means the last one to run wins, silently, and which one that is depends
// on pipeline names. Removing it there is a one-line edit, so this says so
// rather than fighting over the file.
const brandPipeline = existsSync(at("pipelines/brand.toml")) ? read("pipelines/brand.toml") : "";
if (/["']apps\/web\/public\/sitemap\.xml["']/.test(brandPipeline)) {
  fail(
    "pipelines/brand.toml still lists apps/web/public/sitemap.xml as an output.\n" +
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
  for (const page of staticPages) {
    const labels = {
      "/": ["meta.title", "meta.description"],
      "/press": ["press.title", "press.metaDescription"],
      "/blog": ["blog.title", "blog.metaDescription"],
      "/credits": ["credits.title", "credits.intro"],
    }[page];
    if (!labels) fail(`[seo] pages lists ${page}, but no title is declared for it`);
    const [titleKey, descKey] = labels;
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

const tsOut = "apps/web/src/generated/seo.ts";
mkdirSync(dirname(at(tsOut).pathname), { recursive: true });
writeFileSync(at(tsOut), ts);
console.log(`wrote ${tsOut} (${entries.length} route(s), ${locales.length} locale(s))`);

// The same routes as JSON, for the node scripts that also need them — the
// social-image renderer runs before the app is built, so it cannot import a
// TypeScript module. Same rule as the logo pipeline: two readers, one source.
const jsonOut = "apps/web/src/generated/seo.json";
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
  xml.push(`    <loc>${origin}${r.path}</loc>`);
  for (const [loc, path] of Object.entries(r.alternates)) {
    xml.push(`    <xhtml:link rel="alternate" hreflang="${loc}" href="${origin}${path}"/>`);
  }
  if (r.alternates[defaultLocale]) {
    xml.push(
      `    <xhtml:link rel="alternate" hreflang="x-default" href="${origin}${r.alternates[defaultLocale]}"/>`,
    );
  }
  if (r.date) xml.push(`    <lastmod>${r.date}</lastmod>`);
  xml.push("  </url>");
}
xml.push("</urlset>", "");

const xmlOut = "apps/web/public/sitemap.xml";
mkdirSync(dirname(at(xmlOut).pathname), { recursive: true });
writeFileSync(at(xmlOut), xml.join("\n"));
console.log(`wrote ${xmlOut} (${entries.length} url(s))`);

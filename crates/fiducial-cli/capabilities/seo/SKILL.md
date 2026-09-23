# fiducial:seo — Routes, Metadata and Social Images

This product has the `seo` capability. It exists because **routes were the one
fact nothing declared.**

## The failure it fixes

`fid-brand` writes `sitemap.xml` with a single URL in it, and says why in its
own comment: a product's route list "is a router's declaration, not the
brand's", left to be extended by hand "until routes become their own
declaration".

Nobody extends it by hand. What happens instead, every time, is that the site
grows a blog, a press room, legal pages and a handful of entries, and:

- none of them is in the sitemap, so nothing advertises them;
- every page inherits the landing page's `<title>`, description and
  **canonical**, so each one tells a search engine it is the home page;
- no page has a social image, so every link anyone shares is a bare URL.

None of that fails a build. The site looks finished.

## What is derived

`scripts/derive-seo.mjs` computes one entry per route per locale from what
already declares it:

| Source | Gives |
|---|---|
| `[i18n] locales` / `default` | every route, once per language, with the default locale unprefixed |
| `content.toml` `route` | a collection's entries — the collection already says where it mounts |
| `press/<locale>/stories/` | the stories that exist |
| `generated/legal.ts` | the pages the jurisdiction produced |
| `[seo] pages` | the routes that are plain pages |

Outputs:

- **`generated/seo.ts`** — `seoRoutes` and `seoFor(path)`: title, description,
  canonical, alternates, social image, and the date for articles.
- **`generated/seo.json`** — the same, for scripts that cannot import
  TypeScript (the image renderer runs before the app is built).
- **`sitemap.xml`** — every route, with `xhtml:link` alternates and
  `x-default`, which is what tells a crawler two URLs are one page in two
  languages rather than duplicates competing with each other.

All three land wherever `pipelines/seo.toml` says. The seeded paths are
Next.js's (`apps/web/public/…`); on SvelteKit, change `public` to `static`
there and nothing else — the deriver reads its own output paths out of that
pipeline rather than carrying a second copy of them.

Add a post: it is in the sitemap. Add a locale: every route doubles. Neither is
a list anyone maintains.

## Wire the pages up

Deriving the facts is half of it — the pages have to render them:

```tsx
export const metadata = metadataForPath("en", "/press");            // a page
export async function generateMetadata({ params }) {                 // an entry
  const { slug } = await params;
  return metadataForPath("en", `/en/blog/${slug}`);
}
```

`metadataForPath` sets the page's own title, description, canonical, hreflang
alternates, Open Graph and Twitter card, and `publishedTime` for an article.
Articles also render JSON-LD. A path with no entry falls back to the site-wide
metadata rather than throwing: a route may exist before it is declared, and a
missing OG tag is not worth a 500.

## The sitemap moves off `brand`

Remove the `sitemap.xml` line from `pipelines/brand.toml`'s outputs. Two
pipelines writing one artifact is a race decided by pipeline name order,
silently. The deriver refuses to run while both own it, and says so — matching
on the file name, so it catches the collision whatever directory the two
pipelines spell.

## Social images

`scripts/render-og.mjs` renders one 1200×630 PNG per route into `public/og/`
before every web build (`"prebuild"`), from the page's own title and
description, the logo primitives and the brand colours.

**They are not committed, and `public/og/` is gitignored.** An OG image is a
*picture of a declaration*: storing it stores a copy of a title, and that copy
goes stale the day the title changes, with nothing rendering from it to catch
the drift. Rebuilding the whole set takes about a second.

It needs two font files at `assets/fonts/` — the display and body faces, as
`.ttf`. Vendored, not fetched: a build should not depend on a font host being
up, and a deploy should not send a request to one. Ship them with their licence
notice; the common families are under the SIL OFL, which permits exactly this.

**Adapt the layout.** The card in `render-og.mjs` is this product's: its
colours, its mark, its type. It is a starting point, not a contract.

## What an agent should do

- Add a route → add it to `[seo] pages` if it is a page, or let the collection
  declare it. Never hand-edit the sitemap or `generated/seo.*`.
- New page → give it `metadataForPath`. A page without it silently claims the
  home page's canonical.
- Check the live site serves `/sitemap.xml` and one `/og/**.png` after a
  deploy. Both are built rather than committed, so a broken build step shows up
  only in production.

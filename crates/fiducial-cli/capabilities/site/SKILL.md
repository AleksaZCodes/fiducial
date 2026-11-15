# fiducial:site — A Localized Site, Wired

This product has the `site` capability: the pages that turn the platform's
parts into a website a person can read in every declared language.

`fid new <name> --shape site` creates a product with it and everything it
needs. `fid add capability site` adds it to an existing Next.js product that
already has `i18n`, `design`, `content`, `brand`, `seo` and `legal`.

## What it wires

| URL | Page | Reads |
|---|---|---|
| `/`, `/sr` | home: the site's name, its description, the newest entries | `meta.*` messages, every routed collection |
| any `route` in `content.toml` | one entry: date, title, summary, body | `generated/content.ts` |
| `/legal/<page>` | each page the jurisdiction produced | `generated/legal.ts` |
| `/press`, `/press/<slug>` | the press room and its stories (with `press`) | `generated/press.ts` |
| anything else | a 404, inside the site's header and footer | — |

Every page sets its metadata with `metadataForPath()` from the route table
`seo` derives, so a page's title, canonical URL, hreflang alternates and social
image are the same facts the sitemap lists. The footer links every legal page
and the press room from that table too: nothing here lists a page by hand.

## URLs

The default locale is served bare (`/blog/hello`) and every other one under its
prefix (`/sr/blog/hello`). `/en/…` redirects to `/…`, so each page has one
URL per language. The rules are config rewrites in `src/routing.ts`, which
replaces the plain app's empty one: they run in the router with no code per
request, and they behave the same on Workers as on Node.

`localePath(locale, path)` in `src/lib/routes.ts` builds a link. The language
picker keeps the reader on the page they are on (`prefixLocaleHref`).

## What it replaces

The plain app's `app/layout.tsx` and `app/page.tsx` cannot sit beside a
`[locale]` root layout — the page would have two `<html>` elements — so they
are removed on install when unedited. An edited one is kept and named, for you
to move into `app/[locale]/` and delete.

## Adding a page

A page under `app/[locale]/<path>/page.tsx` wins over the content router. Give
it metadata from the route table, and add the path to `[seo] pages` so the
sitemap lists it:

```tsx
export async function generateMetadata({ params }) {
  const { locale } = await params;
  return metadataForPath(localePath(locale, "/about"));
}
```

Words go in `messages/<locale>.json`. The pages read the `site.*` and `press.*`
namespaces, seeded on install in English and Serbian; another declared locale
starts with the English words, and `fid derive` says which are untranslated.

## What is not here yet

- A SvelteKit version. The data side (`src/lib/site.ts`, the generated modules)
  is framework-free and would be reused unchanged; the pages are not written.
- A collection index page (`/blog`). The home page lists every routed entry;
  a collection with enough entries to want its own list should get one.

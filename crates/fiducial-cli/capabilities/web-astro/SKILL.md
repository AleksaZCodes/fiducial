# Skill: web-astro

**Capability:** `web-astro` · **Platform:** Fiducial {{version}}

---

## What this capability adds

An Astro static site at `apps/web/`. Choose it when the app is **content-first**:
a marketing site, a portfolio, docs, a blog. A page that is mostly words ships
mostly words: Astro sends no JavaScript until a component asks for it.

- **Astro 7**, `output: "static"`, every page `<path>/index.html`.
- **Tailwind 4** through its Vite plugin, on the design tokens: `src/styles/app.css`
  imports `tokens.css` and `marks.css` that the `design` capability derives and
  installs into `src/app/`.
- **Response headers in `public/_headers`**, derived by `fid derive` from the platform's
  one table (`[security]` is the only choice: `hsts_preload`), read by Workers Static Assets from
  the build output, because a static site has no server to set them. The same
  names and values as the Next.js set and the self-hosted Caddyfile.
- **Deployed as static assets**: `[deploy] shape = "astro"` derives a
  `wrangler.jsonc` with the build directory and no Worker entry.

## Declare it

```toml
[apps.web]
framework = "astro"

[deploy]
shape = "astro"
```

`fid doctor` holds the two to agreeing, and fails an Astro app whose
`public/_headers` lacks a required security header. Do not edit that file: it is
derived, and `fid derive --check` fails a changed copy.

## Build, run, deploy

```sh
pnpm install
cd apps/web
pnpm dev          # astro dev
pnpm build        # → dist/
pnpm preview
pnpm typecheck    # astro check
```

Deploy with `wrangler deploy` from `apps/web` (the derived config is
`apps/web/wrangler.jsonc`), or serve `dist/` from the self-hosted target with
`[deploy] static_dir = "apps/web/dist"`.

## What is not here yet

Said plainly so it is not rediscovered:

- **No framework islands.** React and Svelte components are an `astro add` away
  and the platform does not wire them. The components below are `.astro` files
  with a few lines of script each, so a page that uses them still ships almost
  no JavaScript. Add a framework island when a page needs real interactivity.
- **No content editor wiring inside the dev server.** `@keystatic/astro` is the
  planned integration. Until then `cms` works as it does for the other
  frameworks: through its own workspace, or hosted with `editor = "pages"`.
- **No i18n routing.** Which URLs a product's languages live at is its own
  decision (`/en/…` prefixes, `?lang=`); `i18n` derives the catalogs and the
  picker takes an `href` builder. `seo` derives the sitemap and `hreflang`.

## Components other capabilities add

Installed by `fid add` when both capabilities are present, in either order. Each
has a React and a Svelte twin; the three are the same decisions, and the
comment at the top of each file lists them.

| Component | From | What it is |
|---|---|---|
| `MobileMenu.astro` | `web-astro` | The nav links in a sheet on a narrow screen: a native modal `<dialog>`, so focus is trapped, the page behind is inert, Escape closes it and focus returns to the button. No library |
| `LocalePicker.astro` | `i18n` | A list of languages, each named in its own, as real links with `hreflang`; a `<details>` disclosure plus a few lines of script for Escape and click-outside |
| `EntryCards.astro` | `content` | A collection's entries: one cover story, a grid of the rest, the whole card one link |
| `PressKit.astro`, `MediaFigure.astro` | `press` | The press room in the order a journalist needs it: assets, facts, copyable boilerplate, stories, contact |

They read the product's design tokens (`--border`, `--radius-md`, `--primary`)
with fallbacks, take their words and routes as props, and are platform-owned:
`fid upgrade` keeps them current, so edit the page that uses them, not them.

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |

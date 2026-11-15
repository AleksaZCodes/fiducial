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
- **Response headers in `public/_headers`**, read by Workers Static Assets from
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
`public/_headers` lacks a required security header.

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

- **No islands.** React and Svelte components are an `astro add` away, and the
  platform's components are not yet wired for either. Add one when a page needs
  interactivity, not before.
- **No content editor wiring.** `@keystatic/astro` is the planned integration
  (the editor would run inside the dev server, with no separate `apps/cms`). Until
  then `cms` works as it does for the other frameworks, through its own workspace.
- **No i18n routing, SEO or `content` accessors.** Those capabilities derive for
  Next.js and SvelteKit; their Astro forms are the next step.

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |

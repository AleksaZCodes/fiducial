# One product, many shapes

*2026-11-13 · status: CMS part shipped in 0.15.0; the rest is the plan*

## The goal

Build any app in as little time, and with as little drift, as possible, with
everything good software needs already included, or a clear way to wrap what
isn't. Three speeds, one platform:

| Speed | What you want | What you get today |
|---|---|---|
| **A ten-minute site** | A website with a design system, shadcn components and an editor, deployed | `fid new` + `design` + `i18n` + `content` + `cms` + `deploy`: works, but is five commands and Next.js only |
| **A full-stack app** | Typed server, database, auth, realtime | Next.js on Workers, D1/Supabase, Durable Objects, `realtime`, `auth`: works |
| **On every device** | The same app as a phone, desktop or installed app | Tauri capability exists; Capacitor proven by hand in one product, not a capability; Electron absent |

The thing that makes this *low-drift* rather than merely fast is already the
platform's core: **one declaration per fact, derived everywhere else, and
`fid derive --check` to prove it.** This plan adds choices without adding
second declarations.

## The model: an app declares its shape

```toml
[apps.web]
framework = "astro"        # next | astro | sveltekit | react-router | tanstack-start
editor    = "keystatic"    # keystatic | tina | pages
shell     = ["capacitor"]  # tauri | capacitor | electron — zero or more

[apps.api]
framework = "hono"
orm       = "drizzle"
```

Everything the product already declares once (tokens, locales, content model,
brand, SEO, legal) is shared by every app and every shell. A choice changes the
*wrapper*, never the facts.

**The escape hatch is the contract, not the framework.** Each concern is a
contract plus a default implementation (`database`, `storage`, `auth`,
`editor`). Anything not shipped is an external capability (`fid add capability
--from`), which already exists. "Make your own wrapper" means writing a
capability against the same contract, and the derive check holds it to it.

## What shipped now: the editor

Decap is gone. **Keystatic is the default and Tina the alternative**, both
derived from one model (`content.toml`, `press/`, `messages/`, `settings.json`,
`[i18n]`) and both editing the *same plain files* (`.md` with YAML frontmatter,
`.json`), so `[cms] editor` is a re-derive, never a content migration.

Why these two, and why they are not redundant:

- **Keystatic** is a form editor: collections, a real image picker, relationship
  pickers, a slug field. It is right for writers who edit fields and for
  code-first teams. It needs no service.
- **Tina** is for editing on the page itself and for a hosted editor later; its
  cost is a service dependency, which is why it is the opt-in.

What got simpler (Decap → now): no proxy process (`decap-server`), no static
server, no CDN script tag, no 130-line serve script, no YAML serializer, no
Decap-specific i18n structure workarounds (`multiple_folders`, flat media
library, `media_folder`/`public_folder` bookkeeping). The editor lives in its own
workspace `apps/cms/`, so it is never in the site's bundle and serves a Next.js
and a SvelteKit product alike.

Verified against the real packages: the derived Keystatic config runs in
Keystatic 0.6.9 under Next 16, lists every collection, edits and saves a
frontmatter entry that `fid derive` then reads; Tina 3.15 builds the derived
schema and serves the Keystatic-edited file over its GraphQL API.

Honest limits, written down so they are not rediscovered:

1. Keystatic has no translation model. Each locale is its own collection, so
   the file name must be kept equal by hand; the slug is editable at creation
   for that reason, and `fid derive` fails on a missing translation.
2. Images are attached per entry, so a translation re-attaches the picture.
3. Tina stores a picture and a relation differently, so they are text fields
   holding the key and the slug there.
4. Hosted editing (Keystatic GitHub mode, Tina Cloud) is not built: media is a
   gitignored staging directory by design, which a hosted editor cannot see.
5. Tina's visual editing is not wired; only its form editor is.

## The plan, in order

Each step has a test that must exist before the step is called done, because
"it generated" is not "it works". Order is by what unblocks the most products.

1. **`[apps.<name>]` and the rule for choosing** (`config.rs`, `doctor`). No new
   framework yet: just the declaration, defaults recorded in comments, and a
   `doctor` check that a combination fits (an editor that needs a framework the
   app is not on fails with the reason). *Test:* a table of valid and invalid
   combinations.
2. **`fid new --shape site`**: the ten-minute path as one command (design,
   i18n, content, cms, seo, deploy), with shadcn components installed and a
   deploy to Workers. *Test:* from empty directory to a served page with an
   editable collection, timed, in CI nightly.
3. **`web-astro`.** The content-first default. Keystatic's own Astro
   integration replaces the separate `apps/cms` workspace there (the editor
   runs inside the dev server). Reuse tokens, i18n, content, seo: only the
   `for/web-astro/` templates are new. *Test:* the `cms_pipeline` end-to-end
   for Astro, plus a `derive --check` over a built site.
4. **`api-hono` and `orm-drizzle`.** Hono as the Worker backend for anything
   without route handlers; Drizzle schema as the declaration whose generated
   SQL feeds the existing `Migrator` (no second hand-written SQL).
5. **`web-router` and `web-start`.** Verify each framework's current major and
   its Workers story first (React Router v8, TanStack Start); do not copy a
   version from this document.
6. **Shells as one family.** `shell-tauri` (exists), `shell-capacitor` (harvest
   from Outreach: `capacitor.config.js`, live/demo flavors, the plugin
   dependencies that `cap sync` otherwise removes), `shell-electron`. A shell
   wraps a built web app, so it takes `url`/`dist` and the app's identity from
   `brand`, never its own copy. *Test:* generated configs checked in CI; the
   Gradle/Xcode build is a documented manual step until a runner has the SDKs.
7. **Hosted editing.** Move media to an adapter-backed picker (R2) so Keystatic
   GitHub mode and Tina Cloud can work; add Pages CMS as a third emitter for
   non-technical editors. Until then the editor is local and that is stated.

## Rules for choosing (the table the `doctor` check encodes)

| The app is | Framework | Editor | Shell |
|---|---|---|---|
| Marketing site, portfolio, docs, blog | Astro | Keystatic | none or PWA |
| Dashboard, internal tool, SPA | React Router | none, or Keystatic for copy | Tauri if desktop |
| Typed full-stack app | TanStack Start or Next.js | Keystatic | Capacitor for phones, Tauri for desktop |
| Anything that needs Node-only packages on desktop | any | any | Electron |
| Needs BLE, GPS, background tasks on a phone | any | any | Capacitor (plugins) |

A default is a recommendation written into the file with its reason. An
existing app is never switched.

## Not doing

- Bundling every option into every product. Choices are per app and install
  only what is chosen (`apps/cms/package.json` is derived for that reason).
- A hosted CMS with its own database. It becomes the source of truth, and
  `fid derive --check` cannot check a database.

# Skill: web-next

**Capability:** `web-next` · **Platform:** Fiducial {{version}}

This skill is loaded by Claude Code automatically when the `web-next` capability
is installed. Read it once at session start; do not re-derive it.

---

## What this capability adds

A Next.js 15 (App Router) web application at `apps/web/`, wired into the
Turborepo pipeline. It uses:

- **Next.js 15** with the App Router and React Server Components
- **Biome** for formatting and linting (no ESLint, no Prettier)
- **TypeScript** (strict mode)
- **pnpm** as the package manager

## Development

```sh
pnpm dev --filter @{{name}}/web   # hot-reload dev server (localhost:3000)
pnpm build --filter @{{name}}/web # production build
pnpm typecheck                     # type-check the whole workspace
```

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-direct-schema-migration` | Running raw SQL migrations by hand — use the declared migration pipeline |
| `no-unpinned-cli-fetch` | `curl | sh` or `wget` without a pinned version |

## Adding pages

Add files under `apps/web/src/app/`. The App Router convention:

```
apps/web/src/app/
├── page.tsx          # /
├── layout.tsx        # root layout (wraps all pages)
├── about/page.tsx    # /about
└── [slug]/page.tsx   # /[slug] (dynamic route)
```

## Environment variables

Declare in `apps/web/.env.local` (git-ignored). Prefix with `NEXT_PUBLIC_` to
expose to the browser. Never commit secrets — use the declared secret store.

## Components (registry model)

Components are **copy-in**, not installed as a package dependency. Use `fid add component`
to copy into `apps/web/src/components/ui/`:

```sh
fid add component button
fid add component card
fid add component badge
fid add component dialog    # requires: pnpm add @base-ui-components/react
```

**Convention:** simple components (button, card, badge) use native HTML styled with token
CSS custom properties. Complex components (dialog, popover, menu) use Base UI
(`@base-ui-components/react`) for accessible keyboard/focus handling and ARIA.

After copying, components are yours — edit freely. `fid upgrade` proposes upstream
changes via 3-way merge (same mechanism as all other template files).

## Key constraints

- **Do not hand-edit** files listed in `fiducial.lock` — change upstream templates
  and re-run `fid upgrade`.
- **Do not commit `node_modules/`** or `.next/`.
- **Do not hand-write database migrations** — use the migration pipeline.

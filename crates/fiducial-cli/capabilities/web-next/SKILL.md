# Skill: web-next

**Capability:** `web-next` · **Platform:** Fiducial {{version}}

This skill is loaded by Claude Code automatically when the `web-next` capability
is installed. Read it once at session start; do not re-derive it.

---

## What this capability adds

A Next.js 16 (App Router) web application at `apps/web/`, wired into the
Turborepo pipeline. It uses:

- **Next.js 16** with the App Router and React Server Components
- **Tailwind CSS 4** through `@tailwindcss/postcss` (`postcss.config.mjs`)
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

## Components (registry model — shadcn conventions, Base UI underneath)

Components are **copy-in**, not installed as a package dependency — either the
platform's (`fid add component <name>`) or shadcn's (`npx shadcn add <name>`),
both into `apps/web/src/components/ui/`:

```sh
fid add component button
fid add component card
fid add component badge
fid add component dialog
```

**Convention:**
- Tailwind utility classes, `cn()` from `@/lib/utils` (clsx + tailwind-merge,
  shipped with this app), and `cva` for variants. The same conventions as
  shadcn/ui.
- **The primitive layer is Base UI (`@base-ui/react`), never Radix.** Do not add
  `radix-ui` or any `@radix-ui/*` package. `components.json` declares the
  `base-vega` style, so `npx shadcn add` writes Base UI components too.
  Composition is `render`, not `asChild`.
- **Icons come from the library the product chose**, declared once as
  `iconLibrary` in `components.json` — `lucide`, `tabler`, `hugeicons`,
  `phosphor` or `remixicon`. Choose it with
  `fid add app next --icons <library>` (lucide when omitted); change it later
  with `npx shadcn migrate icons --from <old> --to <new>`, then `fid upgrade`.
  Platform components are written with `<IconPlaceholder>` and rewritten for
  the declared library when they are installed.

`@base-ui/react`, `clsx`, `tailwind-merge` and `class-variance-authority` are
already in the app's `package.json`.

After copying, components are yours — edit freely. `fid upgrade` proposes upstream
changes via 3-way merge (same mechanism as all other template files).

## Key constraints

- **Do not hand-edit** files listed in `fiducial.lock` — change upstream templates
  and re-run `fid upgrade`.
- **Do not commit `node_modules/`** or `.next/`.
- **Do not hand-write database migrations** — use the migration pipeline.

## `mobile-menu.tsx`

The nav links on a narrow screen, as a native `<dialog>` with `showModal()`.
The browser owns the top layer, the backdrop, the inert page behind it,
Escape, and returning focus to the trigger — five behaviours a hand-rolled
menu has to reimplement and usually does not.

Deliberately not a shadcn `Sheet`: a shadcn component is *copied into* a
product, so a template importing one resolves only where that copy happened,
and the failure is a build error in a file the product never wrote. A
capability template may depend on a package, never on another capability's
copied file.

`links`, `openLabel` and `closeLabel` come in. There is deliberately **no
`SiteNav`** in the platform: arranging a wordmark, links, a language picker
and a call to action is the decision each product makes differently, and
keeping the pieces separate is what lets it.

# web-svelte capability

Installs a SvelteKit web app at `apps/web/`. Uses `@fiducial/tokens` CSS custom
properties and `@fiducial/headless` for typed state.

## Commands

- `fid add app svelte` — scaffold the SvelteKit app
- `pnpm dev` in `apps/web/` — local dev server
- `pnpm build` — production build
- `pnpm typecheck` — TypeScript check

## Token CSS vars

Token CSS vars are declared in `apps/web/src/app.css` (imported in `+layout.svelte`).
Use them as `var(--primary)`, `var(--background)`, etc. Do not hard-code color values.
The full token palette is in `@fiducial/tokens` — run `fid derive` to regenerate the
CSS from the token source when token values change.

## Headless logic

Import from `@fiducial/headless`:
- `Result<T, E>` — typed ok/error union; use in place of `try/catch` or null checks
- `OfflineQueue<T>` — durable action replay with offline support; use for any
  state-mutating action that must survive network drops

Keep business logic in `.ts` files, not in `.svelte` components. A component takes
props and renders; if it reaches for its own data, that is a boundary violation.

## File layout

```
apps/web/
├── src/
│   ├── app.html          HTML shell
│   ├── app.css           Token CSS vars (design system root)
│   └── routes/
│       ├── +layout.svelte   Imports app.css
│       └── +page.svelte     Home page
├── svelte.config.js
├── vite.config.ts
└── tsconfig.json
```

## Components (registry model)

Components are **copy-in**, not installed as a package dependency. Use `fid add component`
to copy into `apps/web/src/components/ui/`:

```sh
fid add component button --framework svelte
fid add component card --framework svelte
fid add component badge --framework svelte
fid add component dialog --framework svelte    # uses native <dialog> element
```

**Convention:** all Svelte components use native HTML styled with token CSS custom
properties. No external UI library is required — the native `<dialog>` element is used
for accessible dialogs (all modern browsers support it).

After copying, components are yours — edit freely. `fid upgrade` proposes upstream
changes via 3-way merge.

## Guard rules

- No direct schema migrations via MCP tools
- No unpinned CLI fetches

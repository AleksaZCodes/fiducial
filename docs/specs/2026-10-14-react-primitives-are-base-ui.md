# React components sit on Base UI, and the icon library is the product's choice

**Date:** 2026-10-14
**Status:** accepted
**Supersedes:** nothing. Corrects one claim in
`2026-09-22-templates-are-scoped-to-frameworks.md` (see Consequences).

---

## Context

On 2026-10-13 a new product (`aleksa-works`) was scaffolded with `fid add
design` and `fid add app next`, and could not run `next build`. Reading why
showed the platform disagreeing with itself about its primitive layer:

- The `web-next` skill (2026-09-08) said complex components use **Base UI**
  "instead of Radix", and the template depended on `@base-ui/react`.
- The `design` capability's `button.tsx`, `card.tsx` and `dropdown-menu.tsx`
  (2026-09-18) imported **`radix-ui`** — harvested from `fon`, which shipped
  Radix — and nothing in the product declared it.
- `components.json` (two copies, from two capabilities, for one path) named the
  `default` / `new-york` styles, so `npx shadcn add` — which the design skill
  tells agents to run first — wrote **Radix** components.
- The dropdown imported `lucide-react`, which nothing declared either.

None of it was caught because no test compiled a template. And `fid upgrade`
could not have repaired a product afterwards: `templates::raw_for` never looked
in `for/<id>/`, so every scoped template reported "no upstream change".

## Decision

1. **Base UI (`@base-ui/react`) is the only primitive layer under Fiducial's
   React components.** Never `radix-ui` or `@radix-ui/*`. `components.json`
   declares shadcn's `base-vega` style, so `npx shadcn add` writes Base UI too.
   Existing products are ported, not grandfathered.
2. **The icon library is a product's choice**, declared once as `iconLibrary`
   in `apps/web/components.json` — `lucide`, `tabler`, `hugeicons`, `phosphor`
   or `remixicon`, chosen by `fid add app next --icons <library>` (lucide by
   default). Platform components write each icon once per library, as
   `<IconPlaceholder lucide="…" tabler="…" …/>` exactly as shadcn's registry
   does, and every path that writes or compares a template rewrites it for the
   declared library and adds that library's packages.
3. **`web-next` requires `design`.** Its stylesheet imports design's
   `tokens.css` and `marks.css`; the dependency was real and undeclared.
4. **A fresh product is built, not read.** `scripts/verify-web-next.sh`
   scaffolds design + Next + i18n + brand once per icon library and runs pnpm
   11's install, the typecheck and `next build` in CI.

## Why not the alternatives

**Radix.** It was what shipped in `fon` and what shadcn generates by default,
and the person who owns the platform chose Base UI over it explicitly. Base UI
is maintained by the people who built Radix, Floating UI and Material UI, has
one composition model (`render`) instead of `asChild` plus `Slot`, and shadcn
now publishes a `base-*` style for every component — so choosing it costs
nothing in what the CLI can generate.

**Both, per product.** Two primitive layers means two composition models, two
sets of state attributes (`data-open` vs `data-[state=open]`) and two
dependency trees in the platform's own templates, and every component the
platform ships written twice. The disagreement this decision ends *was* the
"both" option, arrived at by accident.

**One icon library for every product.** Rejected by the platform's owner: an
icon set is a visible design choice, like a typeface. **Inline SVG in platform
components** was also rejected — "it should use actual libraries" — because a
product then has two icon vocabularies, the platform's hand-drawn ones and the
library it uses everywhere else.

**Declaring the icon library in `fiducial.toml`.** That would be a second
declaration of a fact `components.json` already holds and the shadcn CLI
already reads; the two would disagree the first time someone ran `npx shadcn
migrate icons`.

**A codemod for the port.** Moving from Radix to Base UI changes structure
(`Content` → `Positioner` + `Popup`, `asChild` → `render` with
`nativeButton={false}`), not text. The search-and-replace migrations in
`migration.rs` cannot express it. Platform-owned files are ported by `fid
upgrade`'s 3-way merge; files a product owns are ported by hand, with shadcn's
`migrate-radix-to-base` guide.

## Consequences

- `fid add app next` fails on a product without `design`, and says so.
- `templates::raw_for` resolves `for/<id>/` templates for the capabilities a
  product has. `2026-09-22-templates-are-scoped-to-frameworks.md` said scoped
  templates are merged by `fid upgrade`; until 0.9.5 that was not true.
- A product's `package.json` gains its icon library's packages when a template
  using icons is installed or upgraded — an edit to a product-owned file, made
  by line insertion so its formatting survives.
- Icon package ranges have floors at least a few weeks old, because pnpm 11
  writes an exclusion into the product's `pnpm-workspace.yaml` when a range
  admits nothing older than its `minimumReleaseAge`.

## What this deliberately does not do

- It does not port Svelte. `web-svelte` uses bits-ui, which is the Svelte
  ecosystem's equivalent and has no Radix in it.
- It does not migrate a product's own components automatically. A file the
  product owns is the product's; the platform changes what it ships.
- It does not pick icons for the product's own code. The declared library is
  the one to import from; which icon is a design decision.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->

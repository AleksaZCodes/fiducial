# @fiducial/ui-react

**Component registry source — React. Copied in, not installed.**

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## How this package is meant to be used

Not as a runtime dependency. These are **registry sources**, in shadcn's sense:
`fid add component <name>` copies the source into your product, where you own and
edit it.

That is a deliberate reading of
[principle 3](https://github.com/AleksaZCodes/fiducial/blob/main/MISSION.md) —
*no abstraction without an escape hatch.* A component library you cannot modify
becomes a trap the first time a product needs a variant its author did not
foresee. Copy-in means the escape hatch is the default state.

```sh
fid add component button
fid add component dialog
```

## Components

| Component | Built on |
|---|---|
| `Button` | native HTML + Tailwind + CVA |
| `Card` (+ `Header`/`Title`/`Description`/`Content`/`Footer`) | native HTML |
| `Badge` | native HTML |
| `Dialog` (+ `Trigger`/`Content`/`Overlay`/…) | `@base-ui-components/react` |

Simple components stay native. `Dialog` uses Base UI internally for the focus
trap, keyboard navigation and ARIA wiring — the things that are genuinely hard to
get right and genuinely bad to get wrong. The external API still matches shadcn,
so the dependency is an implementation detail you can replace.

## Styling

Every component reads the CSS variables published by
[`@fiducial/tokens`](../tokens) — `--primary`, `--border`, `--radius` and friends.
The Svelte registry ([`@fiducial/ui-svelte`](../ui-svelte)) reads the *same*
variables, so both frameworks render one design system from one declaration.

## Peer requirements

React ≥ 18, plus `clsx`, `tailwind-merge`, `class-variance-authority`, and
`@base-ui-components/react` (for `Dialog` only).

## License

MIT

# @fiducial/tokens

Typed design tokens — colours, themes, spacing, typography, radii — and the CSS
that carries them into Tailwind v4.

One declaration, many derivations: the same values type-check in TypeScript and
emit as CSS custom properties, so React, Svelte, and plain CSS stay in sync
instead of each re-declaring the palette.

## Install

```sh
pnpm add @fiducial/tokens
```

## Usage

```ts
import { colors, spacing, radii, fontSizes, fontWeights } from '@fiducial/tokens'
import type { Colors, SpacingKey } from '@fiducial/tokens'

const style = { padding: spacing[4], borderRadius: radii.lg }
```

Exports: `colors`, `lightTheme`, `darkTheme`, `generateThemeCss`, `spacing`,
`fontSizes`, `fontWeights`, `fontFamilies`, `leadings`, `radii` — each with its
matching type.

## Tailwind v4

Tailwind v4 has no `tailwind.config.ts`; it is configured entirely in CSS. Paste
the output of `generateThemeCss()` into your `globals.css`:

```css
@import "tailwindcss";
@import "tw-animate-css";

/* generateThemeCss() output goes here */
```

```ts
import { generateThemeCss } from '@fiducial/tokens/tailwind'

console.log(generateThemeCss())
```

The emitted `@theme inline` block registers `--color-*` and `--radius-*`, which
is what makes `bg-background`, `text-foreground`, and `rounded-lg` resolve.
Spacing and typography are Tailwind v4 built-ins and need no registration.

## Themes

`lightTheme` and `darkTheme` are the token sets `generateThemeCss()` renders.
Read them directly when you need a value in JS rather than in CSS:

```ts
import { lightTheme, darkTheme } from '@fiducial/tokens'
```

## License

MIT

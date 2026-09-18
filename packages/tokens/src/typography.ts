/**
 * Typography tokens — Tailwind v4 defaults, resolved for cross-platform use.
 *
 * Tailwind v4 stores line-heights as unitless ratios (calc(1.5 / 1) for base).
 * This table resolves them to absolute rem values for xs–4xl (where absolute
 * spacing matters) and keeps unitless '1' for display sizes (5xl+, where tight
 * line-height is intentional and the ratio IS the value).
 *
 * Cross-platform usage:
 *   CSS / CSS-in-JS : use `size` and `lineHeight` directly
 *   React Native    : fontSize = remToPx(size), lineHeight = remToPx(lineHeight)
 *                     (unitless '1' lineHeights → fontSize × 1 = same as fontSize)
 *   Design tools    : multiply rem by 16 to get px
 */

export type FontSizeEntry = {
  /** CSS font-size value (rem string). */
  size: string
  /**
   * CSS line-height value. Absolute rem string for xs–4xl (preserves v3
   * behaviour and is React-Native-friendly); unitless '1' for display sizes.
   */
  lineHeight: string
}

export const fontSizes = {
  'xs':   { size: '0.75rem',   lineHeight: '1rem'    },  // 12px / 16px
  'sm':   { size: '0.875rem',  lineHeight: '1.25rem' },  // 14px / 20px
  'base': { size: '1rem',      lineHeight: '1.5rem'  },  // 16px / 24px
  'lg':   { size: '1.125rem',  lineHeight: '1.75rem' },  // 18px / 28px
  'xl':   { size: '1.25rem',   lineHeight: '1.75rem' },  // 20px / 28px
  '2xl':  { size: '1.5rem',    lineHeight: '2rem'    },  // 24px / 32px
  '3xl':  { size: '1.875rem',  lineHeight: '2.25rem' },  // 30px / 36px
  '4xl':  { size: '2.25rem',   lineHeight: '2.5rem'  },  // 36px / 40px
  '5xl':  { size: '3rem',      lineHeight: '1'       },  // 48px / tight
  '6xl':  { size: '3.75rem',   lineHeight: '1'       },  // 60px / tight
  '7xl':  { size: '4.5rem',    lineHeight: '1'       },  // 72px / tight
  '8xl':  { size: '6rem',      lineHeight: '1'       },  // 96px / tight
  '9xl':  { size: '8rem',      lineHeight: '1'       },  // 128px / tight
} as const satisfies Record<string, FontSizeEntry>

/** Tailwind v4 font-weight tokens. Values are numeric strings for CSS compat. */
export const fontWeights = {
  thin:       '100',
  extralight: '200',
  light:      '300',
  normal:     '400',
  medium:     '500',
  semibold:   '600',
  bold:       '700',
  extrabold:  '800',
  black:      '900',
} as const

/**
 * Tailwind v4 default font stacks.
 * React Native: use the first family name, or swap in a bundled font.
 * CSS: join with ', ' and set as font-family.
 */
export const fontFamilies = {
  sans:  [
    '-apple-system', 'BlinkMacSystemFont', '"Segoe UI"', 'Roboto',
    '"Helvetica Neue"', '"Noto Sans"', 'Arial', 'sans-serif',
    '"Apple Color Emoji"', '"Segoe UI Emoji"', '"Segoe UI Symbol"', '"Noto Color Emoji"',
  ],
  serif: ['ui-serif', 'Georgia', 'Cambria', '"Times New Roman"', 'Times', 'serif'],
  mono:  [
    'ui-monospace', 'SFMono-Regular', 'Menlo', 'Monaco',
    'Consolas', '"Liberation Mono"', '"Courier New"', 'monospace',
  ],
} as const

/** Named line-height utilities (Tailwind v4 `leading-*` tokens). */
export const leadings = {
  tight:   '1.25',
  snug:    '1.375',
  normal:  '1.5',
  relaxed: '1.625',
  loose:   '2',
} as const

export type FontSizes    = typeof fontSizes
export type FontWeights  = typeof fontWeights
export type FontFamilies = typeof fontFamilies
export type Leadings     = typeof leadings

// ── Type roles ───────────────────────────────────────────────────────────────

/**
 * The four type roles, as CSS custom property references.
 *
 * A product binds `--font-display` / `--font-body` / `--font-script` /
 * `--font-mono` once (next/font, @font-face, whatever) and every consumer reads
 * the role, never the family. This is principle 1 applied to type: the family
 * name is declared in one place, and a component that names "Chakra Petch"
 * directly has forked the declaration.
 *
 * The raw custom properties are `--type-*`, not `--font-*`, because Tailwind v4
 * owns the `--font-*` namespace: `@theme inline { --font-display: var(--type-display) }`
 * is what turns a role into a `font-display` utility, and a var cannot be
 * defined in terms of itself. `generateThemeCss()` emits both halves.
 *
 * Four roles rather than the usual two, because the two the shadcn default
 * ships (sans + mono) cannot express the thing that actually stops a page
 * looking machine-made: a display face that is not the body face, and a hand
 * that can annotate the page in the margin.
 */
export const fontRoles = {
  /** Headlines, wordmarks, numerals that are meant to be looked at. */
  display: 'var(--type-display)',
  /** Running text. Everything a person actually reads. */
  body: 'var(--type-body)',
  /** Margin notes, annotations, the voice beside the page. Never body copy. */
  script: 'var(--type-script)',
  /** Code, identifiers, tabular figures. */
  mono: 'var(--type-mono)',
} as const

/** One role's default family and the weights that must be loaded for it. */
export type FontRoleDefault = {
  /** Exact family name, as it is spelled by the foundry. Never a category. */
  family: string
  /** Weights to load. Loading more than these is waste; fewer breaks the scale. */
  weights: number[]
  /** Fallback stack appended after `family`, for the swap window and for failure. */
  fallback: string[]
  /** Why this face, so the next person changes it for a reason. */
  note: string
}

/**
 * The pairing Fiducial ships. It is a real pairing, not a placeholder palette of
 * `sans-serif` — the whole point of the anti-slop rules is that an unspecified
 * slot reverts to the norm, and "the norm" is Inter at 16px with a blue button.
 *
 * `display` is the slot you are *expected* to replace once you have a brand.
 * The other three are the floor: replace them when you have a reason, not to
 * have chosen something.
 *
 * Every family here carries `latin-ext` and Cyrillic, because Fiducial products
 * ship in Serbian before they ship in English and a font that cannot set `ž`
 * is not a candidate.
 */
export const fontDefaults = {
  display: {
    family: 'Chakra Petch',
    weights: [500, 600, 700],
    fallback: ['ui-sans-serif', 'system-ui', 'sans-serif'],
    note: 'Squarish technical grotesque with 45°-cut terminals. Wants -0.02em at display sizes and reads badly below ~1.25rem — which is the point: it cannot leak into body copy.',
  },
  body: {
    family: 'Source Serif 4',
    weights: [400, 600],
    fallback: ['ui-serif', 'Georgia', 'serif'],
    note: 'Editorial text serif, optical-size variable. Chosen against the slop list: it is not Inter, not Geist, and not a UI sans, and it gives the body a different voice from the display face rather than a different size of the same one.',
  },
  script: {
    family: 'Caveat',
    weights: [500, 600],
    fallback: ['ui-serif', 'cursive'],
    note: 'A marker-pen hand, not a wedding script. Used for annotation only — see the doodle marks. Never for a heading, never for a paragraph, never for anything a screen reader has to get through.',
  },
  mono: {
    family: 'JetBrains Mono',
    weights: [400, 500],
    fallback: ['ui-monospace', 'SFMono-Regular', 'Menlo', 'monospace'],
    note: 'Identifiers, coordinates, packet counts. Tabular figures on by default.',
  },
} as const satisfies Record<keyof typeof fontRoles, FontRoleDefault>

/** One step of the named scale. */
export type TypeStep = {
  /** Which of the four roles renders this step. */
  role: keyof typeof fontRoles
  size: string
  lineHeight: string
  weight: string
  letterSpacing: string
}

/**
 * The named type scale — every step a page is allowed to use, spelled out.
 *
 * This exists because "define an explicit heading scale" is the single most
 * load-bearing anti-slop rule: a scale left implicit becomes `text-4xl` on one
 * page and `text-5xl` on the next, and the two pages stop being one product.
 *
 * Nine steps. If a design needs a tenth, add it here — do not inline it.
 */
export const typeScale = {
  /** Hero only. One per page, at most. */
  display: { role: 'display', size: '3.75rem', lineHeight: '1.04', weight: '600', letterSpacing: '-0.025em' },
  h1:      { role: 'display', size: '2.75rem', lineHeight: '1.1',  weight: '600', letterSpacing: '-0.02em'  },
  h2:      { role: 'display', size: '2rem',    lineHeight: '1.18', weight: '600', letterSpacing: '-0.015em' },
  h3:      { role: 'display', size: '1.25rem', lineHeight: '1.3',  weight: '600', letterSpacing: '-0.01em'  },
  /** The sentence under a heading. Body face, larger, muted. */
  subhead: { role: 'body',    size: '1.1875rem', lineHeight: '1.6',  weight: '400', letterSpacing: '0'      },
  body:    { role: 'body',    size: '1rem',    lineHeight: '1.65', weight: '400', letterSpacing: '0'        },
  small:   { role: 'body',    size: '0.875rem', lineHeight: '1.6', weight: '400', letterSpacing: '0'        },
  /** Section labels. Uppercase, tracked out, primary-coloured. */
  eyebrow: { role: 'display', size: '0.75rem', lineHeight: '1',    weight: '600', letterSpacing: '0.18em'   },
  /** Annotation. Tilted in use; see `.mark-note` in the marks layer. */
  note:    { role: 'script',  size: '1.25rem', lineHeight: '1.25', weight: '500', letterSpacing: '0'        },
  mono:    { role: 'mono',    size: '0.8125rem', lineHeight: '1.5', weight: '400', letterSpacing: '0'       },
} as const satisfies Record<string, TypeStep>

export type FontRoles    = typeof fontRoles
export type FontDefaults = typeof fontDefaults
export type TypeScale    = typeof typeScale

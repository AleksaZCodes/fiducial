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

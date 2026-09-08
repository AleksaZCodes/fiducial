/**
 * Border-radius tokens — derived from `--radius` (set by the active theme).
 *
 * For Tailwind v4: `generateThemeCss()` injects `--radius-*` into `@theme inline`,
 *   making `rounded-sm`, `rounded-lg`, etc. available automatically.
 * For Tailwind v3: pass `radii` into `borderRadius` in `fiducialPreset`.
 */

export const radii = {
  sm:   'calc(var(--radius) - 4px)',
  md:   'calc(var(--radius) - 2px)',
  lg:   'var(--radius)',
  xl:   'calc(var(--radius) + 2px)',
  '2xl': 'calc(var(--radius) + 4px)',
  '3xl': 'calc(var(--radius) + 8px)',
  '4xl': 'calc(var(--radius) + 16px)',
  full:  '9999px',
} as const

export type Radii = typeof radii

/**
 * Border-radius tokens — shadcn/ui-compatible CSS variable references.
 * `--radius` is set by the active theme (default 0.5rem).
 */

export const radii = {
  lg:  'var(--radius)',
  md:  'calc(var(--radius) - 2px)',
  sm:  'calc(var(--radius) - 4px)',
  full: '9999px',
} as const

export type Radii = typeof radii

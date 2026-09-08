/**
 * Semantic color tokens — CSS custom property references, shadcn/ui-compatible.
 *
 * CSS vars store complete OKLCH values, so these reference them as `var(--name)`
 * (not `hsl(var(--name))`). Requires the CSS vars to be injected via
 * `generateThemeCss()` from css.ts.
 *
 * For Tailwind v4: use `generateThemeCss()` only — the `@theme inline` block
 *   makes these available as `bg-background`, `text-foreground`, etc.
 * For Tailwind v3: use `fiducialPreset` from tailwind.ts which passes this
 *   object into `theme.extend.colors`.
 */

export const colors = {
  background: 'var(--background)',
  foreground: 'var(--foreground)',

  card: {
    DEFAULT:    'var(--card)',
    foreground: 'var(--card-foreground)',
  },
  popover: {
    DEFAULT:    'var(--popover)',
    foreground: 'var(--popover-foreground)',
  },

  primary: {
    DEFAULT:    'var(--primary)',
    foreground: 'var(--primary-foreground)',
  },
  secondary: {
    DEFAULT:    'var(--secondary)',
    foreground: 'var(--secondary-foreground)',
  },
  muted: {
    DEFAULT:    'var(--muted)',
    foreground: 'var(--muted-foreground)',
  },
  accent: {
    DEFAULT:    'var(--accent)',
    foreground: 'var(--accent-foreground)',
  },
  destructive: 'var(--destructive)',

  border: 'var(--border)',
  input:  'var(--input)',
  ring:   'var(--ring)',

  chart: {
    '1': 'var(--chart-1)',
    '2': 'var(--chart-2)',
    '3': 'var(--chart-3)',
    '4': 'var(--chart-4)',
    '5': 'var(--chart-5)',
  },

  sidebar: {
    DEFAULT:              'var(--sidebar)',
    foreground:           'var(--sidebar-foreground)',
    primary:              'var(--sidebar-primary)',
    'primary-foreground': 'var(--sidebar-primary-foreground)',
    accent:               'var(--sidebar-accent)',
    'accent-foreground':  'var(--sidebar-accent-foreground)',
    border:               'var(--sidebar-border)',
    ring:                 'var(--sidebar-ring)',
  },
} as const

export type Colors = typeof colors

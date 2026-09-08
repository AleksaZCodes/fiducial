/**
 * Semantic color tokens — CSS custom property references, shadcn/ui-compatible.
 *
 * Values are `hsl(var(--name))` so they resolve at runtime from the active
 * theme's CSS variables. Set `--background`, `--primary`, etc. via `themes.ts`.
 */

export const colors = {
  background: 'hsl(var(--background))',
  foreground: 'hsl(var(--foreground))',

  card: {
    DEFAULT:    'hsl(var(--card))',
    foreground: 'hsl(var(--card-foreground))',
  },
  popover: {
    DEFAULT:    'hsl(var(--popover))',
    foreground: 'hsl(var(--popover-foreground))',
  },

  primary: {
    DEFAULT:    'hsl(var(--primary))',
    foreground: 'hsl(var(--primary-foreground))',
  },
  secondary: {
    DEFAULT:    'hsl(var(--secondary))',
    foreground: 'hsl(var(--secondary-foreground))',
  },
  muted: {
    DEFAULT:    'hsl(var(--muted))',
    foreground: 'hsl(var(--muted-foreground))',
  },
  accent: {
    DEFAULT:    'hsl(var(--accent))',
    foreground: 'hsl(var(--accent-foreground))',
  },
  destructive: {
    DEFAULT:    'hsl(var(--destructive))',
    foreground: 'hsl(var(--destructive-foreground))',
  },

  border: 'hsl(var(--border))',
  input:  'hsl(var(--input))',
  ring:   'hsl(var(--ring))',
} as const

export type Colors = typeof colors

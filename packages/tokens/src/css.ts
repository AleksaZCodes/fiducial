import { lightTheme, darkTheme } from './themes.js'
import type { ThemeVars } from './themes.js'

/** CSS vars that map 1:1 to `--color-*` in the @theme inline block. */
const COLOR_KEYS: Array<keyof ThemeVars> = [
  '--background', '--foreground',
  '--card', '--card-foreground',
  '--popover', '--popover-foreground',
  '--primary', '--primary-foreground',
  '--secondary', '--secondary-foreground',
  '--muted', '--muted-foreground',
  '--accent', '--accent-foreground',
  '--destructive',
  '--border', '--input', '--ring',
  '--chart-1', '--chart-2', '--chart-3', '--chart-4', '--chart-5',
  '--sidebar', '--sidebar-foreground',
  '--sidebar-primary', '--sidebar-primary-foreground',
  '--sidebar-accent', '--sidebar-accent-foreground',
  '--sidebar-border', '--sidebar-ring',
]

function varsToBlock(vars: ThemeVars): string {
  return Object.entries(vars)
    .map(([k, v]) => `  ${k}: ${v};`)
    .join('\n')
}

const colorInline = COLOR_KEYS
  .map(k => `  --color-${k.slice(2)}: var(${k});`)
  .join('\n')

const radiusInline = [
  '  --radius-sm:  calc(var(--radius) - 4px);',
  '  --radius-md:  calc(var(--radius) - 2px);',
  '  --radius-lg:  var(--radius);',
  '  --radius-xl:  calc(var(--radius) + 2px);',
  '  --radius-2xl: calc(var(--radius) + 4px);',
  '  --radius-3xl: calc(var(--radius) + 8px);',
  '  --radius-4xl: calc(var(--radius) + 16px);',
].join('\n')

/**
 * Returns the complete CSS snippet to paste into your global stylesheet.
 *
 * Output structure:
 *   :root          — light theme CSS custom properties (OKLCH)
 *   .dark          — dark theme CSS custom properties (OKLCH)
 *   @theme inline  — maps vars into Tailwind v4 utilities
 *                    (bg-background, text-foreground, rounded-lg, …)
 *
 * For Tailwind v3: skip @theme inline and use `fiducialPreset` from tailwind.ts.
 */
export function generateThemeCss(): string {
  return [
    `:root {\n${varsToBlock(lightTheme)}\n}`,
    `.dark {\n${varsToBlock(darkTheme)}\n}`,
    `@theme inline {\n${colorInline}\n\n${radiusInline}\n}`,
  ].join('\n\n')
}

import { lightTheme, darkTheme } from './themes.js'
import type { ThemeVars } from './themes.js'
import { fontDefaults } from './typography.js'
import type { FontRoleDefault } from './typography.js'
import { shapeDefaults } from './shape.js'
import type { CornerStrategy } from './shape.js'

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
 * A family name is quoted only when it needs to be — an unquoted `Chakra Petch`
 * is legal CSS but a single wrong character away from being parsed as two
 * keywords, and `ui-serif` must stay unquoted or it stops being a keyword.
 */
function familyStack(def: FontRoleDefault): string {
  return [`"${def.family}"`, ...def.fallback].join(', ')
}

/** Options for `generateThemeCss`. Omit any of them to get what Fiducial ships. */
export type ThemeCssOptions = {
  /** The four type roles. Override `display` first; it is the brand-specific one. */
  fonts?: Partial<Record<keyof typeof fontDefaults, FontRoleDefault>>
  /** Corner strategy, corner scale, and stroke weights. */
  shape?: Partial<typeof shapeDefaults> & { strategy?: CornerStrategy }
}

/**
 * Returns the complete CSS snippet to paste into your global stylesheet.
 *
 * Output structure:
 *   :root          — light theme vars (OKLCH), then the type roles and shape
 *   .dark          — dark theme vars (OKLCH)
 *   @theme inline  — maps vars into Tailwind v4 utilities
 *                    (bg-background, text-foreground, font-display, rounded-lg, …)
 *
 * ## Two things worth knowing before you paste this
 *
 * **The type roles are declared as `--type-*` and exposed as `font-*`.** Tailwind
 * v4 owns the `--font-*` namespace, so `@theme inline` is where
 * `--type-display` becomes the `font-display` utility. If you load the family
 * yourself (next/font, `@font-face`), redeclare `--type-display` in your own
 * `:root` *after* this snippet with your loader's variable first in the stack:
 *
 *     :root { --type-display: var(--font-chakra-petch), ui-sans-serif, sans-serif; }
 *
 * **`--radius` is forced to 0 unless the corner strategy is `round`.** Under
 * `chamfer` and `square` the corner comes from the `.cham-*` classes in the
 * marks layer, and leaving `--radius` at its theme value would round the
 * chamfer's own clip — the two treatments fight, and the chamfer loses.
 * The `--radius-*` scale is still emitted either way, because a component
 * copied in from any shadcn-shaped registry expects it to resolve.
 *
 * For Tailwind v3: skip @theme inline and use `fiducialPreset` from tailwind.ts.
 */
export function generateThemeCss(options: ThemeCssOptions = {}): string {
  const fonts = { ...fontDefaults, ...options.fonts }
  const shape = { ...shapeDefaults, ...options.shape }

  const typeBlock = [
    '',
    '  /* Type roles. Four, because two cannot say "the display face is not the',
    '     body face" — which is the difference between a designed page and a',
    '     generated one. Redeclare these if you load the families yourself. */',
    `  --type-display: ${familyStack(fonts.display)};`,
    `  --type-body:    ${familyStack(fonts.body)};`,
    `  --type-script:  ${familyStack(fonts.script)};`,
    `  --type-mono:    ${familyStack(fonts.mono)};`,
  ].join('\n')

  const shapeBlock = [
    '',
    `  /* Shape. Corner strategy: ${shape.strategy}. */`,
    ...(shape.strategy === 'round' ? [] : [
      '  /* Not `round`: the corner is cut by the .cham-* classes, so the radius',
      '     scale must resolve to nothing or the two treatments fight. */',
      '  --radius: 0rem;',
    ]),
    `  --corner-panel:   ${shape.panel};`,
    `  --corner-control: ${shape.control};`,
    `  --corner-chip:    ${shape.chip};`,
    `  --border-w: ${shape.edge};`,
    `  --hair-w:   ${shape.hair};`,
    '',
    '  /* Annotation stroke. Fixed weight — a mark is drawn, not scaled. */',
    `  --doodle-stroke: ${shape.doodleStroke};`,
  ].join('\n')

  const fontInline = [
    '  --font-display: var(--type-display);',
    '  --font-body:    var(--type-body);',
    '  --font-script:  var(--type-script);',
    '  --font-mono:    var(--type-mono);',
    '  /* Anything that asks for the default sans gets the body face, so an',
    '     unstyled paragraph is already correct rather than already wrong. */',
    '  --font-sans:    var(--type-body);',
  ].join('\n')

  return [
    `:root {\n${varsToBlock(lightTheme)}\n${typeBlock}\n${shapeBlock}\n}`,
    `.dark {\n${varsToBlock(darkTheme)}\n}`,
    `@theme inline {\n${colorInline}\n\n${fontInline}\n\n${radiusInline}\n}`,
  ].join('\n\n')
}

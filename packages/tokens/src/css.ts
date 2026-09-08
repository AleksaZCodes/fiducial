import { lightTheme, darkTheme } from './themes.js'
import type { ThemeVars } from './themes.js'

function varsToBlock(vars: ThemeVars): string {
  return Object.entries(vars)
    .map(([k, v]) => `  ${k}: ${v};`)
    .join('\n')
}

/**
 * Returns a CSS string that injects both light and dark theme variables.
 *
 * Light vars are placed on `:root`.
 * Dark vars are placed on `.dark` and `[data-theme="dark"]`.
 *
 * Paste the output into a global CSS file, or inject it via a `<style>` tag.
 */
export function generateThemeCss(): string {
  return [
    `:root {\n${varsToBlock(lightTheme)}\n}`,
    `.dark,\n[data-theme="dark"] {\n${varsToBlock(darkTheme)}\n}`,
  ].join('\n\n')
}

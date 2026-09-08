import { colors } from './colors.js'
import { radii } from './radii.js'

/**
 * Tailwind v3 preset — for projects not yet on Tailwind v4.
 *
 * Usage in tailwind.config.ts:
 *   import { fiducialPreset } from '@fiducial/tokens/tailwind'
 *   export default { presets: [fiducialPreset], content: [...] }
 *
 * Also inject `generateThemeCss()` output into your global CSS so the CSS
 * custom properties (OKLCH values) are defined at runtime.
 *
 * For Tailwind v4: skip this file. Use only `generateThemeCss()` — the
 * `@theme inline` block registers all utilities automatically.
 */
export const fiducialPreset = {
  theme: {
    extend: {
      colors,
      borderRadius: radii,
    },
  },
} as const

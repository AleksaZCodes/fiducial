import { colors } from './colors.js'
import { spacing } from './spacing.js'
import { fontSizes, fontWeights, fontFamilies } from './typography.js'
import { radii } from './radii.js'

/**
 * Tailwind CSS preset — shadcn/ui-compatible.
 *
 * Usage in tailwind.config.ts:
 *   import { fiducialPreset } from '@fiducial/tokens/tailwind'
 *   export default { presets: [fiducialPreset], ... }
 *
 * Requires that `generateThemeCss()` output (from @fiducial/tokens) is
 * injected into your global CSS so the CSS custom properties are defined.
 */
export const fiducialPreset = {
  theme: {
    extend: {
      colors,
      spacing,
      fontSize:     fontSizes,
      fontWeight:   fontWeights,
      fontFamily:   fontFamilies,
      borderRadius: radii,
    },
  },
} as const

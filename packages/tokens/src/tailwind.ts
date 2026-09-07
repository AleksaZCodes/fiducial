import { colors } from './colors.js'
import { spacing } from './spacing.js'
import { fontSizes, fontWeights, fontFamilies } from './typography.js'
import { radii } from './radii.js'

/** Tailwind CSS preset — extend your config with `presets: [fiducialPreset]`. */
export const fiducialPreset = {
  theme: {
    extend: {
      colors,
      spacing,
      fontSize: fontSizes,
      fontWeight: fontWeights,
      fontFamily: fontFamilies,
      borderRadius: radii,
    },
  },
} as const

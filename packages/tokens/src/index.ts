// Colors
export { colors } from './colors.js'
export type { Colors } from './colors.js'

// Themes (CSS custom property values)
export { lightTheme, darkTheme } from './themes.js'
export type { ThemeVars } from './themes.js'

// CSS generation
export { generateThemeCss } from './css.js'
export type { ThemeCssOptions } from './css.js'

// Spacing
export { spacing } from './spacing.js'
export type { Spacing, SpacingKey, SpacingEntry } from './spacing.js'

// Typography
export { fontSizes, fontWeights, fontFamilies, leadings } from './typography.js'
export type { FontSizes, FontWeights, FontFamilies, Leadings, FontSizeEntry } from './typography.js'

// Type roles — the four-role contract, the families that fill it, and the
// named scale. `fontFamilies` above is Tailwind's raw default stacks; these are
// the roles a product actually designs against.
export { fontRoles, fontDefaults, typeScale } from './typography.js'
export type { FontRoles, FontDefaults, TypeScale, FontRoleDefault, TypeStep } from './typography.js'

// Shape — corner strategy, corner scale, stroke weights, annotation ink
export { corners, strokes, doodle, shapeDefaults } from './shape.js'
export type { Corners, Strokes, Doodle, ShapeDefaults, CornerStrategy } from './shape.js'

// Radii
export { radii } from './radii.js'
export type { Radii } from './radii.js'

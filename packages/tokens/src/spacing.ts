/**
 * Spacing scale — Tailwind v4 defaults, resolved to explicit rem/px values.
 *
 * Tailwind v4 derives spacing utilities from `--spacing: 0.25rem` (4px base),
 * so `p-4` = 1rem = 16px, `p-8` = 2rem = 32px, etc. This table records the
 * resolved values so they are usable without Tailwind:
 *   - CSS / CSS-in-JS: use the `rem` string directly
 *   - React Native: convert with `remToPx()` (assumes 16px root)
 *   - Design tools: use the `px` value
 */

export type SpacingEntry = { rem: string; px: number }

export const spacing = {
  'px':   { rem: '1px',     px: 1   },
  '0':    { rem: '0rem',    px: 0   },
  '0.5':  { rem: '0.125rem',px: 2   },
  '1':    { rem: '0.25rem', px: 4   },
  '1.5':  { rem: '0.375rem',px: 6   },
  '2':    { rem: '0.5rem',  px: 8   },
  '2.5':  { rem: '0.625rem',px: 10  },
  '3':    { rem: '0.75rem', px: 12  },
  '3.5':  { rem: '0.875rem',px: 14  },
  '4':    { rem: '1rem',    px: 16  },
  '5':    { rem: '1.25rem', px: 20  },
  '6':    { rem: '1.5rem',  px: 24  },
  '7':    { rem: '1.75rem', px: 28  },
  '8':    { rem: '2rem',    px: 32  },
  '9':    { rem: '2.25rem', px: 36  },
  '10':   { rem: '2.5rem',  px: 40  },
  '11':   { rem: '2.75rem', px: 44  },
  '12':   { rem: '3rem',    px: 48  },
  '14':   { rem: '3.5rem',  px: 56  },
  '16':   { rem: '4rem',    px: 64  },
  '20':   { rem: '5rem',    px: 80  },
  '24':   { rem: '6rem',    px: 96  },
  '28':   { rem: '7rem',    px: 112 },
  '32':   { rem: '8rem',    px: 128 },
  '36':   { rem: '9rem',    px: 144 },
  '40':   { rem: '10rem',   px: 160 },
  '44':   { rem: '11rem',   px: 176 },
  '48':   { rem: '12rem',   px: 192 },
  '52':   { rem: '13rem',   px: 208 },
  '56':   { rem: '14rem',   px: 224 },
  '60':   { rem: '15rem',   px: 240 },
  '64':   { rem: '16rem',   px: 256 },
  '72':   { rem: '18rem',   px: 288 },
  '80':   { rem: '20rem',   px: 320 },
  '96':   { rem: '24rem',   px: 384 },
} as const satisfies Record<string, SpacingEntry>

export type Spacing = typeof spacing
export type SpacingKey = keyof Spacing

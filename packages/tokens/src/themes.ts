/**
 * Light and dark CSS custom property values — HSL channels without hsl().
 *
 * Usage: inject these into `:root` and `.dark` (or `[data-theme="dark"]`)
 * with `generateThemeCss()` from css.ts, or spread them into your own
 * style-injection logic.
 */

export type ThemeVars = {
  '--background': string
  '--foreground': string
  '--card': string
  '--card-foreground': string
  '--popover': string
  '--popover-foreground': string
  '--primary': string
  '--primary-foreground': string
  '--secondary': string
  '--secondary-foreground': string
  '--muted': string
  '--muted-foreground': string
  '--accent': string
  '--accent-foreground': string
  '--destructive': string
  '--destructive-foreground': string
  '--border': string
  '--input': string
  '--ring': string
  '--radius': string
}

export const lightTheme: ThemeVars = {
  '--background':            '0 0% 100%',
  '--foreground':            '222.2 84% 4.9%',
  '--card':                  '0 0% 100%',
  '--card-foreground':       '222.2 84% 4.9%',
  '--popover':               '0 0% 100%',
  '--popover-foreground':    '222.2 84% 4.9%',
  '--primary':               '222.2 47.4% 11.2%',
  '--primary-foreground':    '210 40% 98%',
  '--secondary':             '210 40% 96.1%',
  '--secondary-foreground':  '222.2 47.4% 11.2%',
  '--muted':                 '210 40% 96.1%',
  '--muted-foreground':      '215.4 16.3% 46.9%',
  '--accent':                '210 40% 96.1%',
  '--accent-foreground':     '222.2 47.4% 11.2%',
  '--destructive':           '0 84.2% 60.2%',
  '--destructive-foreground':'210 40% 98%',
  '--border':                '214.3 31.8% 91.4%',
  '--input':                 '214.3 31.8% 91.4%',
  '--ring':                  '222.2 84% 4.9%',
  '--radius':                '0.5rem',
}

export const darkTheme: ThemeVars = {
  '--background':            '222.2 84% 4.9%',
  '--foreground':            '210 40% 98%',
  '--card':                  '222.2 84% 4.9%',
  '--card-foreground':       '210 40% 98%',
  '--popover':               '222.2 84% 4.9%',
  '--popover-foreground':    '210 40% 98%',
  '--primary':               '210 40% 98%',
  '--primary-foreground':    '222.2 47.4% 11.2%',
  '--secondary':             '217.2 32.6% 17.5%',
  '--secondary-foreground':  '210 40% 98%',
  '--muted':                 '217.2 32.6% 17.5%',
  '--muted-foreground':      '215 20.2% 65.1%',
  '--accent':                '217.2 32.6% 17.5%',
  '--accent-foreground':     '210 40% 98%',
  '--destructive':           '0 62.8% 30.6%',
  '--destructive-foreground':'210 40% 98%',
  '--border':                '217.2 32.6% 17.5%',
  '--input':                 '217.2 32.6% 17.5%',
  '--ring':                  '212.7 26.8% 83.9%',
  '--radius':                '0.5rem',
}

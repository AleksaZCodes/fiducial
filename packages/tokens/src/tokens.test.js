/**
 * Tests for @fiducial/tokens.
 * Imports from ../dist — built by tsc before this test runs (turbo: test dependsOn build).
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  colors,
  lightTheme,
  darkTheme,
  generateThemeCss,
  radii,
} from '../dist/index.js'
import { fiducialPreset } from '../dist/tailwind.js'

const VAR_RE = /^var\(--[a-z0-9-]+\)$/

describe('@fiducial/tokens', () => {
  describe('colors', () => {
    it('has all core scalar slots', () => {
      for (const key of ['background', 'foreground', 'border', 'input', 'ring', 'destructive']) {
        assert.ok(key in colors, `missing colors.${key}`)
      }
    })

    it('primary / secondary / muted / accent have DEFAULT + foreground', () => {
      for (const group of ['primary', 'secondary', 'muted', 'accent']) {
        assert.ok(colors[group]?.DEFAULT,    `missing colors.${group}.DEFAULT`)
        assert.ok(colors[group]?.foreground, `missing colors.${group}.foreground`)
      }
    })

    it('card and popover have DEFAULT + foreground', () => {
      assert.ok(colors.card.DEFAULT)
      assert.ok(colors.card.foreground)
      assert.ok(colors.popover.DEFAULT)
      assert.ok(colors.popover.foreground)
    })

    it('chart has slots 1–5', () => {
      for (const n of ['1', '2', '3', '4', '5']) {
        assert.ok(colors.chart[n], `missing colors.chart.${n}`)
      }
    })

    it('sidebar has all eight slots', () => {
      const expected = [
        'DEFAULT', 'foreground', 'primary', 'primary-foreground',
        'accent', 'accent-foreground', 'border', 'ring',
      ]
      for (const key of expected) {
        assert.ok(colors.sidebar[key], `missing colors.sidebar.${key}`)
      }
    })

    it('all scalar values are var(--*) references (no hsl wrapper)', () => {
      const scalars = [
        colors.background, colors.foreground,
        colors.border, colors.input, colors.ring,
        colors.destructive,
      ]
      for (const v of scalars) {
        assert.match(v, VAR_RE, `unexpected format: ${v}`)
      }
    })

    it('primary.DEFAULT is var(--primary)', () => {
      assert.equal(colors.primary.DEFAULT, 'var(--primary)')
    })

    it('chart.1 is var(--chart-1)', () => {
      assert.equal(colors.chart['1'], 'var(--chart-1)')
    })
  })

  describe('themes', () => {
    const EXPECTED_KEYS = [
      '--background', '--foreground',
      '--primary', '--primary-foreground',
      '--destructive',
      '--border', '--input', '--ring',
      '--chart-1', '--chart-5',
      '--radius',
      '--sidebar', '--sidebar-ring',
    ]

    it('lightTheme has all expected keys', () => {
      for (const k of EXPECTED_KEYS) {
        assert.ok(k in lightTheme, `missing lightTheme ${k}`)
      }
    })

    it('darkTheme has all expected keys', () => {
      for (const k of EXPECTED_KEYS) {
        assert.ok(k in darkTheme, `missing darkTheme ${k}`)
      }
    })

    it('has exactly 32 custom properties each', () => {
      assert.equal(Object.keys(lightTheme).length, 32)
      assert.equal(Object.keys(darkTheme).length, 32)
    })

    it('all values are non-empty strings', () => {
      for (const [k, v] of Object.entries(lightTheme)) {
        assert.ok(typeof v === 'string' && v.length > 0, `empty value for ${k}`)
      }
    })

    it('light and dark --background differ (OKLCH values)', () => {
      assert.notEqual(lightTheme['--background'], darkTheme['--background'])
    })

    it('values use oklch() color function', () => {
      assert.match(lightTheme['--background'], /^oklch\(/)
      assert.match(darkTheme['--background'], /^oklch\(/)
    })

    it('--radius is a CSS length', () => {
      assert.match(lightTheme['--radius'], /^\d+(\.\d+)?(rem|px|em)$/)
      assert.equal(lightTheme['--radius'], darkTheme['--radius'])
    })

    it('dark sidebar-primary can have a non-zero hue (blue accent)', () => {
      // dark theme sidebar-primary is oklch(0.488 0.243 264.376) — chroma > 0
      assert.match(darkTheme['--sidebar-primary'], /^oklch\(/)
    })

    it('dark --border uses alpha notation', () => {
      assert.match(darkTheme['--border'], /oklch\(.*\/.*\)/)
    })
  })

  describe('generateThemeCss', () => {
    let css

    it('returns a non-empty string', () => {
      css = generateThemeCss()
      assert.ok(typeof css === 'string' && css.length > 0)
    })

    it('contains :root block', () => {
      assert.ok(generateThemeCss().includes(':root {'))
    })

    it('contains .dark block', () => {
      assert.ok(generateThemeCss().includes('.dark {'))
    })

    it('contains @theme inline block', () => {
      assert.ok(generateThemeCss().includes('@theme inline {'))
    })

    it('@theme inline has --color-background', () => {
      assert.ok(generateThemeCss().includes('--color-background: var(--background)'))
    })

    it('@theme inline has --color-destructive (no foreground in new shadcn)', () => {
      const out = generateThemeCss()
      assert.ok(out.includes('--color-destructive: var(--destructive)'))
    })

    it('@theme inline has --color-sidebar-ring', () => {
      assert.ok(generateThemeCss().includes('--color-sidebar-ring: var(--sidebar-ring)'))
    })

    it('@theme inline has full radius scale', () => {
      const out = generateThemeCss()
      for (const suffix of ['sm', 'md', 'lg', 'xl', '2xl', '3xl', '4xl']) {
        assert.ok(out.includes(`--radius-${suffix}:`), `missing --radius-${suffix}`)
      }
    })

    it('light and dark background vars appear in distinct blocks', () => {
      const out = generateThemeCss()
      const darkIdx = out.indexOf('.dark {')
      const themeIdx = out.indexOf('@theme inline {')
      const lightBg = lightTheme['--background']
      const darkBg = darkTheme['--background']
      // light bg appears before .dark block
      assert.ok(out.indexOf(lightBg) < darkIdx)
      // dark bg appears between .dark and @theme
      const darkBgIdx = out.lastIndexOf(darkBg)
      assert.ok(darkBgIdx > darkIdx && darkBgIdx < themeIdx)
    })
  })

  describe('radii', () => {
    it('lg is var(--radius)', () => {
      assert.equal(radii.lg, 'var(--radius)')
    })

    it('sm / md are calc() shrink expressions', () => {
      assert.match(radii.sm, /^calc\(var\(--radius\) - \d+px\)$/)
      assert.match(radii.md, /^calc\(var\(--radius\) - \d+px\)$/)
    })

    it('xl / 2xl / 3xl / 4xl are calc() grow expressions', () => {
      for (const key of ['xl', '2xl', '3xl', '4xl']) {
        assert.match(radii[key], /^calc\(var\(--radius\) \+ \d+px\)$/, `radii.${key}`)
      }
    })

    it('full is 9999px', () => {
      assert.equal(radii.full, '9999px')
    })

    it('sm < md < lg in pixel offsets', () => {
      // sm subtracts 4px, md subtracts 2px — sm is smaller
      const smOffset = parseInt(radii.sm.match(/(\d+)px\)$/)?.[1] ?? '0')
      const mdOffset = parseInt(radii.md.match(/(\d+)px\)$/)?.[1] ?? '0')
      assert.ok(smOffset > mdOffset, 'sm should subtract more than md')
    })
  })

  describe('fiducialPreset (Tailwind v3)', () => {
    it('has theme.extend.colors and theme.extend.borderRadius', () => {
      assert.ok(fiducialPreset.theme.extend.colors)
      assert.ok(fiducialPreset.theme.extend.borderRadius)
    })

    it('does NOT add custom spacing or fontSize (use Tailwind defaults)', () => {
      assert.ok(!('spacing' in fiducialPreset.theme.extend))
      assert.ok(!('fontSize' in fiducialPreset.theme.extend))
    })

    it('preset colors are reference-equal to exported colors', () => {
      assert.equal(fiducialPreset.theme.extend.colors, colors)
    })

    it('preset borderRadius.lg is var(--radius)', () => {
      assert.equal(fiducialPreset.theme.extend.borderRadius.lg, 'var(--radius)')
    })
  })
})

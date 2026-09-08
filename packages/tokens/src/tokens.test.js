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
  spacing,
  fontSizes,
  fontWeights,
  fontFamilies,
  leadings,
  radii,
} from '../dist/index.js'
import { generateThemeCss as generateThemeCssFromTailwind } from '../dist/tailwind.js'

const VAR_RE = /^var\(--[a-z0-9-]+\)$/

describe('@fiducial/tokens', () => {
  // ── Colors ────────────────────────────────────────────────────────────────

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
      for (const v of [colors.background, colors.foreground, colors.border, colors.input, colors.ring, colors.destructive]) {
        assert.match(v, VAR_RE, `unexpected format: ${v}`)
      }
    })

    it('primary.DEFAULT is var(--primary)', () => {
      assert.equal(colors.primary.DEFAULT, 'var(--primary)')
    })
  })

  // ── Themes ────────────────────────────────────────────────────────────────

  describe('themes', () => {
    it('lightTheme has exactly 32 custom properties', () => {
      assert.equal(Object.keys(lightTheme).length, 32)
    })

    it('darkTheme has exactly 32 custom properties', () => {
      assert.equal(Object.keys(darkTheme).length, 32)
    })

    it('all values are non-empty strings', () => {
      for (const [k, v] of Object.entries(lightTheme)) {
        assert.ok(typeof v === 'string' && v.length > 0, `empty value for ${k}`)
      }
    })

    it('light and dark --background differ', () => {
      assert.notEqual(lightTheme['--background'], darkTheme['--background'])
    })

    it('values use oklch() color function', () => {
      assert.match(lightTheme['--background'], /^oklch\(/)
      assert.match(darkTheme['--background'], /^oklch\(/)
    })

    it('--radius is 0.625rem in both themes', () => {
      assert.equal(lightTheme['--radius'], '0.625rem')
      assert.equal(darkTheme['--radius'], '0.625rem')
    })

    it('dark --border uses alpha notation', () => {
      assert.match(darkTheme['--border'], /oklch\(.*\/.*\)/)
    })
  })

  // ── generateThemeCss ──────────────────────────────────────────────────────

  describe('generateThemeCss', () => {
    let css

    it('returns a non-empty string', () => {
      css = generateThemeCss()
      assert.ok(typeof css === 'string' && css.length > 0)
    })

    it('contains :root, .dark, and @theme inline blocks', () => {
      const out = generateThemeCss()
      assert.ok(out.includes(':root {'))
      assert.ok(out.includes('.dark {'))
      assert.ok(out.includes('@theme inline {'))
    })

    it('@theme inline maps --color-background', () => {
      assert.ok(generateThemeCss().includes('--color-background: var(--background)'))
    })

    it('@theme inline maps all chart slots', () => {
      const out = generateThemeCss()
      for (const n of [1, 2, 3, 4, 5]) {
        assert.ok(out.includes(`--color-chart-${n}: var(--chart-${n})`), `missing chart-${n}`)
      }
    })

    it('@theme inline maps all sidebar slots', () => {
      const out = generateThemeCss()
      for (const slot of ['sidebar', 'sidebar-foreground', 'sidebar-primary', 'sidebar-ring']) {
        assert.ok(out.includes(`--color-${slot}: var(--${slot})`), `missing ${slot}`)
      }
    })

    it('@theme inline has full radius scale sm through 4xl', () => {
      const out = generateThemeCss()
      for (const suffix of ['sm', 'md', 'lg', 'xl', '2xl', '3xl', '4xl']) {
        assert.ok(out.includes(`--radius-${suffix}:`), `missing --radius-${suffix}`)
      }
    })

    it('light and dark backgrounds appear in correct blocks', () => {
      const out = generateThemeCss()
      const darkIdx  = out.indexOf('.dark {')
      const themeIdx = out.indexOf('@theme inline {')
      const lightBg  = lightTheme['--background']
      const darkBg   = darkTheme['--background']
      assert.ok(out.indexOf(lightBg) < darkIdx,    'light bg should appear before .dark block')
      assert.ok(out.lastIndexOf(darkBg) > darkIdx && out.lastIndexOf(darkBg) < themeIdx)
    })

    it('./tailwind re-exports generateThemeCss identically', () => {
      assert.equal(generateThemeCssFromTailwind(), generateThemeCss())
    })
  })

  // ── Spacing ───────────────────────────────────────────────────────────────

  describe('spacing', () => {
    it('has 35 steps covering px, 0, 0.5–96', () => {
      assert.equal(Object.keys(spacing).length, 35)
    })

    it('step 4 = 1rem = 16px (Tailwind 4px base unit)', () => {
      assert.equal(spacing['4'].rem, '1rem')
      assert.equal(spacing['4'].px,  16)
    })

    it('step 8 = 2rem = 32px', () => {
      assert.equal(spacing['8'].rem, '2rem')
      assert.equal(spacing['8'].px,  32)
    })

    it('px step = 1px = 1px', () => {
      assert.equal(spacing['px'].rem, '1px')
      assert.equal(spacing['px'].px,  1)
    })

    it('step 0 = 0rem = 0px', () => {
      assert.equal(spacing['0'].rem, '0rem')
      assert.equal(spacing['0'].px,  0)
    })

    it('px values are exactly rem × 16 (0.25rem base)', () => {
      for (const [key, { rem, px }] of Object.entries(spacing)) {
        if (rem === '1px' || rem === '0rem' || rem === '0px') continue
        const computed = parseFloat(rem) * 16
        assert.ok(Math.abs(computed - px) < 0.01, `spacing[${key}]: ${rem} × 16 ≠ ${px}`)
      }
    })

    it('step 96 = 24rem = 384px (largest step)', () => {
      assert.equal(spacing['96'].rem, '24rem')
      assert.equal(spacing['96'].px,  384)
    })
  })

  // ── Typography ────────────────────────────────────────────────────────────

  describe('typography', () => {
    describe('fontSizes', () => {
      it('has 13 steps from xs to 9xl', () => {
        const keys = ['xs', 'sm', 'base', 'lg', 'xl', '2xl', '3xl', '4xl', '5xl', '6xl', '7xl', '8xl', '9xl']
        assert.equal(Object.keys(fontSizes).length, 13)
        for (const k of keys) assert.ok(k in fontSizes, `missing fontSizes.${k}`)
      })

      it('each entry has size and lineHeight strings', () => {
        for (const [k, v] of Object.entries(fontSizes)) {
          assert.ok(typeof v.size === 'string' && v.size.length > 0, `empty size for ${k}`)
          assert.ok(typeof v.lineHeight === 'string' && v.lineHeight.length > 0, `empty lineHeight for ${k}`)
        }
      })

      it('base is 1rem / 1.5rem', () => {
        assert.equal(fontSizes['base'].size, '1rem')
        assert.equal(fontSizes['base'].lineHeight, '1.5rem')
      })

      it('xs is 0.75rem / 1rem (12px / 16px)', () => {
        assert.equal(fontSizes['xs'].size, '0.75rem')
        assert.equal(fontSizes['xs'].lineHeight, '1rem')
      })

      it('sizes increase monotonically', () => {
        const sizes = Object.values(fontSizes).map(e =>
          e.size === '1px' ? 1 / 16 : parseFloat(e.size),
        )
        for (let i = 1; i < sizes.length; i++) {
          assert.ok(sizes[i] > sizes[i - 1], `sizes not monotonic at index ${i}`)
        }
      })

      it('display sizes (5xl+) have lineHeight 1', () => {
        for (const key of ['5xl', '6xl', '7xl', '8xl', '9xl']) {
          assert.equal(fontSizes[key].lineHeight, '1', `${key} should be tight (1)`)
        }
      })

      it('text sizes (xs–4xl) have absolute rem line-heights', () => {
        for (const key of ['xs', 'sm', 'base', 'lg', 'xl', '2xl', '3xl', '4xl']) {
          assert.match(fontSizes[key].lineHeight, /rem$/, `${key} lineHeight should be in rem`)
        }
      })
    })

    describe('fontWeights', () => {
      it('has nine named weights', () => {
        const expected = ['thin', 'extralight', 'light', 'normal', 'medium', 'semibold', 'bold', 'extrabold', 'black']
        for (const w of expected) assert.ok(w in fontWeights, `missing ${w}`)
      })

      it('normal is 400, bold is 700', () => {
        assert.equal(fontWeights.normal, '400')
        assert.equal(fontWeights.bold,   '700')
      })

      it('black is 900 (heaviest)', () => {
        assert.equal(fontWeights.black, '900')
      })
    })

    describe('fontFamilies', () => {
      it('has sans, serif, mono stacks', () => {
        assert.ok(Array.isArray(fontFamilies.sans)  && fontFamilies.sans.length  > 0)
        assert.ok(Array.isArray(fontFamilies.serif) && fontFamilies.serif.length > 0)
        assert.ok(Array.isArray(fontFamilies.mono)  && fontFamilies.mono.length  > 0)
      })

      it('sans stack starts with system fonts', () => {
        assert.equal(fontFamilies.sans[0], '-apple-system')
      })

      it('mono stack includes ui-monospace', () => {
        assert.ok(fontFamilies.mono.includes('ui-monospace'))
      })
    })

    describe('leadings', () => {
      it('has tight, snug, normal, relaxed, loose', () => {
        for (const k of ['tight', 'snug', 'normal', 'relaxed', 'loose']) {
          assert.ok(k in leadings, `missing leadings.${k}`)
        }
      })

      it('normal is 1.5', () => assert.equal(leadings.normal, '1.5'))
      it('tight is 1.25', () => assert.equal(leadings.tight, '1.25'))
      it('loose is 2', () => assert.equal(leadings.loose, '2'))
    })
  })

  // ── Radii ─────────────────────────────────────────────────────────────────

  describe('radii', () => {
    it('lg is var(--radius)', () => assert.equal(radii.lg, 'var(--radius)'))

    it('sm / md shrink from base', () => {
      assert.match(radii.sm, /^calc\(var\(--radius\) - \d+px\)$/)
      assert.match(radii.md, /^calc\(var\(--radius\) - \d+px\)$/)
    })

    it('xl / 2xl / 3xl / 4xl grow from base', () => {
      for (const key of ['xl', '2xl', '3xl', '4xl']) {
        assert.match(radii[key], /^calc\(var\(--radius\) \+ \d+px\)$/, `radii.${key}`)
      }
    })

    it('full is 9999px', () => assert.equal(radii.full, '9999px'))
  })
})

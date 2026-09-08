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
  radii,
} from '../dist/index.js'
import { fiducialPreset } from '../dist/tailwind.js'

describe('@fiducial/tokens', () => {
  describe('colors', () => {
    it('has all shadcn semantic slots', () => {
      const required = [
        'background', 'foreground', 'border', 'input', 'ring',
      ]
      for (const key of required) {
        assert.ok(key in colors, `missing colors.${key}`)
      }
    })

    it('has primary, secondary, muted, accent, destructive with DEFAULT + foreground', () => {
      for (const group of ['primary', 'secondary', 'muted', 'accent', 'destructive']) {
        assert.ok(colors[group]?.DEFAULT, `missing colors.${group}.DEFAULT`)
        assert.ok(colors[group]?.foreground, `missing colors.${group}.foreground`)
      }
    })

    it('card and popover have DEFAULT + foreground', () => {
      assert.ok(colors.card.DEFAULT)
      assert.ok(colors.card.foreground)
      assert.ok(colors.popover.DEFAULT)
      assert.ok(colors.popover.foreground)
    })

    it('all scalar values reference CSS custom properties via hsl(var(...))', () => {
      const scalars = [colors.background, colors.foreground, colors.border, colors.input, colors.ring]
      for (const v of scalars) {
        assert.match(v, /^hsl\(var\(--[a-z-]+\)\)$/, `unexpected format: ${v}`)
      }
    })

    it('primary.DEFAULT references hsl(var(--primary))', () => {
      assert.equal(colors.primary.DEFAULT, 'hsl(var(--primary))')
    })
  })

  describe('themes', () => {
    it('lightTheme has all 20 CSS custom properties', () => {
      const keys = Object.keys(lightTheme)
      assert.ok(keys.length >= 20, `expected ≥ 20 vars, got ${keys.length}`)
      assert.ok('--background' in lightTheme)
      assert.ok('--primary' in lightTheme)
      assert.ok('--radius' in lightTheme)
    })

    it('darkTheme has all 20 CSS custom properties', () => {
      assert.ok(Object.keys(darkTheme).length >= 20)
      assert.ok('--background' in darkTheme)
    })

    it('light and dark background values differ', () => {
      assert.notEqual(lightTheme['--background'], darkTheme['--background'])
    })

    it('all values are non-empty strings', () => {
      for (const [k, v] of Object.entries(lightTheme)) {
        assert.ok(typeof v === 'string' && v.length > 0, `empty value for ${k}`)
      }
    })

    it('--radius is a CSS length', () => {
      assert.match(lightTheme['--radius'], /^\d+(\.\d+)?(rem|px|em)$/)
    })
  })

  describe('generateThemeCss', () => {
    it('returns a non-empty string', () => {
      const css = generateThemeCss()
      assert.ok(typeof css === 'string' && css.length > 0)
    })

    it('contains :root block', () => {
      assert.ok(generateThemeCss().includes(':root {'))
    })

    it('contains .dark block', () => {
      assert.ok(generateThemeCss().includes('.dark,'))
    })

    it('contains --primary custom property', () => {
      assert.ok(generateThemeCss().includes('--primary:'))
    })

    it('light and dark vars are distinct in output', () => {
      const css = generateThemeCss()
      const rootSection = css.slice(0, css.indexOf('.dark'))
      const darkSection = css.slice(css.indexOf('.dark'))
      // background values are different; both should appear
      assert.ok(rootSection.includes(lightTheme['--background']))
      assert.ok(darkSection.includes(darkTheme['--background']))
    })
  })

  describe('spacing', () => {
    it('px step is 1px', () => assert.equal(spacing['px'], '1px'))
    it('0 step is 0px', () => assert.equal(spacing['0'], '0px'))
    it('step 4 is 16px (4 × 4px base)', () => assert.equal(spacing['4'], '16px'))
    it('step 8 is 32px', () => assert.equal(spacing['8'], '32px'))
    it('all values are CSS length strings', () => {
      for (const [k, v] of Object.entries(spacing)) {
        assert.match(v, /^\d+(\.\d+)?(px|rem|em)$/, `spacing[${k}] = ${v}`)
      }
    })
  })

  describe('typography', () => {
    it('fontSizes.base is 1rem / 1.5rem line-height', () => {
      assert.equal(fontSizes['base'][0], '1rem')
      assert.equal(fontSizes['base'][1].lineHeight, '1.5rem')
    })
    it('has xs through 4xl', () => {
      for (const k of ['xs', 'sm', 'base', 'lg', 'xl', '2xl', '3xl', '4xl']) {
        assert.ok(k in fontSizes, `missing fontSizes.${k}`)
      }
    })
    it('fontWeights maps names to numeric strings', () => {
      assert.equal(fontWeights['normal'], '400')
      assert.equal(fontWeights['bold'], '700')
    })
    it('fontFamilies.sans and .mono are arrays', () => {
      assert.ok(Array.isArray(fontFamilies['sans']) && fontFamilies['sans'].length > 0)
      assert.ok(Array.isArray(fontFamilies['mono']) && fontFamilies['mono'].length > 0)
    })
  })

  describe('radii', () => {
    it('lg references --radius CSS var', () => {
      assert.equal(radii['lg'], 'var(--radius)')
    })
    it('md and sm are calc() expressions', () => {
      assert.match(radii['md'], /^calc\(/)
      assert.match(radii['sm'], /^calc\(/)
    })
    it('full is 9999px', () => {
      assert.equal(radii['full'], '9999px')
    })
  })

  describe('fiducialPreset', () => {
    it('has theme.extend with all six token sets', () => {
      const ext = fiducialPreset.theme.extend
      for (const k of ['colors', 'spacing', 'fontSize', 'fontWeight', 'fontFamily', 'borderRadius']) {
        assert.ok(k in ext, `missing theme.extend.${k}`)
      }
    })
    it('preset colors are reference-equal to exported colors', () => {
      assert.equal(fiducialPreset.theme.extend.colors, colors)
    })
    it('preset borderRadius.lg is var(--radius)', () => {
      assert.equal(fiducialPreset.theme.extend.borderRadius.lg, 'var(--radius)')
    })
  })
})

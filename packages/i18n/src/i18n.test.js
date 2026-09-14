/**
 * Tests for @fiducial/i18n.
 * Imports from ../dist — built by tsc before this runs (turbo: test dependsOn build).
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'

import {
  resolveLocale,
  negotiateLocale,
  parseAcceptLanguage,
  isLocale,
  createTranslator,
  money,
  fromMajor,
  toMajor,
  minorUnitExponent,
  add,
  subtract,
  multiply,
  allocate,
  equals,
  formatMoney,
  MoneyError,
  formatNumber,
  pluralCategory,
  selectPlural,
  formatRelativeTime,
  zonedParts,
  toDateTimeLocalValue,
  fromDateTimeLocalValue,
} from '../dist/index.js'

const CONFIG = { locales: ['sr', 'en'], defaultLocale: 'sr', cookieName: 'fid-locale' }

describe('@fiducial/i18n', () => {
  // ── Locale negotiation ────────────────────────────────────────────────────

  describe('locale', () => {
    it('parses Accept-Language and orders by q-value', () => {
      assert.deepEqual(parseAcceptLanguage('en;q=0.8,sr;q=0.9,de;q=0.1'), ['sr', 'en', 'de'])
    })

    it('treats a missing q-value as 1', () => {
      assert.deepEqual(parseAcceptLanguage('en,sr;q=0.9'), ['en', 'sr'])
    })

    it('drops q=0, which means "explicitly not this one"', () => {
      assert.deepEqual(parseAcceptLanguage('en;q=0,sr'), ['sr'])
    })

    it('survives a malformed header rather than throwing', () => {
      // A header is network input. Degrading to the default beats a 500.
      assert.deepEqual(parseAcceptLanguage('!!!;q=notanumber'), [])
      assert.deepEqual(parseAcceptLanguage(null), [])
      assert.equal(negotiateLocale(CONFIG, parseAcceptLanguage('garbage;q=9')), 'sr')
    })

    it('matches a regional tag against its primary subtag', () => {
      // A reader asking for en-GB in an en-only product should get en, not the
      // default — the case naive equality matching gets wrong.
      assert.equal(negotiateLocale(CONFIG, ['en-GB']), 'en')
      assert.equal(negotiateLocale(CONFIG, ['sr-Latn-RS']), 'sr')
    })

    it('prefers an exact tag over a primary-subtag match', () => {
      const config = { locales: ['pt', 'pt-BR'], defaultLocale: 'pt', cookieName: 'c' }
      assert.equal(negotiateLocale(config, ['pt-BR']), 'pt-BR')
    })

    it('falls back to the default when nothing matches', () => {
      assert.equal(negotiateLocale(CONFIG, ['ja', 'ko']), 'sr')
    })

    it('an explicit cookie choice outranks Accept-Language', () => {
      // Someone who picked a language told you something their browser did not.
      assert.equal(resolveLocale(CONFIG, { cookie: 'en', acceptLanguage: 'sr' }), 'en')
    })

    it('ignores a cookie holding an unsupported locale', () => {
      assert.equal(resolveLocale(CONFIG, { cookie: 'de', acceptLanguage: 'en' }), 'en')
    })

    it('isLocale narrows only to declared locales', () => {
      assert.equal(isLocale(CONFIG, 'en'), true)
      assert.equal(isLocale(CONFIG, 'de'), false)
      assert.equal(isLocale(CONFIG, null), false)
    })
  })

  // ── Translation ───────────────────────────────────────────────────────────

  describe('translate', () => {
    const messages = { 'nav.home': 'Home', 'cart.items': '{count} items in {where}' }

    it('returns the message and substitutes placeholders', () => {
      const t = createTranslator({ locale: 'en', messages })
      assert.equal(t('nav.home'), 'Home')
      assert.equal(t('cart.items', { count: 3, where: 'cart' }), '3 items in cart')
    })

    it('THROWS on a missing key by default', () => {
      // This is the whole point. Ring of Pursuit rendered the key to the reader;
      // one untranslated string sat in production across 1377 keys because of it.
      const t = createTranslator({ locale: 'en', messages })
      assert.throws(() => t('nav.missing'), /no message for "nav.missing"/)
    })

    it('names the file to edit in the error', () => {
      const t = createTranslator({ locale: 'sr', messages })
      assert.throws(() => t('nav.missing'), /messages\/sr\.json/)
    })

    it('reports rather than throws when given an onMissing', () => {
      const seen = []
      const t = createTranslator({
        locale: 'sr',
        messages: {},
        fallbackMessages: messages,
        onMissing: (m) => seen.push(m),
      })

      // The reader still sees readable text — the defect was the SILENCE, not
      // the fallback.
      assert.equal(t('nav.home'), 'Home')
      assert.equal(seen.length, 1)
      assert.equal(seen[0].key, 'nav.home')
      assert.equal(seen[0].locale, 'sr')
      assert.equal(seen[0].recoveredFromDefault, true)
    })

    it('reports an unfilled placeholder — same class of bug as a missing key', () => {
      const seen = []
      const t = createTranslator({ locale: 'en', messages, onMissing: (m) => seen.push(m) })
      // Without this, a reader sees a literal "{where}" on the page.
      assert.equal(t('cart.items', { count: 3 }), '3 items in {where}')
      assert.equal(seen.length, 1)
    })

    it('has() reports the active locale without falling back', () => {
      const t = createTranslator({
        locale: 'sr',
        messages: {},
        fallbackMessages: messages,
        onMissing: () => {},
      })
      assert.equal(t.has('nav.home'), false, 'not present in the ACTIVE locale')
    })
  })

  // ── Money ─────────────────────────────────────────────────────────────────

  describe('money', () => {
    it('rejects a non-integer amount and says what to use instead', () => {
      assert.throws(() => money(10.5, 'EUR'), MoneyError)
      assert.throws(() => money(10.5, 'EUR'), /fromMajor/)
    })

    it('rejects anything that is not a 3-letter code', () => {
      assert.throws(() => money(100, 'EURO'), MoneyError)
      assert.throws(() => money(100, '€'), MoneyError)
    })

    it('normalizes the currency code to upper case', () => {
      assert.equal(money(100, 'eur').currency, 'EUR')
    })

    it('knows currencies whose minor unit is not 1/100', () => {
      // Treating ¥1000 as ¥10.00 is a factor-of-100 error that looks plausible.
      assert.equal(minorUnitExponent('JPY'), 0)
      assert.equal(minorUnitExponent('KWD'), 3)
      assert.equal(minorUnitExponent('EUR'), 2)
      assert.equal(toMajor(money(1000, 'JPY')), 1000)
      assert.equal(toMajor(money(1000, 'EUR')), 10)
    })

    it('fromMajor rounds rather than truncating', () => {
      // 10.5 is not exactly 10.5 in binary; truncation loses a cent.
      assert.equal(fromMajor(10.5, 'EUR').amountMinor, 1050)
      assert.equal(fromMajor(0.1, 'EUR').amountMinor, 10)
      assert.equal(fromMajor(1000, 'JPY').amountMinor, 1000)
    })

    it('adds exactly where floating point would not', () => {
      // The canonical demonstration: 0.1 + 0.2 !== 0.3 in binary floating point.
      const sum = add(fromMajor(0.1, 'EUR'), fromMajor(0.2, 'EUR'))
      assert.equal(sum.amountMinor, 30)
      assert.equal(toMajor(sum), 0.3)
      assert.notEqual(0.1 + 0.2, 0.3, 'the bug this type exists to prevent')
    })

    it('REFUSES arithmetic across currencies', () => {
      assert.throws(() => add(money(100, 'EUR'), money(100, 'RSD')), MoneyError)
      assert.throws(() => add(money(100, 'EUR'), money(100, 'RSD')), /exchange rate/)
      assert.throws(() => subtract(money(100, 'EUR'), money(100, 'USD')), MoneyError)
    })

    it('multiplies by a quantity or a rate', () => {
      assert.equal(multiply(fromMajor(9.99, 'EUR'), 3).amountMinor, 2997)
      // 20% VAT on 9.99 → 2.00 (rounded from 1.998).
      assert.equal(multiply(fromMajor(9.99, 'EUR'), 0.2).amountMinor, 200)
    })

    it('allocate loses nothing — the parts always sum to the whole', () => {
      // €10.00 three ways is the classic: naive division gives 3.33 each and
      // loses a cent, and a cent lost per split is a ledger that does not balance.
      const parts = allocate(fromMajor(10, 'EUR'), 3)
      assert.deepEqual(parts.map((p) => p.amountMinor), [334, 333, 333])
      assert.equal(parts.reduce((s, p) => s + p.amountMinor, 0), 1000)
    })

    it('allocate is exact for many awkward splits', () => {
      for (const [amount, parts] of [[1000, 3], [1, 7], [9999, 11], [100, 6], [5, 5]]) {
        const split = allocate(money(amount, 'EUR'), parts)
        assert.equal(split.length, parts)
        assert.equal(
          split.reduce((s, p) => s + p.amountMinor, 0),
          amount,
          `${amount} split ${parts} ways must sum back to ${amount}`,
        )
      }
    })

    it('allocate handles negative amounts symmetrically', () => {
      const parts = allocate(money(-1000, 'EUR'), 3)
      assert.equal(parts.reduce((s, p) => s + p.amountMinor, 0), -1000)
    })

    it('rejects a non-positive part count', () => {
      assert.throws(() => allocate(money(100, 'EUR'), 0), MoneyError)
      assert.throws(() => allocate(money(100, 'EUR'), 1.5), MoneyError)
    })

    it('equals compares currency as well as amount', () => {
      assert.equal(equals(money(100, 'EUR'), money(100, 'EUR')), true)
      assert.equal(equals(money(100, 'EUR'), money(100, 'USD')), false)
    })

    it('formats the same amount differently per locale, same currency', () => {
      // The locale decides HOW it is written; the currency decides WHAT it is.
      const price = fromMajor(1234.5, 'EUR')
      const en = formatMoney(price, 'en-US')
      const de = formatMoney(price, 'de-DE')
      assert.notEqual(en, de, 'locale must change the rendering')
      assert.ok(en.includes('1,234.50'), `en-US: ${en}`)
      assert.ok(de.includes('1.234,50'), `de-DE: ${de}`)
    })

    it('renders a currency the reader does not share a locale with', () => {
      // A Serbian reader invoiced in EUR. Inferring currency from locale is
      // wrong for every cross-border product.
      const out = formatMoney(fromMajor(50, 'EUR'), 'sr')
      assert.ok(out.length > 0)
      assert.ok(/50/.test(out), out)
    })

    it('omits minor units on request when they are zero', () => {
      assert.ok(!formatMoney(fromMajor(10, 'EUR'), 'en-US', { hideZeroMinorUnits: true })
        .includes('.00'))
      // …but keeps them when they are not zero.
      assert.ok(formatMoney(fromMajor(10.5, 'EUR'), 'en-US', { hideZeroMinorUnits: true })
        .includes('.50'))
    })
  })

  // ── Formatting ────────────────────────────────────────────────────────────

  describe('format', () => {
    it('formats numbers per locale', () => {
      assert.equal(formatNumber(1234.56, 'en-US'), '1,234.56')
      assert.equal(formatNumber(1234.56, 'de-DE'), '1.234,56')
    })

    it('Serbian has three plural categories, not two', () => {
      // A catalog carrying only singular/plural cannot be translated into
      // Serbian correctly — which is why plurals belong in the catalog format.
      assert.equal(pluralCategory(1, 'sr'), 'one')
      assert.equal(pluralCategory(2, 'sr'), 'few')
      assert.equal(pluralCategory(5, 'sr'), 'other')
      assert.equal(pluralCategory(21, 'sr'), 'one')

      assert.equal(pluralCategory(1, 'en'), 'one')
      assert.equal(pluralCategory(2, 'en'), 'other')
    })

    it('selectPlural picks the right form and falls back to other', () => {
      const forms = { one: 'jedan', few: 'nekoliko', other: 'mnogo' }
      assert.equal(selectPlural(1, 'sr', forms), 'jedan')
      assert.equal(selectPlural(3, 'sr', forms), 'nekoliko')
      assert.equal(selectPlural(9, 'sr', forms), 'mnogo')
      assert.equal(selectPlural(2, 'en', { other: 'many' }), 'many')
    })

    it('throws when a plural message has no "other" form', () => {
      assert.throws(() => selectPlural(5, 'sr', { one: 'jedan' }), /"other"/)
    })

    it('formats relative time in the reader language', () => {
      const now = new Date('2026-09-14T12:00:00Z')
      const out = formatRelativeTime(new Date('2026-09-12T12:00:00Z'), 'en', now)
      assert.match(out, /2 days ago/)
      assert.match(formatRelativeTime(new Date('2026-09-14T15:00:00Z'), 'en', now), /in 3 hours/)
    })
  })

  // ── Timezones ─────────────────────────────────────────────────────────────

  describe('timezone', () => {
    it('renders an instant as wall-clock time in a named zone', () => {
      // 12:00 UTC is 14:00 in Belgrade in summer (CEST, UTC+2).
      const p = zonedParts(new Date('2026-07-14T12:00:00Z'), 'Europe/Belgrade')
      assert.equal(p.hour, 14)
      assert.equal(p.day, 14)
    })

    it('accounts for daylight saving', () => {
      // The same clock time in winter is UTC+1, so 13:00 not 14:00.
      const winter = zonedParts(new Date('2026-01-14T12:00:00Z'), 'Europe/Belgrade')
      assert.equal(winter.hour, 13)
    })

    it('round-trips an instant through a datetime-local value', () => {
      const instant = new Date('2026-07-14T12:00:00Z')
      const local = toDateTimeLocalValue(instant, 'Europe/Belgrade')
      assert.equal(local, '2026-07-14T14:00')
      assert.equal(fromDateTimeLocalValue(local, 'Europe/Belgrade').getTime(), instant.getTime())
    })

    it('round-trips across a DST boundary', () => {
      // The case offset arithmetic gets wrong: the offset depends on the instant
      // you are asking about, which is circular until you probe twice.
      for (const iso of [
        '2026-03-29T00:30:00Z', // just before the EU spring transition
        '2026-03-29T01:30:00Z', // just after
        '2026-10-25T00:30:00Z', // autumn, the ambiguous hour
        '2026-12-31T23:00:00Z',
      ]) {
        const instant = new Date(iso)
        const local = toDateTimeLocalValue(instant, 'Europe/Belgrade')
        const back = fromDateTimeLocalValue(local, 'Europe/Belgrade')
        assert.equal(
          toDateTimeLocalValue(back, 'Europe/Belgrade'),
          local,
          `${iso} must round-trip to the same wall clock`,
        )
      }
    })

    it('rejects a malformed datetime-local value', () => {
      assert.throws(() => fromDateTimeLocalValue('14/07/2026', 'Europe/Belgrade'), /YYYY-MM-DD/)
    })

    it('a zone is a declared fact, not the server timezone', () => {
      // The same instant renders differently per zone. If this depended on the
      // machine's clock, the answer would change when you deployed elsewhere.
      const instant = new Date('2026-07-14T12:00:00Z')
      assert.equal(zonedParts(instant, 'Europe/Belgrade').hour, 14)
      assert.equal(zonedParts(instant, 'UTC').hour, 12)
      assert.equal(zonedParts(instant, 'America/New_York').hour, 8)
    })
  })
})

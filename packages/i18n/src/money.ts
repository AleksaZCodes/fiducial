/**
 * Money.
 *
 * Principle 1 says a physical fact is never a bare number — it carries its unit.
 * An amount of money is the same kind of fact, and it fails the same way when
 * you strip the unit off: silently, until two of them are added together and the
 * result is nonsense in a currency nobody chose.
 *
 * So money is a **type**, and two decisions in it are load-bearing.
 *
 * # Integer minor units
 *
 * The amount is held in the currency's smallest unit — cents, para, yen — as an
 * integer. Never a float.
 *
 * Binary floating point cannot represent `0.10`. `0.1 + 0.2 === 0.30000000000000004`
 * is not a curiosity; it is a rounding error that compounds across a cart, a tax
 * calculation and an invoice total until a customer is charged a cent more than
 * the line items add to. Integers in minor units make the arithmetic exact.
 *
 * # Currency is independent of locale
 *
 * A reader's language does not determine what they pay in. A Serbian reader may
 * be invoiced in EUR; an English-language checkout may charge RSD. Systems that
 * infer currency from locale are wrong for every cross-border product, and the
 * bug surfaces as a price that silently changes meaning rather than as an error.
 *
 * So `Money` carries its currency, and `formatMoney` takes the locale
 * separately: the locale decides *how* it is written, the currency decides
 * *what it is*.
 */

import type { Locale } from './locale.js'

/** An ISO 4217 alphabetic code, e.g. `"EUR"`, `"RSD"`, `"JPY"`. */
export type CurrencyCode = string

/**
 * An amount in a currency.
 *
 * `amountMinor` is in the currency's smallest unit and must be an integer:
 * `{ amountMinor: 1050, currency: "EUR" }` is €10.50.
 */
export type Money = {
  readonly amountMinor: number
  readonly currency: CurrencyCode
}

/**
 * Currencies whose minor-unit exponent is not 2.
 *
 * Most currencies have 100 minor units to the major. These do not, and assuming
 * they do is the classic bug: treating ¥1000 as ¥10.00 is a factor-of-100 error
 * that looks entirely plausible on a page.
 *
 * Derived from ISO 4217. Anything absent here has an exponent of 2.
 */
const MINOR_UNIT_EXPONENTS: Readonly<Record<string, number>> = {
  // Zero-decimal currencies.
  BIF: 0, CLP: 0, DJF: 0, GNF: 0, ISK: 0, JPY: 0, KMF: 0, KRW: 0,
  PYG: 0, RWF: 0, UGX: 0, UYI: 0, VND: 0, VUV: 0, XAF: 0, XOF: 0, XPF: 0,
  // Three-decimal currencies.
  BHD: 3, IQD: 3, JOD: 3, KWD: 3, LYD: 3, OMR: 3, TND: 3,
  // Four-decimal.
  CLF: 4, UYW: 4,
}

/** How many minor units make one major unit of `currency`. */
export function minorUnitExponent(currency: CurrencyCode): number {
  return MINOR_UNIT_EXPONENTS[currency.toUpperCase()] ?? 2
}

export class MoneyError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'MoneyError'
  }
}

/** Construct money from an integer number of minor units. */
export function money(amountMinor: number, currency: CurrencyCode): Money {
  if (!Number.isInteger(amountMinor)) {
    throw new MoneyError(
      `amountMinor must be an integer number of minor units, got ${amountMinor}. ` +
        `For a major-unit amount use fromMajor(${amountMinor}, "${currency}").`,
    )
  }
  if (!Number.isSafeInteger(amountMinor)) {
    throw new MoneyError(`amountMinor ${amountMinor} exceeds safe integer range.`)
  }
  if (!/^[A-Za-z]{3}$/.test(currency)) {
    throw new MoneyError(`currency must be a 3-letter ISO 4217 code, got "${currency}".`)
  }
  return { amountMinor, currency: currency.toUpperCase() }
}

/**
 * Construct money from a major-unit amount: `fromMajor(10.5, "EUR")` → €10.50.
 *
 * Rounds to the currency's minor-unit precision, because `10.5` in binary is
 * not exactly 10.5 and truncating would lose a cent.
 */
export function fromMajor(amountMajor: number, currency: CurrencyCode): Money {
  if (!Number.isFinite(amountMajor)) {
    throw new MoneyError(`amountMajor must be finite, got ${amountMajor}.`)
  }
  const factor = 10 ** minorUnitExponent(currency)
  return money(Math.round(amountMajor * factor), currency)
}

/** The amount in major units, for display or export. Never for arithmetic. */
export function toMajor(value: Money): number {
  return value.amountMinor / 10 ** minorUnitExponent(value.currency)
}

/**
 * Reject arithmetic across currencies.
 *
 * There is no correct answer for EUR + RSD without an exchange rate, and an
 * exchange rate is a fact with a timestamp that belongs to the caller. Guessing
 * is worse than refusing.
 */
function assertSameCurrency(a: Money, b: Money, operation: string): void {
  if (a.currency !== b.currency) {
    throw new MoneyError(
      `cannot ${operation} ${a.currency} and ${b.currency}. ` +
        `Convert one first — an exchange rate is a fact with a timestamp, and ` +
        `this function has neither.`,
    )
  }
}

export function add(a: Money, b: Money): Money {
  assertSameCurrency(a, b, 'add')
  return money(a.amountMinor + b.amountMinor, a.currency)
}

export function subtract(a: Money, b: Money): Money {
  assertSameCurrency(a, b, 'subtract')
  return money(a.amountMinor - b.amountMinor, a.currency)
}

/** Multiply by a plain number — a quantity, a tax rate. Rounds to minor units. */
export function multiply(value: Money, factor: number): Money {
  if (!Number.isFinite(factor)) {
    throw new MoneyError(`factor must be finite, got ${factor}.`)
  }
  return money(Math.round(value.amountMinor * factor), value.currency)
}

/** Negative, zero and positive tests, which read better than comparing fields. */
export function isZero(value: Money): boolean {
  return value.amountMinor === 0
}
export function isNegative(value: Money): boolean {
  return value.amountMinor < 0
}

export function compare(a: Money, b: Money): number {
  assertSameCurrency(a, b, 'compare')
  return a.amountMinor - b.amountMinor
}

export function equals(a: Money, b: Money): boolean {
  return a.currency === b.currency && a.amountMinor === b.amountMinor
}

/**
 * Split money into `parts` as evenly as possible, losing nothing.
 *
 * Dividing €10.00 three ways cannot give three equal parts. Naive division gives
 * €3.33 each and loses a cent — and a cent lost per split is a ledger that does
 * not balance. This distributes the remainder one minor unit at a time, so the
 * parts always sum to the original exactly.
 */
export function allocate(value: Money, parts: number): Money[] {
  if (!Number.isInteger(parts) || parts < 1) {
    throw new MoneyError(`parts must be a positive integer, got ${parts}.`)
  }

  const base = Math.trunc(value.amountMinor / parts)
  let remainder = value.amountMinor - base * parts

  return Array.from({ length: parts }, () => {
    // Remainder is distributed in the direction of the amount's sign, so
    // splitting a negative amount behaves symmetrically with a positive one.
    const extra = remainder !== 0 ? Math.sign(remainder) : 0
    if (extra !== 0) remainder -= extra
    return money(base + extra, value.currency)
  })
}

export type MoneyFormatOptions = {
  /** `"symbol"` → €10.50, `"code"` → EUR 10.50, `"name"` → 10.50 euros. */
  readonly display?: 'symbol' | 'code' | 'name' | 'narrowSymbol'
  /** Drop the minor units when they are zero: €10 rather than €10.00. */
  readonly hideZeroMinorUnits?: boolean
}

/**
 * Format money for a reader.
 *
 * The **locale** decides how it is written — symbol placement, decimal
 * separator, digit grouping. The **currency** decides what it is. They are
 * separate arguments because they are separate facts.
 */
export function formatMoney(
  value: Money,
  locale: Locale,
  options: MoneyFormatOptions = {},
): string {
  const exponent = minorUnitExponent(value.currency)
  const digits =
    options.hideZeroMinorUnits && value.amountMinor % 10 ** exponent === 0 ? 0 : exponent

  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency: value.currency,
    currencyDisplay: options.display ?? 'symbol',
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(toMajor(value))
}

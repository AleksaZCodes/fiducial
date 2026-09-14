/**
 * Locale-aware formatting: dates, times, timezones, numbers, plurals and
 * relative time.
 *
 * Everything here uses only the platform's own `Intl`, which is why it runs
 * unchanged on the server, in the browser, at the edge and inside a Server
 * Action. Generalized from Ring of Pursuit's `intl.ts` and `timezone.ts`, whose
 * one weakness was hard-coding the product's own locale set.
 *
 * `Intl` formatter construction is not free, so formatters are memoized — the
 * same call in a table of a thousand rows builds one formatter, not a thousand.
 */

import type { Locale } from './locale.js'

/** An IANA timezone name, e.g. `"Europe/Belgrade"`. */
export type TimeZone = string

const dateTimeCache = new Map<string, Intl.DateTimeFormat>()
const numberCache = new Map<string, Intl.NumberFormat>()
const pluralCache = new Map<string, Intl.PluralRules>()
const relativeCache = new Map<string, Intl.RelativeTimeFormat>()

function cached<T>(cache: Map<string, T>, key: string, build: () => T): T {
  let value = cache.get(key)
  if (value === undefined) {
    value = build()
    cache.set(key, value)
  }
  return value
}

// ── Dates and times ─────────────────────────────────────────────────────────

/**
 * Format an instant in a named timezone.
 *
 * **An instant plus a zone, never a "local time".** A `Date` is an absolute
 * point on the timeline; what a reader should see depends on which zone you
 * render it in, and that zone is a declared fact about the event — not about the
 * server, and not about the reader's machine.
 *
 * Ring of Pursuit stores an absolute UTC instant in `events.date` *plus* an IANA
 * zone in `events.timezone` for exactly this reason: an event happens at 19:00
 * in Belgrade regardless of where it is being read from.
 */
export function formatDateTime(
  value: Date,
  locale: Locale,
  options: Intl.DateTimeFormatOptions & { readonly timeZone?: TimeZone } = {},
): string {
  const key = `${locale}|${JSON.stringify(options)}`
  return cached(dateTimeCache, key, () => new Intl.DateTimeFormat(locale, options)).format(
    value,
  )
}

export function formatDate(
  value: Date,
  locale: Locale,
  timeZone?: TimeZone,
  dateStyle: Intl.DateTimeFormatOptions['dateStyle'] = 'medium',
): string {
  return formatDateTime(value, locale, { dateStyle, timeZone })
}

export function formatTime(
  value: Date,
  locale: Locale,
  timeZone?: TimeZone,
  timeStyle: Intl.DateTimeFormatOptions['timeStyle'] = 'short',
): string {
  return formatDateTime(value, locale, { timeStyle, timeZone })
}

/**
 * The wall-clock parts of an instant in a zone.
 *
 * This is what a `<input type="datetime-local">` needs: an admin editing an
 * event in Belgrade wants to see and type Belgrade time, while the database
 * holds the absolute instant. Converting between the two by hand — with
 * `getHours()` on a server in a different zone — is the bug this exists to
 * prevent.
 */
export function zonedParts(
  value: Date,
  timeZone: TimeZone,
): { year: number; month: number; day: number; hour: number; minute: number; second: number } {
  const formatter = cached(dateTimeCache, `parts|${timeZone}`, () =>
    new Intl.DateTimeFormat('en-CA', {
      timeZone,
      hour12: false,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }),
  )

  const parts: Record<string, string> = {}
  for (const part of formatter.formatToParts(value)) {
    if (part.type !== 'literal') parts[part.type] = part.value
  }

  return {
    year: Number(parts.year),
    month: Number(parts.month),
    day: Number(parts.day),
    // `hour12: false` renders midnight as 24 in some engines; normalize it.
    hour: Number(parts.hour) % 24,
    minute: Number(parts.minute),
    second: Number(parts.second),
  }
}

/** The `YYYY-MM-DDTHH:mm` string an `<input type="datetime-local">` expects. */
export function toDateTimeLocalValue(value: Date, timeZone: TimeZone): string {
  const p = zonedParts(value, timeZone)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${p.year}-${pad(p.month)}-${pad(p.day)}T${pad(p.hour)}:${pad(p.minute)}`
}

/**
 * The absolute instant for a wall-clock time in a zone — the inverse of
 * {@link toDateTimeLocalValue}.
 *
 * Solved by offset probing rather than arithmetic, because a zone's offset
 * depends on the instant you are asking about and DST makes that circular. The
 * second pass handles the case where the first guess lands on the other side of
 * a transition.
 */
export function fromDateTimeLocalValue(value: string, timeZone: TimeZone): Date {
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/.exec(value)
  if (!match) {
    throw new Error(`expected "YYYY-MM-DDTHH:mm", got "${value}"`)
  }
  const [, y, mo, d, h, mi, s] = match
  const asUtc = Date.UTC(
    Number(y),
    Number(mo) - 1,
    Number(d),
    Number(h),
    Number(mi),
    Number(s ?? 0),
  )

  const offsetAt = (instant: number): number => {
    const p = zonedParts(new Date(instant), timeZone)
    return Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute, p.second) - instant
  }

  const firstGuess = asUtc - offsetAt(asUtc)
  return new Date(asUtc - offsetAt(firstGuess))
}

// ── Numbers ─────────────────────────────────────────────────────────────────

/** `1234.56` → `"1,234.56"` in `en`, `"1.234,56"` in `sr`. */
export function formatNumber(
  value: number,
  locale: Locale,
  options: Intl.NumberFormatOptions = {},
): string {
  const key = `${locale}|${JSON.stringify(options)}`
  return cached(numberCache, key, () => new Intl.NumberFormat(locale, options)).format(value)
}

/** `0.075` → `"7.5%"`. Takes a fraction, not an already-multiplied number. */
export function formatPercent(
  fraction: number,
  locale: Locale,
  maximumFractionDigits = 1,
): string {
  return formatNumber(fraction, locale, { style: 'percent', maximumFractionDigits })
}

// ── Plurals ─────────────────────────────────────────────────────────────────

/**
 * The CLDR plural category for a count in a locale.
 *
 * English has two forms. **Serbian has three** — `one` (1, 21, 31…), `few`
 * (2–4, 22–24…) and `other` — and they do not map onto "singular/plural". A
 * message catalog that only carries singular and plural cannot be translated
 * into Serbian correctly, which is why plural selection belongs in the catalog
 * format rather than at the call site.
 */
export function pluralCategory(count: number, locale: Locale): Intl.LDMLPluralRule {
  return cached(pluralCache, locale, () => new Intl.PluralRules(locale)).select(count)
}

/** Pick the right variant of a message for `count`, falling back to `other`. */
export function selectPlural(
  count: number,
  locale: Locale,
  forms: Partial<Record<Intl.LDMLPluralRule, string>>,
): string {
  const category = pluralCategory(count, locale)
  const chosen = forms[category] ?? forms.other
  if (chosen === undefined) {
    throw new Error(
      `no plural form for category "${category}" in locale "${locale}", and no ` +
        `"other" fallback. Every plural message must define at least "other".`,
    )
  }
  return chosen
}

// ── Relative time ───────────────────────────────────────────────────────────

const RELATIVE_UNITS: readonly (readonly [Intl.RelativeTimeFormatUnit, number])[] = [
  ['year', 365 * 24 * 60 * 60 * 1000],
  ['month', 30 * 24 * 60 * 60 * 1000],
  ['week', 7 * 24 * 60 * 60 * 1000],
  ['day', 24 * 60 * 60 * 1000],
  ['hour', 60 * 60 * 1000],
  ['minute', 60 * 1000],
  ['second', 1000],
]

/**
 * `"2 days ago"`, `"in 3 hours"` — in the reader's language.
 *
 * Not a string you can interpolate: the unit, the sign and the plural form all
 * change together, and languages disagree about all three.
 */
export function formatRelativeTime(
  value: Date,
  locale: Locale,
  now: Date = new Date(),
  options: Intl.RelativeTimeFormatOptions = { numeric: 'auto' },
): string {
  const deltaMs = value.getTime() - now.getTime()
  const formatter = cached(relativeCache, `${locale}|${JSON.stringify(options)}`, () =>
    new Intl.RelativeTimeFormat(locale, options),
  )

  for (const [unit, ms] of RELATIVE_UNITS) {
    if (Math.abs(deltaMs) >= ms) {
      return formatter.format(Math.trunc(deltaMs / ms), unit)
    }
  }
  return formatter.format(0, 'second')
}

/** Format a list: `"a, b and c"` — conjunctions differ by locale. */
export function formatList(
  items: readonly string[],
  locale: Locale,
  type: Intl.ListFormatType = 'conjunction',
): string {
  return new Intl.ListFormat(locale, { style: 'long', type }).format(items)
}

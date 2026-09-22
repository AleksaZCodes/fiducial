/**
 * The generated catalog, bound to `@fiducial/i18n`.
 *
 * ## Why this file is four lines of wiring and no `Intl` calls
 *
 * Every product that renders a date reaches the same fork: write
 * `new Intl.DateTimeFormat(...)` inline, or hand-maintain a map from locale
 * code to BCP-47 tag. Both have shipped here. The inline version rebuilds a
 * formatter per row and picks a timezone from whatever machine happens to be
 * rendering; the hand-maintained map is a second declaration of the locale
 * set, and it is wrong the first time a locale is added and only that map is
 * missed.
 *
 * `@fiducial/i18n` already answers all of it — memoized formatters, CLDR
 * plural categories (Serbian has three, and `one`/`other` cannot express
 * them), money as minor units, relative time, list formatting. So this module
 * adds nothing but the binding: **the BCP-47 tag comes out of the catalog**,
 * where `locale.tag` is already declared once per language, so there is no
 * table here to drift.
 *
 * Framework-agnostic on purpose — no import from React, Svelte or a router.
 * The same file works under `@/lib/intl` and `$lib/intl`, which is what makes
 * porting a page mechanical rather than a rewrite.
 */

import {
  formatDate as fmtDate,
  formatDateTime as fmtDateTime,
  formatList as fmtList,
  formatMoney as fmtMoney,
  formatNumber as fmtNumber,
  formatRelativeTime as fmtRelative,
  type Money,
  pluralCategory as cldrPluralCategory,
  selectPlural as cldrSelectPlural,
  type TimeZone,
} from "@fiducial/i18n";
import { type Locale, messages } from "../generated/messages";

/**
 * The BCP-47 tag `Intl` needs for a locale, read from that locale's own
 * catalog (`"locale": { "tag": "sr-Latn-RS" }`).
 *
 * A locale *code* (`sr`) and a locale *tag* (`sr-Latn-RS`) are not the same
 * fact, and formatting with the code silently gives you Cyrillic for a product
 * written in Latin script. The catalog is where the tag is declared, so this
 * is the only place that has to know.
 */
export function localeTag(locale: Locale): string {
  return messages[locale]["locale.tag"];
}

/**
 * A content date — `YYYY-MM-DD`, as every collection entry's frontmatter
 * stores it — as a reader of `locale` would write it.
 *
 * Parsed at **UTC noon**, not local midnight. `new Date("2026-09-24")` is
 * midnight UTC, and rendering that anywhere west of Greenwich prints the 23rd.
 * A date with no time is a calendar day, and noon is the hour that survives
 * every zone.
 */
export function formatContentDate(
  iso: string,
  locale: Locale,
  dateStyle: Intl.DateTimeFormatOptions["dateStyle"] = "long",
): string {
  const [y, m, d] = iso.split("-").map(Number);
  return fmtDate(new Date(Date.UTC(y, m - 1, d, 12)), localeTag(locale), "UTC", dateStyle);
}

/**
 * An instant, in a named zone.
 *
 * `timeZone` is required rather than defaulted, and that is the interesting
 * part: an event happens at 12:00 in Belgrade whoever is reading it, so the
 * zone is a declared fact about the event and never about the server. Omitting
 * it would render the machine's zone and look right on the developer's laptop.
 */
export function formatInstant(
  value: Date,
  locale: Locale,
  timeZone: TimeZone,
  options: Intl.DateTimeFormatOptions = { dateStyle: "long", timeStyle: "short" },
): string {
  return fmtDateTime(value, localeTag(locale), { ...options, timeZone });
}

/** `1234.56` → `1.234,56` in `sr`, `1,234.56` in `en`. */
export function formatNumber(
  value: number,
  locale: Locale,
  options: Intl.NumberFormatOptions = {},
): string {
  return fmtNumber(value, localeTag(locale), options);
}

/**
 * A price, from minor units.
 *
 * `money(1999, "EUR")`, not `19.99` — a float is the wrong type for money and
 * the rounding error shows up in a total, long after the line item looked
 * fine. `@fiducial/i18n`'s `Money` carries the currency with the amount.
 */
export function formatPrice(value: Money, locale: Locale): string {
  return fmtMoney(value, localeTag(locale));
}

/** `["a", "b", "c"]` → `a, b and c`, in the reader's language. */
export function formatList(
  items: readonly string[],
  locale: Locale,
  type: Intl.ListFormatType = "conjunction",
): string {
  return fmtList(items, localeTag(locale), type);
}

/** `-2 days` → `2 days ago`, in the reader's language. */
export function formatRelativeTime(value: Date, locale: Locale, now: Date = new Date()): string {
  return fmtRelative(value, localeTag(locale), now);
}

/**
 * The CLDR plural category for a count.
 *
 * Serbian has three — `one` (1, 21, 31…), `few` (2–4, 22–24…) and `other` —
 * and they are not "singular and plural". Catalogs carry one key per category
 * (`spots.left.one`, `spots.left.few`, `spots.left.other`); this picks which.
 */
export function pluralCategory(count: number, locale: Locale): Intl.LDMLPluralRule {
  return cldrPluralCategory(count, localeTag(locale));
}

/** Pick the right variant of a message for `count`, falling back to `other`. */
export function selectPlural(
  count: number,
  locale: Locale,
  forms: Partial<Record<Intl.LDMLPluralRule, string>>,
): string {
  return cldrSelectPlural(count, localeTag(locale), forms);
}

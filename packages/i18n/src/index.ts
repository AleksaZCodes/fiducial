// @fiducial/i18n — localized by construction.
//
// See MISSION.md principle 1c: a user-visible string is a fact, declared once,
// with every locale a derivation that must exist.

export type { Locale, LocaleConfig } from './locale.js'
export {
  isLocale,
  parseAcceptLanguage,
  negotiateLocale,
  resolveLocale,
} from './locale.js'

export type {
  MessageVars,
  Messages,
  MissingMessage,
  Translator,
  TranslatorOptions,
} from './translate.js'
export { createTranslator, reportMissing } from './translate.js'

export type { CurrencyCode, Money, MoneyFormatOptions } from './money.js'
export {
  MoneyError,
  money,
  fromMajor,
  toMajor,
  minorUnitExponent,
  add,
  subtract,
  multiply,
  allocate,
  compare,
  equals,
  isZero,
  isNegative,
  formatMoney,
} from './money.js'

export type { TimeZone } from './format.js'
export {
  formatDateTime,
  formatDate,
  formatTime,
  zonedParts,
  toDateTimeLocalValue,
  fromDateTimeLocalValue,
  formatNumber,
  formatPercent,
  pluralCategory,
  selectPlural,
  formatRelativeTime,
  formatList,
} from './format.js'

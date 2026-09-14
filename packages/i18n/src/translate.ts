/**
 * Message lookup.
 *
 * This is the file Ring of Pursuit's `translate.ts` is deliberately **not**
 * harvested into. Its implementation was:
 *
 * ```ts
 * const raw = messages[key] ?? key
 * ```
 *
 * A missing translation rendered the key itself — `nav.dashboard.title` — to a
 * reader, and `key: string` meant a typo was undetectable. That is the failure
 * mode principle 1c exists to delete: a missing translation is a missing
 * artifact, not a fallback.
 *
 * **The defect was the silence, not the fallback.** Falling back so a reader
 * sees readable text is good product behaviour. Falling back without telling
 * anyone is how one untranslated string sat in production across 1377 keys.
 * So this module still falls back — and reports every time it does.
 */

import type { Locale } from './locale.js'

/** Values substituted into `{placeholders}`. */
export type MessageVars = Readonly<Record<string, string | number>>

/**
 * A catalog: message key → template string.
 *
 * `K` is the generated union of valid keys. Passing a key outside it is a
 * compile error rather than a runtime surprise — which is the point of
 * generating the union in the first place.
 */
export type Messages<K extends string = string> = Readonly<Record<K, string>>

/** What happened when a key could not be resolved in the active locale. */
export type MissingMessage = {
  readonly key: string
  readonly locale: Locale
  /** True when the default locale supplied text and the reader saw that. */
  readonly recoveredFromDefault: boolean
}

export type TranslatorOptions<K extends string> = {
  readonly locale: Locale
  readonly messages: Messages<K>
  /**
   * The default locale's catalog, used only to keep a reader from seeing a raw
   * key. Its use is always reported.
   */
  readonly fallbackMessages?: Messages<K>
  /**
   * Called whenever a key is missing, or a placeholder is left unfilled.
   *
   * Default: throw. A missing key means `fid derive --check` was bypassed, and
   * failing loudly during development is the entire point. Products override
   * this in production to report instead of crash — see {@link reportMissing}.
   */
  readonly onMissing?: (missing: MissingMessage) => void
}

export type Translator<K extends string> = {
  (key: K, vars?: MessageVars): string
  /** True when `key` resolves in the active locale without falling back. */
  readonly has: (key: K) => boolean
  readonly locale: Locale
}

/** The default `onMissing`: throw, because this should be impossible. */
function throwOnMissing(missing: MissingMessage): never {
  throw new Error(
    `i18n: no message for "${missing.key}" in locale "${missing.locale}".\n` +
      `Every locale must define every key — a missing translation is a missing ` +
      `artifact, not a fallback.\n` +
      `Add it to messages/${missing.locale}.json and run \`fid derive\`.`,
  )
}

/**
 * An `onMissing` that reports without throwing.
 *
 * For production, where crashing a page over one string is worse than showing
 * the reader default-locale text. It is still **reported** — the thing ROP's
 * version never did.
 */
export function reportMissing(
  report: (missing: MissingMessage) => void,
): (missing: MissingMessage) => void {
  return report
}

const PLACEHOLDER = /\{(\w+)\}/g

export function createTranslator<K extends string>(
  options: TranslatorOptions<K>,
): Translator<K> {
  const { locale, messages, fallbackMessages, onMissing = throwOnMissing } = options

  const translate = (key: K, vars?: MessageVars): string => {
    let template: string | undefined = messages[key]

    if (template === undefined) {
      const recovered = fallbackMessages?.[key]
      onMissing({ key, locale, recoveredFromDefault: recovered !== undefined })
      // Reached only when onMissing chose not to throw.
      if (recovered === undefined) return key
      template = recovered
    }

    if (!vars) return template

    return template.replace(PLACEHOLDER, (whole, name: string) => {
      const value = vars[name]
      if (value === undefined) {
        // An unfilled placeholder renders as literal `{count}` to a reader.
        // Same class of bug as a missing key, so it takes the same path.
        onMissing({ key, locale, recoveredFromDefault: false })
        return whole
      }
      return String(value)
    })
  }

  return Object.assign(translate, {
    has: (key: K) => messages[key] !== undefined,
    locale,
  })
}

/**
 * Locale identity and negotiation.
 *
 * Generalized from Ring of Pursuit's `config.ts` and `resolve-locale.ts`, which
 * hard-coded its own two locales and its own cookie name. Here both are declared
 * by the product.
 */

/** A BCP 47 language tag, e.g. `"sr"`, `"en"`, `"pt-BR"`. */
export type Locale = string

/**
 * The set of locales a product supports, and which one it falls back to.
 *
 * This mirrors the `[i18n]` block in `fiducial.toml`. It is generated rather
 * than hand-written, so the runtime and the build cannot disagree about which
 * locales exist — the disagreement being the whole failure mode.
 */
export type LocaleConfig<L extends Locale = Locale> = {
  readonly locales: readonly L[]
  readonly defaultLocale: L
  /** Cookie used to remember an explicit choice. */
  readonly cookieName: string
}

export function isLocale<L extends Locale>(
  config: LocaleConfig<L>,
  value: string | null | undefined,
): value is L {
  return value != null && (config.locales as readonly string[]).includes(value)
}

/**
 * Parse an `Accept-Language` header into tags, most-preferred first.
 *
 * Respects q-values. A header is user input from the network, so anything
 * malformed is skipped rather than throwing — a bad header should degrade to the
 * default locale, not return a 500.
 */
export function parseAcceptLanguage(header: string | null | undefined): string[] {
  if (!header) return []

  return header
    .split(',')
    .map((part) => {
      const [tag, ...params] = part.trim().split(';')
      if (!tag) return null

      let quality = 1
      for (const param of params) {
        const [key, value] = param.split('=').map((s) => s.trim())
        if (key === 'q') {
          const parsed = Number.parseFloat(value ?? '')
          // An unparseable or out-of-range q-value means the entry is malformed.
          if (!Number.isFinite(parsed) || parsed < 0 || parsed > 1) return null
          quality = parsed
        }
      }

      const trimmed = tag.trim()
      return trimmed ? { tag: trimmed, quality } : null
    })
    .filter((entry): entry is { tag: string; quality: number } => entry !== null)
    .filter((entry) => entry.quality > 0)
    .sort((a, b) => b.quality - a.quality)
    .map((entry) => entry.tag)
}

/**
 * Choose the best supported locale for a list of requested tags.
 *
 * Matching is two-pass: exact tag first, then primary subtag. So a reader asking
 * for `pt-BR` gets `pt-BR` if it exists and `pt` if it does not — and a reader
 * asking for `en-GB` in a product that only ships `en` gets `en` rather than
 * falling through to the default, which is the case naive matching gets wrong.
 */
export function negotiateLocale<L extends Locale>(
  config: LocaleConfig<L>,
  requested: readonly string[],
): L {
  const supported = config.locales

  for (const tag of requested) {
    const exact = supported.find((l) => l.toLowerCase() === tag.toLowerCase())
    if (exact) return exact
  }

  for (const tag of requested) {
    const primary = tag.split('-')[0]?.toLowerCase()
    if (!primary) continue
    const match = supported.find((l) => l.split('-')[0]?.toLowerCase() === primary)
    if (match) return match
  }

  return config.defaultLocale
}

/**
 * Resolve the locale for a request.
 *
 * Precedence, strongest first:
 *
 * 1. **An explicit choice** — a cookie the reader set by picking a language.
 * 2. **`Accept-Language`** — what their browser asks for.
 * 3. **The default.**
 *
 * An explicit choice outranks the header because a reader who has chosen a
 * language has told you something their browser configuration has not.
 */
export function resolveLocale<L extends Locale>(
  config: LocaleConfig<L>,
  input: {
    readonly cookie?: string | null
    readonly acceptLanguage?: string | null
  },
): L {
  if (isLocale(config, input.cookie)) return input.cookie
  return negotiateLocale(config, parseAcceptLanguage(input.acceptLanguage))
}

# @fiducial/i18n

**Localized by construction.**

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

Locale negotiation, message lookup, and locale-aware formatting of dates,
timezones, numbers, plurals and **money** — using nothing but the platform's own
`Intl`, so it runs unchanged on the server, in the browser, at the edge and
inside a Server Action.

It exists because of [`MISSION.md`](../../MISSION.md) principle **1c**:

> A user-visible string is a fact. It is declared once, and every locale is a
> derivation that must exist. A missing translation is a **missing artifact, not
> a fallback**.

## The bug this package is shaped around

Ring of Pursuit's translator was one line:

```ts
const raw = messages[key] ?? key   // missing translation → renders the key
```

Its catalogs were genuinely well kept — 1377 keys across `en` and `sr` with zero
key drift. And exactly one string still slipped through untranslated, invisibly:

```
home.organizer.benefit3.title = "Live dashboard"    ← identical in sr.json
```

Discipline caught 1376 of 1377. **Care is the wrong mechanism.**

**But the defect was the silence, not the fallback.** Showing a reader
default-locale text is good product behaviour; doing it without telling anyone is
how a string sits untranslated in production. So this package still falls back —
and reports every single time.

```ts
// Development: throws. This should be impossible; fid derive --check gates it.
const t = createTranslator({ locale: 'sr', messages })
t('nav.missing')  // ↯ no message for "nav.missing" in locale "sr"
                  //   Add it to messages/sr.json and run `fid derive`.

// Production: report instead of crashing a page over one string.
const t = createTranslator({
  locale: 'sr',
  messages,
  fallbackMessages: en,
  onMissing: (m) => telemetry.warn('i18n.missing', m),
})
```

## Money

Principle 1 again: *a physical fact is never a bare number; it carries its unit.*
An amount without a currency is the same class of bug as a length without one,
and it fails the same way — silently, until two of them are added together.

```ts
import { fromMajor, add, allocate, formatMoney } from '@fiducial/i18n'

const price = fromMajor(9.99, 'EUR')          // stored as 999 minor units

add(fromMajor(0.1, 'EUR'), fromMajor(0.2, 'EUR'))   // exactly 0.30
// 0.1 + 0.2 === 0.30000000000000004 — the bug this type prevents

add(price, fromMajor(5, 'RSD'))               // ↯ refuses: no exchange rate

allocate(fromMajor(10, 'EUR'), 3)             // [3.34, 3.33, 3.33] — sums to 10.00
// naive division gives 3.33 each and loses a cent per split

formatMoney(price, 'de-DE')                   // "9,99 €"
formatMoney(price, 'en-US')                   // "€9.99"
```

Three decisions worth knowing:

| Decision | Because |
|---|---|
| Integer **minor units**, never floats | Binary floating point cannot represent `0.10`, and the error compounds across a cart, a tax line and an invoice total |
| Currency is **independent of locale** | A Serbian reader may be invoiced in EUR. Inferring one from the other is wrong for every cross-border product |
| Cross-currency arithmetic **throws** | There is no correct answer without an exchange rate, and a rate is a fact with a timestamp that belongs to the caller |

`JPY` has no minor units and `KWD` has three — assuming 1/100 turns ¥1000 into
¥10.00, a factor-of-100 error that looks entirely plausible on a page.

## Plurals

**Serbian has three plural categories; English has two.**

```ts
pluralCategory(1,  'sr')   // 'one'    — 1, 21, 31…
pluralCategory(3,  'sr')   // 'few'    — 2–4, 22–24…
pluralCategory(9,  'sr')   // 'other'
```

A catalog carrying only singular and plural cannot be translated into Serbian
correctly, which is why plural selection belongs in the catalog format rather
than at the call site.

## Timezones

An instant plus a named zone — never a "local time".

```ts
const instant = new Date('2026-07-14T12:00:00Z')
zonedParts(instant, 'Europe/Belgrade').hour   // 14 (CEST)
zonedParts(instant, 'America/New_York').hour  //  8
```

`toDateTimeLocalValue` / `fromDateTimeLocalValue` convert between the absolute
instant your database holds and the wall clock an `<input type="datetime-local">`
shows. The inverse is solved by **offset probing**, not arithmetic: a zone's
offset depends on the instant you are asking about, which is circular across a
DST boundary. Asserted round-tripping through both EU transitions.

## Locale negotiation

```ts
resolveLocale(config, { cookie, acceptLanguage })
```

An explicit cookie choice outranks `Accept-Language`, because a reader who picked
a language told you something their browser configuration did not. Regional tags
match their primary subtag, so `en-GB` resolves to `en` rather than falling
through to the default. A malformed header degrades to the default instead of
throwing — it is network input.

## License

MIT

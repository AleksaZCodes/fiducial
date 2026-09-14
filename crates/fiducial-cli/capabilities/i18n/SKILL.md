# fiducial:i18n — Localization Skill

This product has the `i18n` capability. **It is localized by construction** —
`MISSION.md` principle 1c: a user-visible string is a fact, declared once, with
every locale a derivation that must exist.

## The loop

```
messages/<locale>.json        ← you edit these
  ↓  fid derive               (pipeline: i18n — executor fid-i18n)
src/generated/messages.ts     ← generated: MessageKey union + per-locale records
  ↓
fid derive --check            ← fails if any locale is missing a key
```

## Rules

**1 · Never put user-visible text in a component.** Add a key to *every*
`messages/<locale>.json`, then use it:

```tsx
<button>{t("action.save")}</button>
```

**2 · Keys are named, never the English text.** `action.save`, not
`"Save changes"`. English changes — "Save changes" becomes "Save" — and every
translation silently detaches from it.

**3 · A missing translation fails the build.** Not a warning, not a fallback to
the key. If you add a key to one locale, add it to all of them in the same edit.

**4 · Placeholders must match across locales.** `{count}` in one and not the
other means a reader sees a literal `{count}`, or loses the value. Word *order*
may differ freely — only the set is checked.

**5 · Never edit `src/generated/messages.ts`.** It is guard-blocked and
overwritten. Change the catalog and run `fid derive`.

## Plurals

**Serbian has three plural categories; English has two.**

```ts
pluralCategory(1, "sr")   // "one"    — 1, 21, 31…
pluralCategory(3, "sr")   // "few"    — 2–4, 22–24…
pluralCategory(9, "sr")   // "other"
```

A message that varies by count needs a form per category in the catalog. Do not
build one by concatenating at the call site — that cannot be translated.

## Money, dates, numbers

Use `@fiducial/i18n`, never hand-formatting:

```ts
formatMoney(fromMajor(9.99, "EUR"), locale)   // "9,99 €" in de, "€9.99" in en
formatDate(instant, locale, "Europe/Belgrade")
formatNumber(1234.56, locale)                  // "1,234.56" or "1.234,56"
```

**Money is a type, never a number.** It carries its currency, holds the amount
in integer minor units, and refuses cross-currency arithmetic. An amount without
a currency is the same class of bug as a length without a unit.

**Currency is independent of locale.** A Serbian reader may be invoiced in EUR.

## Hardcoded strings

`fid doctor` and `fid dash` report user-visible literals not going through
`t()`. **They warn rather than failing** — the detector cannot perfectly tell
copy from a CSS class or an aria role.

They are still worth clearing. **If you are an agent and you see these
reported, fix them as you go** rather than leaving them for someone else; that
is what makes localization second nature rather than a later refactor.

For a genuine exception — a brand name, a code sample — mark it:

```tsx
<span>{/* i18n-ignore */ "Wi-Fi"}</span>
```

## Adding a locale

1. `messages/<new>.json`, every key present
2. Add it to `[i18n] locales` in `fiducial.toml`
3. `fid derive`

The default locale in `[i18n] default` is a **decision**, not `locales[0]`.

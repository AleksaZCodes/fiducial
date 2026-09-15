# @fiducial/i18n

## 0.2.0

### Minor Changes

- 8f9b89a: New package: `@fiducial/i18n` — localized by construction.
  
  Locale negotiation, strict message lookup, and locale-aware formatting of dates,
  timezones, numbers, plurals and money, using only the platform's own `Intl` so it
  runs on the server, in the browser, at the edge and in a Server Action.
  
  Implements `MISSION.md` principle 1c: a user-visible string is a fact, declared
  once, with every locale a derivation that must exist.
  
  **Money** carries its currency as part of the value, holds the amount in integer
  minor units, refuses cross-currency arithmetic, and allocates without losing a
  minor unit.
- 8f9b89a: New package — localized by construction.
  
  Locale negotiation, strict message lookup, and locale-aware formatting of dates,
  timezones, numbers, plurals and money, using only the platform's own `Intl`.
  
  Implements `MISSION.md` principle 1c: a user-visible string is a fact, declared
  once, with every locale a derivation that must exist. A missing translation is
  reported rather than silently rendering the key — the defect it was designed
  against.
  
  `Money` carries its currency, holds the amount in integer minor units, refuses
  cross-currency arithmetic, and allocates without losing a minor unit.

---
"@fiducial/i18n": minor
---

New package — localized by construction.

Locale negotiation, strict message lookup, and locale-aware formatting of dates,
timezones, numbers, plurals and money, using only the platform's own `Intl`.

Implements `MISSION.md` principle 1c: a user-visible string is a fact, declared
once, with every locale a derivation that must exist. A missing translation is
reported rather than silently rendering the key — the defect it was designed
against.

`Money` carries its currency, holds the amount in integer minor units, refuses
cross-currency arithmetic, and allocates without losing a minor unit.

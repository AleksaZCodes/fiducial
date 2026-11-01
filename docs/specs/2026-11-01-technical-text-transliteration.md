# Technical tokens stay Latin in every script; only words transliterate

**Date:** 2026-11-01
**Status:** accepted
**Supersedes:** nothing

---

## Context

A distributed product built in Serbia ships in Cyrillic. A naive Latin →
Cyrillic transliteration of its documents turned:

- `24 V` into `24 В`;
- `RAM-a` into a word mixing two scripts;
- `Next.js` into nonsense.

Principle 1c makes every user-visible string a fact and every locale a
derivation. So the rule for which tokens survive transliteration belongs in
the derivation, in `@fiducial/i18n`, not in each product.

## Decision

When a locale's script differs from its source catalog's script, the
derivation transliterates **words** and keeps these **Latin**:

1. a unit symbol after a number (`24 V`, `20 000 lx`, `1000 ppm`, `3.3 V`);
2. unit symbols, chemical formulas and quantities (`CO₂`, `°C`, `%RH`);
3. identifiers, acronyms and protocol names (`MQTT`, `RS-485`, `I²C`, `SQLite`,
   `ESP32-C6`, `Next.js`);
4. pin names, topic strings, file paths and code (`GPIO18`,
   `site/roof/state`, `compose.yaml`);
5. a token that mixes letters and digits, or contains `.`, `/`, `_` or `-`
   between letters.

A Latin-script grammatical suffix attached to such a token by a hyphen
(`RAM-a`) keeps the token and transliterates the suffix (`RAM-а`).

`--check` fails when a derived string contains a word in both scripts, naming
the key and the word.

## Why not the alternatives

- **Mark each technical token by hand in the catalog.** It is a second
  declaration of what the rules above already decide, and it is the copy
  that is forgotten.
- **Do not transliterate; author both scripts.** Two catalogs drift. This is
  exactly what principle 1c's "every locale is a derivation" rules out.

## Consequences

- The product's 13 diagrams and its UI are one source with two renderings.
- A new unit or protocol name needs no product change: rules 1–5 decide by
  shape, not by a list.

## What this deliberately does not do

- It does not translate. Transliteration changes script, not language.
- It does not cover right-to-left scripts.

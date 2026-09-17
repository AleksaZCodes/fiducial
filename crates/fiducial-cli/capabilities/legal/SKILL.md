# fiducial:legal — Legal & Compliance Skill

This product has the `legal` capability. **One declaration, localized legal pages
out** — `MISSION.md` principle 1, applied to the facts a product's legal identity
is made of: jurisdiction, data protection contact, and cookie categories, layered
over the entity declared in `[brand]`.

## The loop

```
[legal] in fiducial.toml      ← you edit this
  ↓  fid derive                (pipeline: legal — executor fid-legal)
src/generated/legal.ts         ← generated: never hand-edit this
  ↓
fid derive --check            ← fails if it is missing or stale
```

## The declaration

```toml
[legal]
jurisdiction           = "EU"                      # EU | US | UK | other
data_protection_email  = "privacy@example.com"
cookie_categories      = ["necessary"]             # necessary always included
```

Installing this capability seeds `jurisdiction = "EU"`,
`data_protection_email = "privacy@example.com"`, and
`cookie_categories = ["necessary"]`. **Replace `data_protection_email`** with a
real address before deriving for real — it appears in every generated page.

Valid jurisdictions:

| Value | Pages generated |
|---|---|
| `EU` | privacy, terms, cookies, imprint, accessibility |
| `UK` | privacy, terms, cookies, imprint, accessibility |
| `US` | privacy, terms, cookies, accessibility |
| `other` | privacy, terms, cookies |

Valid cookie categories: `necessary` (always included), `analytics`, `marketing`,
`functional`.

## What each output is

| File | Is |
|---|---|
| `src/generated/legal.ts` | TypeScript file exporting `LegalPage` type and a typed `Record<LegalPage, { title: string; body: string }>` for each declared locale. Placeholder text has `{legal_name}`, `{domain}`, `{jurisdiction}`, and `{email}` substituted from the declarations. |

**Never hand-edit `src/generated/legal.ts`.** It is rewritten on every
`fid derive`. Change `[legal]` or `[brand]`, run `fid derive`.

## GDPR compliance is a legal state, not a code state

The generated file carries a checklist comment at the top naming every decision
a human must still make before this product's legal pages are compliant. No tool
grants compliance. Read the checklist on first generation.

## Rules

**1 · A legal fact is declared once, here.** The data protection email and
jurisdiction typed a second time anywhere else in the codebase is the same drift
`MISSION.md` principle 1 exists to forbid.

**2 · `legal` depends on `brand` and `i18n`.** The entity name, domain, and
contact details come from `[brand]`. The locale set comes from `[i18n]`. Both
must be configured before this pipeline runs.

**3 · The generated text is a starting template, not legal advice.** Substitute
every placeholder, then have a qualified legal professional review the resulting
pages for your jurisdiction before publishing.

## What this capability does not do yet

Consent records, cookie banners, data export flows, and account deletion
endpoints are all named in `ROADMAP.md §"Legal & compliance"` and are not derived
here. Each needs a database adapter and a migrations declaration — a shape no
existing capability has yet. Adding one is a new output in `pipelines/legal.toml`
and a new branch in `fid-legal`, not a redesign of the declaration.

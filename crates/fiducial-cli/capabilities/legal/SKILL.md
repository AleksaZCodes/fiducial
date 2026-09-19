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

## Routing the pages

**A generated catalog nobody can reach is not a legal page.** `fid derive`
produces the content; the product owns the routes, because only the product
knows its locale segments. The capability ships the two components that stand
between them:

| File | What it is |
|---|---|
| `src/components/legal-document.tsx` | Server component. Renders one `LegalPageContent`. Parses the small Markdown subset the generator emits. |
| `src/components/cookie-consent.tsx` | Client component. The banner, the stored decision, and `hasConsent()`. |

Wire one route per page per locale, from the catalog rather than from a list
typed by hand — a hardcoded array of page slugs is a second declaration of
`pages_for_jurisdiction`, and it goes stale the day the jurisdiction changes:

```tsx
export function generateStaticParams() {
  return Object.keys(legalCatalogs[locale]).map((page) => ({ page }));
}
```

Link every page from the footer, in every locale. A privacy policy reachable
only by typing its URL is one nobody has been given.

## Cookie consent

`cookie-consent.tsx` stores a decision and nothing else. **It does not stop
anything from loading.** The product reads `hasConsent("analytics")` before it
loads an analytics script, and that ordering is the entire compliance story: a
banner shown over a tag that has already fired is a banner that has documented
its own violation.

Three properties are load-bearing and should survive any restyle:

- **Reject is one click, at the same weight as accept.** Burying it behind a
  settings panel is the most commonly fined dark pattern in the EU.
- **Optional categories start off.** No pre-ticked boxes.
- **It is a `role="region"`, not a modal.** A focus trap over the page is a
  consent wall, and consent given to get past a wall is not freely given.

A product with `cookie_categories = ["necessary"]` renders no banner at all.
That is correct, not an omission: there is nothing to ask about, and a banner
that asks anyway trains people to dismiss the ones that matter.

## What this capability does not do yet

Consent *records* — a server-side log of who consented to what and when — data
export flows, and account deletion endpoints are named in
`ROADMAP.md §"Legal & compliance"` and are not derived here. Each needs a
database adapter and a migrations declaration, a shape no existing capability
has yet. Adding one is a new output in `pipelines/legal.toml` and a new branch
in `fid-legal`, not a redesign of the declaration.

The client-side banner above is deliberately not a substitute for that: it
records the visitor's choice on the visitor's own device, which is enough to
honour the choice and not enough to prove it was made.

# fiducial:legal — Legal Capability Skill

This product has the `legal` capability. **One declaration, five legal
pages** — jurisdiction, entity, contact, and cookie categories declared
once in `[legal]`; privacy policy, terms of service, cookie notice, imprint,
and accessibility statement derived from that declaration and localized
automatically through the i18n machinery.

## The loop

```
[legal] + [brand] in fiducial.toml   ← you edit this
  ↓  fid derive                       (pipeline: legal — executor fid-legal)
messages/<locale>.legal.json          ← generated message-catalog keys
  ↓  fid derive                       (pipeline: i18n — executor fid-i18n)
src/generated/messages.ts             ← typed keys, never hand-edit
```

## The declaration

```toml
[legal]
jurisdiction        = "EU"                    # required: IETF tag
dpo_email           = "dpo@example.com"       # optional; defaults to [brand] contact_email
cookie_categories   = ["necessary", "analytics"]
charges_users       = false                   # true → EU 14-day withdrawal right + refund policy
data_retention_days = 730                     # optional; omit for "as long as necessary"
generate_dpa        = false                   # true → Data Processing Agreement (B2B)
generate_aup        = false                   # true → Acceptable Use Policy
```

`[brand]` facts used: `legal_name`, `domain`, `contact_email`.
`[i18n]` facts used: all declared locales.

## Jurisdictions

| Tag | Rules applied |
|-----|---------------|
| `EU` | GDPR: DPO contact, lawful bases, data-subject rights, cookie consent |
| `US-CA` | CCPA: "Do Not Sell", opt-out links, California-specific rights |
| `RS` | Serbian DPA, DPO contact, 15-day response window |
| (other) | Generic: minimal privacy notice, no consent banner |

## Cookie categories

| Category | Banner label |
|----------|-------------|
| `necessary` | Always on — no consent required |
| `analytics` | Performance & analytics |
| `marketing` | Marketing & advertising |
| `preferences` | Preferences & personalization |

## Generated message keys

`fid-legal` writes one JSON file per locale into `messages/` (or the
directory `[i18n] messages_dir` names). Each file is a flat key→value map.
The keys are namespaced `legal.*` so they do not collide with product keys.

Keys generated (selection):

| Key | Contents |
|-----|----------|
| `legal.privacy.title` | Page title |
| `legal.privacy.intro` | Opening paragraph, names `legal_name` and `domain` |
| `legal.privacy.dpo_contact` | DPO contact section (EU/RS only) |
| `legal.terms.title` | Page title |
| `legal.terms.governing_law` | Jurisdiction paragraph |
| `legal.cookie.title` | Cookie notice page title |
| `legal.cookie.categories.*` | One entry per declared category |
| `legal.imprint.title` | Imprint page title (EU only) |
| `legal.a11y.title` | Accessibility statement title |

## Adding or changing a locale

Add the locale to `[i18n] locales` and re-run `fid derive`. The legal
pipeline writes the new locale's JSON file with the same keys, using the
declared jurisdiction's template. Translate the values before shipping.

## Dependencies

- `[brand]`: must be declared before `fid add legal`
- `[i18n]`: must be declared before `fid add legal`

`fid add legal` checks both and errors if either is missing.

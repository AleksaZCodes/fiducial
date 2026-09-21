# Legal & compliance capability — spec

**Date:** 2026-09-16  
**Status:** shipped

## Problem

Legal pages (privacy policy, terms of service, cookie notice, imprint, accessibility statement) carry facts that the product has already declared — entity name, domain, contact email. Hand-writing them means those facts live in two places: the declaration and the page. When the entity name changes, someone has to remember to update five pages.

GDPR compliance adds a second dimension: the pages are jurisdiction-specific. An EU product needs DPO contact, lawful bases, and data-subject rights; a CCPA product needs opt-out links; a generic product needs a minimal notice. Tracking jurisdiction as a separate configuration fact means those differences are computed, not maintained.

A third dimension: localization. Legal pages are long-form localized copy. Putting them beside the i18n machinery (as static files in a `legal/` directory) means they bypass `fid derive --check`'s staleness gate and bypass the typed `MessageKey` union. Putting them *through* the machinery means missing translations fail the build, not the reader.

## Decision

The `legal` capability derives `legal.*` message-catalog keys from `[legal]` + `[brand]` declarations, through the existing `fid-i18n` pipeline, rather than generating static pages directly.

**Why not static pages?**  
Static HTML or Markdown pages would have to be placed beside the Next.js/SvelteKit router, in a framework-specific location, and would bypass `fid derive --check`. The i18n path means one output location (`messages/<locale>.json`) that is already gated.

**Why not a separate `legal.json` file per locale?**  
The existing message-catalog format is already one flat key→value map per locale. Adding `legal.*` keys to that map means `fid-i18n` types them automatically — no new file format, no new import in the app.

**`fid-legal` is additive.**  
It patches locale files in place, never removing unrelated keys. A product that already has message catalogs is not broken by running `fid add legal`.

## The declaration

```toml
[legal]
jurisdiction      = "EU"            # required: IETF tag
dpo_email         = "dpo@…"         # optional: defaults to [brand] contact_email
cookie_categories = ["necessary", "analytics"]
```

Dependencies (enforced at derive time):

- `[brand]` must be declared — `legal_name`, `domain`, `contact_email` are named in the text.
- `[i18n]` must be declared — legal text is localized by construction.

## Jurisdictions

| Tag | Rules |
|-----|-------|
| `EU` / `EEA` | GDPR: DPO contact, lawful bases, data-subject rights, imprint |
| `US-CA` | CCPA: "Do Not Sell", opt-out, California-specific rights |
| `RS` | Serbian DPA, DPO contact, 15-day response window |
| (other) | Generic: minimal privacy notice, no consent banner |

## Generated keys

`fid-legal` writes the following `legal.*` keys to every locale's JSON catalog:

| Key | Notes |
|-----|-------|
| `legal.privacy.title` | |
| `legal.privacy.intro` | Names `legal_name` and `domain` |
| `legal.privacy.contact` | Names `dpo_contact` |
| `legal.privacy.lawful_basis` | EU/EEA only |
| `legal.privacy.rights` | EU/EEA and RS |
| `legal.privacy.dpo_contact` | EU/EEA and RS |
| `legal.privacy.ccpa_rights` | US-CA only |
| `legal.privacy.do_not_sell` | US-CA only |
| `legal.privacy.generic_notice` | All other jurisdictions |
| `legal.terms.title` | |
| `legal.terms.intro` | Names `domain` and `legal_name` |
| `legal.terms.governing_law` | Jurisdiction-specific |
| `legal.cookie.title` | |
| `legal.cookie.intro` | Names `domain` |
| `legal.cookie.categories.<cat>` | One entry per declared category |
| `legal.imprint.title` | EU/EEA/DE/AT/CH only |
| `legal.imprint.entity` | EU/EEA/DE/AT/CH only |
| `legal.a11y.title` | |
| `legal.a11y.intro` | Names `legal_name` and `domain` |
| `legal.a11y.contact` | Names `dpo_contact` |

## What this is not

- **Not legal advice.** The generated text is a starting point — a product shipping to real users should have a lawyer review it before launch.
- **Not a consent management platform.** The cookie banner is seeded as message-catalog keys; the actual JavaScript that renders it is the app's concern.
- **Not a GDPR compliance audit.** The pipeline derives text from declarations; whether those declarations are accurate is the product's responsibility.

## Files shipped

- `crates/fiducial-cli/capabilities/legal/capability.toml`
- `crates/fiducial-cli/capabilities/legal/SKILL.md`
- `crates/fiducial-cli/capabilities/legal/pipelines/legal.toml`
- `crates/fiducial-cli/src/config.rs` — `Legal` struct + `[legal]` field on `Config`
- `crates/fiducial-cli/src/commands/derive.rs` — `run_fid_legal`, `legal_keys`, `governing_law_text`
- `crates/fiducial-cli/src/commands/add.rs` — `AddTarget::Legal` variant
- `crates/fiducial-cli/tests/legal_pipeline.rs` — 10 end-to-end tests

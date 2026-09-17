# Legal & compliance capability

**Date:** 2026-09-17
**Status:** accepted

---

## Context

`ROADMAP.md §"Legal & compliance"` records the requirement: legal pages
(privacy, terms, cookies, imprint, accessibility) derived from a single
jurisdiction declaration, bilingual by construction, going through the i18n
machinery rather than beside it.

Both declared dependencies were already shipped: `i18n` (a missing translation
already fails `fid derive --check`) and `brand` (the declared entity this copy
is about). Nothing blocked this.

---

## Declaration

`[legal]` in `fiducial.toml` declares three facts:

```toml
[legal]
jurisdiction           = "EU"                      # EU | US | UK | other
data_protection_email  = "privacy@example.com"
cookie_categories      = ["necessary"]             # necessary always included
```

**Jurisdiction** determines which pages are generated:

| Jurisdiction | Pages |
|---|---|
| `EU` | privacy, terms, cookies, imprint, accessibility |
| `UK` | privacy, terms, cookies, imprint, accessibility |
| `US` | privacy, terms, cookies, accessibility |
| `other` | privacy, terms, cookies |

`imprint` is an EU/DE statutory disclosure requirement. It is absent from the
US set because the US has no direct equivalent.

**Cookie categories** — valid values: `necessary`, `analytics`, `marketing`,
`functional`. `necessary` is always present in the effective set regardless of
what the product lists.

---

## Output

A single TypeScript file — `src/generated/legal.ts` by default — that exports:

- `LegalPage` — a union type of page identifiers (`"privacy" | "terms" | ...`)
- `LegalPageContent` — interface `{ title: string; body: string }`
- `LegalCatalog` — `Record<LegalPage, LegalPageContent>`
- One `const` per locale (e.g. `const en: LegalCatalog = { ... }`)
- `legalCatalogs: Record<string, LegalCatalog>` — keyed by locale string
- `defaultLocale: string` — the declared `[i18n] default`

Placeholder text has `{legal_name}`, `{domain}`, `{jurisdiction}`, and
`{email}` substituted from the declarations. The text is a starting template;
a qualified legal professional must review it before publication.

---

## GDPR checklist comment

The generated file starts with a block comment listing every decision a human
must still make:

- Replace every placeholder in `[legal]` and `[brand]`
- Have a qualified legal professional review all pages
- Confirm the jurisdiction clause matches entity registration, not user location
- Verify `cookie_categories` lists every tracking purpose in use
- Record the date and reviewer in a legal log
- EU/UK: appoint a DPO if required; register with the supervisory authority; complete a DPIA for high-risk activities
- US: check state-specific requirements (CCPA, CPRA, VCDPA, etc.); add "Do Not Sell" link if required

**GDPR compliance is a legal state, not a code state. No tool grants it.**

---

## Dependencies

The `fid-legal` executor reads three declarations:

| Declaration | Needed for |
|---|---|
| `[legal]` | Jurisdiction, email, cookie categories |
| `[brand]` | Entity name, domain, contact |
| `[i18n]` | Locale list and default locale |

If any of the three is absent, `fid derive` fails with a named message
identifying which capability to install.

---

## Files added

| Path | Role |
|---|---|
| `crates/fiducial-cli/capabilities/legal/capability.toml` | Capability manifest — seeds `[legal]` block |
| `crates/fiducial-cli/capabilities/legal/pipelines/legal.toml` | Pipeline definition — `executor = "fid-legal"` |
| `crates/fiducial-cli/capabilities/legal/SKILL.md` | Agent instructions (copied to `.fiducial/skills/legal.md` on install) |
| `crates/fiducial-cli/src/legal.rs` | Pure rendering module — no I/O |
| `crates/fiducial-cli/tests/legal_pipeline.rs` | End-to-end integration tests |

## Files modified

| Path | Change |
|---|---|
| `crates/fiducial-cli/src/config.rs` | Added `Legal` struct with `validate()`, `effective_categories()`, `is_empty()` |
| `crates/fiducial-cli/src/config.rs` | Added `legal: Legal` field to `Config` |
| `crates/fiducial-cli/src/main.rs` | Added `mod legal;` |
| `crates/fiducial-cli/src/commands/derive.rs` | Added `run_fid_legal` function and `"fid-legal"` branch in executor dispatch |
| `crates/fiducial-cli/src/commands/add.rs` | Added `Legal` variant to `AddTarget` enum and dispatch |
| `ROADMAP.md` | Marked `### Legal & compliance` as `✅` |

---

## What this capability does not do yet

Consent records, cookie banners, data export flows, and account deletion
endpoints are noted in `ROADMAP.md §"Legal & compliance"`. Each requires a
`database` adapter and a migrations declaration — a shape no existing
capability has. This capability is the declaration layer; the transactional
layer is future work.

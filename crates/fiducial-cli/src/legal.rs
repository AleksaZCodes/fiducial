//! Pure rendering: `[legal]` + `[brand]` + `[i18n]` in, TypeScript out.
//!
//! The `fid-legal` executor in `commands/derive.rs` owns the I/O — reading
//! `fiducial.toml` and writing outputs. Everything here is a pure function of
//! the declared facts, so it is testable without a scaffolded product, the
//! same split `brand.rs` uses for its rendering.
//!
//! **GDPR compliance is a legal state, not a code state.** The generated file
//! carries a checklist comment at the top naming every decision a human must
//! still make. `render_legal_ts` does not grant compliance; it generates a
//! typed starting point that a lawyer must review.

use anyhow::Result;

use crate::config::{Brand, Legal};

/// Pages included per jurisdiction.
///
/// EU and UK include `imprint` — a legal disclosure requirement in Germany and
/// other EU/UK jurisdictions. US drops it (no equivalent requirement). `other`
/// produces the minimal set every product needs.
fn pages_for_jurisdiction(jurisdiction: &str) -> &'static [&'static str] {
    match jurisdiction {
        "EU" | "UK" => &["privacy", "terms", "cookies", "imprint", "accessibility"],
        "US" => &["privacy", "terms", "cookies", "accessibility"],
        _ => &["privacy", "terms", "cookies"],
    }
}

/// Substitute `{legal_name}`, `{domain}`, `{jurisdiction}`, and `{email}` in a
/// template string.
fn substitute(
    template: &str,
    legal_name: &str,
    domain: &str,
    jurisdiction: &str,
    email: &str,
) -> String {
    template
        .replace("{legal_name}", legal_name)
        .replace("{domain}", domain)
        .replace("{jurisdiction}", jurisdiction)
        .replace("{email}", email)
}

/// Title and body template for each page, before substitution.
fn page_content(page: &str, _jurisdiction: &str, categories: &[&str]) -> (&'static str, String) {
    let cookie_list = categories.join(", ");
    match page {
        "privacy" => (
            "Privacy Policy",
            format!(
                "This Privacy Policy describes how {{legal_name}} (\"we\", \"us\", or \"our\") \
                 collects, uses, and discloses information about you when you use our services \
                 at {{domain}}.\n\n\
                 **Data controller:** {{legal_name}}, reachable at {{email}}.\n\n\
                 **Jurisdiction:** {{jurisdiction}}.\n\n\
                 **What we collect:** We collect information you provide directly to us, \
                 information we collect automatically when you use our services, and information \
                 from third parties.\n\n\
                 **How we use it:** To provide, maintain, and improve our services; to send you \
                 technical notices; to respond to your comments and questions; and to comply with \
                 legal obligations.\n\n\
                 **Your rights:** Depending on your jurisdiction, you may have the right to \
                 access, correct, delete, or port your personal data. Contact us at {{email}} \
                 to exercise these rights.\n\n\
                 **Contact:** {{email}}"
            ),
        ),
        "terms" => (
            "Terms of Service",
            format!(
                "These Terms of Service govern your use of the services provided by \
                 {{legal_name}} at {{domain}}. By accessing or using our services, you agree \
                 to be bound by these terms.\n\n\
                 **Jurisdiction:** These terms are governed by the laws of {{jurisdiction}}.\n\n\
                 **Use of services:** You may use our services only as permitted by these terms \
                 and applicable law. You may not use our services to violate any law or \
                 regulation.\n\n\
                 **Limitation of liability:** To the extent permitted by law, {{legal_name}} \
                 shall not be liable for any indirect, incidental, special, consequential, or \
                 punitive damages.\n\n\
                 **Contact:** {{email}}"
            ),
        ),
        "cookies" => {
            (
                "Cookie Policy",
                format!(
                "{{legal_name}} uses cookies and similar technologies on {{domain}} to provide \
                 and improve our services.\n\n\
                 **Categories in use:** {cookie_list}.\n\n\
                 **Necessary cookies** are required for the site to function and cannot be \
                 disabled.\n\n\
                 {analytics}\
                 {marketing}\
                 {functional}\
                 **Managing cookies:** You can control cookies through your browser settings. \
                 Disabling non-necessary cookies may affect your experience.\n\n\
                 **Contact:** {{{{email}}}}"
            ,
                analytics = if categories.contains(&"analytics") {
                    "**Analytics cookies** help us understand how visitors interact with our \
                     site. We use this data to improve performance and user experience.\n\n"
                } else { "" },
                marketing = if categories.contains(&"marketing") {
                    "**Marketing cookies** are used to deliver relevant advertisements and \
                     track campaign effectiveness.\n\n"
                } else { "" },
                functional = if categories.contains(&"functional") {
                    "**Functional cookies** enable enhanced functionality and personalisation \
                     such as remembering your preferences.\n\n"
                } else { "" },
            ),
            )
        }
        "imprint" => (
            "Imprint",
            "Imprint (Impressum) — required disclosure under {{jurisdiction}} law.\n\n\
             **Responsible for this website:**\n\
             {{legal_name}}\n\
             {{domain}}\n\n\
             **Contact:** {{email}}\n\n\
             **Dispute resolution:** The European Commission provides an online dispute \
             resolution platform: https://ec.europa.eu/consumers/odr — we are not obliged \
             to participate but are willing to engage in out-of-court dispute settlement."
                .to_string(),
        ),
        "accessibility" => (
            "Accessibility Statement",
            "{{legal_name}} is committed to making {{domain}} accessible in accordance with \
             applicable law.\n\n\
             **Conformance status:** We aim for WCAG 2.1 Level AA conformance. Known \
             limitations are documented in our issue tracker and addressed on a rolling basis.\n\n\
             **Feedback:** If you experience barriers, contact us at {{email}} and we will \
             respond within five business days.\n\n\
             **Enforcement procedure ({{jurisdiction}}):** If you are not satisfied with \
             our response, you may contact the relevant national enforcement body for your \
             jurisdiction."
                .to_string(),
        ),
        _ => ("", String::new()),
    }
}

/// Generate `src/generated/legal.ts` from the declarations.
///
/// `locales` is the full list from `[i18n]`; `default_locale` is the fallback.
/// The generated file exports a `LegalPage` union type and, for each locale,
/// a `Record<LegalPage, { title: string; body: string }>` with placeholder
/// values substituted from `legal` and `brand`.
pub fn render_legal_ts(
    legal: &Legal,
    brand: &Brand,
    locales: &[String],
    default_locale: &str,
) -> Result<String> {
    let pages = pages_for_jurisdiction(&legal.jurisdiction);
    let categories = legal.effective_categories();

    let page_union: Vec<String> = pages.iter().map(|p| format!("\"{p}\"")).collect();

    let mut out = String::new();

    // ── Header / GDPR checklist ────────────────────────────────────────────
    out.push_str("// generated by `fid derive` — do not edit\n");
    out.push_str("// source of truth: [legal], [brand], and [i18n] in fiducial.toml\n");
    out.push_str("//\n");
    out.push_str("// ┌─────────────────────────────────────────────────────────────────────┐\n");
    out.push_str("// │  GDPR COMPLIANCE CHECKLIST — decisions a human must make            │\n");
    out.push_str("// │  GDPR compliance is a legal state, not a code state.                │\n");
    out.push_str("// │  No tool grants it. A qualified legal professional must review       │\n");
    out.push_str("// │  these pages before they are published.                              │\n");
    out.push_str("// │                                                                      │\n");
    out.push_str("// │  [ ] Replace every placeholder in [legal] and [brand] with real     │\n");
    out.push_str("// │      values (data_protection_email, legal_name, domain, etc.).       │\n");
    out.push_str("// │  [ ] Have a qualified legal professional review all generated pages. │\n");
    out.push_str("// │  [ ] Confirm the jurisdiction clause matches where your entity is    │\n");
    out.push_str("// │      registered — not just where your users are.                    │\n");
    out.push_str("// │  [ ] Verify cookie_categories lists every tracking purpose in use;  │\n");
    out.push_str("// │      omitting one is a legal, not a technical, error.               │\n");
    out.push_str("// │  [ ] Record the date and reviewer for each page in your legal log.  │\n");
    out.push_str("// │  [ ] Re-review whenever jurisdiction, entity, or cookie categories  │\n");
    out.push_str("// │      change — a new `fid derive` regenerates the template, but      │\n");
    out.push_str("// │      the human review must follow.                                  │\n");
    if legal.jurisdiction == "EU" || legal.jurisdiction == "UK" {
        out.push_str("// │  [ ] Appoint a Data Protection Officer if required (Art. 37 GDPR). │\n");
        out.push_str(
            "// │  [ ] Register with the supervisory authority if required.           │\n",
        );
        out.push_str(
            "// │  [ ] Complete a DPIA for high-risk processing activities.           │\n",
        );
    }
    if legal.jurisdiction == "US" {
        out.push_str("// │  [ ] Check state-specific requirements (CCPA, CPRA, VCDPA, etc.).  │\n");
        out.push_str(
            "// │  [ ] Add a \"Do Not Sell or Share My Personal Information\" link     │\n",
        );
        out.push_str("// │      if subject to CCPA/CPRA.                                      │\n");
    }
    out.push_str("// └─────────────────────────────────────────────────────────────────────┘\n");
    out.push('\n');

    // ── LegalPage type ─────────────────────────────────────────────────────
    out.push_str(&format!(
        "export type LegalPage = {};\n\n",
        page_union.join(" | ")
    ));

    // ── Per-locale content ─────────────────────────────────────────────────
    out.push_str("export interface LegalPageContent {\n");
    out.push_str("  title: string;\n");
    out.push_str("  body: string;\n");
    out.push_str("}\n\n");

    out.push_str("export type LegalCatalog = Record<LegalPage, LegalPageContent>;\n\n");

    // For each locale emit a constant. All locales share the same source text
    // (the template language is English) — a product that needs translated legal
    // copy should override the generated catalog through the i18n pipeline.
    for locale in locales {
        let const_name = locale.replace('-', "_");
        out.push_str(&format!("export const {const_name}: LegalCatalog = {{\n"));
        for page in pages {
            let (title_template, body_template) =
                page_content(page, &legal.jurisdiction, &categories);
            let title = substitute(
                title_template,
                &brand.legal_name,
                &brand.domain,
                &legal.jurisdiction,
                &legal.data_protection_email,
            );
            let body = substitute(
                &body_template,
                &brand.legal_name,
                &brand.domain,
                &legal.jurisdiction,
                &legal.data_protection_email,
            );
            // Escape backticks and `${` for template literals.
            let body_escaped = body.replace('`', "\\`").replace("${", "\\${");
            out.push_str(&format!("  {page}: {{\n"));
            out.push_str(&format!("    title: \"{title}\",\n"));
            out.push_str(&format!("    body: `{body_escaped}`,\n"));
            out.push_str("  },\n");
        }
        out.push_str("};\n\n");
    }

    // ── Default export keyed by locale ─────────────────────────────────────
    out.push_str("export const legalCatalogs: Record<string, LegalCatalog> = {\n");
    for locale in locales {
        let const_name = locale.replace('-', "_");
        out.push_str(&format!("  \"{locale}\": {const_name},\n"));
    }
    out.push_str("};\n\n");

    out.push_str(&format!(
        "export const defaultLocale = \"{default_locale}\";\n"
    ));

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_brand() -> Brand {
        Brand {
            legal_name: "Example LLC".into(),
            trading_name: "Example".into(),
            domain: "example.com".into(),
            contact_email: "hello@example.com".into(),
            primary_color: String::new(),
            background_color: String::new(),
        }
    }

    fn test_legal_eu() -> Legal {
        Legal {
            jurisdiction: "EU".into(),
            data_protection_email: "privacy@example.com".into(),
            cookie_categories: vec!["necessary".into(), "analytics".into()],
        }
    }

    #[test]
    fn eu_pages_include_imprint() {
        assert!(pages_for_jurisdiction("EU").contains(&"imprint"));
        assert!(pages_for_jurisdiction("UK").contains(&"imprint"));
    }

    #[test]
    fn us_pages_omit_imprint() {
        assert!(!pages_for_jurisdiction("US").contains(&"imprint"));
    }

    #[test]
    fn other_jurisdiction_has_minimal_set() {
        let pages = pages_for_jurisdiction("other");
        assert!(pages.contains(&"privacy"));
        assert!(pages.contains(&"terms"));
        assert!(pages.contains(&"cookies"));
        assert!(!pages.contains(&"imprint"));
        assert!(!pages.contains(&"accessibility"));
    }

    #[test]
    fn necessary_is_always_in_effective_categories() {
        let legal = Legal {
            jurisdiction: "EU".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["analytics".into()],
        };
        assert!(legal.effective_categories().contains(&"necessary"));
    }

    #[test]
    fn render_produces_valid_exports() {
        let legal = test_legal_eu();
        let brand = test_brand();
        let locales = vec!["en".into(), "de".into()];
        let ts = render_legal_ts(&legal, &brand, &locales, "en").unwrap();

        assert!(ts.contains("export type LegalPage ="), "{ts}");
        assert!(ts.contains("\"privacy\""), "{ts}");
        assert!(ts.contains("\"imprint\""), "{ts}");
        assert!(ts.contains("export const en: LegalCatalog"), "{ts}");
        assert!(ts.contains("export const de: LegalCatalog"), "{ts}");
        assert!(ts.contains("Example LLC"), "{ts}");
        assert!(ts.contains("example.com"), "{ts}");
        assert!(ts.contains("privacy@example.com"), "{ts}");
        assert!(ts.contains("GDPR COMPLIANCE CHECKLIST"), "{ts}");
    }

    #[test]
    fn us_render_does_not_include_imprint() {
        let legal = Legal {
            jurisdiction: "US".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["necessary".into()],
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["en".into()], "en").unwrap();
        assert!(!ts.contains("\"imprint\""), "{ts}");
        assert!(ts.contains("CCPA"), "{ts}");
    }

    #[test]
    fn substitute_replaces_all_placeholders() {
        let result = substitute(
            "Hello {legal_name} at {domain} ({jurisdiction}) — {email}",
            "Acme LLC",
            "acme.dev",
            "EU",
            "dpo@acme.dev",
        );
        assert_eq!(result, "Hello Acme LLC at acme.dev (EU) — dpo@acme.dev");
    }
}

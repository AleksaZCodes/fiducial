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
/// other EU/UK jurisdictions. US drops it (no equivalent requirement). RS
/// (Serbia) has no Impressum equivalent, so it takes the minimal set plus an
/// accessibility statement, which Serbian public-sector procurement commonly
/// asks for. `other` produces the minimal set every product needs.
fn pages_for_jurisdiction(jurisdiction: &str) -> &'static [&'static str] {
    match jurisdiction {
        "EU" | "UK" => &["privacy", "terms", "cookies", "imprint", "accessibility"],
        "US" => &["privacy", "terms", "cookies", "accessibility"],
        "RS" => &["privacy", "terms", "cookies", "accessibility"],
        _ => &["privacy", "terms", "cookies"],
    }
}

/// Languages this pipeline can render legal copy in.
///
/// A locale outside this list still gets a catalog entry — omitting one would
/// break the `Record<Locale, …>` contract — but the entry is marked untranslated
/// rather than silently served as English. See `render_legal_ts`.
pub const TRANSLATED_LANGS: &[&str] = &["en", "sr"];

/// `sr-Latn-RS` → `sr`. Legal copy varies by language, not by region.
fn lang_of(locale: &str) -> &str {
    locale.split(['-', '_']).next().unwrap_or(locale)
}

/// True when legal copy exists in this locale's language.
pub fn is_translated(locale: &str) -> bool {
    TRANSLATED_LANGS.contains(&lang_of(locale))
}

/// Human-readable jurisdiction name, per language.
///
/// The enum token is never interpolated into prose: "Jurisdiction: other" is
/// not a sentence, and "governed by the laws of other" is worse. An undeclared
/// jurisdiction renders as a bracketed placeholder so that it is visibly unfit
/// to publish rather than plausibly wrong.
fn jurisdiction_name(jurisdiction: &str, lang: &str) -> &'static str {
    match (jurisdiction, lang) {
        ("EU", "sr") => "Evropska unija",
        ("EU", _) => "the European Union",
        ("UK", "sr") => "Ujedinjeno Kraljevstvo",
        ("UK", _) => "the United Kingdom",
        ("US", "sr") => "Sjedinjene Američke Države",
        ("US", _) => "the United States",
        ("RS", "sr") => "Srbija",
        ("RS", _) => "Serbia",
        (_, "sr") => "[jurisdikcija nije navedena]",
        _ => "[jurisdiction not declared]",
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

/// Title and body template for each page, in the given language, before
/// substitution.
///
/// Every arm returns owned strings so that translated and formatted variants
/// can share one signature.
fn page_content(page: &str, lang: &str, categories: &[&str]) -> (String, String) {
    let cookie_list = categories.join(", ");
    match lang {
        "sr" => page_content_sr(page, categories, &cookie_list),
        _ => page_content_en(page, categories, &cookie_list),
    }
}

fn page_content_en(page: &str, categories: &[&str], cookie_list: &str) -> (String, String) {
    match page {
        "privacy" => (
            "Privacy Policy".to_string(),
            "This Privacy Policy describes how {legal_name} (\"we\", \"us\", or \"our\") \
             collects, uses, and discloses information about you when you use our services \
             at {domain}.\n\n\
             **Data controller:** {legal_name}, reachable at {email}.\n\n\
             **Jurisdiction:** {jurisdiction}.\n\n\
             **What we collect:** We collect information you provide directly to us, \
             information we collect automatically when you use our services, and information \
             from third parties.\n\n\
             **How we use it:** To provide, maintain, and improve our services; to send you \
             technical notices; to respond to your comments and questions; and to comply with \
             legal obligations.\n\n\
             **Your rights:** Depending on your jurisdiction, you may have the right to \
             access, correct, delete, or port your personal data. Contact us at {email} \
             to exercise these rights.\n\n\
             **Contact:** {email}"
                .to_string(),
        ),
        "terms" => (
            "Terms of Service".to_string(),
            "These Terms of Service govern your use of the services provided by \
             {legal_name} at {domain}. By accessing or using our services, you agree \
             to be bound by these terms.\n\n\
             **Jurisdiction:** These terms are governed by the laws of {jurisdiction}.\n\n\
             **Use of services:** You may use our services only as permitted by these terms \
             and applicable law. You may not use our services to violate any law or \
             regulation.\n\n\
             **Limitation of liability:** To the extent permitted by law, {legal_name} \
             shall not be liable for any indirect, incidental, special, consequential, or \
             punitive damages.\n\n\
             **Contact:** {email}"
                .to_string(),
        ),
        "cookies" => {
            (
                "Cookie Policy".to_string(),
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
                 **Contact:** {{email}}",
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
            "Imprint".to_string(),
            "Imprint (Impressum) — required disclosure under {jurisdiction} law.\n\n\
             **Responsible for this website:**\n\
             {legal_name}\n\
             {domain}\n\n\
             **Contact:** {email}\n\n\
             **Dispute resolution:** The European Commission provides an online dispute \
             resolution platform: https://ec.europa.eu/consumers/odr — we are not obliged \
             to participate but are willing to engage in out-of-court dispute settlement."
                .to_string(),
        ),
        "accessibility" => (
            "Accessibility Statement".to_string(),
            "{legal_name} is committed to making {domain} accessible in accordance with \
             applicable law.\n\n\
             **Conformance status:** We aim for WCAG 2.1 Level AA conformance. Known \
             limitations are documented in our issue tracker and addressed on a rolling basis.\n\n\
             **Feedback:** If you experience barriers, contact us at {email} and we will \
             respond within five business days.\n\n\
             **Enforcement procedure ({jurisdiction}):** If you are not satisfied with \
             our response, you may contact the relevant national enforcement body for your \
             jurisdiction."
                .to_string(),
        ),
        _ => (String::new(), String::new()),
    }
}

fn page_content_sr(page: &str, categories: &[&str], cookie_list: &str) -> (String, String) {
    match page {
        "privacy" => (
            "Politika privatnosti".to_string(),
            "Ova Politika privatnosti opisuje kako {legal_name} („mi“, „nas“ ili „naš“) \
             prikuplja, koristi i saopštava podatke o vama kada koristite naše usluge \
             na {domain}.\n\n\
             **Rukovalac podacima:** {legal_name}, dostupan na {email}.\n\n\
             **Jurisdikcija:** {jurisdiction}.\n\n\
             **Šta prikupljamo:** Prikupljamo podatke koje nam dostavite neposredno, \
             podatke koje prikupljamo automatski kada koristite naše usluge, kao i \
             podatke od trećih lica.\n\n\
             **Kako ih koristimo:** Da bismo pružali, održavali i unapređivali usluge; \
             slali tehnička obaveštenja; odgovarali na vaša pitanja i primedbe; i \
             ispunjavali zakonske obaveze.\n\n\
             **Vaša prava:** U zavisnosti od jurisdikcije, možete imati pravo na pristup, \
             ispravku, brisanje ili prenosivost svojih podataka o ličnosti. Obratite nam \
             se na {email} radi ostvarivanja tih prava.\n\n\
             **Kontakt:** {email}"
                .to_string(),
        ),
        "terms" => (
            "Uslovi korišćenja".to_string(),
            "Ovi Uslovi korišćenja uređuju vaše korišćenje usluga koje pruža \
             {legal_name} na {domain}. Pristupanjem uslugama ili njihovim korišćenjem \
             prihvatate da budete obavezani ovim uslovima.\n\n\
             **Merodavno pravo:** Na ove uslove primenjuje se pravo koje važi u \
             {jurisdiction}.\n\n\
             **Korišćenje usluga:** Usluge smete koristiti samo na način dozvoljen ovim \
             uslovima i važećim propisima. Usluge ne smete koristiti radi kršenja bilo \
             kog zakona ili propisa.\n\n\
             **Ograničenje odgovornosti:** U meri u kojoj je to dozvoljeno zakonom, \
             {legal_name} ne odgovara za posrednu, slučajnu, posebnu, posledičnu ili \
             kaznenu štetu.\n\n\
             **Kontakt:** {email}"
                .to_string(),
        ),
        "cookies" => (
            "Politika kolačića".to_string(),
            format!(
                "{{legal_name}} koristi kolačiće i slične tehnologije na {{domain}} radi \
                 pružanja i unapređenja usluga.\n\n\
                 **Kategorije u upotrebi:** {cookie_list}.\n\n\
                 **Neophodni kolačići** potrebni su za rad sajta i ne mogu se isključiti.\n\n\
                 {analytics}\
                 {marketing}\
                 {functional}\
                 **Upravljanje kolačićima:** Kolačiće možete kontrolisati kroz podešavanja \
                 pregledača. Isključivanje kolačića koji nisu neophodni može uticati na \
                 vaše korisničko iskustvo.\n\n\
                 **Kontakt:** {{email}}",
                analytics = if categories.contains(&"analytics") {
                    "**Analitički kolačići** pomažu nam da razumemo kako posetioci koriste \
                     sajt. Te podatke koristimo za poboljšanje performansi i iskustva.\n\n"
                } else {
                    ""
                },
                marketing = if categories.contains(&"marketing") {
                    "**Marketinški kolačići** koriste se za prikazivanje relevantnih oglasa \
                     i praćenje uspešnosti kampanja.\n\n"
                } else {
                    ""
                },
                functional = if categories.contains(&"functional") {
                    "**Funkcionalni kolačići** omogućavaju proširene funkcije i \
                     personalizaciju, poput pamćenja vaših podešavanja.\n\n"
                } else {
                    ""
                },
            ),
        ),
        "imprint" => (
            "Impresum".to_string(),
            "Impresum — obavezno obaveštenje prema propisima koji važe u {jurisdiction}.\n\n\
             **Odgovorni za ovaj sajt:**\n\
             {legal_name}\n\
             {domain}\n\n\
             **Kontakt:** {email}\n\n\
             **Rešavanje sporova:** Evropska komisija obezbeđuje platformu za onlajn \
             rešavanje sporova: https://ec.europa.eu/consumers/odr — nismo obavezni da \
             u njoj učestvujemo, ali smo spremni na vansudsko rešavanje sporova."
                .to_string(),
        ),
        "accessibility" => (
            "Izjava o pristupačnosti".to_string(),
            "{legal_name} nastoji da {domain} učini pristupačnim u skladu sa važećim \
             propisima.\n\n\
             **Status usklađenosti:** Ciljamo usklađenost sa WCAG 2.1 nivo AA. Poznata \
             ograničenja vodimo u sistemu za praćenje problema i otklanjamo ih \
             kontinuirano.\n\n\
             **Povratne informacije:** Ako naiđete na prepreke, obratite nam se na {email} \
             i odgovorićemo u roku od pet radnih dana.\n\n\
             **Postupak zaštite ({jurisdiction}):** Ako niste zadovoljni našim odgovorom, \
             možete se obratiti nadležnom državnom organu za vašu jurisdikciju."
                .to_string(),
        ),
        _ => (String::new(), String::new()),
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

    // One constant per locale, rendered in that locale's language. A locale whose
    // language has no templates still gets an entry — omitting one would break the
    // Record<Locale, …> contract downstream — but it is emitted in the default
    // locale's language under a loud marker, never silently as English. Principle
    // 1c: a missing translation is a missing artifact, not a fallback.
    for locale in locales {
        let const_name = locale.replace('-', "_");
        let lang = lang_of(locale);
        let translated = is_translated(locale);
        if !translated {
            out.push_str(&format!(
                "// UNTRANSLATED — no legal templates exist for \"{lang}\".\n\
                 // The text below is {fallback_lang}. It MUST NOT be published to a reader of\n\
                 // \"{lang}\". Supply a translation before routing this locale's pages.\n",
                fallback_lang = lang_of(default_locale),
            ));
        }
        let render_lang = if translated {
            lang
        } else {
            lang_of(default_locale)
        };
        out.push_str(&format!("export const {const_name}: LegalCatalog = {{\n"));
        for page in pages {
            let (title_template, body_template) = page_content(page, render_lang, &categories);
            let jurisdiction = jurisdiction_name(&legal.jurisdiction, render_lang);
            let title = substitute(
                &title_template,
                &brand.legal_name,
                &brand.domain,
                jurisdiction,
                &legal.data_protection_email,
            );
            let body = substitute(
                &body_template,
                &brand.legal_name,
                &brand.domain,
                jurisdiction,
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

    // ── Serbian as a first-class locale ────────────────────────────────────

    #[test]
    fn serbian_renders_in_serbian_not_english() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "en".into()], "sr").unwrap();

        // The Serbian catalog carries Serbian titles, the English one English.
        assert!(ts.contains("Politika privatnosti"), "{ts}");
        assert!(ts.contains("Uslovi korišćenja"), "{ts}");
        assert!(ts.contains("Privacy Policy"), "{ts}");

        // The regression this guards: sr and en must not be the same bytes.
        let sr = ts.split("export const sr").nth(1).unwrap();
        let sr_body = sr.split("export const").next().unwrap();
        assert!(
            !sr_body.contains("This Privacy Policy describes"),
            "sr catalog must not contain the English template: {sr_body}"
        );
    }

    #[test]
    fn jurisdiction_renders_as_a_name_never_the_enum_token() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "en".into()], "sr").unwrap();
        assert!(ts.contains("Srbija"), "{ts}");
        assert!(ts.contains("Serbia"), "{ts}");
        // "Jurisdiction: RS." is not a sentence a reader should ever see.
        assert!(!ts.contains("**Jurisdiction:** RS"), "{ts}");
        assert!(!ts.contains("**Jurisdikcija:** RS"), "{ts}");
    }

    #[test]
    fn undeclared_jurisdiction_is_visibly_unfit_to_publish() {
        let legal = Legal {
            jurisdiction: "other".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["necessary".into()],
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["en".into()], "en").unwrap();
        // Never the bare token: "governed by the laws of other" is worse than a gap.
        assert!(!ts.contains("laws of other"), "{ts}");
        assert!(ts.contains("[jurisdiction not declared]"), "{ts}");
    }

    #[test]
    fn rs_includes_accessibility_but_not_imprint() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into()], "sr").unwrap();
        assert!(ts.contains("\"accessibility\""), "{ts}");
        assert!(!ts.contains("\"imprint\""), "Serbia has no Impressum: {ts}");
        assert!(ts.contains("Izjava o pristupačnosti"), "{ts}");
    }

    #[test]
    fn an_untranslated_locale_is_marked_never_silently_english() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
        };
        // German has no templates.
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "de".into()], "sr").unwrap();
        assert!(ts.contains("export const de"), "de must still exist: {ts}");
        assert!(ts.contains("UNTRANSLATED"), "de must be marked: {ts}");
        assert!(ts.contains("MUST NOT be published"), "{ts}");
    }

    #[test]
    fn region_tagged_locales_resolve_to_their_language() {
        assert_eq!(lang_of("sr-Latn-RS"), "sr");
        assert_eq!(lang_of("en_US"), "en");
        assert_eq!(lang_of("sr"), "sr");
        assert!(is_translated("sr-Latn-RS"));
        assert!(!is_translated("de-AT"));
    }

    #[test]
    fn imprint_and_accessibility_have_no_stray_braces() {
        // Regression: these two templates used `{{name}}` inside a plain
        // .to_string(), where the doubled braces are literal, not escapes —
        // so substitution left `{Acme LLC}` in the rendered page.
        let legal = Legal {
            jurisdiction: "EU".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["necessary".into()],
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["en".into()], "en").unwrap();
        assert!(
            !ts.contains("{Acme"),
            "stray braces around substitution: {ts}"
        );
        assert!(!ts.contains("{{"), "unsubstituted doubled braces: {ts}");
    }
}

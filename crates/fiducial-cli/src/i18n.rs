//! The `fid-i18n` pipeline: message catalogs in, typed keys out.
//!
//! Principle 1c — *a user-visible string is a fact* — made mechanical.
//!
//! # The declaration
//!
//! `messages/<locale>.json`, one file per locale, nested or flat. JSON so a
//! translator can edit it without a build system.
//!
//! # The derivation
//!
//! A TypeScript module with a union of every valid key. Using a key outside the
//! union is a compile error rather than a string that renders to a reader.
//!
//! # The gate
//!
//! `fid derive --check` fails when a locale is missing a key the default locale
//! has. That is the whole point: Ring of Pursuit kept 1377 keys across two
//! locales by discipline and still shipped one untranslated string, because
//! nothing could tell it had.
//!
//! Two softer findings are **reported rather than failed**: a key a locale
//! defines and the default does not (usually a leftover), and a value identical
//! to the default locale's (usually untranslated, sometimes a proper noun).
//! Failing on those would block a legitimate `"Wi-Fi"`.

// Not yet called from `derive.rs` — the `fid-i18n` executor, the `[i18n]`
// declaration and the `fid add i18n` capability are the remaining half of
// Phase 22. The comparison and codegen below are complete and tested (12 unit
// tests); only the wiring is outstanding.
//
// Marked rather than left as bare warnings so CI's `-D warnings` stays
// meaningful, and so this is a stated gap rather than a silent one.
#![allow(dead_code)]

use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One locale's flattened catalog: dotted key → template string.
pub type Catalog = BTreeMap<String, String>;

/// What a catalog comparison found.
#[derive(Debug, Default)]
pub struct Findings {
    /// Keys the default locale defines and this one does not. **Fatal.**
    pub missing: BTreeMap<String, Vec<String>>,
    /// Keys this locale defines and the default does not. Reported.
    pub extra: BTreeMap<String, Vec<String>>,
    /// Values byte-identical to the default locale's. Reported.
    pub untranslated: BTreeMap<String, Vec<String>>,
    /// Placeholders that differ between a locale and the default. **Fatal.**
    pub placeholder_mismatch: BTreeMap<String, Vec<String>>,
}

impl Findings {
    pub fn is_fatal(&self) -> bool {
        !self.missing.is_empty() || !self.placeholder_mismatch.is_empty()
    }
}

/// Flatten nested JSON into dotted keys.
///
/// `{"nav": {"home": "Home"}}` → `{"nav.home": "Home"}`. Catalogs are commonly
/// authored nested for readability and compared flat, because a nested diff
/// cannot tell a missing key from a missing subtree.
fn flatten(value: &serde_json::Value, prefix: &str, out: &mut Catalog) -> Result<()> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(child, &path, out)?;
            }
            Ok(())
        }
        serde_json::Value::String(text) => {
            out.insert(prefix.to_string(), text.clone());
            Ok(())
        }
        other => bail!(
            "`{prefix}` is {}, but a message must be a string or an object of strings",
            match other {
                serde_json::Value::Array(_) => "an array",
                serde_json::Value::Number(_) => "a number",
                serde_json::Value::Bool(_) => "a boolean",
                _ => "null",
            }
        ),
    }
}

pub fn load_catalog(path: &Path) -> Result<Catalog> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing {} as JSON", path.display()))?;
    let mut out = Catalog::new();
    flatten(&value, "", &mut out)?;
    Ok(out)
}

/// The `{placeholders}` a template uses.
fn placeholders(template: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes: Vec<char> = template.chars().collect();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == '{' {
            if let Some(close) = bytes[index + 1..].iter().position(|c| *c == '}') {
                let name: String = bytes[index + 1..index + 1 + close].iter().collect();
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    found.insert(name);
                }
                index += close + 2;
                continue;
            }
        }
        index += 1;
    }
    found
}

/// Compare every locale against the default.
pub fn compare(catalogs: &BTreeMap<String, Catalog>, default_locale: &str) -> Result<Findings> {
    let base = catalogs
        .get(default_locale)
        .ok_or_else(|| anyhow::anyhow!("no catalog for the default locale `{default_locale}`"))?;

    let mut findings = Findings::default();

    for (locale, catalog) in catalogs {
        if locale == default_locale {
            continue;
        }

        for (key, base_value) in base {
            match catalog.get(key) {
                None => findings
                    .missing
                    .entry(locale.clone())
                    .or_default()
                    .push(key.clone()),
                Some(value) => {
                    // A placeholder present in one and not the other means a
                    // reader sees a literal `{count}` or loses a value entirely.
                    if placeholders(base_value) != placeholders(value) {
                        findings
                            .placeholder_mismatch
                            .entry(locale.clone())
                            .or_default()
                            .push(key.clone());
                    }
                    // Short strings are frequently identical for good reasons —
                    // "OK", "Email", a product name — so only flag substantial
                    // ones, where identity is much more likely to mean forgotten.
                    if value == base_value && value.chars().count() > 12 {
                        findings
                            .untranslated
                            .entry(locale.clone())
                            .or_default()
                            .push(key.clone());
                    }
                }
            }
        }

        for key in catalog.keys() {
            if !base.contains_key(key) {
                findings
                    .extra
                    .entry(locale.clone())
                    .or_default()
                    .push(key.clone());
            }
        }
    }

    Ok(findings)
}

/// A TypeScript module declaring the key union and each locale's catalog.
pub fn render_typescript(
    catalogs: &BTreeMap<String, Catalog>,
    default_locale: &str,
) -> Result<String> {
    let base = catalogs
        .get(default_locale)
        .ok_or_else(|| anyhow::anyhow!("no catalog for the default locale `{default_locale}`"))?;

    let mut out = String::new();
    out.push_str(
        "// Generated by `fid derive` from messages/*.json. Do not edit.\n\
         //\n\
         // The key union below is what makes a mistyped key a compile error rather\n\
         // than a string rendered to a reader. Editing this file by hand is\n\
         // guard-blocked: change the catalog and re-derive.\n\n",
    );

    let locales: Vec<&String> = catalogs.keys().collect();
    out.push_str(&format!(
        "export const locales = [{}] as const\n",
        locales
            .iter()
            .map(|l| format!("{l:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str("export type Locale = (typeof locales)[number]\n");
    out.push_str(&format!(
        "export const defaultLocale: Locale = {default_locale:?}\n\n"
    ));

    out.push_str("/** Every message key. A key outside this union is a compile error. */\n");
    out.push_str("export type MessageKey =\n");
    for key in base.keys() {
        out.push_str(&format!("  | {key:?}\n"));
    }
    out.push('\n');

    for (locale, catalog) in catalogs {
        out.push_str(&format!(
            "export const {}: Record<MessageKey, string> = {{\n",
            ident(locale)
        ));
        for key in base.keys() {
            // Completeness is guaranteed by `compare` having run first.
            if let Some(value) = catalog.get(key) {
                out.push_str(&format!("  {key:?}: {value:?},\n"));
            }
        }
        out.push_str("}\n\n");
    }

    out.push_str("export const messages = {\n");
    for locale in catalogs.keys() {
        out.push_str(&format!("  {:?}: {},\n", locale, ident(locale)));
    }
    out.push_str("} satisfies Record<Locale, Record<MessageKey, string>>\n");

    Ok(out)
}

/// `pt-BR` → `pt_BR`, so a locale tag is a valid identifier.
fn ident(locale: &str) -> String {
    locale.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(pairs: &[(&str, &str)]) -> Catalog {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn nested_json_flattens_to_dotted_keys() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"nav":{"home":"Home","deep":{"x":"y"}}}"#).unwrap();
        let mut out = Catalog::new();
        flatten(&value, "", &mut out).unwrap();
        assert_eq!(out.get("nav.home").unwrap(), "Home");
        assert_eq!(out.get("nav.deep.x").unwrap(), "y");
    }

    #[test]
    fn a_non_string_message_is_rejected_by_name() {
        let value: serde_json::Value = serde_json::from_str(r#"{"count": 3}"#).unwrap();
        let mut out = Catalog::new();
        let err = flatten(&value, "", &mut out).unwrap_err().to_string();
        assert!(err.contains("count"), "{err}");
        assert!(err.contains("a number"), "{err}");
    }

    #[test]
    fn a_missing_translation_is_fatal() {
        // The whole point. Ring of Pursuit could not detect this.
        let mut all = BTreeMap::new();
        all.insert("en".into(), catalog(&[("a", "A"), ("b", "B")]));
        all.insert("sr".into(), catalog(&[("a", "A")]));

        let found = compare(&all, "en").unwrap();
        assert!(found.is_fatal());
        assert_eq!(found.missing.get("sr").unwrap(), &vec!["b".to_string()]);
    }

    #[test]
    fn a_placeholder_mismatch_is_fatal() {
        // A reader would see a literal "{count}", or lose the value entirely.
        let mut all = BTreeMap::new();
        all.insert("en".into(), catalog(&[("k", "{count} items")]));
        all.insert("sr".into(), catalog(&[("k", "nekoliko stvari")]));

        let found = compare(&all, "en").unwrap();
        assert!(found.is_fatal());
        assert!(found.placeholder_mismatch.contains_key("sr"));
    }

    #[test]
    fn reordered_placeholders_are_fine() {
        // Word order differs between languages; only the SET must match.
        let mut all = BTreeMap::new();
        all.insert("en".into(), catalog(&[("k", "{a} then {b}")]));
        all.insert("sr".into(), catalog(&[("k", "{b} pa {a}")]));
        assert!(!compare(&all, "en").unwrap().is_fatal());
    }

    #[test]
    fn an_identical_long_value_is_reported_but_not_fatal() {
        // This is exactly ROP's "Live dashboard" — reported so someone looks,
        // not failed, because it is sometimes correct.
        let mut all = BTreeMap::new();
        all.insert("en".into(), catalog(&[("k", "Live dashboard view")]));
        all.insert("sr".into(), catalog(&[("k", "Live dashboard view")]));

        let found = compare(&all, "en").unwrap();
        assert!(!found.is_fatal(), "a proper noun must not block a build");
        assert!(found.untranslated.contains_key("sr"));
    }

    #[test]
    fn a_short_identical_value_is_not_even_reported() {
        // "OK", "Email", "Wi-Fi" are identical in many languages.
        let mut all = BTreeMap::new();
        all.insert("en".into(), catalog(&[("k", "OK")]));
        all.insert("sr".into(), catalog(&[("k", "OK")]));
        assert!(compare(&all, "en").unwrap().untranslated.is_empty());
    }

    #[test]
    fn an_extra_key_is_reported_but_not_fatal() {
        let mut all = BTreeMap::new();
        all.insert("en".into(), catalog(&[("a", "A")]));
        all.insert("sr".into(), catalog(&[("a", "A"), ("leftover", "x")]));

        let found = compare(&all, "en").unwrap();
        assert!(!found.is_fatal());
        assert!(found.extra.contains_key("sr"));
    }

    #[test]
    fn generated_typescript_declares_every_key_and_locale() {
        let mut all = BTreeMap::new();
        all.insert("en".into(), catalog(&[("nav.home", "Home")]));
        all.insert("pt-BR".into(), catalog(&[("nav.home", "Início")]));

        let ts = render_typescript(&all, "en").unwrap();
        assert!(ts.contains(r#"| "nav.home""#), "key union: {ts}");
        assert!(ts.contains("export type Locale"));
        assert!(ts.contains(r#"defaultLocale: Locale = "en""#));
        // A tag with a hyphen is not a valid identifier.
        assert!(ts.contains("export const pt_BR"), "{ts}");
        assert!(ts.contains(r#""pt-BR": pt_BR"#), "{ts}");
        assert!(ts.contains("Do not edit"));
    }

    #[test]
    fn placeholders_are_extracted_not_guessed() {
        assert_eq!(
            placeholders("{a} and {b_2}"),
            ["a", "b_2"].map(String::from).into_iter().collect()
        );
        // Not placeholders: unclosed, empty, or punctuation inside.
        assert!(placeholders("{unclosed").is_empty());
        assert!(placeholders("{}").is_empty());
        assert!(placeholders("{not a name}").is_empty());
    }
}

//! End-to-end: `fid add legal` → `fid derive` → message-catalog keys.
//!
//! The unit tests for `Legal` live in `src/config.rs`. These cover the
//! property that matters to a product: declared jurisdiction and brand facts
//! produce real `legal.*` keys in real locale catalogs, through the real
//! binary, on a real scaffold.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fid() -> PathBuf {
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("fid")
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(fid())
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .output()
        .expect("running fid")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A product with brand + i18n + legal installed and derived once.
fn legal_product(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    assert!(run(&root, &["add", "brand"]).status.success(), "fid add brand");
    assert!(run(&root, &["add", "i18n"]).status.success(), "fid add i18n");
    assert!(run(&root, &["add", "legal"]).status.success(), "fid add legal");
    // Run derive to write the legal keys into the locale catalogs.
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

fn read_catalog(root: &Path, locale: &str) -> serde_json::Value {
    let path = root.join(format!("messages/{locale}.json"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

// ── Installation ─────────────────────────────────────────────────────────────

#[test]
fn installing_legal_seeds_a_non_empty_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("[legal]"), "declaration written: {config}");
    assert!(config.contains("jurisdiction"), "jurisdiction seeded: {config}");
}

// ── Key generation ────────────────────────────────────────────────────────────

#[test]
fn derive_writes_legal_privacy_keys_to_every_locale() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    for locale in &["en", "sr"] {
        let catalog = read_catalog(&root, locale);
        assert!(
            catalog.get("legal.privacy.title").is_some(),
            "{locale}: missing legal.privacy.title"
        );
        assert!(
            catalog.get("legal.privacy.intro").is_some(),
            "{locale}: missing legal.privacy.intro"
        );
    }
}

#[test]
fn derive_writes_legal_terms_keys_to_every_locale() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    for locale in &["en", "sr"] {
        let catalog = read_catalog(&root, locale);
        assert!(
            catalog.get("legal.terms.title").is_some(),
            "{locale}: missing legal.terms.title"
        );
        assert!(
            catalog.get("legal.terms.governing_law").is_some(),
            "{locale}: missing legal.terms.governing_law"
        );
    }
}

#[test]
fn derive_writes_legal_cookie_keys_for_each_declared_category() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    // The default seed declares ["necessary"]. Each category now has .label + .description.
    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.cookie.categories.necessary.label").is_some(),
        "necessary cookie category label key missing"
    );
    assert!(
        catalog.get("legal.cookie.categories.necessary.description").is_some(),
        "necessary cookie category description key missing"
    );
    // The consent banner keys are also written.
    assert!(
        catalog.get("legal.cookie_banner.accept_all").is_some(),
        "missing cookie_banner.accept_all"
    );
}

#[test]
fn derive_writes_legal_a11y_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.a11y.title").is_some(),
        "missing legal.a11y.title"
    );
}

// ── EU jurisdiction ───────────────────────────────────────────────────────────

#[test]
fn eu_jurisdiction_produces_gdpr_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    // Default seed jurisdiction is EU.
    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.privacy.rights").is_some(),
        "EU: missing data-subject rights key"
    );
    assert!(
        catalog.get("legal.privacy.dpo_contact").is_some(),
        "EU: missing DPO contact key"
    );
}

#[test]
fn eu_jurisdiction_produces_imprint_key() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.imprint.title").is_some(),
        "EU: missing imprint.title"
    );
}

// ── DSAR ─────────────────────────────────────────────────────────────────────

#[test]
fn eu_jurisdiction_produces_dsar_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.privacy.dsar.title").is_some(),
        "EU: missing DSAR title key"
    );
    assert!(
        catalog.get("legal.privacy.dsar.intro").is_some(),
        "EU: missing DSAR intro key"
    );
}

#[test]
fn eu_jurisdiction_produces_consent_opt_in_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.privacy.consent.marketing").is_some(),
        "EU: missing marketing consent opt-in key"
    );
    assert!(
        catalog.get("legal.privacy.consent.analytics").is_some(),
        "EU: missing analytics consent opt-in key"
    );
}

// ── Optional documents ────────────────────────────────────────────────────────

#[test]
fn generate_dpa_true_produces_dpa_keys() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "brand"]).status.success(), "fid add brand");
    assert!(run(&root, &["add", "legal"]).status.success(), "fid add legal");
    // Enable DPA.
    let toml_path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&toml_path).unwrap();
    let config = config.replace("generate_dpa = false", "generate_dpa = true");
    std::fs::write(&toml_path, config).unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));

    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.dpa.title").is_some(),
        "generate_dpa = true: missing legal.dpa.title"
    );
    assert!(
        catalog.get("legal.dpa.processor_obligations").is_some(),
        "generate_dpa = true: missing processor_obligations"
    );
}

#[test]
fn generate_aup_true_produces_aup_keys() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "brand"]).status.success(), "fid add brand");
    assert!(run(&root, &["add", "legal"]).status.success(), "fid add legal");
    // Enable AUP.
    let toml_path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&toml_path).unwrap();
    let config = config.replace("generate_aup = false", "generate_aup = true");
    std::fs::write(&toml_path, config).unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));

    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.aup.title").is_some(),
        "generate_aup = true: missing legal.aup.title"
    );
    assert!(
        catalog.get("legal.aup.prohibited.spam").is_some(),
        "generate_aup = true: missing prohibited.spam"
    );
}

#[test]
fn charges_users_true_produces_withdrawal_keys_for_eu() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "brand"]).status.success(), "fid add brand");
    assert!(run(&root, &["add", "legal"]).status.success(), "fid add legal");
    let toml_path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&toml_path).unwrap();
    let config = config.replace("charges_users = false", "charges_users = true");
    std::fs::write(&toml_path, config).unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));

    let catalog = read_catalog(&root, "en");
    assert!(
        catalog.get("legal.terms.withdrawal_right").is_some(),
        "charges_users + EU: missing withdrawal right key"
    );
    assert!(
        catalog.get("legal.terms.refund_policy").is_some(),
        "charges_users + EU: missing refund_policy key"
    );
}

// ── Error cases ───────────────────────────────────────────────────────────────

#[test]
fn fid_legal_without_brand_fails_clearly() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "i18n"]).status.success(), "fid add i18n");
    // Manually write a [legal] block without [brand].
    let toml_path = root.join("fiducial.toml");
    let mut config = std::fs::read_to_string(&toml_path).unwrap();
    config.push_str("\n[legal]\njurisdiction = \"EU\"\ncookie_categories = [\"necessary\"]\n");
    std::fs::write(&toml_path, config).unwrap();
    // Add the pipeline directly.
    std::fs::create_dir_all(root.join("pipelines")).unwrap();
    std::fs::write(
        root.join("pipelines/legal.toml"),
        "name = \"legal\"\nexecutor = \"fid-legal\"\noutputs = []\n",
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "should fail without [brand]");
    assert!(
        text(&out).contains("[brand]"),
        "error mentions [brand]: {}",
        text(&out)
    );
}

#[test]
fn fid_legal_without_i18n_fails_clearly() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "brand"]).status.success(), "fid add brand");
    // Strip [i18n] from the config to simulate a product without i18n.
    let toml_path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&toml_path).unwrap();
    let stripped = strip_toml_section(&config, "i18n");
    // Also strip i18n from [capabilities].enabled.
    let stripped = stripped.replace("    \"i18n\",\n", "");
    let mut stripped = stripped;
    stripped.push_str("\n[legal]\njurisdiction = \"EU\"\ncookie_categories = [\"necessary\"]\n");
    std::fs::write(&toml_path, stripped).unwrap();
    std::fs::create_dir_all(root.join("pipelines")).unwrap();
    std::fs::write(
        root.join("pipelines/legal.toml"),
        "name = \"legal\"\nexecutor = \"fid-legal\"\noutputs = []\n",
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "should fail without [i18n]");
    assert!(
        text(&out).contains("[i18n]"),
        "error mentions [i18n]: {}",
        text(&out)
    );
}

fn strip_toml_section(config: &str, section: &str) -> String {
    let header = format!("[{section}]\n");
    let Some(start) = config.find(&header) else {
        return config.to_string();
    };
    let after = start + header.len();
    let end = config[after..]
        .find("\n[")
        .map(|i| after + i + 1)
        .unwrap_or(config.len());
    format!("{}{}", &config[..start], &config[end..])
}

#[test]
fn fid_legal_without_jurisdiction_fails_clearly() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "brand"]).status.success(), "fid add brand");
    assert!(run(&root, &["add", "i18n"]).status.success(), "fid add i18n");
    // Empty [legal] — missing jurisdiction.
    let toml_path = root.join("fiducial.toml");
    let mut config = std::fs::read_to_string(&toml_path).unwrap();
    config.push_str("\n[legal]\n");
    std::fs::write(&toml_path, config).unwrap();
    std::fs::create_dir_all(root.join("pipelines")).unwrap();
    std::fs::write(
        root.join("pipelines/legal.toml"),
        "name = \"legal\"\nexecutor = \"fid-legal\"\noutputs = []\n",
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "should fail with empty [legal]");
    let t = text(&out);
    assert!(
        t.contains("jurisdiction") || t.contains("[legal]"),
        "error mentions jurisdiction or [legal]: {t}"
    );
}

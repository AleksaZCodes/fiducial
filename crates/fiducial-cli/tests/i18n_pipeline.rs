//! End-to-end: `fid add i18n` → `fid derive` → the gate.
//!
//! The unit tests in `src/i18n.rs` cover comparison and codegen. These cover the
//! property that actually matters to a product: **a missing translation stops
//! the build**, through the real binary, on a real scaffold.

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

/// A scaffolded product with the i18n capability installed and derived once.
fn localized_product(tmp: &Path) -> PathBuf {
    assert!(
        run(tmp, &["new", "p", "--full"]).status.success(),
        "fid new"
    );
    let root = tmp.join("p");
    assert!(
        run(&root, &["add", "i18n"]).status.success(),
        "fid add i18n"
    );
    assert!(run(&root, &["derive"]).status.success(), "fid derive");
    root
}

fn edit_catalog(root: &Path, locale: &str, f: impl FnOnce(&mut serde_json::Value)) {
    let path = root.join(format!("messages/{locale}.json"));
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    f(&mut value);
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

// ── The declaration ─────────────────────────────────────────────────────────

/// Installing the capability seeds the declaration it needs.
///
/// A capability that ships a pipeline but leaves its declaration empty produces
/// a product that fails on the very next `fid derive`.
#[test]
fn installing_i18n_seeds_the_locales_it_ships() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("[i18n]"), "declaration written: {config}");
    assert!(config.contains("\"sr\""));
    assert!(config.contains("\"en\""));
    assert!(
        config.contains("default = \"sr\""),
        "the fallback is declared, not inferred from list order: {config}"
    );
    // A round-trip must not write an empty messages_dir back out.
    assert!(
        !config.contains("messages_dir = \"\""),
        "empty messages_dir would fail to resolve on the next read: {config}"
    );
}

// ── The derivation ──────────────────────────────────────────────────────────

#[test]
fn derive_generates_a_typed_key_union() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    let generated = std::fs::read_to_string(root.join("src/generated/messages.ts")).unwrap();
    assert!(generated.contains("export type MessageKey"));
    assert!(generated.contains(r#"| "action.save""#), "{generated}");
    assert!(generated.contains("export const sr"));
    assert!(generated.contains("export const en"));
    assert!(
        generated.contains("Do not edit"),
        "a generated file says so"
    );
}

// ── The gate ────────────────────────────────────────────────────────────────

/// A translation missing from a non-default locale fails the build.
///
/// This is the Ring of Pursuit failure: 1377 keys maintained by discipline
/// across two locales, and one string that slipped through invisibly because
/// nothing could tell it had.
#[test]
fn a_missing_translation_fails_the_build() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    edit_catalog(&root, "en", |v| {
        v["nav"].as_object_mut().unwrap().remove("home");
    });

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "a missing translation must fail");

    let output = text(&out);
    assert!(
        output.contains("messages/en.json"),
        "names the file: {output}"
    );
    assert!(output.contains("nav.home"), "names the key: {output}");
    assert!(
        output.contains("missing artifact, not a fallback"),
        "states the principle: {output}"
    );
}

/// The **default** locale is not exempt.
///
/// Caught by hand-testing before it was caught by a test. With the default
/// locale's catalog as the reference key set, deleting a key *from the default*
/// reported the other locales as having an "extra" key — and passed. That is the
/// same bug with the locales swapped.
///
/// The reference is the union of all keys. The default locale only decides what
/// a reader falls back to; it does not define what the product is meant to say.
#[test]
fn a_key_missing_from_the_default_locale_also_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    // `sr` is the default.
    edit_catalog(&root, "sr", |v| {
        v["action"].as_object_mut().unwrap().remove("save");
    });

    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "the default locale is not exempt from being complete"
    );
    let output = text(&out);
    assert!(output.contains("messages/sr.json"), "{output}");
    assert!(output.contains("action.save"), "{output}");
}

/// A placeholder present in one locale and not another fails.
///
/// The reader would otherwise see a literal `{count}`, or lose the value.
#[test]
fn a_placeholder_mismatch_fails_the_build() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    edit_catalog(&root, "en", |v| {
        v["cart"]["summary"] = serde_json::json!("some items");
    });

    let out = run(&root, &["derive"]);
    assert!(!out.status.success());
    let output = text(&out);
    assert!(output.contains("placeholders"), "{output}");
    assert!(output.contains("cart.summary"), "{output}");
}

/// Word order may differ freely — only the placeholder *set* is compared.
#[test]
fn reordered_placeholders_are_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    edit_catalog(&root, "sr", |v| {
        v["cart"]["summary"] = serde_json::json!("ukupno {total}, stavki: {count}");
    });

    let out = run(&root, &["derive"]);
    assert!(
        out.status.success(),
        "languages order words differently: {}",
        text(&out)
    );
}

/// An untranslated value is reported, and does **not** stop the build.
///
/// Failing here would block a legitimate proper noun. The finding is surfaced so
/// a person looks — which is the half Ring of Pursuit was missing.
#[test]
fn an_untranslated_value_is_reported_but_does_not_fail() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    // Longer than the 12-character threshold, below which identical values are
    // not even reported — "OK", "Email" and "Wi-Fi" are legitimately identical.
    edit_catalog(&root, "sr", |v| {
        v["cart"]["summary"] = serde_json::json!("{count} items, {total} total");
    });

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "a proper noun must not block a build");
    let output = text(&out);
    assert!(
        output.contains("identical to sr") || output.contains("likely untranslated"),
        "but it is reported: {output}"
    );
}

/// `fid derive --check` catches a hand-edited generated file.
#[test]
fn check_catches_a_hand_edited_generated_module() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    assert!(
        run(&root, &["derive", "--check"]).status.success(),
        "clean to begin with"
    );

    std::fs::write(
        root.join("src/generated/messages.ts"),
        "export type MessageKey = \"i.edited.this.by.hand\"\n",
    )
    .unwrap();

    let out = run(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "a hand-edited derived artifact must not pass: {}",
        text(&out)
    );
}

/// The product stays healthy after installing and deriving.
#[test]
fn doctor_is_clean_after_installing_i18n() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    let out = run(&root, &["doctor"]);
    let output = text(&out);
    assert!(out.status.success(), "doctor: {output}");
    assert!(output.contains("clean"), "doctor: {output}");
}

/// Agents get the rules, at a path any agent can read.
#[test]
fn the_skill_lands_where_every_agent_can_find_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = localized_product(tmp.path());

    let neutral = root.join(".fiducial/skills/i18n.md");
    assert!(neutral.is_file(), "vendor-neutral instructions exist");

    let body = std::fs::read_to_string(&neutral).unwrap();
    assert!(body.contains("Keys are named, never the English text"));
    assert!(body.contains("three plural categories"), "Serbian plurals");

    // Claude Code gets a pointer, not a second copy of the content.
    let pointer = std::fs::read_to_string(root.join(".claude/skills/i18n.md")).unwrap();
    assert!(pointer.contains(".fiducial/skills/i18n.md"));
    assert!(
        pointer.lines().count() < 15,
        "the pointer must not duplicate the content"
    );
}

// ── `fid new --locales` ───────────────────────────────────────────────────────
//
// Principle 1c says monolingual is a state you pass through before the first
// commit, not one you ship. That is only true if a product is *born* localized,
// which is what these cover.

#[test]
fn a_new_product_is_born_localized() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p", "--full"]).status.success());
    let root = tmp.path().join("p");

    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("[i18n]"), "no [i18n] block:\n{config}");
    assert!(config.contains("\"sr\"") && config.contains("\"en\""));
    assert!(root.join("messages/sr.json").is_file());
    assert!(root.join("messages/en.json").is_file());

    // Born with a working gate, not just a declaration.
    assert!(
        run(&root, &["derive", "--check"]).status.success(),
        "a fresh product must derive clean"
    );
}

#[test]
fn a_declared_locale_without_a_shipped_catalog_gets_one() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(
        tmp.path(),
        &[
            "new",
            "p",
            "--locales",
            "en,fr,de",
            "--default-locale",
            "en",
        ],
    );
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for locale in ["en", "fr", "de"] {
        assert!(
            root.join(format!("messages/{locale}.json")).is_file(),
            "messages/{locale}.json missing"
        );
    }
    // The locale set is read from the directory listing, so a catalog for a
    // locale the product does not declare is a language it silently claims.
    assert!(
        !root.join("messages/sr.json").exists(),
        "sr.json survived a locale set that does not include it"
    );

    // Seeded from the default, so the work still to do is reported rather than
    // discovered by a reader.
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("untranslated"),
        "a copy of the default must report as untranslated:\n{}",
        text(&out)
    );
}

#[test]
fn opting_out_is_explicit_and_leaves_no_half_state() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p", "--locales", "none"])
        .status
        .success());
    let root = tmp.path().join("p");
    assert!(!root.join("messages").exists());
    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(
        !config.contains("locales = ["),
        "declared locales:\n{config}"
    );
}

#[test]
fn the_fallback_locale_is_never_taken_from_list_order() {
    let tmp = tempfile::tempdir().unwrap();
    // `en` is first, and the product is still not created — because which
    // language a reader falls back to is a decision, not a list position.
    let out = run(tmp.path(), &["new", "p", "--locales", "en,fr"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("--default-locale"),
        "the error must name the fix:\n{}",
        text(&out)
    );
    assert!(
        !tmp.path().join("p").exists(),
        "a failed `fid new` left a directory behind"
    );
}

#[test]
fn a_fallback_outside_the_locale_set_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(
        tmp.path(),
        &["new", "p", "--locales", "en,fr", "--default-locale", "de"],
    );
    assert!(!out.status.success());
    assert!(text(&out).contains("not in --locales"), "{}", text(&out));
}

// ── Hardcoded-string detection ────────────────────────────────────────────────

/// A component with three real findings and six things that must not be.
const COMPONENT: &str = r#"import { t } from '@fiducial/i18n'

export default function Page({ count }: { count: number }) {
  return (
    <main className="flex flex-col gap-4" data-testid="home page">
      <h1>Welcome to the demo</h1>
      <p>{t('home.intro')}</p>
      <p>{count} items in your cart</p>
      <img src="/logo.svg" alt="Company logo" />
      <a href="https://example.com">https://example.com</a>
      <button aria-label="Close the dialog">x</button>
      <span>nav.home</span>
      <span>{/* i18n-ignore */ "Wi-Fi"}</span>
    </main>
  )
}
"#;

fn with_component(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p", "--full"]).status.success());
    let root = tmp.join("p");
    std::fs::create_dir_all(root.join("app")).unwrap();
    std::fs::write(root.join("app/page.tsx"), COMPONENT).unwrap();
    root
}

#[test]
fn hardcoded_strings_are_reported_and_false_positives_are_not() {
    let tmp = tempfile::tempdir().unwrap();
    let root = with_component(tmp.path());

    let out = run(&root, &["dash", "--section", "i18n", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let dash: serde_json::Value = serde_json::from_str(&text(&out)).expect("dash --json");
    let found = dash["i18n"]["hardcoded"].as_array().expect("hardcoded");

    let texts: Vec<&str> = found
        .iter()
        .map(|h| h["text"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        texts,
        vec!["Welcome to the demo", "Company logo", "Close the dialog"],
        "detector found the wrong set"
    );
    assert_eq!(found[0]["file"], "app/page.tsx");
    assert_eq!(found[0]["line"], 6);
}

/// The whole design rests on this. Making it fatal means every false positive
/// blocks someone until the rule is loosened for everyone.
#[test]
fn hardcoded_strings_never_fail_a_command() {
    let tmp = tempfile::tempdir().unwrap();
    let root = with_component(tmp.path());

    let doctor = run(&root, &["doctor"]);
    assert!(
        doctor.status.success(),
        "doctor must not fail on hardcoded strings:\n{}",
        text(&doctor)
    );
    assert!(
        text(&doctor).contains("hardcoded user-visible string"),
        "doctor must still report them:\n{}",
        text(&doctor)
    );
    assert!(
        run(&root, &["derive", "--check"]).status.success(),
        "the gate must stay about artifacts, not about this warning"
    );
}

#[test]
fn a_product_with_no_locales_is_not_nagged() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p", "--locales", "none"])
        .status
        .success());
    let root = tmp.path().join("p");
    std::fs::create_dir_all(root.join("app")).unwrap();
    std::fs::write(root.join("app/page.tsx"), COMPONENT).unwrap();

    let out = run(&root, &["doctor"]);
    assert!(
        !text(&out).contains("hardcoded"),
        "a product that never asked for localization was told about strings:\n{}",
        text(&out)
    );
}

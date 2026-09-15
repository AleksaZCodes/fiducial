//! End-to-end: `fid add brand` → `fid derive` → the gate.
//!
//! The unit tests in `src/brand.rs` cover rendering; `src/config.rs`'s
//! `brand_tests` cover validation. These cover the property that matters to a
//! product: the declared facts reach real files, through the real binary, on
//! a real scaffold.

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

/// A scaffolded product with the brand capability installed and derived once.
fn branded_product(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    assert!(
        run(&root, &["add", "brand"]).status.success(),
        "fid add brand"
    );
    assert!(run(&root, &["derive"]).status.success(), "fid derive");
    root
}

/// Replace the seeded `[brand]` table wholesale.
///
/// `toml::Value`'s `Table` sorts alphabetically, so `[brand]` is not
/// necessarily the last table in the file — this finds where it starts and
/// where the *next* `[section]` begins (or end of file) rather than assuming
/// position.
fn set_brand(root: &Path, block: &str) {
    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    let start = config.find("[brand]\n").expect("[brand] present");
    let after_header = start + "[brand]\n".len();
    let end = config[after_header..]
        .find("\n[")
        .map(|i| after_header + i + 1)
        .unwrap_or(config.len());
    let mut out = config[..start].to_string();
    out.push_str("[brand]\n");
    out.push_str(block);
    out.push('\n');
    out.push_str(&config[end..]);
    std::fs::write(&path, out).unwrap();
}

// ── The declaration ─────────────────────────────────────────────────────────

/// Installing the capability seeds the declaration it needs.
#[test]
fn installing_brand_seeds_a_non_empty_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = branded_product(tmp.path());

    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("[brand]"), "declaration written: {config}");
    assert!(config.contains("legal_name"));
    assert!(config.contains("domain"));
}

// ── The derivation ──────────────────────────────────────────────────────────

#[test]
fn derive_writes_all_five_artifacts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = branded_product(tmp.path());

    let robots = std::fs::read_to_string(root.join("public/robots.txt")).unwrap();
    assert!(
        robots.contains("Sitemap: https://example.com/sitemap.xml"),
        "{robots}"
    );

    let sitemap = std::fs::read_to_string(root.join("public/sitemap.xml")).unwrap();
    assert!(sitemap.contains("https://example.com/"), "{sitemap}");

    let manifest = std::fs::read_to_string(root.join("public/site.webmanifest")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["name"], "Example");

    let favicon = std::fs::read_to_string(root.join("public/favicon.svg")).unwrap();
    assert!(favicon.contains("<svg"), "{favicon}");
    assert!(favicon.contains('E'), "initials render: {favicon}");

    let jsonld = std::fs::read_to_string(root.join("public/organization.jsonld")).unwrap();
    let jsonld: serde_json::Value = serde_json::from_str(&jsonld).unwrap();
    assert_eq!(jsonld["@type"], "Organization");
    assert_eq!(jsonld["name"], "Example LLC");
}

#[test]
fn a_changed_declaration_regenerates_every_output() {
    let tmp = tempfile::tempdir().unwrap();
    let root = branded_product(tmp.path());

    set_brand(
        &root,
        "legal_name = \"Acme Robotics, Inc.\"\n\
         trading_name = \"Acme Robotics\"\n\
         domain = \"acme.dev\"\n\
         contact_email = \"hello@acme.dev\"\n",
    );
    assert!(run(&root, &["derive"]).status.success());

    let robots = std::fs::read_to_string(root.join("public/robots.txt")).unwrap();
    assert!(robots.contains("acme.dev"), "{robots}");

    let favicon = std::fs::read_to_string(root.join("public/favicon.svg")).unwrap();
    assert!(favicon.contains("AR"), "{favicon}");
}

// ── The gate ────────────────────────────────────────────────────────────────

/// An incomplete declaration fails the build, naming what is missing.
#[test]
fn an_incomplete_declaration_fails_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let root = branded_product(tmp.path());

    set_brand(&root, "legal_name = \"Acme\"\n");
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "a missing field must fail");
    let t = text(&out);
    assert!(t.contains("trading_name"), "{t}");
    assert!(t.contains("domain"), "{t}");
    assert!(t.contains("contact_email"), "{t}");
}

/// A malformed colour fails the build rather than shipping a broken manifest.
#[test]
fn a_malformed_colour_fails_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let root = branded_product(tmp.path());

    set_brand(
        &root,
        "legal_name = \"Example LLC\"\n\
         trading_name = \"Example\"\n\
         domain = \"example.com\"\n\
         contact_email = \"hello@example.com\"\n\
         primary_color = \"blue\"\n",
    );
    let out = run(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("primary_color"));
}

/// `fid derive --check` catches a hand-edited generated artifact.
#[test]
fn check_catches_a_hand_edited_output() {
    let tmp = tempfile::tempdir().unwrap();
    let root = branded_product(tmp.path());

    std::fs::write(root.join("public/robots.txt"), "hand-edited\n").unwrap();
    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("robots.txt"));
}

#[test]
fn doctor_is_clean_after_installing_brand() {
    let tmp = tempfile::tempdir().unwrap();
    let root = branded_product(tmp.path());

    let out = run(&root, &["doctor"]);
    assert!(out.status.success(), "{}", text(&out));

    let neutral = root.join(".fiducial/skills/brand.md");
    assert!(neutral.exists(), "instructions at the vendor-neutral path");
    let pointer = std::fs::read_to_string(root.join(".claude/skills/brand.md")).unwrap();
    assert!(pointer.contains(".fiducial/skills/brand.md"));
}

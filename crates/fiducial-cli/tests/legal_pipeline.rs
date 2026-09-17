//! End-to-end: `fid add legal` → `fid derive` → the gate.
//!
//! The unit tests in `src/legal.rs` cover rendering; `src/config.rs` covers
//! validation. These cover the property that matters to a product: the
//! declared facts reach the generated TypeScript file, through the real binary,
//! on a real scaffold.

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

/// A scaffolded product with `brand`, `i18n`, and `legal` installed and derived.
fn legal_product(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    assert!(
        run(&root, &["add", "brand"]).status.success(),
        "fid add brand"
    );
    assert!(
        run(&root, &["add", "i18n"]).status.success(),
        "fid add i18n"
    );
    assert!(
        run(&root, &["add", "legal"]).status.success(),
        "fid add legal"
    );
    assert!(run(&root, &["derive"]).status.success(), "fid derive");
    root
}

// ── Installation ──────────────────────────────────────────────────────────────

/// Installing `legal` writes the pipeline file.
#[test]
fn installing_legal_adds_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "legal"]).status.success());

    assert!(
        root.join("pipelines/legal.toml").exists(),
        "pipeline file written"
    );
}

// ── Derivation ────────────────────────────────────────────────────────────────

/// After a full setup and `fid derive`, the generated file exists and exports
/// the expected TypeScript symbols.
#[test]
fn derive_writes_legal_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let ts = std::fs::read_to_string(root.join("src/generated/legal.ts")).unwrap();
    assert!(ts.contains("export type LegalPage ="), "{ts}");
    assert!(ts.contains("\"privacy\""), "{ts}");
    assert!(ts.contains("\"imprint\""), "{ts}"); // EU jurisdiction includes imprint
    assert!(ts.contains("export interface LegalPageContent"), "{ts}");
    assert!(ts.contains("export type LegalCatalog"), "{ts}");
    assert!(ts.contains("legalCatalogs"), "{ts}");
    assert!(ts.contains("GDPR COMPLIANCE CHECKLIST"), "{ts}");
    assert!(ts.contains("Example LLC"), "{ts}");
    assert!(ts.contains("example.com"), "{ts}");
    assert!(ts.contains("privacy@example.com"), "{ts}");
}

// ── The gate ─────────────────────────────────────────────────────────────────

/// Hand-editing the generated file causes `fid derive --check` to fail.
#[test]
fn check_catches_a_hand_edited_legal_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let path = root.join("src/generated/legal.ts");
    std::fs::write(&path, "// hand-edited\n").unwrap();
    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "should fail after hand-edit");
    let t = text(&out);
    assert!(t.contains("legal.ts"), "{t}");
}

/// `fid doctor` passes after `fid add legal` (with brand and i18n also installed).
#[test]
fn doctor_is_clean_after_installing_legal() {
    let tmp = tempfile::tempdir().unwrap();
    let root = legal_product(tmp.path());

    let out = run(&root, &["doctor"]);
    assert!(out.status.success(), "{}", text(&out));

    let neutral = root.join(".fiducial/skills/legal.md");
    assert!(neutral.exists(), "instructions at the vendor-neutral path");
    let pointer = std::fs::read_to_string(root.join(".claude/skills/legal.md")).unwrap();
    assert!(pointer.contains(".fiducial/skills/legal.md"));
}

/// Without `[brand]`, `fid derive` fails with a clear message naming the
/// missing capability.
#[test]
fn derive_fails_without_brand() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");

    // Install i18n and legal, but not brand.
    assert!(run(&root, &["add", "i18n"]).status.success());
    assert!(run(&root, &["add", "legal"]).status.success());

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "should fail without brand");
    let t = text(&out);
    assert!(
        t.contains("brand") || t.contains("[brand]"),
        "error should mention brand: {t}"
    );
}

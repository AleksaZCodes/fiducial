//! End-to-end tests for `fid capability extract`.

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

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn scaffold(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "acme"]);
    assert!(out.status.success(), "fid new failed: {}", text(&out));
    tmp.join("acme")
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Extract stages the files into staged/<name>/ by default.
#[test]
fn extract_writes_to_staged_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/payments/checkout.ts", "export function pay() {}");

    let out = run(
        &root,
        &["capability", "extract", "payments", "--files", "src/payments/"],
    );
    assert!(out.status.success(), "{}", text(&out));

    assert!(
        root.join("staged/payments").is_dir(),
        "staged/payments/ not created"
    );
}

/// Extracted files land under templates/ in the staged directory.
#[test]
fn extract_files_land_under_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/pdf/renderer.ts", "export function render() {}");

    let out = run(
        &root,
        &["capability", "extract", "pdf-export", "--files", "src/pdf/"],
    );
    assert!(out.status.success(), "{}", text(&out));

    assert!(
        root.join("staged/pdf-export/templates/src/pdf/renderer.ts").exists(),
        "templates/ file not written"
    );
}

/// SKILL.md and capability.toml are both generated.
#[test]
fn extract_writes_skill_and_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/telemetry.ts", "export function track() {}");

    let out = run(
        &root,
        &["capability", "extract", "telemetry", "--files", "src/telemetry.ts"],
    );
    assert!(out.status.success(), "{}", text(&out));

    let staged = root.join("staged/telemetry");
    assert!(staged.join("SKILL.md").exists(), "SKILL.md not written");
    assert!(staged.join("capability.toml").exists(), "capability.toml not written");
}

/// Product name occurrences in file content are replaced with {{name}}.
#[test]
fn extract_generalises_product_name_in_content() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    // The product is named "acme" (from fid new acme above).
    write(
        &root,
        "src/config.ts",
        "export const APP_NAME = 'acme';\nexport const API_BASE = 'https://api.acme.com';\n",
    );

    let out = run(
        &root,
        &["capability", "extract", "config", "--files", "src/config.ts"],
    );
    assert!(out.status.success(), "{}", text(&out));

    let staged = std::fs::read_to_string(
        root.join("staged/config/templates/src/config.ts"),
    ).unwrap();
    assert!(
        staged.contains("{{name}}"),
        "product name not generalised: {staged}"
    );
    assert!(
        !staged.contains("acme"),
        "product name still present after generalisation: {staged}"
    );
}

/// The single-consumer warning is always printed.
#[test]
fn extract_prints_single_consumer_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/feature.ts", "export const x = 1;");

    let out = run(
        &root,
        &["capability", "extract", "feature", "--files", "src/feature.ts"],
    );
    assert!(out.status.success(), "{}", text(&out));

    let stdout = text(&out);
    assert!(
        stdout.contains("Single-consumer warning"),
        "single-consumer warning not printed: {stdout}"
    );
}

/// --output lets the caller choose where the staged directory is written.
#[test]
fn extract_respects_output_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/widget.ts", "export const Widget = {};");
    let out_dir = tmp.path().join("my-staging");

    let out = run(
        &root,
        &[
            "capability",
            "extract",
            "widget",
            "--files",
            "src/widget.ts",
            "--output",
            out_dir.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", text(&out));
    assert!(out_dir.join("SKILL.md").exists(), "SKILL.md not at --output path");
}

/// Extracting a second time without --force fails with a clear error.
#[test]
fn extract_refuses_to_overwrite_without_force() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/x.ts", "export const x = 1;");

    let first = run(
        &root,
        &["capability", "extract", "x-cap", "--files", "src/x.ts"],
    );
    assert!(first.status.success(), "{}", text(&first));

    let second = run(
        &root,
        &["capability", "extract", "x-cap", "--files", "src/x.ts"],
    );
    assert!(!second.status.success(), "should have failed without --force");
    assert!(
        text(&second).contains("--force"),
        "--force not mentioned in error: {}",
        text(&second)
    );
}

/// --force allows overwriting an existing staged directory.
#[test]
fn extract_force_overwrites_existing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/y.ts", "export const y = 1;");

    let first = run(
        &root,
        &["capability", "extract", "y-cap", "--files", "src/y.ts"],
    );
    assert!(first.status.success(), "{}", text(&first));

    let second = run(
        &root,
        &["capability", "extract", "y-cap", "--files", "src/y.ts", "--force"],
    );
    assert!(second.status.success(), "{}", text(&second));
}

/// An invalid name is rejected with a clear error before any filesystem work.
#[test]
fn extract_rejects_invalid_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/z.ts", "export const z = 1;");

    let out = run(
        &root,
        &["capability", "extract", "MyCapital", "--files", "src/z.ts"],
    );
    assert!(!out.status.success(), "should have rejected PascalCase name");
}

/// A missing --files argument gives a clear error.
#[test]
fn extract_without_files_fails_clearly() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let out = run(&root, &["capability", "extract", "nothing"]);
    assert!(!out.status.success(), "should have failed without --files");
}

/// capability.toml generated by extract references the staged files.
#[test]
fn extract_capability_toml_lists_staged_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "src/hooks/useAuth.ts", "export function useAuth() {}");

    let out = run(
        &root,
        &["capability", "extract", "auth-hook", "--files", "src/hooks/useAuth.ts"],
    );
    assert!(out.status.success(), "{}", text(&out));

    let toml_content = std::fs::read_to_string(
        root.join("staged/auth-hook/capability.toml"),
    ).unwrap();
    assert!(
        toml_content.contains("useAuth.ts"),
        "useAuth.ts not referenced in capability.toml: {toml_content}"
    );
}

//! `fid derive --check` fails when a declaration a pipeline reads has moved
//! since it last ran — for every built-in executor, not only `fid-hardware`.
//!
//! Until 0.9.5 only the hardware executor reported what it read. Every other
//! pipeline's outputs were checked against the lock but its declarations were
//! not, so a message catalogue or `[brand]` edited and never re-derived passed
//! `--check`, and the stale artifact could merge. The drift-injection study in
//! the paper found it; these are its cases W2 and W7, plus the two a file
//! pattern adds: a file that appears, and one that goes.

use std::{path::Path, process::Command};

fn fid(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fid"))
        .args(args)
        .current_dir(dir)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to invoke fid")
}

fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn ok(root: &Path, args: &[&str]) {
    let out = fid(root, args);
    assert!(out.status.success(), "fid {args:?} failed: {}", text(&out));
}

fn edit(root: &Path, file: &str, from: &str, to: &str) {
    let p = root.join(file);
    let s = std::fs::read_to_string(&p).unwrap();
    assert!(s.contains(from), "{file} does not contain {from:?}");
    std::fs::write(&p, s.replacen(from, to, 1)).unwrap();
}

fn product(tmp: &Path) -> std::path::PathBuf {
    ok(tmp, &["new", "--locales", "en,sr", "demo"]);
    let root = tmp.join("demo");
    for cap in ["i18n", "brand", "migrations"] {
        ok(&root, &["add", "capability", cap]);
    }
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
    root
}

fn check_fails_naming(root: &Path, file: &str) {
    let out = fid(root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "--check passed although {file} moved:\n{}",
        text(&out)
    );
    assert!(
        text(&out).contains(file),
        "--check failed without naming {file}:\n{}",
        text(&out)
    );
}

#[test]
fn an_edited_message_catalogue_fails_check_until_derived() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(&root, "messages/en.json", "\"Save changes\"", "\"Save\"");
    check_fails_naming(&root, "messages/en.json");
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
}

#[test]
fn an_edited_brand_declaration_fails_check_until_derived() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let toml = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    let line = toml
        .lines()
        .find(|l| l.trim_start().starts_with("domain"))
        .expect("[brand] declares a domain")
        .to_string();
    edit(&root, "fiducial.toml", &line, "domain = \"example.org\"");
    check_fails_naming(&root, "fiducial.toml");
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
}

#[test]
fn a_file_added_under_a_pattern_is_a_changed_input() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    std::fs::write(
        root.join("migrations/0002_notes.sql"),
        "CREATE TABLE notes (id INTEGER PRIMARY KEY);\n",
    )
    .unwrap();
    check_fails_naming(&root, "migrations/0002_notes.sql");
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
}

#[test]
fn a_file_removed_from_under_a_pattern_is_a_changed_input() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    std::fs::write(
        root.join("migrations/0002_notes.sql"),
        "CREATE TABLE notes (id INTEGER PRIMARY KEY);\n",
    )
    .unwrap();
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
    std::fs::remove_file(root.join("migrations/0002_notes.sql")).unwrap();
    check_fails_naming(&root, "migrations/0002_notes.sql");
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
}

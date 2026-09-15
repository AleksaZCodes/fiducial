//! End-to-end: `fid add migrations` → `fid derive` → the ordered manifest.
//!
//! The runner itself is tested against real SQLite in
//! `packages/adapters/src/migrate.test.js` — ordering, idempotency and drift
//! are properties of what a database ends up containing, and against a mock
//! every one of them passes whether or not the code is right.
//!
//! These cover generation and the gate.

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
        .env("RUST_BACKTRACE", "0")
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

fn product(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    let out = run(&root, &["add", "migrations"]);
    assert!(out.status.success(), "fid add migrations: {}", text(&out));
    root
}

fn migration(root: &Path, name: &str, sql: &str) {
    let dir = root.join("migrations");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), sql).unwrap();
}

fn manifest(root: &Path) -> String {
    std::fs::read_to_string(root.join("src/migrations.generated.ts")).expect("manifest")
}

#[test]
fn a_product_with_no_migrations_generates_an_empty_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    // Not an error: a product with no schema is the normal starting state.
    assert!(manifest(&root).contains("migrations: readonly Migration[] = ["));
}

#[test]
fn the_manifest_orders_numerically_and_embeds_the_sql() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    migration(&root, "0010_ten.sql", "SELECT 10;");
    migration(&root, "0009_nine.sql", "SELECT 9;");

    assert!(run(&root, &["derive"]).status.success());
    let m = manifest(&root);

    let nine = m.find("0009_nine").expect("0009 present");
    let ten = m.find("0010_ten").expect("0010 present");
    assert!(nine < ten, "order must be numeric:\n{m}");
    // Embedded, because the runner has no filesystem.
    assert!(m.contains("SELECT 9;"), "{m}");
}

#[test]
fn adding_a_migration_without_deriving_fails_the_check() {
    // The failure this whole system exists to prevent, and the one it nearly
    // shipped with: `--check` hashes outputs, so a new migration left the
    // manifest byte-identical to what the lock recorded and the check passed
    // while the migration silently never ran.
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    migration(&root, "0001_a.sql", "SELECT 1;");
    assert!(run(&root, &["derive"]).status.success());
    assert!(run(&root, &["derive", "--check"]).status.success());

    migration(&root, "0002_b.sql", "SELECT 2;");
    let out = run(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "a migration added without deriving must fail: {}",
        text(&out)
    );
    assert!(text(&out).contains("inputs changed"), "{}", text(&out));
}

#[test]
fn editing_a_migration_fails_the_check_too() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    migration(&root, "0001_a.sql", "SELECT 1;");
    assert!(run(&root, &["derive"]).status.success());

    migration(&root, "0001_a.sql", "SELECT 2;");
    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "{}", text(&out));
}

#[test]
fn two_migrations_sharing_a_number_fail_to_generate() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    migration(&root, "0001_a.sql", "SELECT 1;");
    migration(&root, "0001_b.sql", "SELECT 2;");

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("two migrations numbered 1"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_sql_file_not_named_like_a_migration_fails_rather_than_being_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    migration(&root, "0001_a.sql", "SELECT 1;");
    migration(&root, "0002-add-index.sql", "SELECT 2;");

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("0002-add-index.sql"), "{}", text(&out));
}

#[test]
fn the_identity_migration_is_picked_up_by_the_manifest() {
    // The two capabilities compose: identity derives the first table, and
    // migrations is what applies it and everything after it.
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    assert!(run(&root, &["add", "identity"]).status.success());
    assert!(run(&root, &["derive"]).status.success());

    assert!(
        manifest(&root).contains("0001_grants"),
        "{}",
        manifest(&root)
    );
}

#[test]
fn a_migration_containing_a_backtick_still_generates_valid_typescript() {
    // The SQL is embedded in a template literal, so a backtick or `${` in a
    // migration would otherwise produce a file that does not parse — or, if it
    // happened to parse, an interpolation.
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    migration(
        &root,
        "0001_quoted.sql",
        "-- a `quoted` identifier and ${not_interpolated}\nSELECT 1;",
    );

    assert!(run(&root, &["derive"]).status.success());
    let m = manifest(&root);
    assert!(m.contains("\\`quoted\\`"), "{m}");
    assert!(m.contains("\\${not_interpolated}"), "{m}");
}

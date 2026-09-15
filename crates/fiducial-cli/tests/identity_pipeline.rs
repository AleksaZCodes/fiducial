//! End-to-end: `fid add identity` → `fid derive` → the grants table.
//!
//! The property that matters: **the schema is a derivation of the identity
//! model**, so the two cannot drift. Its `CHECK` lists are the `Principal`,
//! `Resource` and `Role` variants; adding a role and forgetting the migration
//! fails `fid derive --check` rather than shipping a table that silently
//! rejects the new value.
//!
//! The SQL is *executed* — against real SQLite, which is what D1 runs — in
//! `packages/identity/src/grants.test.js`. These tests cover the generation
//! and the gate.

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

fn product_with_identity(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    assert!(
        run(&root, &["add", "identity"]).status.success(),
        "fid add identity"
    );
    root
}

fn schema(root: &Path) -> String {
    std::fs::read_to_string(root.join("migrations/0001_grants.sql")).unwrap()
}

// ── Installation ──────────────────────────────────────────────────────────────

#[test]
fn installing_identity_adds_the_pipeline_and_seeds_the_table_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    assert!(root.join("pipelines/identity.toml").exists());
    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("[identity]"), "{config}");
    assert!(config.contains("grants"), "{config}");
}

// ── Derivation ───────────────────────────────────────────────────────────────

#[test]
fn derive_writes_the_grants_table() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let sql = schema(&root);
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS grants"), "{sql}");
    assert!(sql.contains("do not edit"), "{sql}");
}

/// The point of deriving rather than shipping a `.sql` template: the CHECK
/// lists are the model's variants, so they cannot fall out of step with it.
#[test]
fn the_check_constraints_are_the_identity_models_variants() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let sql = schema(&root);

    for role in ["viewer", "member", "admin", "owner"] {
        assert!(sql.contains(&format!("'{role}'")), "role {role}: {sql}");
    }
    for kind in ["user", "device", "service"] {
        assert!(sql.contains(&format!("'{kind}'")), "kind {kind}: {sql}");
    }
}

/// Two rules `can()` enforces at runtime, enforced by the schema as well — an
/// unusable row cannot even be written.
#[test]
fn the_schema_refuses_rows_the_rule_would_ignore() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let sql = schema(&root);

    // `anonymous` is never a valid principal_kind.
    let principal_check = sql
        .lines()
        .find(|l| l.contains("principal_kind TEXT"))
        .expect("principal_kind line");
    assert!(
        !principal_check.contains("'anonymous'"),
        "anonymous must not be storable: {principal_check}"
    );

    // An all-zero sentinel id is not an identity.
    assert!(
        sql.contains("principal_id GLOB '*[^0]*'"),
        "zero-sentinel ids must be refused: {sql}"
    );
}

#[test]
fn both_read_paths_are_indexed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let sql = schema(&root);

    assert!(sql.contains("grants_by_principal"), "{sql}");
    assert!(sql.contains("grants_by_resource"), "{sql}");
}

#[test]
fn a_custom_table_name_is_used_everywhere_it_appears() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("table = \"grants\"", "table = \"acl\""),
    )
    .unwrap();

    assert!(run(&root, &["derive"]).status.success());
    let sql = schema(&root);
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS acl"), "{sql}");
    assert!(sql.contains("acl_by_principal"), "{sql}");
    assert!(!sql.contains("grants_by_principal"), "{sql}");
}

// ── Validation ───────────────────────────────────────────────────────────────

/// The table name reaches SQL by interpolation — a parameter cannot bind an
/// identifier — so it is validated rather than trusted.
#[test]
fn a_table_name_that_is_not_an_identifier_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("table = \"grants\"", "table = \"grants; DROP TABLE users\""),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("valid SQL identifier"),
        "{}",
        text(&out)
    );
}

// ── Gate ─────────────────────────────────────────────────────────────────────

#[test]
fn check_catches_a_hand_edited_schema() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let path = root.join("migrations/0001_grants.sql");
    let sql = std::fs::read_to_string(&path).unwrap();
    // The exact drift this pipeline exists to prevent: someone adds a role to
    // the table by hand, and the model never learns about it.
    std::fs::write(
        &path,
        sql.replace("'viewer', 'member'", "'viewer', 'auditor', 'member'"),
    )
    .unwrap();

    let out = run(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "a hand-edited schema must fail --check"
    );
    assert!(text(&out).contains("0001_grants.sql"), "{}", text(&out));
}

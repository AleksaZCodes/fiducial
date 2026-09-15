//! End-to-end: `fid add identity` → `fid derive` → the grants table.
//!
//! The property that matters: **the schema is a derivation of the identity
//! model**, so the two cannot drift. Its `CHECK` lists are the `Principal`,
//! `Resource` and `Role` variants; adding a role and forgetting the migration
//! fails `fid derive --check` rather than shipping a table that silently
//! rejects the new value.
//!
//! The SQL is *executed* rather than only read: against real SQLite, which is
//! what D1 runs, in `packages/identity/src/grants.test.js`; and against a real
//! PostgreSQL server, policies included, in `scripts/verify-postgres.sh`.
//! These tests cover the generation and the gate.

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

fn module(root: &Path) -> String {
    std::fs::read_to_string(root.join("src/identity.generated.ts")).unwrap()
}

/// Rewrite `[identity]` with the given body and re-derive.
fn with_identity(root: &Path, body: &str) -> String {
    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("[identity]", &format!("[identity]\n{body}")),
    )
    .unwrap();
    let out = run(root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    schema(root)
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

// ── Dialect ──────────────────────────────────────────────────────────────────
//
// "SQL" is not one language where this touches it. The first version of this
// pipeline generated `GLOB` unconditionally — SQLite syntax that Postgres
// rejects with a plain syntax error, so a product on Supabase could not run
// its own generated migration.

#[test]
fn the_default_dialect_is_sqlite_because_d1_is() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let sql = schema(&root);

    assert!(sql.contains("Dialect: sqlite"), "{sql}");
    assert!(sql.contains("GLOB"), "{sql}");
    assert!(sql.contains("granted_at     TEXT"), "{sql}");
}

#[test]
fn postgres_gets_postgres_syntax_and_no_sqlite_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    let sql = with_identity(&root, "dialect = \"postgres\"");

    assert!(sql.contains("Dialect: postgres"), "{sql}");
    assert!(sql.contains("principal_id ~ '[^0]'"), "{sql}");
    assert!(sql.contains("granted_at     TIMESTAMPTZ"), "{sql}");
    // The actual defect: SQLite syntax reaching a Postgres server.
    assert!(!sql.contains("GLOB"), "GLOB is SQLite-only: {sql}");
}

/// The dialect is **derived** from the vendor the product already selected.
/// Declaring it again is the copy that goes wrong.
#[test]
fn a_postgres_vendor_implies_the_postgres_dialect_with_nothing_declared() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    assert!(
        !config.contains("dialect"),
        "nothing about a dialect is declared: {config}"
    );
    std::fs::write(
        &path,
        format!("{config}\n[adapters]\ndatabase = \"supabase\"\n"),
    )
    .unwrap();

    assert!(run(&root, &["derive"]).status.success());
    let sql = schema(&root);
    assert!(sql.contains("Dialect: postgres"), "{sql}");
    assert!(!sql.contains("GLOB"), "{sql}");
}

// ── Row-level security ───────────────────────────────────────────────────────

#[test]
fn rls_is_off_unless_asked_for() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    assert!(!schema(&root).contains("ROW LEVEL SECURITY"));
}

#[test]
fn rls_generates_policies_for_all_four_statements() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    let sql = with_identity(&root, "dialect = \"postgres\"\nrls = true");

    assert!(
        sql.contains("ALTER TABLE grants ENABLE ROW LEVEL SECURITY"),
        "{sql}"
    );
    for verb in ["SELECT", "INSERT", "UPDATE", "DELETE"] {
        assert!(
            sql.contains(&format!("FOR {verb}")),
            "no {verb} policy: {sql}"
        );
    }
}

/// The bug this shape exists to avoid: a policy on `grants` that reads
/// `grants` re-enters itself, and Postgres refuses the query at runtime with
/// "infinite recursion detected in policy". The lookup has to sit in a
/// `SECURITY DEFINER` function, which runs as the owner and is therefore not
/// subject to the policies.
#[test]
fn the_administers_check_is_a_security_definer_function_not_an_inline_subquery() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    let sql = with_identity(&root, "dialect = \"postgres\"\nrls = true");

    assert!(sql.contains("SECURITY DEFINER"), "{sql}");
    assert!(sql.contains("grants_administers"), "{sql}");
    // An inherited search_path in a SECURITY DEFINER function lets the caller
    // run its own code as the owner.
    assert!(sql.contains("SET search_path = ''"), "{sql}");
    // Forcing RLS onto the owner would put the helper back inside the loop.
    assert!(!sql.contains("FORCE ROW LEVEL SECURITY"), "{sql}");

    let policies = sql.split("ENABLE ROW LEVEL SECURITY").nth(1).unwrap();
    let body = policies.split("$fn$").nth(2).unwrap(); // after the function
    assert!(
        !body.contains("SELECT 1 FROM"),
        "a policy must not read the table it guards: {body}"
    );
}

#[test]
fn rls_on_sqlite_is_refused_rather_than_silently_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("[identity]", "[identity]\nrls = true"),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("Postgres feature"), "{}", text(&out));
}

#[test]
fn the_current_user_expression_can_be_something_other_than_supabase() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    let sql = with_identity(
        &root,
        "dialect = \"postgres\"\nrls = true\ncurrent_user_sql = \"current_setting('app.user_id', true)\"",
    );

    assert!(
        sql.contains("current_setting('app.user_id', true)"),
        "{sql}"
    );
    assert!(!sql.contains("auth.uid()"), "{sql}");
}

// ── The facts a product would otherwise retype ───────────────────────────────

/// The schema and the code that queries it are generated from the same two
/// declarations. A Postgres table queried with SQLite's `?` placeholders is a
/// syntax error on every statement, found at runtime — so the dialect reaches
/// TypeScript rather than being retyped there.
#[test]
fn the_dialect_and_table_reach_typescript_too() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let ts = module(&root);

    assert!(ts.contains("do not edit"), "{ts}");
    assert!(ts.contains("export const grantsTable = \"grants\""), "{ts}");
    assert!(ts.contains("sqlDialect: SqlDialect = \"sqlite\""), "{ts}");
    assert!(
        ts.contains("new SqlGrantStore(db, grantStoreOptions)"),
        "{ts}"
    );
}

#[test]
fn the_typescript_module_follows_the_dialect_and_the_table_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config
            .replace("[identity]", "[identity]\ndialect = \"postgres\"")
            .replace("table = \"grants\"", "table = \"acl\""),
    )
    .unwrap();
    assert!(run(&root, &["derive"]).status.success());

    let ts = module(&root);
    assert!(ts.contains("sqlDialect: SqlDialect = \"postgres\""), "{ts}");
    assert!(ts.contains("export const grantsTable = \"acl\""), "{ts}");
    // And the migration it describes was generated for the same dialect.
    assert!(schema(&root).contains("Dialect: postgres"));
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

// ── `[identity] storage = "none"` ────────────────────────────────────────────
//
// `fid add identity` installed the rule and the storage together. A product
// wanting `can()` with grants from its own config still got a migration it
// would never apply and a module it would never import. `none` is the word
// this platform already uses for "wired in, reported, does nothing" — see the
// `NONE` adapter, which is a real implementation rather than a placeholder.

/// Set `[identity] storage` in a product that already has the capability.
fn set_storage(root: &Path, value: &str) {
    let path = root.join("fiducial.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let patched = if text.contains("storage =") {
        let mut out = String::new();
        for line in text.lines() {
            if line.trim_start().starts_with("storage =") {
                out.push_str(&format!("storage = \"{value}\"\n"));
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    } else {
        text.replace(
            "[identity]\n",
            &format!("[identity]\nstorage = \"{value}\"\n"),
        )
    };
    std::fs::write(&path, patched).unwrap();
}

#[test]
fn storage_none_derives_no_migration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    set_storage(&root, "none");

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        !root.join("migrations/0001_grants.sql").exists(),
        "storage = none must not derive a table"
    );
    assert!(
        root.join("src/identity.generated.ts").exists(),
        "the rule is still installed, so its module is still generated"
    );
}

#[test]
fn storage_none_still_passes_the_freshness_gate() {
    // The reason this needed a mechanism rather than an `if`: the pipeline's
    // declared outputs are written by the capability author, who cannot know
    // which of them a given product wants. Without skipping the inapplicable
    // one, `--check` demands an artifact the declaration says must not exist
    // and reports a correctly configured product as broken.
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    set_storage(&root, "none");

    assert!(run(&root, &["derive"]).status.success());
    let out = run(&root, &["derive", "--check"]);
    assert!(
        out.status.success(),
        "a product with storage = none is not stale: {}",
        text(&out)
    );
}

#[test]
fn the_generated_module_names_its_storage_kind_either_way() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    let sql = std::fs::read_to_string(root.join("src/identity.generated.ts")).unwrap();
    assert!(sql.contains(r#"grantStorage = "sql""#), "{sql}");

    set_storage(&root, "none");
    assert!(run(&root, &["derive"]).status.success());
    let none = std::fs::read_to_string(root.join("src/identity.generated.ts")).unwrap();
    assert!(none.contains(r#"grantStorage = "none""#), "{none}");
    // Importing it and calling grantStore() is a mistake the declaration
    // forbids, so it fails loudly rather than returning a store over a table
    // that does not exist.
    assert!(none.contains("never"), "{none}");
}

#[test]
fn switching_to_none_stops_tracking_the_migration_and_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    assert!(root.join("migrations/0001_grants.sql").exists());

    set_storage(&root, "none");
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));

    // The file is not deleted — it may already have been applied to a real
    // database — but it stops being tracked, and the note is how a reader
    // finds out they are carrying a migration nothing derives.
    // Matched on the artifacts table header rather than the bare path: the
    // lock also stores the identity pipeline's own file, whose text declares
    // `outputs = ["migrations/0001_grants.sql", …]`. A substring search finds
    // that and passes for the wrong reason.
    let lock = std::fs::read_to_string(root.join("fiducial.lock")).unwrap();
    assert!(
        !lock.contains(r#"[artifacts."migrations/0001_grants.sql"]"#),
        "lock still tracks an artifact no longer derived:\n{lock}"
    );
    assert!(text(&out).contains("no longer derived"), "{}", text(&out));
}

#[test]
fn switching_back_to_sql_derives_the_table_again() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    set_storage(&root, "none");
    assert!(run(&root, &["derive"]).status.success());

    set_storage(&root, "sql");
    assert!(run(&root, &["derive"]).status.success());
    assert!(root.join("migrations/0001_grants.sql").exists());
    assert!(run(&root, &["derive", "--check"]).status.success());
}

#[test]
fn an_unknown_storage_kind_is_rejected_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    set_storage(&root, "mongodb");

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "an unknown storage kind must fail");
    let t = text(&out);
    assert!(t.contains("mongodb"), "{t}");
    assert!(t.contains("sql"), "the message names what is known: {t}");
}

#[test]
fn rls_alongside_storage_none_is_a_contradiction_rather_than_ignored() {
    // Silently ignoring it is how a product ends up believing `rls = true`
    // protects a table that does not exist.
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    set_storage(&root, "none");
    let path = root.join("fiducial.toml");
    let text_before = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text_before.replace("[identity]\n", "[identity]\nrls = true\n"),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("rls"), "{}", text(&out));
}

#[test]
fn the_seeded_table_name_does_not_block_storage_none() {
    // `fid add identity` seeds `table = "grants"`, so it is present in every
    // product and says nothing about intent. Treating it as a contradiction
    // would make `storage = "none"` unreachable without hand-editing.
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_identity(tmp.path());
    let toml = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(toml.contains(r#"table = "grants""#), "seed changed: {toml}");

    set_storage(&root, "none");
    assert!(run(&root, &["derive"]).status.success());
}

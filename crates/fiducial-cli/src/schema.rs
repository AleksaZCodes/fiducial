//! Schema migrations — ordering, idempotency, and drift against a live database.
//!
//! # Not to be confused with `migration.rs`
//!
//! `migration.rs` holds **codemod** migrations: source transformations `fid
//! upgrade` applies to a product's files when the platform API changes. This
//! module holds **schema** migrations: SQL applied to a product's database.
//! Both are "migrations" in ordinary speech and they share almost nothing, so
//! the distinction is in the module names and in every user-facing string.
//!
//! # What grant storage left open
//!
//! `docs/specs/2026-09-15-grant-storage.md` was explicit that it generated the
//! first table and stopped:
//!
//! > **This is not a migrations system.** It generates the first table. The
//! > moment the schema changes you need ordering, idempotency and drift
//! > detection against a live database, and none of that exists yet.
//!
//! Those three, and the reason each is not optional:
//!
//! - **Ordering.** `0002` may reference what `0001` created. Applying them in
//!   directory-listing order works until a filesystem returns them differently,
//!   which is a bug you get in production and not in development.
//! - **Idempotency.** Deploys re-run. A migration applied twice is at best an
//!   error and at worst a duplicate column, and "we only deploy once" is not a
//!   property any system has.
//! - **Drift.** A migration edited *after* it was applied is the dangerous one:
//!   every environment that already ran it has the old schema, every new one
//!   gets the new schema, and nothing is out of date in a way either can see.
//!
//! # Where the facts live
//!
//! `migrations/NNNN_slug.sql` is the declaration. This module derives an
//! ordered manifest with a hash per migration, which `fid derive` writes to
//! `src/migrations.generated.ts`.
//!
//! The manifest exists because the runner has no filesystem. A Worker applying
//! migrations at deploy time cannot read `migrations/`, so the SQL is embedded
//! with it. That also makes the hash meaningful: it is taken over the text that
//! actually shipped.

use std::path::Path;

use anyhow::{bail, Result};

use crate::lock::sha256_hex;

/// The directory a product keeps schema migrations in.
pub const DIR: &str = "migrations";

/// The ledger table recording what has been applied.
///
/// Leading underscore and a `fiducial_` prefix: it shares a namespace with the
/// product's own tables, and a product is entitled to have a table called
/// `migrations`.
pub const LEDGER: &str = "_fiducial_migrations";

/// One migration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    /// Zero-padded sequence number, as written: `0001`.
    pub number: String,
    /// The part after the number: `grants`.
    pub slug: String,
    /// File name, relative to `migrations/`.
    pub file: String,
    /// SHA-256 of the file's bytes.
    pub hash: String,
    /// The SQL itself.
    pub sql: String,
}

impl Migration {
    /// The stable identity of a migration: `0001_grants`.
    ///
    /// The ledger stores this rather than the file name, so renaming
    /// `0001_grants.sql` to `0001_grants.pgsql` does not re-apply it.
    pub fn id(&self) -> String {
        format!("{}_{}", self.number, self.slug)
    }
}

/// Read and validate every migration in `root/migrations/`.
///
/// Returns them in applied order. An empty or absent directory is not an
/// error: a product with no schema is the normal starting state.
pub fn discover(root: &Path) -> Result<Vec<Migration>> {
    let dir = root.join(DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };

    let mut found: Vec<Migration> = Vec::new();
    let mut ignored: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".sql") {
            continue;
        }
        let stem = name.trim_end_matches(".sql");
        let Some((number, slug)) = stem.split_once('_') else {
            ignored.push(name);
            continue;
        };
        if number.is_empty()
            || !number.chars().all(|c| c.is_ascii_digit())
            || slug.is_empty()
            || !slug
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            ignored.push(name);
            continue;
        }

        let sql = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("reading {DIR}/{name}: {e}"))?;
        found.push(Migration {
            number: number.to_string(),
            slug: slug.to_string(),
            file: name,
            hash: sha256_hex(sql.as_bytes()),
            sql,
        });
    }

    // A file that looks like a migration and is not one is reported, never
    // skipped quietly. `0002-add-index.sql` (a hyphen, not an underscore) is a
    // migration nobody notices is not running.
    if !ignored.is_empty() {
        ignored.sort();
        bail!(
            "{DIR}/ holds {} file(s) that are not named `NNNN_slug.sql`:\n  {}\n\n\
             A `.sql` file here that is not a migration is one that silently \
             never runs.\nRename it, or move it out of {DIR}/.",
            ignored.len(),
            ignored.join("\n  ")
        );
    }

    // Sort by number as a string only after padding, so `0009` precedes `0010`
    // and a mixed-width set still orders numerically.
    found.sort_by(|a, b| {
        let (x, y) = (numeric(&a.number), numeric(&b.number));
        x.cmp(&y).then_with(|| a.slug.cmp(&b.slug))
    });

    let mut seen: Vec<(u64, &str)> = Vec::new();
    for m in &found {
        let n = numeric(&m.number);
        if let Some((_, other)) = seen.iter().find(|(existing, _)| *existing == n) {
            bail!(
                "{DIR}/ has two migrations numbered {n}: `{other}` and `{}`.\n\n\
                 Order is the only thing that makes a migration set replayable, \
                 and two files\nwith one number have no order. Renumber the later one.",
                m.file
            );
        }
        seen.push((n, &m.file));
    }

    Ok(found)
}

fn numeric(number: &str) -> u64 {
    number.parse().unwrap_or(u64::MAX)
}

/// The TypeScript manifest a runner consumes.
pub fn render_manifest(migrations: &[Migration]) -> String {
    let mut s = String::new();
    s.push_str(
        "// generated by `fid derive` — do not edit\n\
         //\n\
         // The ordered schema migrations for this product, with the SQL embedded.\n\
         //\n\
         // Embedded rather than read from `migrations/` because the runner has no\n\
         // filesystem: a Worker applying these at deploy time cannot open a file.\n\
         // It also makes each hash mean something — it is taken over the text that\n\
         // actually shipped, so a migration edited after it was applied is\n\
         // detectable rather than merely regrettable.\n\
         //\n\
         //   import { Migrator } from \"@fiducial/adapters/migrate\";\n\
         //   import { migrations } from \"./migrations.generated.js\";\n\
         //\n\
         //   const report = await new Migrator(db, migrations).apply();\n\n\
         import type { Migration } from \"@fiducial/adapters/migrate\";\n\n",
    );
    s.push_str(&format!(
        "/** The ledger table this product's migrations are recorded in. */\nexport const ledgerTable = {:?};\n\n",
        LEDGER
    ));
    s.push_str("export const migrations: readonly Migration[] = [\n");
    for m in migrations {
        s.push_str("  {\n");
        s.push_str(&format!("    id: {:?},\n", m.id()));
        s.push_str(&format!("    file: {:?},\n", m.file));
        s.push_str(&format!("    hash: {:?},\n", m.hash));
        s.push_str(&format!("    sql: {},\n", ts_string(&m.sql)));
        s.push_str("  },\n");
    }
    s.push_str("];\n");
    s
}

/// A SQL body as a TypeScript string literal.
///
/// A template literal keeps the SQL readable in the generated file, which
/// matters because a reader debugging a failed migration will look here first.
/// Backticks, backslashes and `${` are escaped — a migration containing `${`
/// would otherwise be interpolated, which is both a syntax error and, if it
/// happened to parse, an injection.
fn ts_string(sql: &str) -> String {
    let escaped = sql
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${");
    format!("`{escaped}`")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(dir.join(DIR)).unwrap();
        std::fs::write(dir.join(DIR).join(name), body).unwrap();
    }

    #[test]
    fn no_migrations_directory_is_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(discover(tmp.path()).unwrap().is_empty());
    }

    #[test]
    fn migrations_are_ordered_numerically_not_lexically() {
        // The bug this prevents: `0010` sorting before `0009` as strings is
        // fine, but `9` before `10` is not, and a product that drops the
        // padding gets a set that applies in the wrong order.
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "10_ten.sql", "SELECT 10;");
        write(tmp.path(), "9_nine.sql", "SELECT 9;");
        write(tmp.path(), "0001_one.sql", "SELECT 1;");

        let found = discover(tmp.path()).unwrap();
        let ids: Vec<String> = found.iter().map(Migration::id).collect();
        assert_eq!(ids, vec!["0001_one", "9_nine", "10_ten"]);
    }

    #[test]
    fn two_migrations_with_one_number_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "0001_a.sql", "SELECT 1;");
        write(tmp.path(), "0001_b.sql", "SELECT 2;");

        let err = discover(tmp.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("two migrations numbered 1"), "{msg}");
    }

    #[test]
    fn a_sql_file_that_is_not_named_like_a_migration_is_reported() {
        // Silently skipping it is how `0002-add-index.sql` never runs and
        // nobody finds out until the column is missing in production.
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "0001_ok.sql", "SELECT 1;");
        write(tmp.path(), "0002-add-index.sql", "SELECT 2;");

        let err = discover(tmp.path()).unwrap_err();
        assert!(format!("{err:#}").contains("0002-add-index.sql"));
    }

    #[test]
    fn the_hash_is_over_the_file_contents() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "0001_a.sql", "SELECT 1;");
        let first = discover(tmp.path()).unwrap()[0].hash.clone();

        write(tmp.path(), "0001_a.sql", "SELECT 2;");
        let second = discover(tmp.path()).unwrap()[0].hash.clone();
        assert_ne!(first, second, "editing a migration must change its hash");
    }

    #[test]
    fn an_id_survives_a_change_of_extension() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "0001_grants.sql", "SELECT 1;");
        assert_eq!(discover(tmp.path()).unwrap()[0].id(), "0001_grants");
    }

    #[test]
    fn a_template_literal_cannot_be_broken_out_of() {
        // A migration containing a backtick or `${` would otherwise produce a
        // generated file that does not parse — or, worse, one that does.
        let out = ts_string("SELECT `a`, ${b}, 'c\\d';");
        assert!(out.starts_with('`') && out.ends_with('`'));

        // Every interpolation and backtick inside the literal is escaped.
        // Checking for the absence of `${` would be wrong: the escaped form
        // `\${` still contains it, and that form is exactly what is correct.
        let body = &out[1..out.len() - 1];
        for (i, _) in body.match_indices("${") {
            assert!(
                i > 0 && body.as_bytes()[i - 1] == b'\\',
                "an unescaped `${{` would interpolate:\n{out}"
            );
        }
        for (i, _) in body.match_indices('`') {
            assert!(
                i > 0 && body.as_bytes()[i - 1] == b'\\',
                "an unescaped backtick would close the literal:\n{out}"
            );
        }
    }

    #[test]
    fn the_manifest_lists_every_migration_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "0002_b.sql", "SELECT 2;");
        write(tmp.path(), "0001_a.sql", "SELECT 1;");

        let out = render_manifest(&discover(tmp.path()).unwrap());
        let first = out.find("0001_a").unwrap();
        let second = out.find("0002_b").unwrap();
        assert!(first < second, "manifest must preserve order:\n{out}");
        assert!(out.contains(LEDGER));
    }
}

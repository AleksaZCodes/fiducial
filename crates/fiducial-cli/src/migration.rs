//! Codemod migration infrastructure.
//!
//! A **migration** is a versioned, named transformation that `fid upgrade` applies
//! to a product's source files when the platform API changes. Migrations are:
//!
//! - Defined in the platform binary (safe, auditable, no network fetch)
//! - Applied in version order, idempotently
//! - Recorded in `fiducial.lock` under `applied_migrations` so they run exactly once
//!
//! ## Adding a migration
//!
//! Add a new entry to `BUILTIN_MIGRATIONS`. The id format is:
//!   `<capability>/<introduced-in-version>/<slug>`
//!
//! Each `MigrationOp` is a literal search-and-replace applied to all files
//! matching `file_glob`. Matching is currently exact path (no glob expansion);
//! true glob support is a Phase 5 enhancement.

use std::path::Path;

use anyhow::{Context, Result};

// ── Types ─────────────────────────────────────────────────────────────────────

/// A single codemod migration.
pub struct Migration {
    /// Unique id: `capability/version/slug`.
    /// Also used as the key in `fiducial.lock [applied_migrations]`.
    pub id: &'static str,
    /// Human-readable description shown in `fid upgrade` output.
    pub description: &'static str,
    /// Platform version that introduced this migration.
    #[allow(dead_code)]
    pub introduced_in: &'static str,
    /// Operations to apply, in order.
    pub ops: &'static [MigrationOp],
}

/// One search-and-replace operation within a migration.
pub struct MigrationOp {
    /// Repo-relative path of the file to modify (exact match, forward slashes).
    pub file_path: &'static str,
    /// Exact literal string to search for.
    pub search: &'static str,
    /// Replacement string.
    pub replace: &'static str,
}

// ── Built-in migration registry ───────────────────────────────────────────────

/// All first-party codemod migrations, in the order they should be applied.
/// New entries go at the end.
pub static BUILTIN_MIGRATIONS: &[Migration] = &[
    Migration {
        id: "web-next/0.2.0/font-inter-to-geist",
        description: "Rename default font from Inter to Geist (Next.js 15+ scaffold default)",
        introduced_in: "0.2.0",
        ops: &[
            MigrationOp {
                file_path: "apps/web/src/app/layout.tsx",
                search: r#"import { Inter } from "next/font/google""#,
                replace: r#"import { Geist } from "next/font/google""#,
            },
            MigrationOp {
                file_path: "apps/web/src/app/layout.tsx",
                search: "const inter = Inter(",
                replace: "const geist = Geist(",
            },
            MigrationOp {
                file_path: "apps/web/src/app/layout.tsx",
                search: "inter.variable",
                replace: "geist.variable",
            },
        ],
    },
    Migration {
        id: "worker-cloudflare/0.2.0/wrangler-compatibility-date",
        description: "Update Wrangler compatibility_date to 2025-01-01 for Workers v2 pipeline",
        introduced_in: "0.2.0",
        ops: &[MigrationOp {
            file_path: "apps/worker/wrangler.toml",
            search: "compatibility_date = \"2024-01-01\"",
            replace: "compatibility_date = \"2025-01-01\"",
        }],
    },
];

// ── Apply logic ───────────────────────────────────────────────────────────────

/// Result of attempting to apply one migration.
pub struct ApplyResult {
    pub migration_id: String,
    #[allow(dead_code)]
    pub description: String,
    /// Number of files that were actually modified.
    pub files_modified: usize,
    /// Files that the migration touches but which do not exist in this product.
    /// Skipped silently — the capability may not be installed.
    #[allow(dead_code)]
    pub files_skipped: usize,
}

/// Apply all pending migrations (those not listed in `already_applied`) to the
/// product rooted at `root`.
///
/// Returns one `ApplyResult` per migration that was attempted.
/// Errors from individual migrations are propagated — a failed migration is not
/// silently skipped or marked as applied.
pub fn apply_pending(
    root: &Path,
    already_applied: &[String],
    dry_run: bool,
) -> Result<Vec<ApplyResult>> {
    let mut results = Vec::new();
    for m in BUILTIN_MIGRATIONS
        .iter()
        .filter(|m| !already_applied.iter().any(|s| s == m.id))
    {
        results.push(apply_one(root, m, dry_run)?);
    }
    Ok(results)
}

/// List all pending migrations (not yet applied).
pub fn pending(already_applied: &[String]) -> Vec<&'static Migration> {
    BUILTIN_MIGRATIONS
        .iter()
        .filter(|m| !already_applied.iter().any(|s| s == m.id))
        .collect()
}

pub(crate) fn apply_one(root: &Path, migration: &Migration, dry_run: bool) -> Result<ApplyResult> {
    // Group ops by file path so we read/write each file exactly once.
    // The BTreeMap guarantees each path appears as a key exactly once.
    use std::collections::BTreeMap;
    let mut file_ops: BTreeMap<&'static str, Vec<&MigrationOp>> = BTreeMap::new();
    for op in migration.ops {
        file_ops.entry(op.file_path).or_default().push(op);
    }

    let mut files_modified = 0usize;
    let mut files_skipped = 0usize;

    for (file_path, ops) in &file_ops {
        let target = root.join(file_path);
        if !target.exists() {
            files_skipped += 1;
            continue;
        }

        let original = std::fs::read_to_string(&target)
            .with_context(|| format!("reading {}", target.display()))?;

        // Apply all ops for this file in one pass.
        let mut patched = original.clone();
        let mut any_match = false;
        for op in ops {
            if patched.contains(op.search) {
                patched = patched.replace(op.search, op.replace);
                any_match = true;
            }
        }

        if !any_match {
            continue; // Already applied or not applicable.
        }

        if !dry_run {
            std::fs::write(&target, &patched)
                .with_context(|| format!("writing {}", target.display()))?;
        }

        // Each key in file_ops is unique (BTreeMap), so this always increments once per file.
        files_modified += 1;
    }

    Ok(ApplyResult {
        migration_id: migration.id.to_string(),
        description: migration.description.to_string(),
        files_modified,
        files_skipped,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a temporary directory with a file, apply a migration, check output.
    #[test]
    fn apply_font_migration_renames_inter_to_geist() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();

        // Write a layout.tsx that uses the old Inter font (pre-migration state).
        let layout_dir = root.join("apps/web/src/app");
        fs::create_dir_all(&layout_dir).unwrap();
        let layout_path = layout_dir.join("layout.tsx");
        fs::write(
            &layout_path,
            r#"import { Inter } from "next/font/google"
const inter = Inter({ subsets: ["latin"] })
export default function Layout({ children }: { children: React.ReactNode }) {
  return <html className={inter.variable}>{children}</html>
}
"#,
        )
        .unwrap();

        let migration = BUILTIN_MIGRATIONS
            .iter()
            .find(|m| m.id == "web-next/0.2.0/font-inter-to-geist")
            .expect("font migration must exist in registry");

        let result = apply_one(root, migration, false).unwrap();
        assert_eq!(result.files_modified, 1, "layout.tsx should be modified");

        let updated = fs::read_to_string(&layout_path).unwrap();
        assert!(
            updated.contains(r#"import { Geist } from "next/font/google""#),
            "import should be renamed"
        );
        assert!(
            updated.contains("const geist = Geist("),
            "variable should be renamed"
        );
        assert!(
            updated.contains("geist.variable"),
            "usage should be renamed"
        );
        assert!(
            !updated.contains("Inter"),
            "no Inter reference should remain"
        );
    }

    #[test]
    fn already_applied_migration_is_skipped() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();

        // No files at all — migration should be filtered by applied list.
        let already_applied = vec!["web-next/0.2.0/font-inter-to-geist".to_string()];
        let results = apply_pending(root, &already_applied, false).unwrap();

        // Only the worker migration should remain pending (or none if worker file missing)
        for r in &results {
            assert_ne!(
                r.migration_id, "web-next/0.2.0/font-inter-to-geist",
                "applied migration must not rerun"
            );
        }
    }

    #[test]
    fn pending_filters_applied_correctly() {
        let applied = vec!["web-next/0.2.0/font-inter-to-geist".to_string()];
        let pending = pending(&applied);
        assert!(
            pending
                .iter()
                .all(|m| m.id != "web-next/0.2.0/font-inter-to-geist"),
            "applied migration must not appear as pending"
        );
    }

    #[test]
    fn dry_run_does_not_write_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();

        let layout_dir = root.join("apps/web/src/app");
        fs::create_dir_all(&layout_dir).unwrap();
        let layout_path = layout_dir.join("layout.tsx");
        let original = r#"import { Inter } from "next/font/google"
const inter = Inter({ subsets: ["latin"] })
"#;
        fs::write(&layout_path, original).unwrap();

        let migration = BUILTIN_MIGRATIONS
            .iter()
            .find(|m| m.id == "web-next/0.2.0/font-inter-to-geist")
            .unwrap();

        let result = apply_one(root, migration, true /* dry_run */).unwrap();
        // In dry run, files_modified counts how many *would* change.
        assert_eq!(result.files_modified, 1);

        // File must be unchanged.
        let after = fs::read_to_string(&layout_path).unwrap();
        assert_eq!(after, original, "dry run must not write files");
    }
}

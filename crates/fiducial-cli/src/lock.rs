//! `fiducial.lock` — template version tracking.
//!
//! Each scaffolded file is recorded here with the platform version it came from
//! and a SHA-256 of its content at scaffold time. `fid upgrade` uses this to
//! perform a 3-way merge: diff platform@lock_version..platform@new_version,
//! apply to local, surface conflicts.
//!
//! **Never edit this file by hand.** It is updated exclusively by `fid new` and
//! `fid upgrade`. Guard rules enforce this.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub const LOCK_FILE: &str = "fiducial.lock";

/// The on-disk format of `fiducial.lock`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Lock {
    /// Schema version — bumped when the format changes.
    pub version: u32,
    /// Template file records, keyed by repo-relative path.
    #[serde(default)]
    pub templates: BTreeMap<String, TemplateRecord>,
    /// Codemod migrations that have already been applied to this product.
    #[serde(default)]
    pub applied_migrations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRecord {
    /// Platform release that produced this template (e.g. "0.1.0").
    pub source_version: String,
    /// Lowercase hex SHA-256 of the file content at scaffold time.
    pub hash: String,
    /// Expanded template content at scaffold/upgrade time.
    /// Used as the merge base by `fid upgrade` for 3-way template merge.
    /// `None` for records written by pre-Phase-4 versions of `fid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_content: Option<String>,
}

impl Lock {
    /// Create a fresh, empty lock at schema version 1.
    pub fn new() -> Self {
        Self {
            version: 1,
            templates: BTreeMap::new(),
            applied_migrations: Vec::new(),
        }
    }

    /// Record a template file. `path` is repo-relative (forward slashes).
    /// `content` is the expanded file content (placeholders already substituted).
    pub fn record(&mut self, path: impl Into<String>, content: &[u8], source_version: &str) {
        let hash = sha256_hex(content);
        let base_content = String::from_utf8(content.to_vec()).ok();
        self.templates.insert(
            path.into(),
            TemplateRecord {
                source_version: source_version.into(),
                hash,
                base_content,
            },
        );
    }

    /// Load from a path.
    pub fn load(path: &Path) -> Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    /// Write to a path.
    pub fn save(&self, path: &Path) -> Result<()> {
        let raw = toml::to_string_pretty(self).context("serialising fiducial.lock")?;
        let header = "# fiducial.lock — template version tracking.\n\
                      # Do not edit by hand. Updated by `fid upgrade`.\n\n";
        std::fs::write(path, format!("{header}{raw}"))
            .with_context(|| format!("writing {}", path.display()))
    }

    /// Verify all recorded template files against their stored hashes.
    /// Returns a list of (path, problem) for any that differ.
    pub fn verify(&self, root: &Path) -> Vec<(PathBuf, String)> {
        let mut issues = Vec::new();
        for (rel, record) in &self.templates {
            let abs = root.join(rel);
            match std::fs::read(&abs) {
                Err(e) => {
                    issues.push((abs, format!("missing: {e}")));
                }
                Ok(content) => {
                    let actual = sha256_hex(&content);
                    if actual != record.hash {
                        issues.push((
                            abs,
                            format!(
                                "modified since scaffold (lock: {}, file: {})",
                                &record.hash[..8],
                                &actual[..8],
                            ),
                        ));
                    }
                }
            }
        }
        issues
    }
}

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_stores_base_content() {
        let mut lock = Lock::new();
        let content = b"Hello, world!\n";
        lock.record("some/file.txt", content, "0.1.0");

        let record = lock.templates.get("some/file.txt").unwrap();
        assert_eq!(record.source_version, "0.1.0");
        assert_eq!(record.hash, sha256_hex(content));
        assert_eq!(record.base_content.as_deref(), Some("Hello, world!\n"));
    }

    #[test]
    fn round_trip_serialise_with_base_content() {
        let mut lock = Lock::new();
        lock.record("AGENTS.md", b"# Product\nPlatform 0.1.0\n", "0.1.0");

        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("fiducial.lock");
        lock.save(&path).unwrap();

        let loaded = Lock::load(&path).unwrap();
        let record = loaded.templates.get("AGENTS.md").unwrap();
        assert_eq!(
            record.base_content.as_deref(),
            Some("# Product\nPlatform 0.1.0\n")
        );
    }

    #[test]
    fn applied_migrations_round_trip() {
        let mut lock = Lock::new();
        lock.applied_migrations
            .push("web-next/0.2.0/font-inter-to-geist".to_string());

        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("fiducial.lock");
        lock.save(&path).unwrap();

        let loaded = Lock::load(&path).unwrap();
        assert_eq!(loaded.applied_migrations.len(), 1);
        assert_eq!(
            loaded.applied_migrations[0],
            "web-next/0.2.0/font-inter-to-geist"
        );
    }

    /// Verify that the 3-way merge primitive (diffy) surfaces conflicts correctly.
    /// This is the core mechanism behind `fid upgrade` template merging.
    #[test]
    fn three_way_merge_conflict_is_surfaced() {
        let base = "Line A\nLine B\nLine C\n";
        let ours = "Line A\nLine B — local edit\nLine C\n"; // local changed B
        let theirs = "Line A\nLine B — upstream change\nLine C\n"; // upstream also changed B

        // Both sides changed the same line → conflict.
        let result = diffy::merge(base, ours, theirs);
        assert!(
            result.is_err(),
            "conflicting edits on the same line must produce a conflict"
        );
        let conflict_text = result.unwrap_err();
        assert!(
            conflict_text.contains("<<<<<<<"),
            "conflict markers must be present"
        );
    }

    #[test]
    fn three_way_merge_non_overlapping_changes_succeed() {
        let base = "Line A\nLine B\nLine C\n";
        let ours = "Line A — local\nLine B\nLine C\n"; // local changed A
        let theirs = "Line A\nLine B\nLine C — upstream\n"; // upstream changed C

        // Different lines changed on each side → clean merge.
        let merged = diffy::merge(base, ours, theirs).expect("non-overlapping merge must succeed");
        assert!(merged.contains("Line A — local"), "local edit preserved");
        assert!(
            merged.contains("Line C — upstream"),
            "upstream change applied"
        );
    }

    #[test]
    fn three_way_merge_local_unmodified_takes_upstream() {
        let base = "Hello {{name}}, version {{version}}\n";
        let ours = base; // user did not touch this file
        let theirs = "Hello {{name}}, platform version {{version}} (updated)\n";

        let merged =
            diffy::merge(base, ours, theirs).expect("unmodified local must auto-take upstream");
        assert_eq!(merged, theirs);
    }
}

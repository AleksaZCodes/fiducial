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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRecord {
    /// Platform release that produced this template (e.g. "0.1.0").
    pub source_version: String,
    /// Lowercase hex SHA-256 of the file content at scaffold time.
    pub hash: String,
}

impl Lock {
    /// Create a fresh, empty lock at schema version 1.
    pub fn new() -> Self {
        Self {
            version: 1,
            templates: BTreeMap::new(),
        }
    }

    /// Record a template file. `path` is repo-relative (forward slashes).
    pub fn record(&mut self, path: impl Into<String>, content: &[u8], source_version: &str) {
        let hash = sha256_hex(content);
        self.templates.insert(
            path.into(),
            TemplateRecord {
                source_version: source_version.into(),
                hash,
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

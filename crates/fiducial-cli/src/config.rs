//! `fiducial.toml` — product configuration.
//!
//! A product declares its capabilities, its spine preference, and its guard
//! rules here. `fid` reads this file to decide what to check, what to allow,
//! and what to forbid.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "fiducial.toml";

/// The full `fiducial.toml` schema.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub product: Product,
    #[serde(default)]
    pub spine: Spine,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub guard: Guard,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    /// Snake-case product identifier; also used as the crate/package name prefix.
    pub name: String,
    /// Semver version of the product.
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "0.1.0".into()
}

/// Whether this product uses the L0 Rust spine (fiducial-core etc.).
/// Pure web products may leave this false.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Spine {
    #[serde(default)]
    pub enabled: bool,
}

/// Capability declarations.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Capabilities {
    /// Installed capability identifiers, e.g. ["web-next", "firmware-rp2040"].
    #[serde(default)]
    pub enabled: Vec<String>,
}

/// Guard configuration — what `fid guard-check` enforces in this product.
#[derive(Debug, Serialize, Deserialize)]
pub struct Guard {
    /// Named rule sets that are active.
    #[serde(default = "default_rules")]
    pub rules: Vec<String>,
}

impl Default for Guard {
    fn default() -> Self {
        Self {
            rules: default_rules(),
        }
    }
}

fn default_rules() -> Vec<String> {
    vec![
        "no-direct-main-push".into(),
        "no-hand-edit-generated".into(),
        "no-unpinned-cli-fetch".into(),
    ]
}

impl Config {
    /// Load from a specific path.
    pub fn load(path: &Path) -> Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    /// Walk up from `start` to find the product root (directory containing `fiducial.toml`).
    pub fn find_root(start: &Path) -> Result<PathBuf> {
        let mut dir = start.to_path_buf();
        loop {
            if dir.join(CONFIG_FILE).exists() {
                return Ok(dir);
            }
            if !dir.pop() {
                bail!(
                    "no `fiducial.toml` found in `{}` or any parent directory.\n\
                     Run `fid new <name>` to scaffold a product.",
                    start.display()
                );
            }
        }
    }
}

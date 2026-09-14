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
    /// Skipped when the product is not localized, so a non-localized product's
    /// `fiducial.toml` carries no empty `[i18n]` block.
    #[serde(default, skip_serializing_if = "I18n::is_empty")]
    pub i18n: I18n,
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

/// `[i18n]` — the locales this product ships.
///
/// Declared rather than inferred. `default` is **not** `locales[0]`: which
/// language a product falls back to is a decision, and inferring it from list
/// order is exactly the kind of implicit fact this platform exists to delete.
///
/// Empty `locales` means the product is not localized. That is a valid state
/// for a CLI or a firmware image — but `fid new` seeds `sr` and `en`, because a
/// product that can be built monolingual will be, and principle 1c says
/// monolingual is a state you pass through before the first commit.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct I18n {
    #[serde(default)]
    pub locales: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Directory holding `<locale>.json`, relative to the product root.
    ///
    /// `serde(default)` fills this when *reading*; without the matching skip it
    /// round-tripped a `Default::default()` value back out as `""`, which then
    /// failed to resolve on the next read.
    #[serde(
        default = "default_messages_dir",
        skip_serializing_if = "is_default_messages_dir"
    )]
    pub messages_dir: String,
}

fn is_default_messages_dir(dir: &str) -> bool {
    dir.is_empty() || dir == default_messages_dir()
}

fn default_messages_dir() -> String {
    "messages".to_string()
}

impl I18n {
    /// True when the product declares no locales.
    pub fn is_empty(&self) -> bool {
        self.locales.is_empty()
    }

    /// The messages directory, defaulted for a value that round-tripped empty.
    pub fn messages_dir(&self) -> &str {
        if self.messages_dir.is_empty() {
            "messages"
        } else {
            &self.messages_dir
        }
    }

    /// The fallback locale, or an error naming what to add.
    pub fn default_locale(&self) -> Result<&str> {
        match (&self.default, self.locales.first()) {
            (Some(d), _) => {
                if self.locales.iter().any(|l| l == d) {
                    Ok(d)
                } else {
                    anyhow::bail!(
                        "[i18n] default = \"{d}\" is not in locales = {:?}.\n\
                         The fallback must be one of the locales the product ships.",
                        self.locales
                    )
                }
            }
            (None, Some(_)) => anyhow::bail!(
                "[i18n] declares locales but no `default`.\n\
                 Add `default = \"<locale>\"` — which language a reader falls back to \n\
                 is a decision, not the first entry in a list."
            ),
            (None, None) => anyhow::bail!("[i18n] declares no locales"),
        }
    }
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

/// Guard rules every product starts with.
///
/// Only names `guard::rule_by_name` resolves belong here — asserted by
/// `every_declared_guard_rule_is_implemented`. This list used to carry
/// `no-hand-edit-generated`, which has never existed: the guard reads Bash
/// commands, and hand-editing a generated file happens through an editor, not
/// through a shell. Naming it bought nothing and told `fid dash` to report the
/// product as guarded by three rules when one worked.
fn default_rules() -> Vec<String> {
    vec!["no-direct-main-push".into(), "no-unpinned-cli-fetch".into()]
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

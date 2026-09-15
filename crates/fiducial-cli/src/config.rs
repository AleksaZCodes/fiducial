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
    /// `[adapters]` — one vendor per contract.
    #[serde(default, skip_serializing_if = "Adapters::is_empty")]
    pub adapters: Adapters,
    /// Skipped when the product declares no brand, so a product that has not
    /// added the `brand` capability carries no empty `[brand]` block.
    #[serde(default, skip_serializing_if = "Brand::is_empty")]
    pub brand: Brand,
    /// `[deploy]` — the facts a deploy target needs that nothing can derive.
    #[serde(default, skip_serializing_if = "Deploy::is_empty")]
    pub deploy: Deploy,
    /// `[identity]` — where this product keeps its permission rows.
    #[serde(default, skip_serializing_if = "Identity::is_empty")]
    pub identity: Identity,
}

/// Which SQL dialect the grants table is generated for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDialect {
    Sqlite,
    Postgres,
}

impl SqlDialect {
    pub fn as_str(&self) -> &'static str {
        match self {
            SqlDialect::Sqlite => "sqlite",
            SqlDialect::Postgres => "postgres",
        }
    }
}

/// `[identity]` — what grant storage cannot derive from the model.
///
/// The grants table's *shape* follows from the identity model: its `CHECK`
/// constraints are the `Principal`, `Resource` and `Role` variants. Three
/// things do not follow from it, and live here.
///
/// **The dialect is derived, not declared** — by default. `[adapters]
/// database` already names the vendor, and the vendor implies the dialect:
/// `d1` is SQLite, `supabase`/`neon`/`postgres` are Postgres. Declaring it
/// again would be the copy that goes wrong, so `dialect` is an *override* for
/// the case the platform cannot see (a product pointing `none` at a real
/// database of its own).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Identity {
    /// Table holding the permission rows. Defaults to `grants`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub table: String,
    /// `sqlite` | `postgres`. Empty means "derive it from `[adapters]
    /// database`", which is the normal case.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub dialect: String,
    /// Generate row-level security policies on the grants table.
    ///
    /// Postgres only — SQLite has no RLS. Off by default: a policy that
    /// cannot identify the current user locks the table rather than
    /// protecting it, and whether the database *can* identify them depends on
    /// how the product connects (see `current_user_sql`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rls: bool,
    /// The SQL expression yielding the current user's id, for RLS.
    ///
    /// Defaults to Supabase's `auth.uid()`. A product connecting with a
    /// per-request role instead would use something like
    /// `current_setting('app.user_id', true)`. Normalized to this platform's
    /// hex form by the generator, because `auth.uid()` returns a hyphenated
    /// UUID and `principal_id` holds 32 hex characters.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_user_sql: String,
}

impl Identity {
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
            && self.dialect.is_empty()
            && !self.rls
            && self.current_user_sql.is_empty()
    }

    /// The table name to generate against, defaulted.
    pub fn table_name(&self) -> &str {
        if self.table.is_empty() {
            "grants"
        } else {
            &self.table
        }
    }

    /// The expression that yields the current user id in SQL, defaulted.
    pub fn current_user_expr(&self) -> &str {
        if self.current_user_sql.is_empty() {
            "auth.uid()"
        } else {
            &self.current_user_sql
        }
    }

    /// The dialect to generate: declared if overridden, otherwise **derived
    /// from the database vendor the product already selected**.
    ///
    /// An unknown or absent vendor falls back to SQLite, which is what `fid
    /// new` scaffolds toward and what D1 — the one real vendor — runs.
    pub fn resolve_dialect(&self, adapters: &Adapters) -> Result<SqlDialect> {
        if !self.dialect.is_empty() {
            return match self.dialect.as_str() {
                "sqlite" => Ok(SqlDialect::Sqlite),
                "postgres" | "postgresql" => Ok(SqlDialect::Postgres),
                other => bail!(
                    "[identity] dialect = \"{other}\" is not a SQL dialect this \
                     generates. Known: sqlite, postgres. Leave it empty to derive \
                     the dialect from [adapters] database."
                ),
            };
        }
        Ok(match adapters.get("database") {
            Some("supabase") | Some("neon") | Some("postgres") => SqlDialect::Postgres,
            _ => SqlDialect::Sqlite,
        })
    }

    /// Everything the pipeline needs, each problem named.
    pub fn validate(&self, adapters: &Adapters) -> Result<()> {
        let name = self.table_name();
        let ok = !name.is_empty()
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !ok {
            bail!(
                "[identity] table = \"{name}\" is not a valid SQL identifier \
                 (letters, digits and underscore; not starting with a digit)."
            );
        }

        let dialect = self.resolve_dialect(adapters)?;
        if self.rls && dialect != SqlDialect::Postgres {
            bail!(
                "[identity] rls = true, but the dialect resolves to {}. Row-level \
                 security is a Postgres feature; SQLite has none. Either select a \
                 Postgres vendor in [adapters] database, or set rls = false and \
                 rely on `can()` in the application.",
                dialect.as_str()
            );
        }
        Ok(())
    }
}

/// `[deploy]` — what a deploy config needs that `[adapters]` cannot imply.
///
/// The split is the whole point. Which bindings a Worker needs **is** derivable
/// — `[adapters]` already says `database = "d1"`, and the `D1Database` adapter
/// already reads a fixed `env.DB`. What is not derivable is everything only the
/// Cloudflare account knows: the database's id, the bucket's name, the route.
/// Those live here; the bindings are generated around them.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Deploy {
    /// Worker name. Defaults to `<product>-worker` when left empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Worker entrypoint, relative to the worker app.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub main: String,
    /// Wrangler `compatibility_date`. Pinned, never "today": a date that moves
    /// changes runtime behaviour under a product that did not ask it to.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub compatibility_date: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatibility_flags: Vec<String>,
    /// D1 database id — from `wrangler d1 create`. Only the account knows it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub d1_database_id: String,
    /// D1 database name, defaulting to the product name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub d1_database_name: String,
    /// R2 bucket name — from `wrangler r2 bucket create`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub r2_bucket_name: String,
    /// Queue name — from `wrangler queues create`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub queue_name: String,
    /// Route or custom domain this Worker answers on. Optional: a Worker on
    /// its `workers.dev` subdomain declares none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub route: String,
}

impl Deploy {
    /// True when the product declares no deploy target at all.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
            && self.main.is_empty()
            && self.compatibility_date.is_empty()
            && self.compatibility_flags.is_empty()
            && self.d1_database_id.is_empty()
            && self.r2_bucket_name.is_empty()
            && self.queue_name.is_empty()
            && self.route.is_empty()
    }

    /// The facts the pipeline needs, each named when it is missing.
    ///
    /// `compatibility_date` is the one field with no sensible default: every
    /// other gap either has a product-derived fallback or belongs to a vendor
    /// the product did not select. An id placeholder that survived to a real
    /// derive is named here rather than written into a `wrangler.toml` that
    /// would fail at `wrangler deploy` with a far less obvious message.
    pub fn validate(&self, adapters: &Adapters) -> Result<()> {
        if self.compatibility_date.is_empty() {
            bail!(
                "[deploy] compatibility_date is missing. Pin one (e.g. \
                 \"2025-01-01\") — a date that moves changes runtime behaviour \
                 under a product that did not ask it to."
            );
        }

        // Only demand the ids for vendors this product actually selected.
        let needs: [(&str, &str, &str, &str); 3] = [
            (
                "database",
                "d1",
                "d1_database_id",
                "wrangler d1 create <name>",
            ),
            (
                "storage",
                "r2",
                "r2_bucket_name",
                "wrangler r2 bucket create <name>",
            ),
            (
                "queue",
                "cloudflare-queues",
                "queue_name",
                "wrangler queues create <name>",
            ),
        ];
        for (contract, vendor, field, how) in needs {
            if adapters.get(contract) != Some(vendor) {
                continue;
            }
            let value = match field {
                "d1_database_id" => &self.d1_database_id,
                "r2_bucket_name" => &self.r2_bucket_name,
                _ => &self.queue_name,
            };
            if value.is_empty() || value.starts_with("REPLACE_") {
                bail!(
                    "[deploy] {field} is required because [adapters] {contract} = \
                     \"{vendor}\". Get it from `{how}`."
                );
            }
        }
        Ok(())
    }
}

/// `[adapters]` — which implementation satisfies each contract.
///
/// A plain map rather than a struct with a field per contract: the contract set
/// is declared once in `crate::adapter::CONTRACTS`, and a struct here would be
/// a second copy of it that drifts the first time one is added.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Adapters {
    #[serde(flatten)]
    pub selected: std::collections::BTreeMap<String, String>,
}

impl Adapters {
    /// True when the product selects no adapters at all.
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    /// The vendor chosen for a contract, if the product chose one.
    pub fn get(&self, contract: &str) -> Option<&str> {
        self.selected.get(contract).map(|s| s.as_str())
    }
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
/// for a CLI or a firmware image, and `fid new --locales none` asks for it — but
/// `fid new` seeds the `i18n` capability's own locales otherwise, because a product that can be
/// built monolingual will be, and principle 1c says monolingual is a state you
/// pass through before the first commit.
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

/// `[brand]` — the one declaration every brand-derived artifact reads.
///
/// Legal name, trading name, domain and contact email have no platform
/// default — they are facts about one product, not a decision this platform
/// can make on a product's behalf. The `brand` capability seeds them as
/// placeholder text precisely so `is_empty` is false and the pipeline it
/// installs does not fail on the very next `fid derive`; `validate` then
/// catches whichever placeholder a product forgot to replace.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Brand {
    #[serde(default)]
    pub legal_name: String,
    #[serde(default)]
    pub trading_name: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub contact_email: String,
    #[serde(
        default = "default_primary_color",
        skip_serializing_if = "is_default_primary_color"
    )]
    pub primary_color: String,
    #[serde(
        default = "default_background_color",
        skip_serializing_if = "is_default_background_color"
    )]
    pub background_color: String,
}

fn default_primary_color() -> String {
    "#0EA5E9".to_string()
}

fn is_default_primary_color(c: &str) -> bool {
    c.is_empty() || c == default_primary_color()
}

fn default_background_color() -> String {
    "#0B1120".to_string()
}

fn is_default_background_color(c: &str) -> bool {
    c.is_empty() || c == default_background_color()
}

impl Brand {
    /// True when the product declares no brand at all.
    pub fn is_empty(&self) -> bool {
        self.legal_name.is_empty()
            && self.trading_name.is_empty()
            && self.domain.is_empty()
            && self.contact_email.is_empty()
    }

    /// The colour to render with, defaulted for a value that round-tripped empty.
    pub fn primary_color(&self) -> &str {
        if self.primary_color.is_empty() {
            "#0EA5E9"
        } else {
            &self.primary_color
        }
    }

    pub fn background_color(&self) -> &str {
        if self.background_color.is_empty() {
            "#0B1120"
        } else {
            &self.background_color
        }
    }

    /// Every fact the brand pipeline needs is present and well-formed.
    ///
    /// Named field by field, like `[i18n] default`'s error, rather than one
    /// generic "invalid `[brand]`" — an agent fixing this reads it once.
    pub fn validate(&self) -> Result<()> {
        let mut missing = Vec::new();
        if self.legal_name.trim().is_empty() {
            missing.push("legal_name");
        }
        if self.trading_name.trim().is_empty() {
            missing.push("trading_name");
        }
        if self.domain.trim().is_empty() {
            missing.push("domain");
        }
        if self.contact_email.trim().is_empty() {
            missing.push("contact_email");
        }
        if !missing.is_empty() {
            bail!(
                "[brand] is missing: {}.\n\
                 Every brand-derived artifact reads these — fill them in before running `fid derive`.",
                missing.join(", ")
            );
        }
        for (field, value) in [
            ("primary_color", self.primary_color()),
            ("background_color", self.background_color()),
        ] {
            if !is_hex_color(value) {
                bail!("[brand] {field} = \"{value}\" is not a `#RRGGBB` hex colour.");
            }
        }
        Ok(())
    }
}

/// `#` followed by exactly six hex digits.
fn is_hex_color(s: &str) -> bool {
    let Some(digits) = s.strip_prefix('#') else {
        return false;
    };
    digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit())
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

#[cfg(test)]
mod brand_tests {
    use super::Brand;

    fn filled() -> Brand {
        Brand {
            legal_name: "Example LLC".into(),
            trading_name: "Example".into(),
            domain: "example.com".into(),
            contact_email: "hello@example.com".into(),
            primary_color: String::new(),
            background_color: String::new(),
        }
    }

    #[test]
    fn a_product_with_no_brand_declared_is_empty() {
        assert!(Brand::default().is_empty());
    }

    #[test]
    fn a_declared_brand_is_not_empty() {
        assert!(!filled().is_empty());
    }

    #[test]
    fn empty_colors_round_trip_to_the_platform_default() {
        let brand = filled();
        assert_eq!(brand.primary_color(), "#0EA5E9");
        assert_eq!(brand.background_color(), "#0B1120");
        assert!(brand.validate().is_ok());
    }

    #[test]
    fn validate_names_every_missing_field_at_once() {
        let err = Brand::default().validate().unwrap_err();
        let msg = format!("{err:#}");
        for field in ["legal_name", "trading_name", "domain", "contact_email"] {
            assert!(msg.contains(field), "{msg}");
        }
    }

    #[test]
    fn a_malformed_colour_is_named_rather_than_a_generic_error() {
        let mut brand = filled();
        brand.primary_color = "blue".into();
        let err = brand.validate().unwrap_err();
        assert!(format!("{err:#}").contains("primary_color"));
    }
}

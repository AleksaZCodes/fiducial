//! Capability model and built-in registry.
//!
//! A **capability** is a named, versioned extension to the platform. It may
//! contribute any of:
//!
//! - Template files written into a product on `fid add <capability>`
//! - Guard rules that activate when the capability is installed
//! - A `SKILL.md` that agents load on session start (mandatory per §3.3)
//! - Crates / packages / pipelines / migrations (future phases)
//!
//! The minimum viable capability is a single `SKILL.md`. Everything else is
//! optional and added when the capability actually grows into it.
//!
//! ## Capability identifiers
//!
//! Capabilities are referenced by a kebab-case slug:
//!   `web-next`, `firmware-rp2040`, `tauri`, `worker-cloudflare`, …
//!
//! First-party capabilities are defined here. Third-party capabilities (Phase 5+)
//! resolve from git repos using the same source model as the Claude Code plugin
//! marketplace.

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::config::{Config, CONFIG_FILE};
use crate::lock::Lock;

pub const PLATFORM_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Capability definition ────────────────────────────────────────────────────

pub mod manifest;
pub mod source;

pub use manifest::{Capability, Declaration, FileEntry, Source};

/// Every built-in capability's files, embedded by `build.rs` from
/// `capabilities/<id>/`.
///
/// Bytes, not parsed capabilities: they go through the same
/// `manifest::derive` a third-party capability does, so there is one
/// definition of what a capability is rather than one per source.
mod builtin_files {
    include!(concat!(env!("OUT_DIR"), "/capabilities.rs"));
}

/// The built-in capabilities, derived once on first use.
///
/// `OnceLock` rather than a `static` literal because a `Capability` owns its
/// strings. Deriving at startup rather than at compile time is the price of
/// having one derivation; it is a few hundred microseconds against a registry
/// that could otherwise disagree with the directory it came from.
pub fn builtins() -> &'static [Capability] {
    static BUILTINS: std::sync::OnceLock<Vec<Capability>> = std::sync::OnceLock::new();
    BUILTINS.get_or_init(|| {
        builtin_files::BUILTIN_FILES
            .iter()
            .map(|(id, files)| {
                let map: std::collections::BTreeMap<String, String> = files
                    .iter()
                    .map(|(p, c)| ((*p).to_string(), (*c).to_string()))
                    .collect();
                manifest::derive(id, &map, Source::Builtin).unwrap_or_else(|e| {
                    // A built-in that does not derive is a broken build, not a
                    // runtime condition: the directory ships inside the binary.
                    panic!("built-in capability `{id}` does not derive: {e:#}")
                })
            })
            .collect()
    })
}

/// The locale set a capability's `[i18n]` seed declares.
///
/// `fid new --locales` defaults to whatever the `i18n` capability seeds, rather
/// than to a constant of its own. Two copies of "the locales a product is born
/// with" would drift the first time either changed — and one of them lives in
/// a manifest a third party could replace, which is the whole point of the
/// capability owning its own defaults.
///
/// Returns `(locales, default)` when the capability declares them.
pub fn seeded_locales() -> Option<(Vec<String>, String)> {
    let cap = find("i18n")?;
    let seed = cap.declarations.iter().find_map(|d| match d {
        Declaration::ConfigBlock { name, seed } if name == "i18n" => seed.as_ref(),
        _ => None,
    })?;
    let table = seed.as_table()?;
    let locales = table
        .get("locales")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let default = table.get("default")?.as_str()?.to_string();
    if locales.is_empty() {
        return None;
    }
    Some((locales, default))
}

/// Look up a built-in capability by id.
pub fn find(id: &str) -> Option<&'static Capability> {
    builtins().iter().find(|c| c.id == id)
}

// ── Install ──────────────────────────────────────────────────────────────────

/// Install a capability into the product rooted at `root`.
///
/// 1. Write template files (tracked in `fiducial.lock`).
/// 2. Write `SKILL.md` to `.fiducial/skills/<capability-id>.md`, plus a pointer
///    at `.claude/skills/<capability-id>.md` for Claude Code's auto-discovery.
/// 3. Add capability id to `fiducial.toml [capabilities]`.
/// 4. Merge capability guard rules into `fiducial.toml [guard]`.
/// Refuse to install a capability whose prerequisites are not present.
///
/// The case this exists for: **anything deriving user-visible copy depends on
/// `i18n`.** Copy is a fact, a fact has one declaration and one derivation per
/// locale (MISSION.md 1c), and a capability that writes prose without the
/// locale machinery underneath it produces a monolingual artifact that looks
/// finished. Nothing fails. The product discovers it when somebody who reads
/// the other language opens the page — which is the precise failure 1c exists
/// to prevent, arriving by the one route 1c does not cover.
///
/// So it is refused at install, where the fix is one more `fid add`, rather
/// than surfaced later as a page in the wrong language.
fn require_capabilities(cap: &Capability, root: &Path) -> Result<()> {
    if cap.requires_capabilities.is_empty() {
        return Ok(());
    }
    let config = Config::load(&root.join(crate::config::CONFIG_FILE))
        .context("reading fiducial.toml to check capability prerequisites")?;
    let missing: Vec<&String> = cap
        .requires_capabilities
        .iter()
        .filter(|id| !config.capabilities.enabled.contains(id))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let list = missing
        .iter()
        .map(|id| format!("`{id}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let adds = missing
        .iter()
        .map(|id| format!("fid add capability {id}"))
        .collect::<Vec<_>>()
        .join("\n  ");
    bail!(
        "[{}] requires {list}, which this product does not have.\n\n  \
         {adds}\n\n\
         Install the prerequisite first, then add this capability again.",
        cap.id
    );
}

pub fn install(cap: &Capability, root: &Path, product_name: &str) -> Result<()> {
    println!("✦ fid add {} — installing into {}", cap.id, root.display());

    require_capabilities(cap, root)?;

    let mut lock = load_or_new_lock(root)?;

    // 1. Declarations, pipelines, then templates.
    //
    // Declarations first: a pipeline whose declaration is not yet on disk
    // fails on the next `fid derive`, and install order is the cheapest place
    // to make that impossible.
    let mut files: Vec<&FileEntry> = Vec::new();
    for decl in &cap.declarations {
        if let Declaration::File(f) = decl {
            files.push(f);
        }
    }
    files.extend(cap.pipelines.iter());
    files.extend(cap.templates.iter());

    // A declared directory is created empty. The product fills it — that is
    // what makes it a declaration of the product's rather than the
    // capability's — but leaving it absent means the first `fid derive` reads
    // a directory that is not there, and the reader has nowhere obvious to put
    // the first file.
    for decl in &cap.declarations {
        if let Declaration::Directory { path } = decl {
            let abs = root.join(path);
            std::fs::create_dir_all(&abs)
                .with_context(|| format!("creating declared directory `{path}`"))?;
            println!("  wrote  {path}/");
        }
    }

    for FileEntry { path: rel, content } in files {
        let expanded = content
            .replace("{{name}}", product_name)
            .replace("{{version}}", PLATFORM_VERSION);
        write_file(root, rel, &expanded)?;
        lock.record(
            rel.replace('\\', "/"),
            expanded.as_bytes(),
            PLATFORM_VERSION,
        );
    }

    // 2. SKILL.md → a vendor-neutral path, plus a pointer for Claude Code.
    //
    // The instructions used to be written only to `.claude/skills/`, which meant
    // a Codex, Copilot or Cursor session in this product could install the `eda`
    // capability and receive no instructions for using it. The content is
    // ordinary Markdown and portable; only *discovery* is vendor-specific.
    //
    // So the content lives once, at a neutral path that `AGENTS.md` points every
    // agent at, and Claude Code gets a short pointer file so its automatic skill
    // discovery still works. The pointer carries no instructions of its own —
    // two copies of the content would be the duplication this platform exists
    // to delete.
    let skill_content = cap
        .skill_md
        .replace("{{name}}", product_name)
        .replace("{{version}}", PLATFORM_VERSION);
    write_file(
        root,
        &format!(".fiducial/skills/{}.md", cap.id),
        &skill_content,
    )?;

    write_file(
        root,
        &format!(".claude/skills/{}.md", cap.id),
        &format!(
            "---\n\
             name: fiducial-{id}\n\
             description: How to use the `{id}` capability in this product.\n\
             ---\n\n\
             Read `.fiducial/skills/{id}.md` — the instructions live there so that \
             every agent can find them, not only Claude Code.\n",
            id = cap.id
        ),
    )?;
    // Skills are not tracked in the lock — they are platform-owned and rewritten
    // on every `fid upgrade`, not merged.

    // 3 + 4. Update fiducial.toml.
    patch_config(root, cap)?;

    // Re-record fiducial.toml in the lock — its hash changed after the patch.
    let new_config_content = std::fs::read(root.join(crate::config::CONFIG_FILE))
        .context("reading updated fiducial.toml")?;
    lock.record("fiducial.toml", &new_config_content, PLATFORM_VERSION);

    // Record what was installed, and from where. Without this a capability
    // resolved from outside the binary is invisible to every command that
    // reports on a product — which is most of them.
    lock.capabilities.insert(
        cap.id.clone(),
        crate::lock::CapabilityRecord {
            source: cap.source.label(),
            source_version: PLATFORM_VERSION.to_string(),
            hash: content_hash(cap),
            description: cap.description.clone(),
            declarations: cap
                .declarations
                .iter()
                .map(|d| d.name().to_string())
                .collect(),
            pipelines: cap.pipelines.iter().map(|f| f.path.clone()).collect(),
            requires_adapters: cap.requires_adapters.clone(),
        },
    );

    // Persist updated lock.
    lock.save(&root.join("fiducial.lock"))
        .context("writing fiducial.lock")?;

    println!("  ✓ {} installed", cap.id);
    if cap.guard_rules.is_empty() {
        println!("  ✓ no guard rules — this capability adds none");
    } else {
        println!("  ✓ guard rules added: {}", cap.guard_rules.join(", "));
    }
    println!(
        "  ✓ instructions → .fiducial/skills/{}.md (any agent; see AGENTS.md)",
        cap.id
    );
    Ok(())
}

/// Bring `messages/` in line with a declared locale set.
///
/// The `i18n` capability ships catalogs for the locales it seeds, and no more. A product
/// created with a different set needs a catalog for each locale it declares and
/// none for a locale it does not — the executor reads the locale set **from the
/// directory listing**, so a stray catalog is a language the product silently
/// claims to ship, and a missing one is a language it silently does not.
///
/// A locale with no shipped catalog starts as a copy of the default's. That is
/// deliberately visible rather than convenient: identical values are exactly
/// what `compare` reports as untranslated, so the work still to do shows up in
/// the first `fid derive` instead of being discovered by a reader.
pub fn reconcile_i18n_catalogs(root: &Path, locales: &[String], default: &str) -> Result<()> {
    let dir = root.join("messages");
    if !dir.is_dir() {
        return Ok(());
    }
    let mut lock = load_or_new_lock(root)?;

    let default_catalog = std::fs::read_to_string(dir.join(format!("{default}.json")))
        .with_context(|| format!("reading the default catalog messages/{default}.json"))?;

    // Add a catalog for every declared locale that has none.
    for locale in locales {
        let path = dir.join(format!("{locale}.json"));
        if path.exists() {
            continue;
        }
        let rel = format!("messages/{locale}.json");
        write_file(root, &rel, &default_catalog)?;
        lock.record(rel, default_catalog.as_bytes(), PLATFORM_VERSION);
    }

    // Remove the shipped catalogs for locales this product does not declare.
    for entry in std::fs::read_dir(&dir)
        .context("reading messages/")?
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(locale) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if locales.iter().any(|l| l == locale) {
            continue;
        }
        let rel = format!("messages/{locale}.json");
        // Only ours to remove. A catalog someone added by hand is theirs.
        if lock.templates.contains_key(&rel) {
            std::fs::remove_file(&path).with_context(|| format!("removing {rel}"))?;
            lock.templates.remove(&rel);
            println!("  removed {rel} (locale not declared)");
        }
    }

    lock.save(&root.join("fiducial.lock"))
        .context("writing fiducial.lock")?;
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn load_or_new_lock(root: &Path) -> Result<Lock> {
    let path = root.join("fiducial.lock");
    if path.exists() {
        Lock::load(&path).context("loading fiducial.lock")
    } else {
        Ok(Lock::new())
    }
}

fn write_file(root: &Path, rel: &str, content: &str) -> Result<()> {
    let dest = root.join(rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating parent for `{rel}`"))?;
    }
    std::fs::write(&dest, content).with_context(|| format!("writing `{rel}`"))?;
    println!("  wrote  {rel}");
    Ok(())
}

/// Patch `fiducial.toml` to add the capability id and its guard rules.
///
/// Uses line-level editing rather than a full TOML rewrite to preserve
/// comments and formatting. This is safe because the relevant lines have
/// a predictable shape written by `fid new`.
fn patch_config(root: &Path, cap: &Capability) -> Result<()> {
    let config_path = root.join(CONFIG_FILE);
    let mut cfg = Config::load(&config_path)?;

    // Add capability id if not already present.
    if !cfg.capabilities.enabled.contains(&cap.id.to_string()) {
        cfg.capabilities.enabled.push(cap.id.to_string());
    }

    // Add guard rules that aren't already present.
    for rule in &cap.guard_rules {
        if !cfg.guard.rules.contains(rule) {
            cfg.guard.rules.push(rule.clone());
        }
    }

    // Serialise back. We use a structured round-trip here rather than line
    // editing because the config schema is small and comments are at the top.
    let mut value = toml::Value::try_from(&cfg).context("serialising fiducial.toml")?;

    // A capability that introduces a DECLARATION must seed it, or the pipeline
    // it also installs fails on the next `fid derive` with an empty block.
    //
    // Merged at the `toml::Value` level rather than through the typed `Config`,
    // because the seed comes from a manifest a third party wrote: it names a
    // block this binary may know nothing about. Typing it would mean only
    // blocks compiled into `fid` could be declared, which is the limitation
    // this phase exists to remove.
    for decl in &cap.declarations {
        let Declaration::ConfigBlock { name, seed } = decl else {
            continue;
        };
        let Some(table) = value.as_table_mut() else {
            break;
        };
        let already_declared = table
            .get(name)
            .is_some_and(|existing| !is_empty_block(existing));
        if !already_declared {
            if let Some(seed) = seed {
                table.insert(name.clone(), seed.clone());
            }
        }
    }

    let raw = toml::to_string_pretty(&value).context("serialising fiducial.toml")?;
    let header = format!(
        "# fiducial.toml — updated by `fid add {}` (fiducial {})\n\n",
        cap.id, PLATFORM_VERSION
    );
    std::fs::write(&config_path, format!("{header}{raw}")).context("writing fiducial.toml")?;

    println!("  patched fiducial.toml");
    Ok(())
}

/// A hash over everything a capability installs.
///
/// Path and content of every file, plus the skill, in a fixed order. Two
/// resolutions of the same source that differ here are different capabilities,
/// whatever the revision says — which is what makes a `path:` source, with no
/// revision of its own, still checkable.
fn content_hash(cap: &Capability) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("skill\u{1f}{}", cap.skill_md));
    for decl in &cap.declarations {
        match decl {
            Declaration::File(f) => parts.push(format!("decl\u{1f}{}\u{1f}{}", f.path, f.content)),
            Declaration::Directory { path } => parts.push(format!("decldir\u{1f}{path}")),
            Declaration::ConfigBlock { name, seed } => {
                let seed_str = seed.as_ref().map(|s| s.to_string()).unwrap_or_default();
                parts.push(format!("block\u{1f}{name}\u{1f}{seed_str}"))
            }
        }
    }
    for f in &cap.pipelines {
        parts.push(format!("pipe\u{1f}{}\u{1f}{}", f.path, f.content));
    }
    for f in &cap.templates {
        parts.push(format!("tmpl\u{1f}{}\u{1f}{}", f.path, f.content));
    }
    parts.sort();
    crate::lock::sha256_hex(parts.join("\u{1e}").as_bytes())
}

/// Whether a `fiducial.toml` block carries no decision yet.
///
/// A block present but empty is the state `serde(default)` leaves behind, and
/// treating it as "already declared" would install a pipeline against nothing.
fn is_empty_block(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(t) => t.is_empty() || t.values().all(is_empty_block),
        toml::Value::Array(a) => a.is_empty(),
        toml::Value::String(s) => s.is_empty(),
        _ => false,
    }
}

// ── Conformance check ─────────────────────────────────────────────────────────

/// Check that a capability definition is well-formed.
/// Returns a list of conformance errors.
pub fn check_capability(cap: &Capability) -> Vec<String> {
    let mut errors = Vec::new();

    if cap.id.is_empty() {
        errors.push("capability id must not be empty".into());
    } else if cap
        .id
        .contains(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-')
    {
        errors.push(format!(
            "capability id `{}` must be kebab-case (lowercase letters, digits, and hyphens)",
            cap.id
        ));
    }

    if cap.description.is_empty() {
        errors.push(format!("[{}] description must not be empty", cap.id));
    }

    if cap.skill_md.is_empty() {
        errors.push(format!(
            "[{}] SKILL.md is mandatory (§3.3: no SKILL.md, not a capability)",
            cap.id
        ));
    } else if cap.skill_md.len() < 100 {
        errors.push(format!(
            "[{}] SKILL.md is suspiciously short ({} bytes) — \
             it must teach an agent how to use this capability correctly",
            cap.id,
            cap.skill_md.len()
        ));
    }

    // ── Taxonomy conformance ────────────────────────────────────────────────
    //
    // The split is only worth having if the three lists mean what they say. A
    // pipeline filed under `templates` is not gated by `fid derive --check`,
    // and a declaration filed there is one `fid dash` cannot report as missing.

    for FileEntry { path, .. } in &cap.pipelines {
        if !path.starts_with("pipelines/") {
            errors.push(format!(
                "[{}] pipeline `{path}` must live under `pipelines/` — that is \
                 where `pipeline::discover` looks, so one filed elsewhere is \
                 installed and never run",
                cap.id
            ));
        }
    }

    for FileEntry { path, .. } in &cap.templates {
        if path.starts_with("pipelines/") {
            errors.push(format!(
                "[{}] `{path}` is under `pipelines/` but is declared as a \
                 template. Move it to `pipelines` so it is recognised as one",
                cap.id
            ));
        }
    }

    // A capability that derives something must say what it derives it *from*.
    // Without this the taxonomy is decoration: a pipeline with no declared
    // input reads a fact nothing is responsible for putting there.
    if !cap.pipelines.is_empty() && cap.declarations.is_empty() {
        errors.push(format!(
            "[{}] installs {} pipeline(s) and declares nothing for them to \
             read — name the facts they consume in `declarations`",
            cap.id,
            cap.pipelines.len()
        ));
    }

    for contract in &cap.requires_adapters {
        if crate::adapter::find(contract).is_none() {
            errors.push(format!(
                "[{}] requires adapter contract `{contract}`, which is not one \
                 of: {}",
                cap.id,
                crate::adapter::CONTRACTS
                    .iter()
                    .map(|c| c.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every built-in capability conforms to its own rules.
    ///
    /// `fid capability check` runs this against a product's installed set; this
    /// runs it against the registry, so a first-party capability cannot ship
    /// violating a rule the CLI enforces on everyone else's.
    #[test]
    fn every_builtin_capability_conforms() {
        for cap in builtins() {
            let errors = check_capability(cap);
            assert!(errors.is_empty(), "capability `{}`: {errors:?}", cap.id);
        }
    }

    /// No file is installed from two lists.
    ///
    /// The three lists are written into the product by one loop, so a path in
    /// two of them is written twice and recorded in the lock twice — and which
    /// kind `fid dash` reports it as becomes a matter of iteration order.
    #[test]
    fn no_path_is_declared_twice_within_a_capability() {
        for cap in builtins() {
            let mut paths: Vec<&str> = Vec::new();
            for decl in &cap.declarations {
                if let Declaration::File(f) = decl {
                    paths.push(&f.path);
                }
            }
            paths.extend(cap.pipelines.iter().map(|f| f.path.as_str()));
            paths.extend(cap.templates.iter().map(|f| f.path.as_str()));

            let mut seen = paths.clone();
            seen.sort_unstable();
            let before = seen.len();
            seen.dedup();
            assert_eq!(
                before,
                seen.len(),
                "capability `{}` installs a path from two lists: {paths:?}",
                cap.id
            );
        }
    }

    /// Ids are unique — they are the key `fid add` and `[capabilities]` use.
    #[test]
    fn capability_ids_are_unique() {
        let mut ids: Vec<&str> = builtins().iter().map(|c| c.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate capability id");
    }

    /// The two capabilities the taxonomy was generalized from still exercise
    /// all three kinds between them.
    ///
    /// Not trivia: the spec's argument for building this now is that `eda` and
    /// `i18n` are *working* declaration→pipeline cases. If a refactor quietly
    /// flattened them back into `templates`, the taxonomy would still compile
    /// and mean nothing.
    #[test]
    fn the_taxonomy_is_exercised_by_real_capabilities() {
        let i18n = find("i18n").expect("i18n is built in");
        assert!(
            i18n.declarations
                .iter()
                .any(|d| matches!(d, Declaration::ConfigBlock { .. })),
            "i18n must declare its [i18n] block, not have it special-cased"
        );
        assert!(!i18n.pipelines.is_empty(), "i18n derives typed keys");

        let eda = find("eda").expect("eda is built in");
        assert_eq!(
            eda.pipelines.len(),
            2,
            "eda's board interface is read by two pipelines — the test for a declaration"
        );
        assert!(!eda.declarations.is_empty());
    }

    /// A config-block declaration actually fills its block in.
    ///
    /// A `seed` that does nothing installs clean and fails on the next
    /// `fid derive` with an empty declaration — the exact failure the seeding
    /// exists to prevent, moved one step later.
    #[test]
    fn every_config_block_seed_populates_something() {
        for cap in builtins() {
            for decl in &cap.declarations {
                let Declaration::ConfigBlock { name, seed } = decl else {
                    continue;
                };
                let Some(seed) = seed else {
                    // No seed means the block pre-exists in the scaffold; skip.
                    continue;
                };
                assert!(
                    !is_empty_block(seed),
                    "[{}] declares config block `{name}` with a seed that fills in nothing — \
                     the pipeline it installs would fail on the next derive",
                    cap.id
                );
            }
        }
    }

    /// A built-in and the directory it came from are the same capability.
    ///
    /// The registry is derived from `capabilities/<id>/` by `build.rs`, and a
    /// third-party capability is derived from a directory at runtime. This
    /// asserts the two paths agree — that "built in" is only a statement about
    /// where the bytes live, not about what a capability is.
    #[test]
    fn a_builtin_derives_the_same_from_its_directory() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
        for cap in builtins() {
            let from_disk = manifest::from_dir(&root.join(&cap.id), Source::Builtin)
                .unwrap_or_else(|e| {
                    panic!("`{}` does not derive from its directory: {e:#}", cap.id)
                });

            assert_eq!(from_disk.id, cap.id);
            assert_eq!(from_disk.description, cap.description, "[{}]", cap.id);
            assert_eq!(from_disk.declarations, cap.declarations, "[{}]", cap.id);
            assert_eq!(from_disk.pipelines, cap.pipelines, "[{}]", cap.id);
            assert_eq!(from_disk.templates, cap.templates, "[{}]", cap.id);
            assert_eq!(from_disk.guard_rules, cap.guard_rules, "[{}]", cap.id);
            assert_eq!(
                from_disk.requires_adapters, cap.requires_adapters,
                "[{}]",
                cap.id
            );
            assert_eq!(from_disk.skill_md, cap.skill_md, "[{}]", cap.id);
        }
    }
}

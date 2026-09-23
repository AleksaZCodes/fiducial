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
pub fn install(cap: &Capability, root: &Path, product_name: &str) -> Result<()> {
    println!("✦ fid add {} — installing into {}", cap.id, root.display());

    require_capabilities(cap, root)?;

    let mut lock = load_or_new_lock(root)?;

    // Which capabilities are already here decides which `for/<id>/` templates
    // this one contributes. Read before the config is patched, so `cap.id` is
    // not yet in the set and a capability cannot condition on itself.
    let already_installed: Vec<String> = Config::load(&root.join(CONFIG_FILE))
        .map(|c| c.capabilities.enabled)
        .unwrap_or_default();

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

    // `for/<id>/` templates for the capabilities this product already has.
    for (target, entries) in &cap.conditional_templates {
        if already_installed.iter().any(|id| id == target) {
            files.extend(entries.iter());
        } else {
            println!(
                "  · skipped {} file(s) for `{target}` (not installed)",
                entries.len()
            );
        }
    }

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

    // The other direction: capabilities already installed may have been
    // holding `for/<cap.id>/` templates back, waiting for this one. Without
    // this pass the result would depend on the order somebody ran `fid add`
    // in, which is the kind of state nobody can reason about later — install
    // `i18n` then `web-svelte` and you get the picker; the other order and you
    // silently do not.
    backfill_conditional_templates(root, &cap.id, &already_installed, &mut lock, product_name)?;

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

/// Install the `for/<new_id>/` templates of capabilities already present.
///
/// Only built-in definitions can be re-read here: an external capability was
/// resolved from a path or a git revision this function does not have, and
/// guessing one would install bytes nobody asked for. Those are named instead,
/// with the command that re-runs them — honest about the gap rather than
/// silently leaving a product half-wired.
fn backfill_conditional_templates(
    root: &Path,
    new_id: &str,
    already_installed: &[String],
    lock: &mut Lock,
    product_name: &str,
) -> Result<()> {
    let mut unresolvable: Vec<&str> = Vec::new();
    for installed_id in already_installed {
        let Some(other) = find(installed_id) else {
            unresolvable.push(installed_id);
            continue;
        };
        let Some(entries) = other.conditional_templates.get(new_id) else {
            continue;
        };
        println!(
            "  · {installed_id} contributes {} file(s) for {new_id}",
            entries.len()
        );
        for FileEntry { path: rel, content } in entries {
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
    }
    if !unresolvable.is_empty() {
        println!(
            "  ! could not check {} for `{new_id}` templates — \
             re-run `fid add capability <id> --from <source>` if it ships any",
            unresolvable.join(", ")
        );
    }
    Ok(())
}

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

/// Patch `fiducial.toml` to add the capability id, its guard rules, and any
/// declaration block it seeds.
///
/// ## Why this edits the document instead of re-serialising it
///
/// `fiducial.toml` is **authored**. It is the one file in a product where a
/// human writes down the judgments that nothing can derive — which KV id is
/// load-bearing, why a Worker name matches by declaration rather than
/// coincidence, what an artbox actually measures. Principle 1b is that a
/// judgment which cannot be derived is still declared, and in practice those
/// declarations live in the comments around the values.
///
/// This function used to read the file into the typed `Config`, mutate it, and
/// write `toml::to_string_pretty` back. That round-trip cannot preserve a
/// comment — `toml::Value` has nowhere to put one — so **every `fid add` silently
/// deleted the entire authored commentary** and reflowed the tables besides
/// (`vars = { … }` became `[deploy.vars]`, blocks re-sorted alphabetically).
/// On 2026-09-23 a single `fid add seo` destroyed 27 comment lines in
/// upoznaj-biznis. Nothing failed, the values all survived, and the loss is
/// invisible in review unless you happen to diff a file you did not expect to
/// change.
///
/// `toml_edit` is the same parser `toml` already uses underneath, exposed as a
/// format-preserving document: untouched bytes stay byte-identical, and only
/// the arrays and tables named below are rewritten.
///
/// The typed `Config` is still loaded first, because it validates — an invalid
/// `fiducial.toml` should fail here rather than be edited into a worse one.
fn patch_config(root: &Path, cap: &Capability) -> Result<()> {
    let config_path = root.join(CONFIG_FILE);

    // Validation only. The edit below is applied to the document, not to this.
    let cfg = Config::load(&config_path)?;

    let raw = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let mut doc: toml_edit::DocumentMut = raw
        .parse()
        .with_context(|| format!("parsing {}", config_path.display()))?;

    push_unique(
        &mut doc,
        "capabilities",
        "enabled",
        std::slice::from_ref(&cap.id),
    );
    push_unique(&mut doc, "guard", "rules", &cap.guard_rules);

    // A capability that introduces a DECLARATION must seed it, or the pipeline
    // it also installs fails on the next `fid derive` with an empty block.
    //
    // Merged as a document item rather than through the typed `Config`, because
    // the seed comes from a manifest a third party wrote: it names a block this
    // binary may know nothing about. Typing it would mean only blocks compiled
    // into `fid` could be declared, which is the limitation this phase exists
    // to remove.
    //
    // `cfg` answers "is it already declared?" because `is_empty_block` reasons
    // over `toml::Value`, and a seeded-but-empty block must still be replaced.
    let existing: toml::Value =
        toml::Value::try_from(&cfg).unwrap_or(toml::Value::Table(toml::map::Map::new()));
    for decl in &cap.declarations {
        let Declaration::ConfigBlock { name, seed } = decl else {
            continue;
        };
        let Some(seed) = seed else { continue };

        let already_declared = existing
            .get(name)
            .is_some_and(|block| !is_empty_block(block))
            || doc
                .get(name)
                .and_then(|item| item.as_table())
                .is_some_and(|t| !t.is_empty());
        if already_declared {
            continue;
        }

        // `toml::Value` → `toml_edit::Item` through a one-key document. There is
        // no direct conversion between the two crates' trees, and rendering the
        // seed as text is the conversion both of them already agree on.
        let mut wrapper = toml::map::Map::new();
        wrapper.insert(name.clone(), seed.clone());
        let rendered = toml::to_string_pretty(&toml::Value::Table(wrapper))
            .with_context(|| format!("rendering the `{name}` seed"))?;
        let seeded: toml_edit::DocumentMut = rendered
            .parse()
            .with_context(|| format!("re-reading the `{name}` seed"))?;
        if let Some(item) = seeded.get(name) {
            let mut item = item.clone();
            // A blank line before the block. Without it a seeded table opens
            // flush against the previous one's last value, which reads as a
            // continuation of it.
            if let Some(table) = item.as_table_mut() {
                table.decor_mut().set_prefix("\n");
            }
            doc.insert(name, item);
        }
    }

    let rendered = stamp_header(&doc.to_string(), &cap.id);
    std::fs::write(&config_path, rendered).context("writing fiducial.toml")?;

    println!("  patched fiducial.toml");
    Ok(())
}

/// Append values to `[table] key`, skipping any already present, creating the
/// table and the array if the product has neither.
///
/// Multi-line arrays stay multi-line. `toml_edit` appends a new entry with the
/// decor of the previous one, which for `enabled = [\n    "design",\n]` is the
/// leading newline and indent — so the result keeps the one-per-line shape a
/// human wrote instead of collapsing it onto a single line.
fn push_unique(doc: &mut toml_edit::DocumentMut, table: &str, key: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }

    let entry = doc
        .entry(table)
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
    let Some(table) = entry.as_table_mut() else {
        return;
    };
    let item = table
        .entry(key)
        .or_insert_with(|| toml_edit::value(toml_edit::Array::new()));
    let Some(array) = item.as_array_mut() else {
        return;
    };

    // An array a human wrote one-per-line stays that way. An EMPTY array adopts
    // that shape too: `fid new` scaffolds `enabled = []`, every filled-in
    // `fiducial.toml` in the wild is one-per-line, and the serialiser this
    // replaces produced one-per-line — so growing an empty array inline would
    // be a gratuitous style change on the first `fid add`.
    let multiline = array.is_empty() || array.to_string().contains('\n');

    for value in values {
        if array.iter().any(|v| v.as_str() == Some(value.as_str())) {
            continue;
        }
        array.push(value.as_str());
    }

    // Re-apply the one-per-line shape when that is what the file used. Doing it
    // for every element (not just the appended one) keeps the block uniform
    // rather than leaving the newcomer indented differently from its
    // neighbours.
    if multiline {
        for element in array.iter_mut() {
            element.decor_mut().set_prefix("\n    ");
            element.decor_mut().set_suffix("");
        }
        array.set_trailing_comma(true);
        array.set_trailing("\n");
    }
}

/// Replace the generated header line, or add one when the file has no header.
///
/// The header names the command and version that last touched the file, so it
/// is rewritten rather than prepended — prepending produced a growing stack of
/// stale headers, each claiming to describe the file.
///
/// Done on the rendered text rather than through the document's decor, and the
/// reason is the bug this replaces: leading trivia in a TOML document does not
/// belong to the root table. It attaches to the first *item* — here the first
/// table header — so reading the root's prefix found nothing and every `fid
/// add` prepended a fresh header above the last one. The rendered form is the
/// one place the header is unambiguously the first line.
///
/// Only a line this tool wrote is removed. A product whose file opens with its
/// own comment keeps it, and the header goes above it.
fn stamp_header(rendered: &str, id: &str) -> String {
    let header =
        format!("# fiducial.toml — updated by `fid add {id}` (fiducial {PLATFORM_VERSION})\n");

    let mut rest = rendered;
    loop {
        let trimmed = rest.trim_start_matches('\n');
        match trimmed.strip_prefix("# fiducial.toml — updated by") {
            Some(after) => {
                rest = after.split_once('\n').map(|(_, tail)| tail).unwrap_or("");
            }
            None => {
                rest = trimmed;
                break;
            }
        }
    }

    format!("{header}\n{rest}")
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
    // Keyed by target, because the same path shipped for two frameworks is two
    // different files and a hash that could not tell them apart would call two
    // different capabilities identical.
    for (target, entries) in &cap.conditional_templates {
        for f in entries {
            parts.push(format!(
                "tmpl-for\u{1f}{target}\u{1f}{}\u{1f}{}",
                f.path, f.content
            ));
        }
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

    // A `for/<id>/` directory naming nothing real installs nothing, forever,
    // and says so nowhere. Only built-in ids can be checked — a third-party
    // capability may legitimately target another third-party one.
    for target in cap.conditional_templates.keys() {
        if find(target).is_none() {
            errors.push(format!(
                "[{}] ships templates under `for/{target}/`, but `{target}` is not a \
                 known capability — those files would never install. Check the spelling \
                 against `fid capability list --all`",
                cap.id
            ));
        }
        if target == &cap.id {
            errors.push(format!(
                "[{}] conditions templates on itself via `for/{target}/` — \
                 they are plain templates; move them out of `for/`",
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
            // A conditional template sharing a path with an unconditional one
            // is written twice with whichever content the loop reached last.
            // Two conditional templates for *different* targets may share a
            // path — that is the feature — so they are checked per target.
            for entries in cap.conditional_templates.values() {
                let mut per_target: Vec<&str> = entries.iter().map(|f| f.path.as_str()).collect();
                per_target.sort_unstable();
                let n = per_target.len();
                per_target.dedup();
                assert_eq!(
                    n,
                    per_target.len(),
                    "capability `{}` ships a path twice under one `for/` target",
                    cap.id
                );
                for p in per_target {
                    assert!(
                        !paths.contains(&p),
                        "capability `{}` installs `{p}` both conditionally and unconditionally",
                        cap.id
                    );
                }
            }

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
            assert_eq!(
                from_disk.conditional_templates, cap.conditional_templates,
                "[{}]",
                cap.id
            );
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

//! `fid upgrade` — propagate platform updates into this product.
//!
//! Three propagation paths (§11 of the design spec):
//!
//! 1. **Scaffolded files → 3-way merge.**
//!    `fiducial.lock` records `base_content` (the template as it was expanded at
//!    scaffold time). Upgrade diffs the current in-binary template against that
//!    base, then 3-way merges against local edits on disk.
//!
//! 2. **API changes → codemods.**
//!    Migrations in `migration.rs` are applied in version order, exactly once,
//!    and recorded in `fiducial.lock [applied_migrations]`.
//!
//! 3. **Learnings → the Claude Code plugin, free.**
//!    Skill files (`.claude/skills/*.md`) are not tracked by the lock; they are
//!    overwritten on every `fid upgrade` from the in-binary capability SKILL.md.
//!    No conflict possible — the platform owns them entirely.

use anyhow::{Context, Result};
use std::env;

use crate::{
    capability::{self, PLATFORM_VERSION},
    config::{Config, CONFIG_FILE},
    lock,
    lock::{Lock, LOCK_FILE},
    migration, templates,
};

// ── Entry point ───────────────────────────────────────────────────────────────

/// `fid upgrade --upstream <path>`: the platform's current version of a file
/// this product tracks, expanded for this product — what an upgrade did not
/// merge into a file the product rewrote.
pub fn print_upstream(rel_path: &str) -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(CONFIG_FILE))?;
    let rel = rel_path.trim_start_matches("./").replace('\\', "/");
    let raw = templates::raw_for(&rel, &cfg.capabilities.enabled).ok_or_else(|| {
        anyhow::anyhow!("`{rel}` is not a file the platform ships a template for")
    })?;
    print!(
        "{}",
        templates::expand(raw, &cfg.product.name, PLATFORM_VERSION)
    );
    Ok(())
}

/// `only`: refresh one capability's files and instructions and nothing else —
/// no scaffold templates, no codemods, no other capability. A product mid-way
/// through a platform migration can take one capability's fix without taking
/// the whole migration with it.
pub fn run(dry_run: bool, portfolio: bool, only: Option<String>) -> Result<()> {
    if portfolio {
        println!(
            "✦ fid upgrade --portfolio\n\n\
             Portfolio fan-out is not available yet.\n\
             Run `fid upgrade` in each product repo individually for now."
        );
        return Ok(());
    }

    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(CONFIG_FILE))?;
    let lock_path = root.join(LOCK_FILE);

    println!(
        "✦ fid upgrade — {} v{}{}",
        cfg.product.name,
        cfg.product.version,
        if dry_run { " (dry run)" } else { "" }
    );
    println!();

    let mut lock = if lock_path.exists() {
        Lock::load(&lock_path).context("loading fiducial.lock")?
    } else {
        println!("  ⚠ fiducial.lock not found — run `fid new` to scaffold first.");
        return Ok(());
    };

    let mut any_changes = false;
    // Which capability's version of a shared path is the right one depends on
    // what this product installed — `firmware/Cargo.toml` ships from both
    // firmware capabilities with different content.
    let enabled = cfg.capabilities.enabled.clone();

    let scope: Option<(&'static capability::Capability, Vec<String>)> = match &only {
        None => None,
        Some(id) => {
            if !enabled.iter().any(|e| e == id) {
                anyhow::bail!(
                    "`{id}` is not installed in this product (installed: {})",
                    enabled.join(", ")
                );
            }
            let cap = capability::find(id).ok_or_else(|| {
                anyhow::anyhow!(
                    "`{id}` is not a built-in capability; `--capability` refreshes built-ins only"
                )
            })?;
            println!("  Scope: the `{id}` capability only");
            println!();
            Some((cap, capability::owned_paths(cap, &enabled)))
        }
    };

    // ── 1. Template 3-way merge ───────────────────────────────────────────────
    println!("  Templates");
    let template_paths: Vec<String> = lock
        .templates
        .keys()
        .filter(|p| scope.as_ref().is_none_or(|(_, owned)| owned.contains(p)))
        .cloned()
        .collect();
    for rel_path in &template_paths {
        if let Some(outcome) = merge_one_template(
            &root,
            rel_path,
            &cfg.product.name,
            &enabled,
            &mut lock,
            dry_run,
        )? {
            any_changes = true;
            match outcome {
                TemplateOutcome::Clean(msg) => println!("  ✓ {rel_path}: {msg}"),
                TemplateOutcome::Conflict(msg) => println!("  ⚠ {rel_path}: {msg}"),
                TemplateOutcome::Kept(msg) => println!("  · {rel_path}: {msg}"),
            }
        } else {
            println!("  · {rel_path}: no upstream change");
        }
    }
    println!();

    // ── 1a. Templates the platform has renamed ────────────────────────────────
    //
    // Must run BEFORE the "added" pass below, which would otherwise see the new
    // path as simply missing and install it, leaving the old file behind — and
    // the old file is the whole problem, because a subagent's filename is its
    // identity and the stale one keeps claiming the colliding name.
    // Scaffold-level passes (renames, new scaffold files, codemods) belong to a
    // whole-product upgrade; a scoped one skips them.
    let renamed: Vec<(&str, &str)> = templates::RENAMED_TEMPLATES
        .iter()
        .copied()
        .filter(|_| scope.is_none())
        .filter(|(old, _)| lock.templates.contains_key(*old) || root.join(old).exists())
        .collect();

    if !renamed.is_empty() {
        println!("  Renamed by the platform");
        for (old, new) in &renamed {
            let old_path = root.join(old);
            let new_path = root.join(new);

            // Is the local copy still the one we shipped? If the product edited
            // it, those edits are theirs and deleting them to fix our naming
            // problem is the worse outcome.
            let unmodified = match (std::fs::read(&old_path), lock.templates.get(*old)) {
                (Ok(bytes), Some(record)) => lock::sha256_hex(&bytes) == record.hash,
                (Err(_), _) => true, // already gone
                (Ok(_), None) => false,
            };

            if dry_run {
                any_changes = true;
                if unmodified {
                    println!("  ✓ would rename: {old} → {new}");
                } else {
                    println!("  ⚠ {old}: locally modified — would install {new} and keep both");
                }
                continue;
            }

            // Install the new path from the current template.
            if let Some(template) = templates::raw_for(new, &cfg.capabilities.enabled) {
                let content = templates::expand(template, &cfg.product.name, PLATFORM_VERSION);
                if let Some(parent) = new_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("creating parent for `{new}`"))?;
                }
                std::fs::write(&new_path, &content).with_context(|| format!("writing `{new}`"))?;
                lock.record(new.to_string(), content.as_bytes(), PLATFORM_VERSION);
            }

            if unmodified {
                let _ = std::fs::remove_file(&old_path);
                lock.templates.remove(*old);
                println!("  ✓ renamed: {old} → {new}");
            } else {
                lock.templates.remove(*old);
                println!("  ⚠ {old}: locally modified — kept, and {new} installed alongside");
                println!("    Move your changes across, then delete the old file.");
            }
            any_changes = true;
        }
        println!();
    }

    // ── 1a′. Templates the platform has retired ───────────────────────────────
    // A whole upgrade drops their lock records; the file goes only when it is
    // still exactly what the platform wrote. See `templates::RETIRED_TEMPLATES`.
    let retired: Vec<(&str, &str)> = templates::RETIRED_TEMPLATES
        .iter()
        .copied()
        .filter(|_| scope.is_none())
        .filter(|(old, _)| lock.templates.contains_key(*old))
        .collect();
    if !retired.is_empty() {
        println!("  Retired by the platform");
        for (old, why) in &retired {
            let path = root.join(old);
            let unmodified = match (std::fs::read(&path), lock.templates.get(*old)) {
                (Ok(bytes), Some(record)) => lock::sha256_hex(&bytes) == record.hash,
                _ => false,
            };
            any_changes = true;
            let fate = if !path.exists() {
                "already gone"
            } else if unmodified {
                "removed"
            } else {
                "edited here, so kept as the product's own"
            };
            if dry_run {
                println!("  · {old}: would stop tracking ({why}); {fate}");
                continue;
            }
            if path.exists() && unmodified {
                std::fs::remove_file(&path).with_context(|| format!("removing {old}"))?;
            }
            lock.templates.remove(*old);
            println!("  ✓ {old}: no longer tracked ({why}); {fate}");
        }
        println!();
    }

    // ── 1b. Templates the platform has added since this product was scaffolded ─
    //
    // A 3-way merge can only update files the lock already knows about, so
    // without this a template added to `fid new` reached only products created
    // afterwards. `.github/workflows/ci.yml` — the workflow that gates artifact
    // freshness — was added in Phase 18 and would have arrived in no existing
    // product at all. Principle 7: a fix that cannot propagate is half-finished.
    let added: Vec<(&str, &str)> = templates::SCAFFOLD_FILES
        .iter()
        .copied()
        .filter(|_| scope.is_none())
        .filter(|(rel_path, _)| !lock.templates.contains_key(*rel_path))
        .collect();

    if !added.is_empty() {
        println!("  New since this product was scaffolded");
        for (rel_path, template) in &added {
            let dest = root.join(rel_path);

            // Never clobber a file the product already wrote by hand; report it
            // and let the author reconcile.
            if dest.exists() {
                println!("  ⚠ {rel_path}: exists on disk but is untracked — leaving it alone");
                continue;
            }
            // A stylesheet for a component the product wrote itself is the
            // stock component's, and would sit unused beside the product's.
            if let Some(own) = capability::companion_of_own(&root, &lock, rel_path) {
                println!("  · {rel_path}: not added — {own} is your own component");
                continue;
            }

            any_changes = true;
            if dry_run {
                println!("  ✓ would add: {rel_path}");
                continue;
            }

            let content = templates::expand(template, &cfg.product.name, PLATFORM_VERSION);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating parent for `{rel_path}`"))?;
            }
            std::fs::write(&dest, &content).with_context(|| format!("writing `{rel_path}`"))?;
            lock.record(
                rel_path.replace('\\', "/"),
                content.as_bytes(),
                PLATFORM_VERSION,
            );
            println!("  ✓ added: {rel_path}");
        }
        println!();
    }

    // ── 1c. Files a capability has gained since it was installed ─────────────
    for cap_id in &enabled {
        if scope.as_ref().is_some_and(|(c, _)| c.id != *cap_id) {
            continue;
        }
        let Some(def) = capability::find(cap_id) else {
            continue;
        };
        for rel in
            capability::install_added(&root, def, &enabled, &mut lock, &cfg.product.name, dry_run)?
        {
            any_changes = true;
            let verb = if dry_run { "would add" } else { "added" };
            println!("  ✓ {verb}: {rel} (new in `{cap_id}`)");
        }
    }

    // ── 2. Codemods ───────────────────────────────────────────────────────────
    println!("  Codemods");
    let pending_count = if scope.is_some() {
        0
    } else {
        migration::pending(&lock.applied_migrations).len()
    };
    if scope.is_some() {
        println!("  · skipped (scoped upgrade)");
    } else if pending_count == 0 {
        println!("  · no pending migrations");
    } else {
        // Apply all pending migrations in one pass. Propagate errors — a failed
        // migration is not silently skipped.
        let results = migration::apply_pending(&root, &lock.applied_migrations, dry_run)?;
        for r in results {
            let verb = if dry_run { "would apply" } else { "applied" };
            println!(
                "  ✓ {verb}: {} ({} file(s) modified)",
                r.migration_id, r.files_modified
            );
            if !dry_run {
                lock.applied_migrations.push(r.migration_id);
            }
            any_changes = true;
        }
    }
    println!();

    // ── 3. Skill files (platform-owned, always overwrite) ─────────────────────
    println!("  Skills (agent instructions)");
    for cap_id in &cfg.capabilities.enabled {
        if scope.as_ref().is_some_and(|(c, _)| &c.id != cap_id) {
            continue;
        }
        if let Some(def) = capability::find(cap_id) {
            let skill_path = format!(".fiducial/skills/{cap_id}.md");
            if !dry_run {
                capability::write_skill(&root, def, &cfg.product.name)?;
                println!("  ✓ {skill_path}: refreshed from platform (+ .claude/skills pointer)");
            } else {
                println!("  · {skill_path}: would refresh from platform");
            }
            any_changes = true;
        }
    }
    println!();

    // ── 4. The platform version the product says it is built on ───────────────
    // `fid new` writes "Built on [Fiducial](…) X." into AGENTS.md and
    // README.md; nothing moved it after, so a product said 0.1.0 at 0.9.1.
    // Only that one line, only on a whole upgrade: a scoped one is not a move.
    if scope.is_none() {
        for file in ["AGENTS.md", "README.md"] {
            let path = root.join(file);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(updated) = stamp_platform_version(&text, PLATFORM_VERSION) {
                if dry_run {
                    println!("  · {file}: would say it is built on fiducial {PLATFORM_VERSION}");
                } else {
                    std::fs::write(&path, updated).with_context(|| format!("writing {file}"))?;
                    println!("  ✓ {file}: built on fiducial {PLATFORM_VERSION}");
                }
                any_changes = true;
            }
        }
    }

    // ── Persist lock ──────────────────────────────────────────────────────────
    if !dry_run && any_changes {
        lock.save(&lock_path).context("saving fiducial.lock")?;
        println!("  ✓ fiducial.lock updated");
        println!();
        println!("✦ upgrade complete. Run `fid doctor` to verify.");
    } else if dry_run {
        println!("✦ dry run complete — no files written.");
    } else {
        println!("✦ already up to date.");
    }

    Ok(())
}

// ── Template merge ────────────────────────────────────────────────────────────

enum TemplateOutcome {
    Clean(String),
    Conflict(String),
    /// A product-owned file the product rewrote: upstream changed, the file
    /// was left as the product's. Reported, never merged.
    Kept(String),
}

/// Does this file still carry unresolved conflict markers?
///
/// Both an opening and a closing marker must be present, each at the start of
/// its own line. Requiring the pair keeps prose safe: a document that happens
/// to discuss `<<<<<<<` — this repository has several, including the one
/// explaining this function — is not a file mid-conflict, and refusing to
/// upgrade it would be a new bug in place of the old one.
fn has_conflict_markers(text: &str) -> bool {
    let mut opened = false;
    let mut closed = false;
    for line in text.lines() {
        if line.starts_with("<<<<<<<") {
            opened = true;
        } else if line.starts_with(">>>>>>>") {
            closed = true;
        }
    }
    opened && closed
}

/// Top-level `[section]` headers a merge would remove, named, or `None` when it
/// removes nothing.
///
/// Only applied to `fiducial.toml`. Every other template is prose or code where
/// a bracketed line means nothing, and where losing a line is a normal outcome
/// of an upstream edit rather than a lost declaration.
///
/// Deliberately textual. Parsing both sides as TOML would be more precise and
/// would also fail on the half-merged file this exists to catch, which is the
/// wrong direction: the check has to work on output that may not parse.
/// Settle each conflict hunk where one side already holds every line of the
/// other: the product had already made the platform's change in its own words
/// (or the platform's version is the product's plus more). `None` while any
/// hunk is a real disagreement.
fn resolve_contained(conflicted: &str) -> Option<String> {
    use std::collections::HashSet;
    let mut out: Vec<&str> = Vec::new();
    let (mut ours, mut theirs): (Vec<&str>, Vec<&str>) = (Vec::new(), Vec::new());
    let mut mode = 0; // 0 outside, 1 ours, 2 base, 3 theirs
    for line in conflicted.split('\n') {
        match (mode, line) {
            (0, l) if l.starts_with("<<<<<<< ") => mode = 1,
            (1, l) if l.starts_with("||||||| ") => mode = 2,
            (1 | 2, l) if l.starts_with("=======") => mode = 3,
            (3, l) if l.starts_with(">>>>>>> ") => {
                let set = |v: &[&str]| -> HashSet<String> {
                    v.iter()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect()
                };
                let (o, t) = (set(&ours), set(&theirs));
                if t.is_subset(&o) {
                    out.append(&mut ours);
                } else if o.is_subset(&t) {
                    out.append(&mut theirs);
                } else {
                    return None;
                }
                ours.clear();
                theirs.clear();
                mode = 0;
            }
            (0, l) => out.push(l),
            (1, l) => ours.push(l),
            (2, _) => {}
            (_, l) => theirs.push(l),
        }
    }
    (mode == 0).then(|| out.join("\n"))
}

/// A three-way merge of a package.json by dependency. Upstream's added
/// dependency is added, its new version taken where the product kept the old
/// one, a dependency it dropped dropped where the product left it alone;
/// everything else stays the product's text. `None` if any side does not
/// parse, so the line merge decides instead.
fn merge_package_json(base: &str, ours: &str, theirs: &str) -> Option<String> {
    use serde_json::Value;
    let (b, o, t): (Value, Value, Value) = (
        serde_json::from_str(base).ok()?,
        serde_json::from_str(ours).ok()?,
        serde_json::from_str(theirs).ok()?,
    );
    let mut text = ours.to_string();
    for section in ["dependencies", "devDependencies"] {
        let map = |v: &Value| -> std::collections::BTreeMap<String, String> {
            v[section]
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                        .collect()
                })
                .unwrap_or_default()
        };
        let (bm, om, tm) = (map(&b), map(&o), map(&t));
        let mut want = om.clone();
        for (k, tv) in &tm {
            match (bm.get(k), om.get(k)) {
                (None, None) => {
                    want.insert(k.clone(), tv.clone());
                }
                (Some(bv), Some(ov)) if bv == ov && tv != ov => {
                    want.insert(k.clone(), tv.clone());
                }
                _ => {}
            }
        }
        for (k, bv) in &bm {
            if !tm.contains_key(k) && om.get(k) == Some(bv) {
                want.remove(k);
            }
        }
        if want == om {
            continue;
        }
        // Rewrite only this section's body, in the product's indentation.
        let head = format!("\"{section}\"");
        let start = text.find(&head)?;
        let open = start + text[start..].find('{')?;
        let close = open + text[open..].find('}')?;
        let indent = text[open + 1..close]
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l[..l.len() - l.trim_start().len()].to_string())
            .unwrap_or_else(|| "    ".into());
        let outer = indent.get(2..).unwrap_or("").to_string();
        let body: Vec<String> = want
            .iter()
            .map(|(k, v)| {
                format!(
                    "{indent}{}: {}",
                    Value::from(k.as_str()),
                    Value::from(v.as_str())
                )
            })
            .collect();
        text = format!(
            "{}{{\n{}\n{outer}{}",
            &text[..open],
            body.join(",\n"),
            &text[close..]
        );
    }
    serde_json::from_str::<Value>(&text).ok()?;
    Some(text)
}

/// When a merge would conflict or is refused: does the product's file already
/// carry every code change the platform made since the base — each line it
/// added present, each it removed absent — so that only comments differ? Then
/// there is nothing to merge: the file is kept, the base moves, and the
/// platform's comments are one `--upstream` away. A product that had already
/// made the platform's fix in its own words got conflicts over prose before.
fn keep_if_nothing_to_add(
    rel_path: &str,
    base: &str,
    local: &str,
    upstream: &str,
    lock: &mut Lock,
    dry_run: bool,
) -> Option<TemplateOutcome> {
    use std::collections::HashSet;
    let ext = rel_path.rsplit('.').next().unwrap_or("");
    let slash = matches!(ext, "css" | "js" | "mjs" | "ts" | "tsx" | "jsx");
    let hash = matches!(ext, "toml" | "yml" | "yaml" | "sh" | "py");
    if !slash && !hash {
        return None;
    }
    let code = |text: &str| -> HashSet<String> {
        let mut t = text.to_string();
        if slash {
            while let Some(a) = t.find("/*") {
                let b = t[a..].find("*/").map(|e| a + e + 2).unwrap_or(t.len());
                t.replace_range(a..b, "");
            }
        }
        t.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .filter(|l| !(slash && l.starts_with("//")) && !(hash && l.starts_with('#')))
            .map(String::from)
            .collect()
    };
    let (b, o, u) = (code(base), code(local), code(upstream));
    let added_here = u.difference(&b).all(|l| o.contains(l));
    let removed_here = b.difference(&u).all(|l| !o.contains(l));
    if !(added_here && removed_here) {
        return None;
    }
    if !dry_run {
        let record = lock.templates.get_mut(rel_path)?;
        record.base_content = Some(upstream.to_string());
        record.source_version = PLATFORM_VERSION.into();
    }
    Some(TemplateOutcome::Kept(format!(
        "yours — it already has every code change in the platform's version; \
         only comments differ (`fid upgrade --upstream {rel_path}` prints them)"
    )))
}

/// Has the product rewritten this file, rather than edited it? Fewer than half
/// of the base's non-blank lines survive in it.
fn rewritten(base: &str, local: &str) -> bool {
    use std::collections::HashSet;
    let have: HashSet<&str> = local.lines().map(str::trim).collect();
    let lines: Vec<&str> = base
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() || base == local {
        return false;
    }
    let kept = lines.iter().filter(|l| have.contains(*l)).count();
    kept * 2 < lines.len()
}

/// Why a merge that diffy calls clean is not one: a JSON or TOML file that no
/// longer parses, or a block of lines that ends up in it more often than in
/// either side — a section pasted in twice.
fn broken_merge(rel_path: &str, local: &str, upstream: &str, merged: &str) -> Option<String> {
    if rel_path.ends_with(".json")
        && serde_json::from_str::<serde_json::Value>(local).is_ok()
        && serde_json::from_str::<serde_json::Value>(merged).is_err()
    {
        return Some("is not valid JSON".into());
    }
    if rel_path.ends_with(".toml")
        && toml::from_str::<toml::Value>(local).is_ok()
        && toml::from_str::<toml::Value>(merged).is_err()
    {
        return Some("is not valid TOML".into());
    }
    let blocks = |text: &str| -> std::collections::HashMap<String, usize> {
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let mut n = std::collections::HashMap::new();
        for w in lines.windows(3) {
            if w.iter().all(|l| l.len() > 3) {
                *n.entry(w.join("\n")).or_insert(0) += 1;
            }
        }
        n
    };
    let (m, l, u) = (blocks(merged), blocks(local), blocks(upstream));
    m.iter()
        .find(|(k, &c)| {
            c > 1
                && c > l
                    .get(*k)
                    .copied()
                    .unwrap_or(0)
                    .max(u.get(*k).copied().unwrap_or(0))
        })
        .map(|(k, _)| {
            format!(
                "repeats a block that neither side repeats (\"{}\")",
                k.lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect::<String>()
            )
        })
}

fn dropped_sections(rel_path: &str, local: &str, merged: &str) -> Option<String> {
    if rel_path != crate::config::CONFIG_FILE {
        return None;
    }

    let sections = |s: &str| -> Vec<String> {
        s.lines()
            .map(str::trim)
            .filter(|l| l.starts_with('[') && l.ends_with(']'))
            .map(|l| l.to_string())
            .collect()
    };

    let after = sections(merged);
    let lost: Vec<String> = sections(local)
        .into_iter()
        .filter(|s| !after.contains(s))
        .collect();

    if lost.is_empty() {
        None
    } else {
        Some(lost.join(", "))
    }
}

/// Merge one template. Returns `None` if no upstream change, `Some(outcome)` otherwise.
fn merge_one_template(
    root: &std::path::Path,
    rel_path: &str,
    product_name: &str,
    enabled: &[String],
    lock: &mut Lock,
    dry_run: bool,
) -> Result<Option<TemplateOutcome>> {
    let record = match lock.templates.get(rel_path) {
        Some(r) => r.clone(),
        None => return Ok(None),
    };

    // Look up the current (in-binary) template and expand placeholders.
    let raw = match templates::raw_for(rel_path, enabled) {
        Some(r) => r,
        None => {
            // Template path not in registry — third-party or removed capability.
            return Ok(None);
        }
    };
    let upstream = templates::expand(raw, product_name, PLATFORM_VERSION);

    // Base content from lock (what was installed).
    let base = match &record.base_content {
        Some(b) => templates::merge_base(rel_path, b, raw, product_name, &record.source_version),
        None => {
            // Pre-Phase-4 lock: no base stored. Treat as "cannot merge, skip".
            return Ok(None);
        }
    };

    // Has upstream changed since last install/upgrade?
    //
    // Compared at the version the product recorded rather than at this
    // binary's. `upstream` above carries the *current* `{{version}}` stamp, so
    // comparing against it made every version-stamped template look changed
    // the moment the platform moved — and then opened a 3-way merge on a file
    // nobody upstream had touched, which is how a product accumulates
    // conflicts. See `templates::upstream_changed`.
    // A file still mid-conflict is refused before anything else. Merged
    // again, `ours` would be the file *containing the markers*, nesting
    // markers inside markers and losing more of the file each run — a real
    // product's CI once carried the note that `fid upgrade` "rewrites conflict
    // markers into fiducial.toml — corrupting it further every time". And
    // after a conflict the base has moved to upstream, so "no upstream
    // change" would otherwise hide the markers a person still has to resolve.
    if std::fs::read_to_string(root.join(rel_path)).is_ok_and(|l| has_conflict_markers(&l)) {
        return Ok(Some(TemplateOutcome::Conflict(format!(
            "UNRESOLVED — {rel_path} still contains conflict markers from an \
             earlier upgrade. Nothing was written. Resolve the markers, then re-run"
        ))));
    }

    if !templates::upstream_changed(raw, product_name, &record.source_version, &base) {
        return Ok(None); // No upstream change.
    }

    // Read local file.
    let local_path = root.join(rel_path);
    let local = match std::fs::read_to_string(&local_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File was deleted locally — restore from upstream.
            if !dry_run {
                if let Some(parent) = local_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&local_path, &upstream)
                    .with_context(|| format!("restoring deleted {rel_path}"))?;
                let record = lock.templates.get_mut(rel_path).unwrap();
                record.base_content = Some(upstream.clone());
                record.hash = crate::lock::sha256_hex(upstream.as_bytes());
                record.source_version = PLATFORM_VERSION.into();
            }
            return Ok(Some(TemplateOutcome::Clean(
                "restored (was deleted locally)".into(),
            )));
        }
        Err(e) => {
            // Other I/O error (e.g. permission denied) — surface it; do not overwrite.
            return Err(e).with_context(|| format!("reading {rel_path}"));
        }
    };

    // A product-owned file the product has rewritten is not merged into. Its
    // base is the scaffold's prompt, its content is the product's own, and a
    // line merge between them only produces noise: a real product's design
    // system "merged cleanly" with the seed's grey palette pasted into it, and
    // its own copy came back as conflicts it had to resolve, every one of them
    // to its own side. Upstream's change is reported, the base moves so it is
    // reported once, and `fid upgrade --upstream <path>` prints it.
    if crate::ownership::of(rel_path).is_product() && rewritten(&base, &local) {
        if !dry_run {
            let record = lock.templates.get_mut(rel_path).unwrap();
            record.base_content = Some(upstream.clone());
            record.source_version = PLATFORM_VERSION.into();
        }
        return Ok(Some(TemplateOutcome::Kept(format!(
            "yours, rewritten here; the platform's template changed and was not \
             merged in — `fid upgrade --upstream {rel_path}` prints it"
        ))));
    }

    // 3-way merge: base=what-was-installed, ours=local-file, theirs=upstream.
    // A package.json merges by dependency, not by line; anything else by line,
    // with a conflict whose one side already holds all of the other's lines
    // settled to the fuller side.
    let merged = match (rel_path.ends_with("package.json"))
        .then(|| merge_package_json(&base, &local, &upstream))
        .flatten()
    {
        Some(m) => Ok(m),
        None => diffy::merge(&base, &local, &upstream)
            .or_else(|conflict| resolve_contained(&conflict).ok_or(conflict)),
    };

    match merged {
        Ok(clean) => {
            // A clean merge that silently deletes a declared block is not a
            // clean merge, and `fiducial.toml` is the file where that costs the
            // most: every fact the product owns lives in it.
            //
            // On 2026-09-18 an upgrade of a real product reported
            // "merged cleanly from upstream" and replaced the whole file with
            // the scaffolding template, dropping `[brand]`, `[i18n]`, `[legal]`
            // and every entry in `[capabilities] enabled`. Nothing failed. The
            // product looked scaffolded-but-empty, and the damage was only
            // visible by reading the diff.
            //
            // The cause is that the serialized config orders its blocks
            // alphabetically while the template orders them by narrative, so
            // ours reads to a line-based merge as "deleted the file and wrote a
            // different one" — and where theirs also touched those lines, diffy
            // resolves toward theirs without conflicting.
            //
            // A proper fix merges this file semantically, key by key, which is
            // real work and not this function's job. What is this function's
            // job is refusing to write a result that loses a declaration. So
            // the check is dumb on purpose: every `[section]` the local file had
            // must still be there.
            if let Some(lost) = dropped_sections(rel_path, &local, &clean) {
                return Ok(Some(TemplateOutcome::Conflict(format!(
                    "REFUSED — merging upstream would delete {lost} from your \
                     declaration. Nothing was written. Reconcile {rel_path} by hand \
                     against the platform template, then re-run"
                ))));
            }
            // "Clean" is diffy's word for "no overlapping hunk", not for a
            // file that still works: it has pasted a section in twice, and a
            // paragraph after the comment that held it closed. Refused, not
            // written.
            if let Some(why) = broken_merge(rel_path, &local, &upstream, &clean) {
                if let Some(kept) =
                    keep_if_nothing_to_add(rel_path, &base, &local, &upstream, lock, dry_run)
                {
                    return Ok(Some(kept));
                }
                return Ok(Some(TemplateOutcome::Conflict(format!(
                    "REFUSED — the merge {why}. Nothing was written. Reconcile \
                     {rel_path} by hand (`fid upgrade --upstream {rel_path}` prints \
                     the platform's version), then re-run"
                ))));
            }
            if !dry_run {
                std::fs::write(&local_path, &clean)
                    .with_context(|| format!("writing {rel_path}"))?;
                let record = lock.templates.get_mut(rel_path).unwrap();
                record.base_content = Some(upstream.clone());
                record.hash = crate::lock::sha256_hex(clean.as_bytes());
                record.source_version = PLATFORM_VERSION.into();
            }
            Ok(Some(TemplateOutcome::Clean(
                "merged cleanly from upstream".into(),
            )))
        }
        Err(conflict) => {
            if let Some(kept) =
                keep_if_nothing_to_add(rel_path, &base, &local, &upstream, lock, dry_run)
            {
                return Ok(Some(kept));
            }
            // Write conflict markers to the file — the human must resolve.
            if !dry_run {
                std::fs::write(&local_path, &conflict)
                    .with_context(|| format!("writing conflict markers to {rel_path}"))?;
                // The base moves to upstream; the hash does not. Resolving the
                // markers is the human merging toward this upstream, so the next
                // run merges their file against it and upstream again — clean,
                // and recorded. Left on the old base, that run redid the same
                // merge and wrote the same conflict over the resolution, every
                // time. (A file still holding markers is refused above.)
                let record = lock.templates.get_mut(rel_path).unwrap();
                record.base_content = Some(upstream.clone());
                // And the hash with it: what the platform now ships is the
                // reference. A file resolved to exactly that is no drift; one
                // still holding markers, or a local edit, still is.
                record.hash = crate::lock::sha256_hex(upstream.as_bytes());
            }
            Ok(Some(TemplateOutcome::Conflict(
                "CONFLICT — conflict markers written; resolve and run `fid upgrade` again".into(),
            )))
        }
    }
}

/// The "Built on [Fiducial](…) X." line `fid new` writes, moved to
/// `version`. `None` when there is no such line or it already says so.
fn stamp_platform_version(text: &str, version: &str) -> Option<String> {
    const LEAD: &str = "Built on [Fiducial](https://github.com/AleksaZCodes/fiducial) ";
    let line = text
        .lines()
        .find(|l| l.starts_with(LEAD) && l.ends_with('.'))?;
    let want = format!("{LEAD}{version}.");
    (line != want).then(|| text.replacen(line, &want, 1))
}

#[cfg(test)]
mod stamp_tests {
    use super::stamp_platform_version;

    #[test]
    fn the_built_on_line_moves_to_the_platform_version_and_nothing_else_does() {
        let text = "# p\n\nBuilt on [Fiducial](https://github.com/AleksaZCodes/fiducial) 0.1.0.\n\nBuilt on sand.\n";
        let out = stamp_platform_version(text, "0.9.1").unwrap();
        assert_eq!(
            out,
            "# p\n\nBuilt on [Fiducial](https://github.com/AleksaZCodes/fiducial) 0.9.1.\n\nBuilt on sand.\n"
        );
        assert_eq!(stamp_platform_version(&out, "0.9.1"), None);
        assert_eq!(stamp_platform_version("no such line\n", "0.9.1"), None);
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    // Each of these is a case a real product's first full upgrade hit.

    #[test]
    fn a_stock_stylesheet_is_not_added_beside_the_products_own_component() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("src/components");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("picker.tsx"), "mine").unwrap();
        let mut lock = Lock::new();
        let css = "src/components/picker.module.css";
        assert_eq!(
            capability::companion_of_own(tmp.path(), &lock, css).as_deref(),
            Some("src/components/picker.tsx")
        );
        // The platform's own component: its stylesheet belongs with it.
        lock.record("src/components/picker.tsx", b"mine", "0.1.0");
        assert_eq!(capability::companion_of_own(tmp.path(), &lock, css), None);
    }

    #[test]
    fn a_file_the_product_rewrote_is_told_apart_from_one_it_edited() {
        let base = "a line\nb line\nc line\nd line\n";
        assert!(rewritten(base, "something else entirely\nand more\n"));
        assert!(!rewritten(base, "a line\nb line\nc line\nd line\nmine\n"));
        assert!(!rewritten(base, base));
    }

    #[test]
    fn a_clean_merge_that_pastes_a_block_twice_or_breaks_json_is_refused() {
        let block = "one long line\ntwo long line\nthree long line\n";
        let ours = format!("head\n{block}tail\n");
        let twice = format!("head\n{block}{block}tail\n");
        assert!(broken_merge("x.css", &ours, &ours, &twice).is_some());
        assert!(broken_merge("x.css", &ours, &ours, &ours).is_none());
        assert!(broken_merge("x.json", "{}", "{}", "{,}").is_some());
    }

    #[test]
    fn a_conflict_one_side_already_contains_is_settled_to_the_fuller_side() {
        let c = "a\n<<<<<<< ours\nmine\nshared\n||||||| original\nold\n=======\nshared\n>>>>>>> theirs\nz";
        assert_eq!(resolve_contained(c).as_deref(), Some("a\nmine\nshared\nz"));
        let real = "<<<<<<< ours\nmine\n||||||| original\nold\n=======\ntheirs\n>>>>>>> theirs";
        assert_eq!(resolve_contained(real), None);
    }

    #[test]
    fn a_package_json_merges_by_dependency_and_keeps_the_products_pins() {
        let base = r#"{ "dependencies": { "next": "^15.0.0", "tokens": "*" } }"#;
        let theirs =
            r#"{ "dependencies": { "next": "^16.0.0", "tokens": "*", "added": "^1.0.0" } }"#;
        let ours = "{\n  \"dependencies\": {\n    \"mine\": \"^2.0.0\",\n    \"next\": \"^15.0.0\",\n    \"tokens\": \"^0.2.0\"\n  }\n}\n";
        let m = merge_package_json(base, ours, theirs).unwrap();
        let v: serde_json::Value = serde_json::from_str(&m).unwrap();
        let d = &v["dependencies"];
        assert_eq!(
            d["next"], "^16.0.0",
            "upstream's bump, where the product kept the old pin"
        );
        assert_eq!(d["tokens"], "^0.2.0", "the product's own pin stays");
        assert_eq!(d["added"], "^1.0.0");
        assert_eq!(d["mine"], "^2.0.0");
    }

    #[test]
    fn a_file_that_already_has_upstreams_code_is_kept_when_only_comments_differ() {
        let mut lock = Lock::new();
        lock.record("m.css", b"a {}\n", "0.1.0");
        let base = "a {}\n";
        let upstream = "/* new note */\na {}\nb {}\n";
        let ours = "/* my words */\na {}\nb {}\nc {}\n";
        assert!(matches!(
            keep_if_nothing_to_add("m.css", base, ours, upstream, &mut lock, true),
            Some(TemplateOutcome::Kept(_))
        ));
        // Upstream removed a line the product still has: there is something to do.
        assert!(keep_if_nothing_to_add(
            "m.css",
            "a {}\nold {}\n",
            "a {}\nold {}\n",
            "a {}\n",
            &mut lock,
            true
        )
        .is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::dropped_sections;
    use crate::config::CONFIG_FILE;

    /// The regression this exists for.
    ///
    /// A real product's `fiducial.toml` was replaced wholesale by the
    /// scaffolding template, and `fid upgrade` called it "merged cleanly from
    /// upstream". Four declarations went with it.
    #[test]
    fn a_merge_that_deletes_a_declaration_is_not_clean() {
        let local = "[brand]\nlegal_name = \"Acme Instruments\"\n\n\
                     [capabilities]\nenabled = [\"legal\"]\n\n\
                     [i18n]\ndefault = \"sr\"\n\n\
                     [legal]\njurisdiction = \"RS\"\n\n\
                     [product]\nname = \"acme\"\n";
        let merged = "[product]\nname = \"acme\"\n\n[capabilities]\nenabled = []\n";

        let lost = dropped_sections(CONFIG_FILE, local, merged).expect("must refuse");
        assert!(lost.contains("[brand]"), "{lost}");
        assert!(lost.contains("[i18n]"), "{lost}");
        assert!(lost.contains("[legal]"), "{lost}");
    }

    #[test]
    fn an_upgrade_that_only_adds_is_allowed_through() {
        // The normal case, and the one this must not block: upstream introduces
        // a block the product did not have.
        let local = "[product]\nname = \"acme\"\n";
        let merged = "[product]\nname = \"acme\"\n\n[freshness]\ngates = []\n";
        assert!(dropped_sections(CONFIG_FILE, local, merged).is_none());
    }

    #[test]
    fn only_the_config_file_is_guarded() {
        // Every other template is prose or code. A bracketed line there is a
        // Markdown link or an array, and losing one is an ordinary upstream
        // edit rather than a lost declaration.
        let local = "[a link](x)\n[brand]\n";
        let merged = "nothing\n";
        assert!(dropped_sections("README.md", local, merged).is_none());
        assert!(dropped_sections(CONFIG_FILE, local, merged).is_some());
    }
}

#[cfg(test)]
mod conflict_marker_tests {
    use super::has_conflict_markers;

    #[test]
    fn a_file_mid_conflict_is_recognised() {
        let mid = "a = 1\n<<<<<<< ours\nb = 2\n=======\nb = 3\n>>>>>>> theirs\n";
        assert!(has_conflict_markers(mid));
    }

    #[test]
    fn an_ordinary_file_is_not() {
        assert!(!has_conflict_markers("a = 1\nb = 2\n"));
    }

    #[test]
    fn prose_discussing_the_markers_is_not() {
        // This function's own doc comment names `<<<<<<<`, and so do several
        // files in this repository. Matching a lone mention would refuse to
        // upgrade them — a new bug in place of the old one.
        let prose = "A conflict writes <<<<<<< into the file.\nResolve it by hand.\n";
        assert!(!has_conflict_markers(prose));
    }

    #[test]
    fn a_marker_must_start_its_line() {
        // Indented or quoted, it is content rather than a marker.
        let quoted = "note = \"see <<<<<<< ours\"\nother = \">>>>>>> theirs\"\n";
        assert!(!has_conflict_markers(quoted));
    }

    #[test]
    fn an_opening_marker_alone_is_not_enough() {
        // Half a marker pair is likelier to be prose than a conflict, and a
        // genuinely truncated conflict is a corrupted file rather than one
        // this can reason about either way.
        assert!(!has_conflict_markers("<<<<<<< ours\na = 1\n"));
    }
}

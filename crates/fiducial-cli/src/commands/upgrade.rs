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
    lock::{Lock, LOCK_FILE},
    migration, templates,
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(dry_run: bool, portfolio: bool) -> Result<()> {
    if portfolio {
        println!(
            "✦ fid upgrade --portfolio\n\n\
             Portfolio fan-out is coming in Phase 5.\n\
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

    // ── 1. Template 3-way merge ───────────────────────────────────────────────
    println!("  Templates");
    let template_paths: Vec<String> = lock.templates.keys().cloned().collect();
    for rel_path in &template_paths {
        if let Some(outcome) =
            merge_one_template(&root, rel_path, &cfg.product.name, &mut lock, dry_run)?
        {
            any_changes = true;
            match outcome {
                TemplateOutcome::Clean(msg) => println!("  ✓ {rel_path}: {msg}"),
                TemplateOutcome::Conflict(msg) => println!("  ⚠ {rel_path}: {msg}"),
            }
        } else {
            println!("  · {rel_path}: no upstream change");
        }
    }
    println!();

    // ── 2. Codemods ───────────────────────────────────────────────────────────
    println!("  Codemods");
    let pending_count = migration::pending(&lock.applied_migrations).len();
    if pending_count == 0 {
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
        if let Some(def) = capability::find(cap_id) {
            let skill_path = format!(".claude/skills/{cap_id}.md");
            let dest = root.join(&skill_path);
            let expanded = def
                .skill_md
                .replace("{{name}}", &cfg.product.name)
                .replace("{{version}}", PLATFORM_VERSION);
            if !dry_run {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("creating dir for {skill_path}"))?;
                }
                std::fs::write(&dest, &expanded)
                    .with_context(|| format!("writing {skill_path}"))?;
                println!("  ✓ {skill_path}: refreshed from platform");
            } else {
                println!("  · {skill_path}: would refresh from platform");
            }
            any_changes = true;
        }
    }
    println!();

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
}

/// Merge one template. Returns `None` if no upstream change, `Some(outcome)` otherwise.
fn merge_one_template(
    root: &std::path::Path,
    rel_path: &str,
    product_name: &str,
    lock: &mut Lock,
    dry_run: bool,
) -> Result<Option<TemplateOutcome>> {
    let record = match lock.templates.get(rel_path) {
        Some(r) => r.clone(),
        None => return Ok(None),
    };

    // Look up the current (in-binary) template and expand placeholders.
    let raw = match templates::raw(rel_path) {
        Some(r) => r,
        None => {
            // Template path not in registry — third-party or removed capability.
            return Ok(None);
        }
    };
    let upstream = templates::expand(raw, product_name, PLATFORM_VERSION);

    // Base content from lock (what was installed).
    let base = match &record.base_content {
        Some(b) => b.clone(),
        None => {
            // Pre-Phase-4 lock: no base stored. Treat as "cannot merge, skip".
            return Ok(None);
        }
    };

    // Has upstream changed since last install/upgrade?
    if upstream == base {
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

    // 3-way merge: base=what-was-installed, ours=local-file, theirs=upstream.
    let merged = diffy::merge(&base, &local, &upstream);

    match merged {
        Ok(clean) => {
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
            // Write conflict markers to the file — the human must resolve.
            if !dry_run {
                std::fs::write(&local_path, &conflict)
                    .with_context(|| format!("writing conflict markers to {rel_path}"))?;
                // Do NOT update lock — leave base unchanged so next `fid upgrade`
                // can retry after the human resolves the conflict.
            }
            Ok(Some(TemplateOutcome::Conflict(
                "CONFLICT — conflict markers written; resolve and run `fid upgrade` again".into(),
            )))
        }
    }
}

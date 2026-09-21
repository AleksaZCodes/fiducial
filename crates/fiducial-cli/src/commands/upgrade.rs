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

pub fn run(dry_run: bool, portfolio: bool) -> Result<()> {
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

    // ── 1. Template 3-way merge ───────────────────────────────────────────────
    println!("  Templates");
    let template_paths: Vec<String> = lock.templates.keys().cloned().collect();
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
    let renamed: Vec<(&str, &str)> = templates::RENAMED_TEMPLATES
        .iter()
        .copied()
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
        Some(b) => b.clone(),
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

    // Refuse to merge a file that still has conflict markers in it.
    //
    // The conflict branch below writes markers and deliberately leaves the lock
    // alone, so that a human can resolve them and re-run. That intent is right
    // and the implementation did not deliver it: on the next run `ours` is the
    // file *containing the markers*, so the 3-way merge runs again against a
    // base that never moved and writes markers around markers. Each run
    // compounds the damage, which is why a real product's CI carries the note
    // that `fid upgrade` "rewrites conflict markers into fiducial.toml —
    // corrupting it further every time" and why `fid doctor` is
    // `continue-on-error` there.
    //
    // Detecting the markers is what makes "resolve and re-run" true. A file
    // mid-conflict is not a file this can reason about, so it says so and
    // writes nothing.
    if has_conflict_markers(&local) {
        return Ok(Some(TemplateOutcome::Conflict(format!(
            "UNRESOLVED — {rel_path} still contains conflict markers from an \
             earlier upgrade. Nothing was written: merging it again would nest \
             markers inside markers and lose more of the file each run. Resolve \
             the markers, then re-run"
        ))));
    }

    // 3-way merge: base=what-was-installed, ours=local-file, theirs=upstream.
    let merged = diffy::merge(&base, &local, &upstream);

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
        let local = "[brand]\nlegal_name = \"Fire Outreach Network\"\n\n\
                     [capabilities]\nenabled = [\"legal\"]\n\n\
                     [i18n]\ndefault = \"sr\"\n\n\
                     [legal]\njurisdiction = \"RS\"\n\n\
                     [product]\nname = \"fon\"\n";
        let merged = "[product]\nname = \"fon\"\n\n[capabilities]\nenabled = []\n";

        let lost = dropped_sections(CONFIG_FILE, local, merged).expect("must refuse");
        assert!(lost.contains("[brand]"), "{lost}");
        assert!(lost.contains("[i18n]"), "{lost}");
        assert!(lost.contains("[legal]"), "{lost}");
    }

    #[test]
    fn an_upgrade_that_only_adds_is_allowed_through() {
        // The normal case, and the one this must not block: upstream introduces
        // a block the product did not have.
        let local = "[product]\nname = \"fon\"\n";
        let merged = "[product]\nname = \"fon\"\n\n[freshness]\ngates = []\n";
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

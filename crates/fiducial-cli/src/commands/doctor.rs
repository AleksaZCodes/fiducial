//! `fid doctor` — check for drift in the current product.
//!
//! Checks:
//!   1. `fiducial.toml` exists and parses.
//!   2. `fiducial.lock` exists and parses.
//!   3. Every template file recorded in the lock is present.
//!   4. No template file recorded in the lock has been hand-modified
//!      (its SHA-256 matches the lock record).
//!   5. (Phase 4) Templates with upstream changes available — run `fid upgrade`.
//!   6. (Phase 4) Pending codemod migrations — run `fid upgrade`.

use anyhow::{Context, Result};
use std::{env, path::Path};

use crate::{
    capability::PLATFORM_VERSION,
    config::{Config, CONFIG_FILE},
    lock::{Lock, LOCK_FILE},
    migration, templates,
};

pub fn run() -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;

    println!("✦ fid doctor — {}", root.display());
    println!();

    let mut issues: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut ok: Vec<String> = Vec::new();

    // ── 1. fiducial.toml ──────────────────────────────────────────────────
    let cfg = check_config(&root, &mut ok, &mut issues);

    // ── 2–4. fiducial.lock ────────────────────────────────────────────────
    let lock_path = root.join(LOCK_FILE);
    let lock = check_lock(&root, lock_path.as_path(), &mut ok, &mut issues);

    // ── 5. Upstream template changes available ────────────────────────────
    if let (Some(cfg), Some(lock)) = (&cfg, &lock) {
        check_upstream_templates(&root, cfg, lock, &mut warnings, &mut ok);
        // ── 6. Pending migrations ─────────────────────────────────────────
        check_pending_migrations(cfg, lock, &mut warnings, &mut ok);
    }

    // ── Report ────────────────────────────────────────────────────────────
    for msg in &ok {
        println!("  ✓ {msg}");
    }

    let has_problems = !issues.is_empty();

    if !warnings.is_empty() {
        println!();
        for w in &warnings {
            println!("  ⚠ {w}");
        }
    }

    if has_problems {
        println!();
        for issue in &issues {
            println!("  ✗ {issue}");
        }
        println!();
        anyhow::bail!(
            "fiducial doctor: {} issue{} found",
            issues.len(),
            if issues.len() == 1 { "" } else { "s" }
        )
    } else {
        println!();
        if warnings.is_empty() {
            println!("✦ fiducial doctor: clean");
        } else {
            println!(
                "✦ fiducial doctor: clean (with {} upgrade hint(s) — run `fid upgrade`)",
                warnings.len()
            );
        }
        Ok(())
    }
}

// ── Check helpers ─────────────────────────────────────────────────────────────

fn check_config(root: &Path, ok: &mut Vec<String>, issues: &mut Vec<String>) -> Option<Config> {
    let path = root.join(CONFIG_FILE);
    match Config::load(&path) {
        Ok(cfg) => {
            ok.push(format!(
                "fiducial.toml valid  (product: {}, v{})",
                cfg.product.name, cfg.product.version
            ));
            Some(cfg)
        }
        Err(e) => {
            issues.push(format!("fiducial.toml: {e}"));
            None
        }
    }
}

fn check_lock(
    root: &Path,
    lock_path: &Path,
    ok: &mut Vec<String>,
    issues: &mut Vec<String>,
) -> Option<Lock> {
    if !lock_path.exists() {
        issues.push(
            "fiducial.lock not found. Run `fid new` to scaffold, or \
             `fid upgrade` if upgrading an existing product."
                .into(),
        );
        return None;
    }

    let lock = match Lock::load(lock_path) {
        Ok(l) => l,
        Err(e) => {
            issues.push(format!("fiducial.lock: {e}"));
            return None;
        }
    };

    ok.push(format!(
        "fiducial.lock valid  ({} template(s) tracked, {} migration(s) applied)",
        lock.templates.len(),
        lock.applied_migrations.len()
    ));

    // Check every recorded template file using lock.verify().
    let raw_issues = lock.verify(root);
    let mut drifted: Vec<String> = raw_issues
        .into_iter()
        .map(|(path, problem)| {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            format!("{rel}: {problem}. If intentional, run `fid upgrade` to re-baseline.")
        })
        .collect();

    if drifted.is_empty() {
        ok.push("all template files unmodified".into());
    } else {
        issues.append(&mut drifted);
    }

    Some(lock)
}

fn check_upstream_templates(
    _root: &Path,
    cfg: &Config,
    lock: &Lock,
    warnings: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    let mut upgrades_available = 0usize;

    for (rel_path, record) in &lock.templates {
        let base = match &record.base_content {
            Some(b) => b,
            None => continue, // Pre-Phase-4 lock entry — skip.
        };
        let raw = match templates::raw(rel_path) {
            Some(r) => r,
            None => continue, // Path not in registry.
        };
        let upstream = templates::expand(raw, &cfg.product.name, PLATFORM_VERSION);
        if upstream != *base {
            warnings.push(format!(
                "{rel_path}: upstream template updated (installed: {}, current: {}) — run `fid upgrade`",
                record.source_version, PLATFORM_VERSION
            ));
            upgrades_available += 1;
        }
    }

    if upgrades_available == 0 {
        ok.push(format!(
            "templates up to date with platform v{PLATFORM_VERSION}"
        ));
    }
}

fn check_pending_migrations(
    cfg: &Config,
    lock: &Lock,
    warnings: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    // Only surface migrations whose capability prefix is installed in this product.
    // A migration id has the form `<capability-id>/<version>/<slug>`.
    // Migrations for uninstalled capabilities are silently skipped — they would
    // modify 0 files anyway, and showing them to the user is misleading.
    let pending: Vec<_> = migration::pending(&lock.applied_migrations)
        .into_iter()
        .filter(|m| {
            let cap_prefix = m.id.split('/').next().unwrap_or("");
            cfg.capabilities.enabled.iter().any(|c| c == cap_prefix)
        })
        .collect();

    if pending.is_empty() {
        ok.push("no pending codemod migrations".into());
    } else {
        for m in &pending {
            warnings.push(format!(
                "migration pending: {} — {} (run `fid upgrade`)",
                m.id, m.description
            ));
        }
    }
}

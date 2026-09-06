//! `fid doctor` — check for drift in the current product.
//!
//! Checks (Phase 3 scope):
//!   1. `fiducial.toml` exists and parses.
//!   2. `fiducial.lock` exists and parses.
//!   3. Every template file recorded in the lock is present.
//!   4. No template file recorded in the lock has been hand-modified
//!      (its SHA-256 matches the lock record).
//!
//! Checks deferred to Phase 4+:
//!   - `fid upgrade` available migrations not yet applied.
//!   - Outdated capability versions.
//!   - Stale derived artifacts (`fid derive --check`).

use anyhow::{Context, Result};
use std::{env, path::Path};

use crate::{
    config::{Config, CONFIG_FILE},
    lock::{Lock, LOCK_FILE},
};

pub fn run() -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;

    println!("✦ fid doctor — {}", root.display());
    println!();

    let mut issues: Vec<String> = Vec::new();
    let mut ok: Vec<String> = Vec::new();

    // ── 1. fiducial.toml ──────────────────────────────────────────────────
    check_config(&root, &mut ok, &mut issues);

    // ── 2. fiducial.lock ──────────────────────────────────────────────────
    let lock_path = root.join(LOCK_FILE);
    check_lock(&root, lock_path.as_path(), &mut ok, &mut issues);

    // ── Report ────────────────────────────────────────────────────────────
    for msg in &ok {
        println!("  ✓ {msg}");
    }

    if issues.is_empty() {
        println!();
        println!("✦ fiducial doctor: clean");
        Ok(())
    } else {
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
    }
}

fn check_config(root: &Path, ok: &mut Vec<String>, issues: &mut Vec<String>) {
    let path = root.join(CONFIG_FILE);
    match Config::load(&path) {
        Ok(cfg) => {
            ok.push(format!(
                "fiducial.toml valid  (product: {}, v{})",
                cfg.product.name, cfg.product.version
            ));
        }
        Err(e) => {
            issues.push(format!("fiducial.toml: {e}"));
        }
    }
}

fn check_lock(root: &Path, lock_path: &Path, ok: &mut Vec<String>, issues: &mut Vec<String>) {
    if !lock_path.exists() {
        issues.push(
            "fiducial.lock not found. Run `fid new` to scaffold, or \
             `fid upgrade` if upgrading an existing product."
                .into(),
        );
        return;
    }

    let lock = match Lock::load(lock_path) {
        Ok(l) => l,
        Err(e) => {
            issues.push(format!("fiducial.lock: {e}"));
            return;
        }
    };

    ok.push(format!(
        "fiducial.lock valid  ({} template(s) tracked)",
        lock.templates.len()
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
}

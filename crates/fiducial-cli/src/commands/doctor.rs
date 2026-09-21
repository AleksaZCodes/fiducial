//! `fid doctor` — check for drift in the current product.
//!
//! Checks:
//!   1. `fiducial.toml` exists and parses.
//!   2. `fiducial.lock` exists and parses.
//!   3. Every template file recorded in the lock is present.
//!   4. No template file recorded in the lock has been hand-modified
//!      (its SHA-256 matches the lock record).
//!   5. Templates with upstream changes available — run `fid upgrade`.
//!   6. Pending codemod migrations — run `fid upgrade`.

use anyhow::{Context, Result};
use std::{collections::BTreeMap, env, path::Path};

use crate::{
    adapter,
    capability::PLATFORM_VERSION,
    config::{Config, CONFIG_FILE},
    guard,
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
    // Kept apart from `warnings`, which the summary line calls "upgrade
    // hint(s)". A hardcoded string is not something `fid upgrade` fixes, and
    // folding it in would make that line untrue.
    let mut reports: Vec<String> = Vec::new();

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

    // ── 7. Adapter selections ─────────────────────────────────────────────
    if let Some(cfg) = &cfg {
        check_adapters(cfg, lock.as_ref(), &mut issues, &mut ok);
    }

    // ── 8. Hardcoded user-visible strings ─────────────────────────────────
    if let Some(cfg) = &cfg {
        check_hardcoded_strings(&root, cfg, &mut reports, &mut ok);
    }

    // ── 9. External tools the installed capabilities need ─────────────────
    if let Some(cfg) = &cfg {
        check_tools(cfg, &mut reports, &mut ok);
    }

    // ── 10. Does the remote enforce what `[guard]` claims? ────────────────
    if let Some(cfg) = &cfg {
        check_branch_protection(&root, cfg, &mut reports, &mut ok);
    }

    // ── 11. Does every web app still send its security headers? ───────────
    check_security_headers(&root, &mut issues, &mut ok);

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

    if !reports.is_empty() {
        println!();
        for r in &reports {
            println!("  · {r}");
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

/// Every `[adapters]` selection resolves, and every required contract is filled.
///
/// An **issue**, not a report: a selection that names nothing is a build that
/// cannot work, and a capability whose contract nobody filled is a product that
/// will discover the gap at runtime. Both are unambiguous, which is the test
/// for whether something belongs in `issues`.
fn check_adapters(
    cfg: &Config,
    lock: Option<&Lock>,
    issues: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    let mut bad = false;
    for (contract, vendor) in &cfg.adapters.selected {
        if let Some(problem) = adapter::problem(contract, vendor) {
            issues.push(format!("fiducial.toml [adapters]: {problem}"));
            bad = true;
        }
    }

    // A capability can need a contract without choosing the vendor — that is
    // the point of the split. What it cannot do is need one nobody filled.
    for id in &cfg.capabilities.enabled {
        // The lock first: a capability resolved from outside the binary is not
        // in the built-in registry, and its requirements are still its
        // requirements.
        let required: Vec<String> = match lock.and_then(|l| l.capabilities.get(id)) {
            Some(record) => record.requires_adapters.clone(),
            None => match crate::capability::find(id) {
                Some(cap) => cap.requires_adapters.clone(),
                None => continue,
            },
        };
        for contract in &required {
            if cfg.adapters.get(contract).is_none() {
                issues.push(format!(
                    "capability `{id}` requires the `{contract}` adapter, and \
                     `[adapters]` selects none. Add `{contract} = \"{}\"` to \
                     wire it in as a no-op, or name a vendor.",
                    adapter::NONE
                ));
                bad = true;
            }
        }
    }

    if !bad && !cfg.adapters.is_empty() {
        ok.push(format!(
            "adapters: {} contract(s) selected, all resolvable",
            cfg.adapters.selected.len()
        ));
    }
}

/// Report user-visible strings that never reached a catalog.
///
/// **Never an issue.** `fid doctor` exits non-zero on issues, and the detector
/// is a heuristic — it cannot tell prose from a `data-testid` with certainty.
/// Making it fatal would mean every false positive blocks someone until the
/// rule gets loosened for everybody. Reported instead, where it cannot be
/// scrolled past, and carried in `fid dash --json` for an agent to act on.
fn check_hardcoded_strings(
    root: &Path,
    cfg: &Config,
    reports: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    if cfg.i18n.is_empty() {
        return;
    }
    let found = crate::i18n::scan_product(root);
    if found.is_empty() {
        ok.push("no hardcoded user-visible strings found".to_string());
        return;
    }
    reports.push(format!(
        "{} hardcoded user-visible string(s) — these warn, they never fail a build:",
        found.len()
    ));
    for h in found.iter().take(MAX_HARDCODED_LISTED) {
        reports.push(format!(
            "  {}:{}  {} {:?}",
            h.file, h.line, h.context, h.text
        ));
    }
    if found.len() > MAX_HARDCODED_LISTED {
        reports.push(format!(
            "  … and {} more — `fid dash --section i18n --json` for the full list",
            found.len() - MAX_HARDCODED_LISTED
        ));
    }
}

/// How many findings `fid doctor` lists before summarising.
const MAX_HARDCODED_LISTED: usize = 10;

/// Is every external command the installed capabilities need actually on PATH?
///
/// A capability can install every file it owns and still not work. `deploy`
/// derives a `wrangler.toml` that only `wrangler` can act on; a product whose
/// CI opens PRs needs `gh`. Those are dependencies, and until now they were the
/// kind that announces itself as a command-not-found halfway through a release
/// rather than at the moment the capability is installed.
///
/// Reported, not failed: a missing tool is a fact about this machine, and CI
/// runners legitimately have a different set from a laptop. `fid doctor` says
/// what is absent; it does not decide that absence is wrong.
fn check_tools(cfg: &Config, reports: &mut Vec<String>, ok: &mut Vec<String>) {
    // `git` is not capability-specific — `fid new` runs it, and every guard
    // rule about pushing presumes it.
    let mut needed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    needed
        .entry("git".to_string())
        .or_default()
        .push("the platform".into());

    for id in &cfg.capabilities.enabled {
        let Some(cap) = crate::capability::find(id) else {
            continue;
        };
        for tool in &cap.requires_tools {
            needed.entry(tool.clone()).or_default().push(id.clone());
        }
    }

    let mut missing: Vec<String> = Vec::new();
    for (tool, wanted_by) in &needed {
        if which(tool).is_none() {
            missing.push(format!("`{tool}` — needed by {}", wanted_by.join(", ")));
        }
    }

    if missing.is_empty() {
        ok.push(format!("tooling: {} command(s) present", needed.len()));
    } else {
        for m in missing {
            reports.push(format!("not on PATH: {m}"));
        }
    }
}

/// Is `<tool>` on PATH?
///
/// Spelled out rather than shelling out to `which`/`where`: the answer differs
/// per platform and a subprocess to answer "does this file exist" is a
/// subprocess per tool per run.
fn which(tool: &str) -> Option<std::path::PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(tool);
        candidate.is_file().then_some(candidate)
    })
}

/// Does the remote enforce the guard rule this product declares?
///
/// `no-direct-main-push` is a local hook. It fires on `git push` from a machine
/// that has the hook installed, and on no other machine, and on nothing CI or a
/// token does. A product declaring it therefore believes main is protected while
/// GitHub allows anyone to push to it.
///
/// This repository already treats a guard rule with no implementation as a
/// finding rather than a shrug — "a product listing it believes it is guarded
/// and is not". This is the same sentence one layer out, and it went unsaid
/// until someone noticed the branch was open.
///
/// Reported, not failed: not every product has a GitHub remote, and protection
/// needs admin rights `fid` cannot assume it has.
fn check_branch_protection(
    root: &Path,
    cfg: &Config,
    reports: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    if !cfg.guard.rules.iter().any(|r| r == "no-direct-main-push") {
        return;
    }
    let Some(slug) = github_slug(root) else {
        return; // No GitHub remote — nothing to ask about.
    };
    if which("gh").is_none() {
        reports.push(format!(
            "`[guard] no-direct-main-push` is declared and `gh` is not on PATH, \
             so whether {slug} actually protects main could not be checked"
        ));
        return;
    }

    let out = std::process::Command::new("gh")
        .args([
            "api",
            &format!("repos/{slug}/branches/main/protection"),
            "--silent",
        ])
        .output();

    match out {
        Ok(o) if o.status.success() => {
            ok.push(format!("{slug}: main is protected on the remote"));
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            if err.contains("Branch not protected") {
                reports.push(format!(
                    "{slug}: `[guard] no-direct-main-push` is declared, but main is \
                     NOT protected on the remote.\n      \
                     The hook stops this machine; it stops nothing else.\n      \
                     Fix it: fid repo protect --apply"
                ));
            } else {
                // 403 without admin, no network, not logged in — all real, none
                // of them evidence that the branch is open.
                reports.push(format!(
                    "{slug}: could not read branch protection ({})",
                    err.lines().next().unwrap_or("unknown error").trim()
                ));
            }
        }
        Err(e) => reports.push(format!("{slug}: could not run `gh` ({e})")),
    }
}

/// `owner/repo` for the `origin` remote, when it is a GitHub one.
fn github_slug(root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))?;
    Some(rest.trim_end_matches(".git").to_string())
}

fn check_config(root: &Path, ok: &mut Vec<String>, issues: &mut Vec<String>) -> Option<Config> {
    let path = root.join(CONFIG_FILE);
    match Config::load(&path) {
        Ok(cfg) => {
            ok.push(format!(
                "fiducial.toml valid  (product: {}, v{})",
                cfg.product.name, cfg.product.version
            ));

            // A guard rule name with no implementation is the one thing worse
            // than no guard rule: the product believes it is protected. This
            // shipped for five phases — `no-hand-edit-generated` was in every
            // scaffold and has never existed — so an existing product is told
            // rather than silently downgraded when the scaffold drops it.
            let unknown: Vec<&String> = cfg
                .guard
                .rules
                .iter()
                .filter(|r| guard::rule_by_name(r).is_none())
                .collect();
            if unknown.is_empty() {
                ok.push(format!(
                    "guard: {} rule(s), all implemented",
                    cfg.guard.rules.len()
                ));
            } else {
                for name in unknown {
                    issues.push(format!(
                        "fiducial.toml [guard]: `{name}` is not an implemented rule — it \
                         guards nothing. Remove it from `rules`, or run `fid upgrade` to \
                         take the current list."
                    ));
                }
            }
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

    // Check every recorded template file using lock.verify(), then split the
    // result by who owns the file.
    //
    // Editing a product-owned file is the intended use — `MISSION.md`'s own
    // footer says "It is product-owned — edit it freely" — so reporting it as
    // an issue meant the platform contradicted its own templates. fon had
    // seventeen of them and runs `fid doctor` with `continue-on-error` as a
    // result, which costs every real finding too. See `crate::ownership`.
    let mut drifted: Vec<String> = Vec::new();
    let mut owned_edits = 0usize;

    for (path, problem) in lock.verify(root) {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        // `fiducial.lock` keys are forward-slashed; a Windows path is not.
        let key = rel.replace('\\', "/");

        if crate::ownership::of(&key).is_product() {
            owned_edits += 1;
            continue;
        }
        drifted.push(format!(
            "{rel}: {problem}. This file is platform-owned — `fid upgrade` will \
             rewrite it, so an edit here will be merged over."
        ));
    }

    if owned_edits > 0 {
        // Counted rather than silent: "you have edited 17 of your own files" is
        // a true and occasionally useful thing to know. It is not a problem,
        // so it goes in the ✓ column.
        ok.push(format!(
            "{owned_edits} product-owned file(s) edited — expected, not drift"
        ));
    }

    if drifted.is_empty() {
        ok.push("no platform-owned file has been modified".into());
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
        let raw = match templates::raw_for(rel_path, &cfg.capabilities.enabled) {
            Some(r) => r,
            None => continue, // Path not in registry.
        };
        // Compared at the version the product recorded, not at this binary's:
        // otherwise every template carrying a `{{version}}` stamp reads as
        // drift the moment the platform version moves. See
        // `templates::upstream_changed`.
        if templates::upstream_changed(raw, &cfg.product.name, &record.source_version, base) {
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

/// Every web app this product has still declares the required response headers.
///
/// # Why this is an issue and not a warning
///
/// A missing security header is not drift from a preference — it is protection
/// the product is believed to have and does not. `fire.outreachnet.work` served
/// none of these for its whole life: the capability template never had them, no
/// build failed, no page broke, and the gap was visible only to someone who
/// thought to run `curl -I`. A warning would have been read the same way the
/// absence was.
///
/// # Why it reads the config and not the deployed response
///
/// `fid` makes no network calls, and a gate that probes production cannot run in
/// CI before a deploy, cannot run offline, and answers about what *is* deployed
/// rather than what is about to be. The declaration is the thing a commit can
/// change, so the declaration is what this checks — see `crate::security` for
/// the coarseness that buys and what it therefore cannot catch.
///
/// Shapes the product does not have are skipped silently. A product with no web
/// app is not failing a web rule.
fn check_security_headers(root: &Path, issues: &mut Vec<String>, ok: &mut Vec<String>) {
    use crate::security::{missing_headers, WEB_APP_SHAPES};

    let mut checked = 0usize;

    for shape in WEB_APP_SHAPES {
        if !root.join(shape.marker).exists() {
            continue;
        }
        checked += 1;

        let config = root.join(shape.config);
        let Ok(source) = std::fs::read_to_string(&config) else {
            issues.push(format!(
                "{} app has no `{}`, so nothing sets its response headers.\n      \
                 Every request it serves goes out without HSTS, CSP, or any of the rest.",
                shape.name, shape.config
            ));
            continue;
        };

        let missing = missing_headers(&source);
        if missing.is_empty() {
            continue;
        }

        let mut detail = format!(
            "{} app is missing {} required security header(s) in `{}`:",
            shape.name,
            missing.len(),
            shape.config
        );
        for h in &missing {
            detail.push_str(&format!(
                "\n      · {} — without it: {}",
                h.name, h.protects
            ));
        }
        detail.push_str(
            "\n      Fix by extending the platform set rather than restating it:\n        \
             import { securityHeaders } from … ; headers: [...securityHeaders]\n      \
             Reasoning and the HSTS preload caveat: docs/specs/2026-09-20-hsts-preload.md",
        );
        issues.push(detail);
    }

    if checked > 0 && issues.is_empty() {
        ok.push(format!(
            "security headers: {checked} web app(s) declare all {} required",
            crate::security::SECURITY_HEADERS
                .iter()
                .filter(|h| h.required)
                .count()
        ));
    }
}

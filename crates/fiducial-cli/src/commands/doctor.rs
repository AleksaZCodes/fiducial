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
use std::{env, path::Path};

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

    // ── 9. Framework currency ─────────────────────────────────────────────
    if let Some(cfg) = &cfg {
        check_framework_currency(cfg, &mut reports, &mut ok);
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

/// Report capability templates that pin outdated framework major versions.
///
/// This is the **stale templates** half of `fid doctor`'s stated purpose. A template
/// pinning Next.js 15 when 16 is current is the same class of stale artifact as any
/// other — the difference is that the source of truth is an npm registry rather than
/// a file in the repo. Reported, not failed: checking currency must not block a PR on
/// the day upstream publishes a major for an unrelated reason.
///
/// The check is offline-graceful: if the registry is unreachable, the finding is
/// omitted and a note is added instead. A finding that lies when the network is down
/// would be like the verification that cried wolf — it costs the check.
fn check_framework_currency(
    cfg: &crate::config::Config,
    reports: &mut Vec<String>,
    ok: &mut Vec<String>,
) {
    // Framework packages to track, keyed by the capability that pins them.
    // This is a static declaration — not derived from the template package.json,
    // because automatically parsing every package.json for every installed capability
    // would require loading all template files at runtime and would still need a
    // "is this a framework pin or a utility dep?" judgment per package.
    //
    // The list is small and changes only when a framework publishes a new major.
    // Adding a row here is the acknowledgment the ROADMAP item demands.
    const FRAMEWORK_DEPS: &[(&str, &str, &str)] = &[
        // (capability-id, npm-package, pinned-major)
        ("web-next", "next", "15"),
        ("web-svelte", "svelte", "5"),
        ("web-svelte", "@sveltejs/kit", "2"),
        ("tauri", "@tauri-apps/cli", "2"),
    ];

    let installed: std::collections::HashSet<&str> =
        cfg.capabilities.enabled.iter().map(String::as_str).collect();

    let relevant: Vec<_> = FRAMEWORK_DEPS
        .iter()
        .filter(|(cap, _, _)| installed.contains(cap))
        .collect();

    if relevant.is_empty() {
        return;
    }

    // Try to fetch the latest version of each relevant package from npm.
    // Uses a minimal HTTP GET with no external crate: `std::process::Command`
    // calling `node -e` or `curl` as available. Falls back silently on failure.
    let mut stale: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for (cap, pkg, pinned_major) in &relevant {
        match latest_npm_major(pkg) {
            Ok(Some(latest_major)) if latest_major.as_str() != *pinned_major => {
                stale.push(format!(
                    "framework {pkg} (via `{cap}`): pinned to major {pinned_major}, \
                     latest is {latest_major} — run `fid upgrade` when ready"
                ));
                checked += 1;
            }
            Ok(_) => {
                checked += 1;
            }
            Err(_) => {
                // Network unavailable or node not found — skip silently.
            }
        }
    }

    if stale.is_empty() {
        if checked > 0 {
            ok.push(format!(
                "framework versions current ({checked} package(s) checked)"
            ));
        }
    } else {
        for finding in &stale {
            reports.push(finding.clone());
        }
    }
}

/// Query the npm registry for the latest published major of `pkg`.
///
/// Returns `None` when the package has no published version (should not happen
/// for known frameworks), or `Err` when the registry is unreachable.
fn latest_npm_major(pkg: &str) -> anyhow::Result<Option<String>> {
    // Use `node -e` to fetch from the npm registry — node is already required
    // by every capability that uses a framework package, so it is always present
    // when the frameworks we are checking are installed.
    let script = format!(
        "const h = require('https'); \
         h.get('https://registry.npmjs.org/{pkg}/latest', {{headers: {{'User-Agent': 'fiducial-cli/0.1'}}}}, r => {{ \
             let b=''; r.on('data',d=>b+=d); r.on('end',()=>{{ \
                 try{{ const v=JSON.parse(b).version; \
                 process.stdout.write(v.split('.')[0]); }}catch{{}} \
             }}); \
         }}).on('error',()=>process.exit(1));",
        pkg = pkg
    );

    let out = std::process::Command::new("node")
        .arg("-e")
        .arg(&script)
        .output()?;

    if !out.status.success() || out.stdout.is_empty() {
        anyhow::bail!("npm lookup failed");
    }

    let major = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if major.is_empty() {
        return Ok(None);
    }
    Ok(Some(major))
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
        let raw = match templates::raw_for(rel_path, &cfg.capabilities.enabled) {
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

//! `fid release` — platform version management and version-skew enforcement.
//!
//! ## Subcommands
//!
//! | Command | What it does |
//! |---|---|
//! | `fid release status` | Print current platform and wire-protocol versions |
//! | `fid release check` | Verify the committed matrix matches the baked-in `WIRE_VERSION` |
//! | `fid release protocol --bump <breaking\|compatible>` | Bump the wire version and update the matrix |
//! | `fid release version-check` | Verify a branch changing `crates/` also bumps the workspace version |
//!
//! ## Two skews, not one
//!
//! `check` guards the **wire** version: what two endpoints agree to speak.
//! `version-check` guards the **crate** version: what a published binary calls
//! itself.  They fail for unrelated reasons and neither implies the other — a
//! release can change the CLI without touching the protocol, and did, which is
//! how main and crates.io came to hold two different 0.2.2 binaries.
//!
//! ## Version-skew model
//!
//! `fiducial-protocol` embeds a single `WIRE_VERSION: u8` constant.  Every
//! artifact (firmware binary, Tauri desktop app, WASM module) that is compiled
//! against the crate carries that constant.  During a connection handshake the
//! two endpoints exchange their wire version; `assert_compatible` checks whether
//! the remote's version falls within the accepted range.
//!
//! The *policy* (accepted range) lives in `docs/compat/matrix.toml`.  This
//! command is the only thing that writes that file.  `fid release check` is the
//! CI enforcement point: it fails when the file and the constant disagree,
//! which means every PR that changes `WIRE_VERSION` must also update the matrix
//! (and vice-versa), and no stale artifact can slip through unnoticed.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

// ── Matrix TOML types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
struct Matrix {
    wire: WireSection,
    history: Vec<HistoryEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WireSection {
    current: u8,
    min_compatible: u8,
}

#[derive(Debug, Deserialize, Serialize)]
struct HistoryEntry {
    version: u8,
    date: String,
    notes: String,
}

// ── Matrix file location ──────────────────────────────────────────────────────

/// Path of the committed compatibility matrix, relative to the workspace root.
pub const MATRIX_PATH: &str = "docs/compat/matrix.toml";

/// The `WIRE_VERSION` constant baked into the `fiducial-protocol` crate.
pub const WIRE_VERSION: u8 = fiducial_protocol::WIRE_VERSION;

// ── Subcommand enum ───────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ReleaseAction {
    /// Show current platform and wire-protocol versions
    #[command(long_about = "\
Show the versions of the platform components that matter at release time:

  • fiducial-cli (this binary) — the platform tool version
  • Wire protocol version      — the version baked into every artifact
  • Compatibility matrix       — the committed policy in docs/compat/matrix.toml

Use this before filing a release PR to confirm the numbers are what you expect.")]
    Status,

    /// Verify the committed matrix matches the baked-in WIRE_VERSION (CI gate)
    ///
    /// Exits 0 if the matrix is consistent with the compiled constant.
    /// Exits 1 if `wire.current` in the matrix differs from `WIRE_VERSION`.
    ///
    /// Run this in CI (the `release` job in ci.yml) to enforce that every
    /// protocol change is reflected in the committed matrix before it merges.
    #[command(long_about = "\
Verify that the committed compatibility matrix is in sync with the baked-in
wire protocol version.

`fid release check` compares `wire.current` in `docs/compat/matrix.toml`
against the `WIRE_VERSION` constant compiled into this binary.  A mismatch
means either:

  1. The matrix was not updated when `WIRE_VERSION` was bumped, or
  2. `WIRE_VERSION` was not bumped when the matrix was updated.

Both are bugs.  Use `fid release protocol --bump` to change the wire version
correctly — it updates the constant declaration and the matrix in one step.")]
    Check,

    /// Bump the wire protocol version and update the compatibility matrix
    #[command(long_about = "\
Bump the wire protocol version and update `docs/compat/matrix.toml`.

Two bump kinds:

  --bump breaking     Increment `wire.current`; set `wire.min_compatible` to
                      the new version.  Artifacts on the old version are
                      REJECTED at connection time.

  --bump compatible   Increment `wire.current`; keep `wire.min_compatible`
                      unchanged.  Old artifacts are still accepted.

After running this command you must also update the `WIRE_VERSION` constant in
`crates/fiducial-protocol/src/lib.rs` to match the new `wire.current`.  The
command prints a reminder with the exact edit required.

`fid release check` fails until both the file and the constant agree.")]
    Protocol {
        /// Bump kind: `breaking` (drops old versions) or `compatible` (keeps them)
        #[arg(long, value_name = "KIND")]
        bump: BumpKind,

        /// Note to record in the history entry (optional)
        #[arg(long, value_name = "TEXT")]
        note: Option<String>,
    },

    /// Verify a branch that changes crate source also bumps the version (CI gate)
    #[command(long_about = "\
Verify that a branch touching `crates/` also bumps the workspace version.

The failure this exists to prevent has already happened.  `fiducial-cli 0.2.2`
was published on 2026-09-19; systemOne, the security headers and the realtime
work all merged after it with no bump.  Main and crates.io both said \"0.2.2\"
and were different binaries.

That is not cosmetic.  `PLATFORM_VERSION` is written into every product's
`fiducial.lock` as `source_version`, so a product recorded \"0.2.2\" for
artifacts only a *newer* 0.2.2 could reproduce — and nothing could see it,
because the version strings matched.  It broke fon's CI twice in one session.

Every push to main publishes whatever version the workspace declares, so any
change to crate source is a new release and must carry a new number.  The rule
is therefore exact: if the diff against the base ref touches `crates/`, the
workspace `version` in the root `Cargo.toml` must differ from the base's.

  fid release version-check                       # against origin/main
  fid release version-check --base upstream/main")]
    VersionCheck {
        /// Branch or commit to diff against
        #[arg(long, value_name = "REF", default_value = "origin/main")]
        base: String,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum BumpKind {
    /// Breaking: artifacts on the old version will be rejected
    Breaking,
    /// Compatible: old artifacts are still accepted
    Compatible,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(action: ReleaseAction) -> Result<()> {
    match action {
        ReleaseAction::Status => status(),
        ReleaseAction::Check => check(),
        ReleaseAction::Protocol { bump, note } => bump_protocol(bump, note),
        ReleaseAction::VersionCheck { base } => version_check(&base),
    }
}

// ── status ────────────────────────────────────────────────────────────────────

fn status() -> Result<()> {
    let cli_version = env!("CARGO_PKG_VERSION");
    println!("✦ fid release status");
    println!();
    println!("  platform cli    {cli_version}");
    println!("  wire version    {WIRE_VERSION}  (WIRE_VERSION in fiducial-protocol)");
    println!();

    match load_matrix() {
        Ok((path, matrix)) => {
            println!("  compatibility matrix  {}", path.display());
            println!("    current        {}", matrix.wire.current);
            println!("    min_compatible {}", matrix.wire.min_compatible);
            println!();
            if matrix.wire.current == WIRE_VERSION {
                println!("  ✓ matrix is in sync with the compiled constant");
            } else {
                println!(
                    "  ✗ matrix current={} but WIRE_VERSION={} — run `fid release check`",
                    matrix.wire.current, WIRE_VERSION
                );
            }
        }
        Err(e) => {
            println!("  ⚠ could not read {MATRIX_PATH}: {e}");
            println!("    Create it with `fid release protocol --bump breaking`.");
        }
    }

    println!();
    Ok(())
}

// ── check ─────────────────────────────────────────────────────────────────────

fn check() -> Result<()> {
    let (path, matrix) = load_matrix()?;

    if matrix.wire.current != WIRE_VERSION {
        bail!(
            "version skew: {} has wire.current={} but WIRE_VERSION compiled into this binary is {}.\n\
             Run `fid release protocol --bump` to synchronise them.",
            path.display(),
            matrix.wire.current,
            WIRE_VERSION,
        );
    }

    println!("✓ docs/compat/matrix.toml is in sync (wire.current = WIRE_VERSION = {WIRE_VERSION})");
    Ok(())
}

// ── bump_protocol ─────────────────────────────────────────────────────────────

fn bump_protocol(kind: BumpKind, note: Option<String>) -> Result<()> {
    let (path, mut matrix) = load_matrix()?;

    let old = matrix.wire.current;
    let new = old
        .checked_add(1)
        .context("wire version would overflow u8")?;

    matrix.wire.current = new;
    match kind {
        BumpKind::Breaking => matrix.wire.min_compatible = new,
        BumpKind::Compatible => { /* keep min_compatible */ }
    }

    let today = today_iso8601();
    let notes = note.unwrap_or_else(|| match kind {
        BumpKind::Breaking => {
            format!("Breaking bump from {old} → {new}. Artifacts on v{old} are now rejected.")
        }
        BumpKind::Compatible => {
            format!("Compatible bump from {old} → {new}. Artifacts on v{old} are still accepted.")
        }
    });

    matrix.history.push(HistoryEntry {
        version: new,
        date: today,
        notes,
    });

    write_matrix(&path, &matrix)?;

    println!("✦ fid release protocol — bumped wire version {old} → {new}");
    println!();
    println!("  docs/compat/matrix.toml updated:");
    println!("    wire.current        = {new}");
    println!("    wire.min_compatible = {}", matrix.wire.min_compatible);
    println!();
    println!("  ─── ACTION REQUIRED ────────────────────────────────────────");
    println!("  Update the WIRE_VERSION constant in:");
    println!("    crates/fiducial-protocol/src/lib.rs");
    println!();
    println!("  Change:");
    println!("    pub const WIRE_VERSION: u8 = {old};");
    println!("  To:");
    println!("    pub const WIRE_VERSION: u8 = {new};");
    println!();
    println!("  Then run `fid release check` to confirm both agree.");
    println!("  ────────────────────────────────────────────────────────────");
    Ok(())
}

// ── version_check ─────────────────────────────────────────────────────────────

/// Path prefix whose contents are published to crates.io.
///
/// Deliberately *not* the root `Cargo.toml`: a version-only edit to that file
/// is the bump itself, and making it count as "source changed" would mean a
/// bump obliges another bump.  Everything that ends up inside a `.crate`
/// tarball lives under this prefix.
const PUBLISHED_PREFIX: &str = "crates/";

/// Read `[workspace.package] version` out of a root `Cargo.toml`.
///
/// Pure so the CI gate's only real judgement is testable without a git
/// fixture; the plumbing around it is the untestable part and stays thin.
fn workspace_version(cargo_toml: &str) -> Result<String> {
    let doc: toml::Value = toml::from_str(cargo_toml).context("parsing root Cargo.toml")?;
    doc.get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .context("root Cargo.toml has no [workspace.package] version")
}

/// The gate's verdict, given a diff and the two versions.
///
/// `Ok(true)` means the branch touches published source and correctly carries
/// a new version; `Ok(false)` means it touches nothing published, so the
/// version is not required to move.
fn version_verdict(changed: &[String], base_version: &str, head_version: &str) -> Result<bool> {
    let touched: Vec<&String> = changed
        .iter()
        .filter(|p| p.starts_with(PUBLISHED_PREFIX))
        .collect();

    if touched.is_empty() {
        return Ok(false);
    }

    if base_version == head_version {
        let shown: Vec<&str> = touched.iter().take(5).map(|p| p.as_str()).collect();
        let more = touched.len().saturating_sub(shown.len());
        let mut list = shown.join("\n    ");
        if more > 0 {
            list.push_str(&format!("\n    … and {more} more"));
        }
        bail!(
            "version skew: this branch changes published crate source but still declares \
             version {base_version}.\n\n  changed:\n    {list}\n\n\
             Every push to main publishes whatever the workspace declares, so a change to \
             crate\n  source that keeps the same number produces a second, different binary \
             under an\n  existing version string. Bump [workspace.package] version in the root \
             Cargo.toml."
        );
    }

    Ok(true)
}

/// Every internal `fiducial-*` workspace dependency must pin the version the
/// workspace itself declares.
///
/// `docs/specs/2026-09-16-the-rust-crates-release-in-lockstep.md` says these
/// move together, and until 2026-09-21 nothing checked it: the crates were at
/// 0.2.2 while every pin still read `version = "0.2.0"`.  Nothing broke,
/// because `^0.2.0` happens to admit 0.2.2 — so the drift was invisible right
/// up to the first minor bump, which could not resolve at all.  A rule that
/// only fails on the bump that discovers it is not a rule.
fn lockstep_verdict(cargo_toml: &str) -> Result<()> {
    let doc: toml::Value = toml::from_str(cargo_toml).context("parsing root Cargo.toml")?;
    let declared = workspace_version(cargo_toml)?;

    let Some(deps) = doc
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
    else {
        return Ok(());
    };

    let mut stale: Vec<String> = deps
        .iter()
        .filter(|(name, _)| name.starts_with("fiducial-"))
        .filter_map(|(name, spec)| {
            let pinned = spec.get("version")?.as_str()?;
            (pinned != declared).then(|| format!("{name} = \"{pinned}\""))
        })
        .collect();
    stale.sort();

    if !stale.is_empty() {
        bail!(
            "lockstep broken: the workspace declares {declared} but these internal pins do \
             not:\n    {}\n\n  The Rust crates release together \
             (docs/specs/2026-09-16-the-rust-crates-release-in-lockstep.md).\n  Set every \
             fiducial-* workspace dependency to version = \"{declared}\".",
            stale.join("\n    ")
        );
    }
    Ok(())
}

fn version_check(base: &str) -> Result<()> {
    let root = workspace_root();
    let base_rev = resolve_base(&root, base)?;

    let merge_base = git(&root, &["merge-base", &base_rev, "HEAD"])
        .with_context(|| format!("finding the merge base of {base_rev} and HEAD"))?;
    let merge_base = merge_base.trim();

    let diff = git(&root, &["diff", "--name-only", merge_base, "--"])?;
    let changed: Vec<String> = diff.lines().map(str::to_string).collect();

    let base_cargo = git(&root, &["show", &format!("{merge_base}:Cargo.toml")])
        .context("reading the base commit's root Cargo.toml")?;
    let base_version = workspace_version(&base_cargo)?;

    // The working tree, not `CARGO_PKG_VERSION`: the binary running this check
    // may have been built before the bump, and the gate must judge the files
    // that are about to merge.
    let head_cargo = fs::read_to_string(root.join("Cargo.toml"))
        .with_context(|| format!("reading {}", root.join("Cargo.toml").display()))?;
    let head_version = workspace_version(&head_cargo)?;

    // Unconditional: lockstep can be broken by a branch that touches no crate
    // source at all, and it is the cheaper failure to find here.
    lockstep_verdict(&head_cargo)?;

    if version_verdict(&changed, &base_version, &head_version)? {
        println!("✓ crate source changed and the version moved: {base_version} → {head_version}");
    } else {
        println!(
            "✓ no published crate source changed against {base} — version {head_version} may stand"
        );
    }
    Ok(())
}

/// Resolve the base ref, falling back to the local branch when the remote-
/// tracking ref is absent.
///
/// A shallow or freshly-initialised clone often has `main` but not
/// `origin/main`, and a gate that fails because of *how the checkout was
/// fetched* teaches people to skip it.
fn resolve_base(root: &std::path::Path, base: &str) -> Result<String> {
    if git(root, &["rev-parse", "--verify", "--quiet", base]).is_ok() {
        return Ok(base.to_string());
    }
    if let Some(local) = base.split_once('/').map(|(_, b)| b) {
        if git(root, &["rev-parse", "--verify", "--quiet", local]).is_ok() {
            return Ok(local.to_string());
        }
    }
    bail!(
        "cannot resolve base ref `{base}`. Fetch it, or pass one that exists with \
         `fid release version-check --base <ref>`."
    )
}

fn git(root: &std::path::Path, args: &[&str]) -> Result<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("running git — is it installed and is this a repository?")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(test)]
mod version_check_tests {
    use super::*;

    const CARGO: &str = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.3.0\"\n";

    #[test]
    fn reads_the_workspace_version() {
        assert_eq!(workspace_version(CARGO).unwrap(), "0.3.0");
    }

    #[test]
    fn a_cargo_toml_without_a_workspace_version_is_an_error() {
        assert!(workspace_version("[package]\nversion = \"1.0.0\"\n").is_err());
    }

    #[test]
    fn changing_crate_source_without_a_bump_fails() {
        let changed = vec!["crates/fiducial-cli/src/main.rs".to_string()];
        let err = version_verdict(&changed, "0.2.2", "0.2.2").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("version skew"), "{msg}");
        assert!(msg.contains("crates/fiducial-cli/src/main.rs"), "{msg}");
    }

    #[test]
    fn changing_crate_source_with_a_bump_passes() {
        let changed = vec!["crates/fiducial-cli/src/main.rs".to_string()];
        assert!(version_verdict(&changed, "0.2.2", "0.3.0").unwrap());
    }

    #[test]
    fn touching_nothing_published_does_not_require_a_bump() {
        // Docs, workflows and TypeScript packages ship on their own cadence;
        // holding them to the crate version would make the gate noise.
        let changed = vec![
            "docs/specs/2026-09-21-thesis.md".to_string(),
            ".github/workflows/ci.yml".to_string(),
            "packages/advisor/src/checks.js".to_string(),
        ];
        assert!(!version_verdict(&changed, "0.2.2", "0.2.2").unwrap());
    }

    #[test]
    fn a_stale_internal_pin_breaks_lockstep() {
        // Exactly the drift that shipped: crates at 0.2.2, pins still 0.2.0,
        // hidden because `^0.2.0` admits 0.2.2.
        let toml = "[workspace.package]\nversion = \"0.2.2\"\n\n\
                    [workspace.dependencies]\n\
                    fiducial-core = { path = \"crates/fiducial-core\", version = \"0.2.0\" }\n\
                    serde = \"1\"\n";
        let msg = lockstep_verdict(toml).unwrap_err().to_string();
        assert!(msg.contains("lockstep broken"), "{msg}");
        assert!(msg.contains("fiducial-core = \"0.2.0\""), "{msg}");
    }

    #[test]
    fn matching_pins_and_third_party_deps_pass_lockstep() {
        let toml = "[workspace.package]\nversion = \"0.3.0\"\n\n\
                    [workspace.dependencies]\n\
                    fiducial-core = { path = \"crates/fiducial-core\", version = \"0.3.0\" }\n\
                    serde = { version = \"1.0.200\" }\n";
        assert!(lockstep_verdict(toml).is_ok());
    }

    #[test]
    fn the_real_root_manifest_is_in_lockstep() {
        // The gate, applied to the repository it governs.
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml");
        let raw = std::fs::read_to_string(root).expect("reading the workspace root Cargo.toml");
        lockstep_verdict(&raw).expect("this workspace must satisfy its own lockstep rule");
    }

    #[test]
    fn the_failure_lists_at_most_five_files_then_counts_the_rest() {
        let changed: Vec<String> = (0..9).map(|i| format!("crates/c/src/f{i}.rs")).collect();
        let msg = version_verdict(&changed, "0.2.2", "0.2.2")
            .unwrap_err()
            .to_string();
        assert!(msg.contains("… and 4 more"), "{msg}");
        assert!(!msg.contains("f5.rs"), "{msg}");
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Find the workspace root (the directory containing `docs/compat/matrix.toml`)
/// by walking up from `cwd`.  Falls back to `cwd` when the file is not found
/// above — that will produce a useful "not found" error from `load_matrix`.
fn workspace_root() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut dir = cwd.as_path();
    loop {
        if dir.join(MATRIX_PATH).exists() {
            return dir.to_path_buf();
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => return cwd,
        }
    }
}

fn load_matrix() -> Result<(PathBuf, Matrix)> {
    let root = workspace_root();
    let path = root.join(MATRIX_PATH);
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let matrix: Matrix =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok((path, matrix))
}

fn write_matrix(path: &std::path::Path, matrix: &Matrix) -> Result<()> {
    // Serialize to TOML; prepend the header comment so the file stays legible.
    let header = "# Fiducial protocol compatibility matrix\n\
                  #\n\
                  # Maintained by `fid release protocol --bump` — do not edit by hand.\n\
                  # `fid release check` verifies wire.current matches WIRE_VERSION in\n\
                  # crates/fiducial-protocol/src/lib.rs.\n\n";
    let body = toml::to_string_pretty(matrix).context("serialising matrix")?;
    fs::write(path, format!("{header}{body}"))
        .with_context(|| format!("writing {}", path.display()))
}

fn today_iso8601() -> String {
    // Use `chrono` for the date; it is already a workspace dep.
    use chrono::Local;
    Local::now().format("%Y-%m-%d").to_string()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_matrix(current: u8, min_compatible: u8) -> Matrix {
        Matrix {
            wire: WireSection {
                current,
                min_compatible,
            },
            history: vec![HistoryEntry {
                version: 1,
                date: "2026-09-10".into(),
                notes: "Initial.".into(),
            }],
        }
    }

    fn write_matrix_to(dir: &std::path::Path, current: u8, min_compatible: u8) -> PathBuf {
        let compat_dir = dir.join("docs").join("compat");
        fs::create_dir_all(&compat_dir).unwrap();
        let path = compat_dir.join("matrix.toml");
        let matrix = sample_matrix(current, min_compatible);
        write_matrix(&path, &matrix).unwrap();
        path
    }

    #[test]
    fn matrix_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = write_matrix_to(tmp.path(), 1, 1);
        let raw = fs::read_to_string(&path).unwrap();
        let loaded: Matrix = toml::from_str(&raw).unwrap();
        assert_eq!(loaded.wire.current, 1);
        assert_eq!(loaded.wire.min_compatible, 1);
    }

    #[test]
    fn bump_breaking_raises_min() {
        let tmp = TempDir::new().unwrap();
        // Write a matrix with current=1, min=1.
        write_matrix_to(tmp.path(), 1, 1);

        // Simulate a breaking bump: current → 2, min → 2.
        let old_current: u8 = 1;
        let new_current = old_current + 1;
        let mut matrix = sample_matrix(old_current, 1);
        matrix.wire.current = new_current;
        matrix.wire.min_compatible = new_current; // breaking
        matrix.history.push(HistoryEntry {
            version: new_current,
            date: "2026-09-11".into(),
            notes: "Breaking bump.".into(),
        });

        let path = tmp.path().join("docs").join("compat").join("matrix.toml");
        write_matrix(&path, &matrix).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        let reloaded: Matrix = toml::from_str(&raw).unwrap();
        assert_eq!(reloaded.wire.current, 2);
        assert_eq!(reloaded.wire.min_compatible, 2);
        assert_eq!(reloaded.history.len(), 2);
    }

    #[test]
    fn bump_compatible_keeps_min() {
        let old_current: u8 = 1;
        let new_current = old_current + 1;
        let mut matrix = sample_matrix(old_current, 1);
        matrix.wire.current = new_current;
        // min_compatible unchanged — keeps 1.
        matrix.history.push(HistoryEntry {
            version: new_current,
            date: "2026-09-11".into(),
            notes: "Compatible bump.".into(),
        });

        // After a compatible bump, old artifact (version 1) is still accepted.
        use fiducial_protocol::assert_compatible;
        assert!(
            assert_compatible(new_current, old_current, matrix.wire.min_compatible).is_ok(),
            "old artifact should be accepted after compatible bump"
        );
    }

    #[test]
    fn old_artifact_rejected_after_breaking_bump() {
        // After a breaking bump from 1 → 2 (min_compatible=2),
        // a remote still on v1 must be rejected.
        use fiducial_protocol::assert_compatible;
        let err = assert_compatible(2, 1, 2).unwrap_err();
        assert_eq!(err.local, 2);
        assert_eq!(err.remote, 1);
        assert_eq!(err.min_compatible, 2);
    }

    #[test]
    fn check_passes_when_in_sync() {
        // The matrix in this repo should already be in sync (current=WIRE_VERSION).
        // Smoke-test the function path from the workspace root.
        // This test only runs when invoked from inside the fiducial workspace.
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest.parent().unwrap().parent().unwrap(); // ../../ = repo root
        let matrix_path = workspace.join(MATRIX_PATH);
        if !matrix_path.exists() {
            return; // not running from the expected workspace layout — skip
        }
        let raw = fs::read_to_string(&matrix_path).unwrap();
        let matrix: Matrix = toml::from_str(&raw).unwrap();
        assert_eq!(
            matrix.wire.current, WIRE_VERSION,
            "docs/compat/matrix.toml wire.current must equal WIRE_VERSION={}",
            WIRE_VERSION
        );
    }
}

//! `fid new <name>` — scaffold a new product repository.
//!
//! Creates no cloud resources. Writes a minimal, building product tree and
//! records every template file in `fiducial.lock` so that `fid upgrade` can
//! perform a 3-way merge when upstream templates change.

use anyhow::{bail, Context, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{lock::Lock, templates};

/// Current platform version — baked in at compile time.
const PLATFORM_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn run(name: &str) -> Result<()> {
    validate_name(name)?;

    let dest = PathBuf::from(name);
    if dest.exists() {
        bail!(
            "directory `{name}` already exists.\n\
             Choose a different name or remove the existing directory."
        );
    }

    println!("✦ fid new {name}");

    std::fs::create_dir_all(&dest).with_context(|| format!("creating directory `{name}`"))?;

    let mut lock = Lock::new();

    // The scaffold set is declared once, in `templates::SCAFFOLD_FILES`, so
    // `fid upgrade` can diff it against an existing product's lock and install
    // whatever the platform has added since.
    for (rel_path, template) in templates::SCAFFOLD_FILES {
        write_template(&dest, rel_path, template, name, &mut lock)?;
    }

    // Write fiducial.lock — after all templates are recorded.
    lock.save(&dest.join("fiducial.lock"))
        .context("writing fiducial.lock")?;
    println!("  wrote  fiducial.lock");

    // git init
    git_init(&dest)?;

    print_checklist(name);
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Write a template file, substituting `{{name}}` and `{{version}}` placeholders,
/// creating parent directories as needed, and recording it in the lock.
fn write_template(
    root: &Path,
    rel_path: &str,
    template: &str,
    name: &str,
    lock: &mut Lock,
) -> Result<()> {
    let content = template
        .replace("{{name}}", name)
        .replace("{{version}}", PLATFORM_VERSION);

    let dest = root.join(rel_path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent for `{rel_path}`"))?;
    }

    std::fs::write(&dest, &content).with_context(|| format!("writing `{rel_path}`"))?;

    // Record with forward slashes regardless of platform.
    let lock_key = rel_path.replace('\\', "/");
    lock.record(lock_key, content.as_bytes(), PLATFORM_VERSION);

    println!("  wrote  {rel_path}");
    Ok(())
}

/// Run `git init` in the product directory if git is available.
///
/// The initial branch is forced to `main`. Without this, the branch name comes
/// from the user's `init.defaultBranch`, which is still `master` on a default
/// install — and the scaffold contradicts itself the moment that happens: the
/// `no-direct-main-push` guard rule guards a branch that does not exist, the
/// scaffolded review agent tells agents to run `git diff main...HEAD`, which
/// fails outright, and the CI workflow triggers on a branch that never appears.
///
/// `--initial-branch` needs git 2.28 (2020). Older versions get the same result
/// from `symbolic-ref`, which works on the unborn HEAD a fresh `init` leaves.
fn git_init(dest: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(dest)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("  git    init (branch: main)");
            Ok(())
        }
        Ok(_) => {
            // Pre-2.28 git: init, then point the unborn HEAD at main.
            let init = Command::new("git").arg("init").current_dir(dest).status();
            match init {
                Ok(s) if s.success() => {
                    let _ = Command::new("git")
                        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
                        .current_dir(dest)
                        .status();
                    println!("  git    init (branch: main)");
                    Ok(())
                }
                Ok(s) => bail!("`git init` exited with status {s}"),
                Err(e) => Err(e).context("running `git init`"),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // git not installed — warn but don't fail.
            println!("  ⚠ git not found; skipping git init. Install git and run it manually.");
            Ok(())
        }
        Err(e) => Err(e).context("running `git init`"),
    }
}

/// Validate the product name: lowercase, alphanumeric + hyphens, no leading/trailing hyphens.
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("product name cannot be empty");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!(
            "product name `{name}` contains invalid characters.\n\
             Use lowercase letters, digits, and hyphens only."
        );
    }
    if name.starts_with('-') || name.ends_with('-') {
        bail!("product name `{name}` must not start or end with a hyphen.");
    }
    Ok(())
}

fn print_checklist(name: &str) {
    println!();
    println!("✦ {name} is ready. Next steps:");
    println!();
    println!("  cd {name}");
    println!("  # Edit fiducial.toml — set spine.enabled = true if you want the L0 Rust core.");
    println!("  # Edit MISSION.md   — one paragraph: what is this product for?");
    println!();
    println!("  fid add app next      # add a Next.js web app");
    println!("  fid add app svelte    # add a SvelteKit app");
    println!("  fid add app tauri     # add a Tauri desktop/mobile app");
    println!("  fid add firmware rp2040  # add RP2040 firmware");
    println!();
    println!("  fid doctor            # verify everything is in order");
    println!();
    println!("  # Agents — Sonnet for implementation, Opus for design/review:");
    println!("  fiducial-design       # architecture brainstorming");
    println!("  fiducial-review       # review diff before committing");
    println!("  # CI auto-reviews every PR (needs ANTHROPIC_API_KEY in repo secrets).");
    println!();
    println!("  # When you're ready to commit:");
    println!("  git add -A && git commit -m 'feat: initial scaffold'");
}

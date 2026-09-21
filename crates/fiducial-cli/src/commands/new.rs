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

use crate::{capability, config, lock::Lock, templates};

/// Current platform version — baked in at compile time.
const PLATFORM_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn run(name: &str, locales: Option<&str>, default_locale: Option<&str>) -> Result<()> {
    validate_name(name)?;
    let requested = locales.map(str::to_string).unwrap_or_else(default_locales);
    let locales = parse_locales(&requested)?;
    let default_locale = resolve_default_locale(&locales, default_locale)?;

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

    // A design system from the first commit, for the same reason as i18n:
    // the alternative to having one is not "no design", it is the default one —
    // Inter, an indigo primary, `rounded-xl`, a gradient hero. Those are absent
    // decisions, not neutral ones, and they are absent in the same direction in
    // every product that never wrote anything down.
    //
    // `design-system.md` ships filled in rather than as a checklist, and its
    // pipeline measures the palette's contrast pairs on the first derive. If
    // the product has no web app yet the stylesheet is simply not written —
    // see `outputs_not_applicable` — so this costs a firmware product nothing
    // but the document, which is the part worth having early anyway.
    {
        let cap = capability::find("design").expect("the design capability is built in");
        capability::install(cap, &dest, name)?;
    }

    // Localized from the first commit, not as a later pass.
    //
    // Installing the capability here rather than reimplementing it keeps one
    // definition of what "this product has i18n" means — `fid new` and
    // `fid add i18n` produce the same tree.
    if !locales.is_empty() {
        let cap = capability::find("i18n").expect("the i18n capability is built in");
        capability::install(cap, &dest, name)?;
        seed_locales(&dest, &locales, &default_locale)?;
        capability::reconcile_i18n_catalogs(&dest, &locales, &default_locale)?;

        // Leave the scaffold derived. The CI `fid new` also scaffolds runs
        // `fid derive --check`, so a product whose generated messages have
        // never been produced fails its own pipeline on the first commit,
        // before anyone has changed anything.
        println!();
        super::derive::run_in(&dest, false, None)?;
    } else {
        // No i18n, but `design` still declares a pipeline — leave the scaffold
        // derived either way, so a product never starts life failing its own
        // `fid derive --check`.
        println!();
        super::derive::run_in(&dest, false, None)?;
    }

    // git init
    git_init(&dest)?;

    print_checklist(name);
    Ok(())
}

/// Split `--locales`, rejecting what cannot be a locale directory name.
///
/// `none` is the opt-out. A product with genuinely no user-visible text — a
/// CLI, a firmware image — should not carry catalogs, and saying so explicitly
/// is better than leaving an empty list to be read as an oversight.
fn parse_locales(raw: &str) -> Result<Vec<String>> {
    if raw.trim().eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in raw.split(',') {
        let tag = part.trim();
        if tag.is_empty() {
            continue;
        }
        // A locale becomes `messages/<tag>.json` and a TypeScript identifier,
        // so it has to survive both.
        if !tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            bail!(
                "`{tag}` is not a usable locale tag.\n\
                 Use BCP 47 tags such as `en`, `sr`, `pt-BR` — letters, digits, \
                 `-` and `_` only."
            );
        }
        if !out.iter().any(|l| l == tag) {
            out.push(tag.to_string());
        }
    }
    if out.is_empty() {
        bail!(
            "--locales named no locales.\n\
             Pass a list such as `--locales en,fr`, or `--locales none` for a \
             product with no user-visible text."
        );
    }
    Ok(out)
}

/// The locale set a product is born with, owned by the `i18n` capability.
///
/// Not a constant here: the capability declares what it seeds, and `fid new`
/// asking it is the difference between one declaration and two.
fn default_locales() -> String {
    capability::seeded_locales()
        .map(|(l, _)| l.join(","))
        .unwrap_or_else(|| "none".to_string())
}

/// The fallback the capability declares.
fn fallback_locale() -> String {
    capability::seeded_locales()
        .map(|(_, d)| d)
        .unwrap_or_default()
}

/// The fallback locale: declared, never inferred from list order.
fn resolve_default_locale(locales: &[String], declared: Option<&str>) -> Result<String> {
    if locales.is_empty() {
        return Ok(String::new());
    }
    let candidate = match declared {
        Some(d) => d.to_string(),
        None => fallback_locale(),
    };
    let candidate = candidate.as_str();
    if locales.iter().any(|l| l == candidate) {
        return Ok(candidate.to_string());
    }
    if declared.is_some() {
        bail!(
            "--default-locale `{candidate}` is not in --locales {locales:?}.\n\
             The fallback must be one of the locales the product ships."
        );
    }
    bail!(
        "--locales {locales:?} does not include the default fallback \
         `{candidate}`.\n\
         Name the fallback explicitly: --default-locale <one of {locales:?}>.\n\
         It is not taken from list order — which language a reader falls back to \
         is a decision."
    )
}

/// Write the declared locale set into the product's `fiducial.toml`.
///
/// Runs after `capability::install`, which seeds the capability's own locales
/// when the block is empty. Overwriting that is cheaper and less brittle than
/// teaching `install` about a locale set only `fid new` has.
fn seed_locales(root: &Path, locales: &[String], default_locale: &str) -> Result<()> {
    let path = root.join(config::CONFIG_FILE);
    let mut cfg = config::Config::load(&path)?;
    cfg.i18n.locales = locales.to_vec();
    cfg.i18n.default = Some(default_locale.to_string());

    let raw = toml::to_string_pretty(&cfg).context("serialising fiducial.toml")?;
    let header = format!(
        "# fiducial.toml — created by `fid new {}` (fiducial {})\n\n",
        root.file_name().unwrap_or_default().to_string_lossy(),
        PLATFORM_VERSION
    );
    let content = format!("{header}{raw}");
    std::fs::write(&path, &content).context("writing fiducial.toml")?;

    // Re-record: the lock holds the hash of what was written, and this just
    // rewrote it. Without this the product is born reporting its own config as
    // hand-edited — `fid doctor` failing on a tree `fid new` produced.
    let lock_path = root.join("fiducial.lock");
    let mut lock = Lock::load(&lock_path).unwrap_or_else(|_| Lock::new());
    lock.record("fiducial.toml", content.as_bytes(), PLATFORM_VERSION);
    lock.save(&lock_path).context("writing fiducial.lock")?;
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
    // One expansion, in `templates::expand`. This function used to inline its own
    // `.replace()` chain, so a placeholder added to the template — `{{principles}}`
    // was the one that found it — was substituted by `fid upgrade` and left raw
    // by `fid new`.
    let content = templates::expand(template, name, PLATFORM_VERSION);

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
    println!();
    println!("  # Start here. One sentence someone could disagree with:");
    println!("  fid thesis set \"<what this product claims>\"");
    println!("  fid thesis            # what is still unanswered about it");
    println!();
    println!("  # Edit fiducial.toml — set spine.enabled = true if you want the L0 Rust core.");
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

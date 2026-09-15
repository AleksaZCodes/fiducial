//! `fid capability` — list, check, and scaffold capabilities.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use std::env;

use crate::{
    capability::{self, builtins, Capability},
    config::{Config, CONFIG_FILE},
};

#[derive(Subcommand, Debug)]
pub enum CapabilityAction {
    /// List installed capabilities and their versions
    #[command(
        long_about = "\
List every capability currently installed in this product, together with the
platform version it was installed from and the status of its SKILL.md file.",
        after_long_help = "\
EXAMPLES
  fid capability list        show installed capabilities
  fid capability list --all  also show available-but-not-installed capabilities"
    )]
    List {
        /// Also show capabilities that are available but not installed
        #[arg(long, default_value_t = false)]
        all: bool,
    },

    /// Check installed capabilities for conformance issues
    #[command(
        long_about = "\
Validate every installed capability against the conformance rules:

  - SKILL.md is present and non-trivial (§3.3: mandatory)
  - Guard rules are syntactically valid
  - Template files are present in the product root

Exits non-zero if any check fails. Safe to run in CI.",
        after_long_help = "\
EXAMPLES
  fid capability check                        check all installed capabilities
  fid capability check --capability web-next  check one capability"
    )]
    Check {
        /// Check only this capability (default: all installed)
        #[arg(long, value_name = "ID")]
        capability: Option<String>,
    },

    /// Scaffold a new first-party capability in this repo
    #[command(
        long_about = "\
Scaffold a new first-party capability directory at `capabilities/<name>/`.

Creates the minimum viable capability structure:
  - `capabilities/<name>/SKILL.md`    mandatory agent instructions
  - `capabilities/<name>/README.md`   human documentation
  - a conformance test stub

The capability is NOT installed into any product automatically — use
`fid add <name>` to install it.",
        after_long_help = "\
EXAMPLES
  fid capability new my-sensor        scaffold a new capability
  fid capability new eda-kicad        scaffold the KiCad EDA capability

NAME RULES
  Kebab-case only: lowercase letters and hyphens."
    )]
    New {
        /// Capability identifier (kebab-case, e.g. `my-sensor`)
        #[arg(value_name = "NAME")]
        name: String,
    },
}

pub fn run(action: CapabilityAction) -> Result<()> {
    match action {
        CapabilityAction::List { all } => cmd_list(all),
        CapabilityAction::Check { capability } => cmd_check(capability.as_deref()),
        CapabilityAction::New { name } => cmd_new(&name),
    }
}

// ── list ─────────────────────────────────────────────────────────────────────

fn cmd_list(show_all: bool) -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(CONFIG_FILE))?;

    let installed: Vec<&str> = cfg
        .capabilities
        .enabled
        .iter()
        .map(|s| s.as_str())
        .collect();

    println!("✦ fid capability list — {}", cfg.product.name);
    println!();

    if installed.is_empty() && !show_all {
        println!("  No capabilities installed.");
        println!("  Run `fid add <capability>` to install one.");
        println!("  Run `fid capability list --all` to see what is available.");
        return Ok(());
    }

    // Installed capabilities.
    if !installed.is_empty() {
        println!("  INSTALLED");
        for id in &installed {
            match capability::find(id) {
                Some(def) => {
                    let skill_path = root.join(format!(".claude/skills/{id}.md"));
                    let skill_status = if skill_path.exists() {
                        "✓"
                    } else {
                        "✗ SKILL.md missing"
                    };
                    println!("  ✓ {id:<24} {desc}", id = id, desc = def.description);
                    println!("    skill: {skill_status}");
                    print_contributions(def);
                }
                None => {
                    println!("  ? {id:<24} (third-party or unknown — not in built-in registry)");
                }
            }
        }
        println!();
    }

    // Available (not installed).
    if show_all {
        let available: Vec<&Capability> = builtins()
            .iter()
            .filter(|c| !installed.contains(&c.id.as_str()))
            .collect();

        if !available.is_empty() {
            println!("  AVAILABLE (not installed)");
            for def in available {
                println!("  · {id:<24} {desc}", id = def.id, desc = def.description);
                print_contributions(def);
            }
            println!();
            println!("  Install with: fid add <capability-id>");
        }

        print_contracts(&cfg);
    }

    Ok(())
}

/// The adapter contracts, what satisfies each today, and what is intended.
///
/// Shown under `--all` because a contract is not discoverable otherwise: the
/// set lives in the binary, and a product cannot select a vendor it has never
/// been told exists. The split between selectable and planned is the honest
/// half — naming a vendor nothing implements as if it worked is the failure
/// this repository spent a phase removing from its guard rules.
fn print_contracts(cfg: &crate::config::Config) {
    use crate::adapter;

    println!();
    println!("  ADAPTER CONTRACTS");
    for c in adapter::CONTRACTS {
        let selected = cfg
            .adapters
            .get(c.name)
            .map(|v| format!("  [selected: {v}]"))
            .unwrap_or_default();
        println!(
            "  · {name:<24} {desc}{selected}",
            name = c.name,
            desc = c.description
        );
        println!(
            "    selectable: {}{}",
            c.implementations.join(", "),
            if c.candidates.is_empty() {
                String::new()
            } else {
                format!("   planned: {}", c.candidates.join(", "))
            }
        );
    }
    println!();
    println!("  Select with `[adapters]` in fiducial.toml. `none` is a working no-op.");
}

// ── check ─────────────────────────────────────────────────────────────────────

/// What a capability contributes, by taxonomy kind.
///
/// Printed under the description so `fid capability list` answers the question
/// the taxonomy exists to make askable: not just "is it installed", but *what
/// does it bring* — which facts, which derivations, which contracts.
fn print_contributions(def: &crate::capability::Capability) {
    use crate::capability::Declaration;

    let mut parts: Vec<String> = Vec::new();
    let decls: Vec<&str> = def.declarations.iter().map(|d| d.name()).collect();
    if !decls.is_empty() {
        parts.push(format!("declares {}", decls.join(", ")));
    }
    if !def.pipelines.is_empty() {
        let names: Vec<&str> = def
            .pipelines
            .iter()
            .map(|f| f.path.trim_start_matches("pipelines/"))
            .collect();
        parts.push(format!("derives via {}", names.join(", ")));
    }
    if !def.requires_adapters.is_empty() {
        parts.push(format!("requires {}", def.requires_adapters.join(", ")));
    }
    if !def.templates.is_empty() {
        parts.push(format!("{} template file(s)", def.templates.len()));
    }
    // A config block is a declaration whose absence is invisible on disk, so
    // say so rather than leaving the reader to infer it from the name.
    if def
        .declarations
        .iter()
        .any(|d| matches!(d, Declaration::ConfigBlock { .. }))
    {
        parts.push("seeds a fiducial.toml block".to_string());
    }

    if !parts.is_empty() {
        println!("    {}", parts.join("; "));
    }
}

fn cmd_check(filter: Option<&str>) -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(CONFIG_FILE))?;

    println!("✦ fid capability check — {}", cfg.product.name);
    println!();

    let to_check: Vec<&str> = if let Some(id) = filter {
        vec![id]
    } else {
        cfg.capabilities
            .enabled
            .iter()
            .map(|s| s.as_str())
            .collect()
    };

    if to_check.is_empty() {
        println!("  No capabilities installed. Nothing to check.");
        return Ok(());
    }

    let mut all_ok = true;

    for id in &to_check {
        match capability::find(id) {
            None => {
                println!("  ? {id}: not in built-in registry (skipping)");
            }
            Some(def) => {
                let errors = capability::check_capability(def);

                // Also check that SKILL.md landed in the product.
                let skill_path = root.join(format!(".claude/skills/{id}.md"));
                let mut combined = errors;
                if !skill_path.exists() {
                    combined.push(format!(
                        "SKILL.md not found at `.claude/skills/{id}.md` — \
                         re-run `fid add {id}` to install"
                    ));
                }

                if combined.is_empty() {
                    println!("  ✓ {id}: ok");
                } else {
                    all_ok = false;
                    for e in combined {
                        println!("  ✗ {id}: {e}");
                    }
                }
            }
        }
    }

    println!();
    if all_ok {
        println!("✦ capability check: clean");
        Ok(())
    } else {
        bail!("one or more capability conformance issues found")
    }
}

// ── new ───────────────────────────────────────────────────────────────────────

fn cmd_new(name: &str) -> Result<()> {
    // Validate name.
    if name.is_empty()
        || name.contains(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-')
        || name.starts_with('-')
        || name.ends_with('-')
    {
        bail!(
            "capability name `{name}` is invalid.\n\
             Use kebab-case: lowercase letters, digits, and hyphens only."
        );
    }

    let cwd = env::current_dir().context("getting current directory")?;
    // Capability directories live in the platform repo, not a product repo.
    // We scaffold under cwd/capabilities/<name>/.
    let cap_dir = cwd.join("capabilities").join(name);
    if cap_dir.exists() {
        bail!(
            "capability directory `capabilities/{name}` already exists.\n\
             Edit it directly or choose a different name."
        );
    }

    std::fs::create_dir_all(&cap_dir).with_context(|| format!("creating capabilities/{name}"))?;

    // SKILL.md — mandatory.
    let skill = format!(
        "# Skill: {name}\n\n\
         **Capability:** `{name}` · **Platform:** Fiducial\n\n\
         ---\n\n\
         ## What this capability adds\n\n\
         <!-- Describe what installing this capability gives a product. -->\n\n\
         ## Development\n\n\
         ```sh\n\
         # Add usage examples here\n\
         ```\n\n\
         ## Guard rules activated\n\n\
         | Rule | What it prevents |\n\
         |---|---|\n\
         | (none yet) | |\n\n\
         ## Key constraints\n\n\
         <!-- List invariants the agent must respect. -->\n"
    );
    let skill_path = cap_dir.join("SKILL.md");
    std::fs::write(&skill_path, &skill).context("writing SKILL.md")?;
    println!("  wrote  capabilities/{name}/SKILL.md");

    // README.md.
    let readme = format!(
        "# {name} capability\n\n\
         Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform.\n\n\
         ## Install\n\n\
         ```sh\n\
         fid add {name}\n\
         ```\n\n\
         ## Usage\n\n\
         See `SKILL.md` for agent instructions.\n"
    );
    std::fs::write(cap_dir.join("README.md"), readme).context("writing README.md")?;
    println!("  wrote  capabilities/{name}/README.md");

    println!();
    println!("✦ capabilities/{name}/ scaffolded. Next steps:");
    println!();
    println!("  1. Edit  capabilities/{name}/SKILL.md — teach agents how to use this capability.");
    println!("  2. Decide what this capability contributes, by kind:");
    println!("       declarations  typed facts it introduces (a file, or a fiducial.toml block)");
    println!("       pipelines     what it derives from them, under pipelines/ — gated by fid derive --check");
    println!("       adapters      contracts it needs a vendor for, without choosing one");
    println!("       templates     plain files copied in, belonging to no pipeline");
    println!("     The test for a declaration: could two pipelines read it and both be correct?");
    // No registration step: `build.rs` walks `capabilities/`, so the directory
    // is the declaration. This used to say "register it in capability.rs",
    // which was true until the `CapabilityDef` literals were deleted and the
    // manifest started being derived from the layout.
    println!("  3. Run   fid capability check --capability {name}");
    println!("  4. Install it:");
    println!("       fid add {name}                      built in, once it is in this repo");
    println!("       fid add capability {name} --from ./capabilities/{name}");

    Ok(())
}

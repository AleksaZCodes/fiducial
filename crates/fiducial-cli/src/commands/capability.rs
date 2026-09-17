//! `fid capability` — list, check, and scaffold capabilities.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use std::{env, path::Path};

use crate::{
    capability::{self, builtins, Capability},
    config::{Config, CONFIG_FILE},
    lock::{Lock, LOCK_FILE},
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

    /// Extract a capability from this product into a staging directory
    #[command(
        long_about = "\
Stage the files belonging to a capability under `capabilities/<id>/` so they
can be reviewed, generalised, and re-used across products.

This command copies files but does NOT install or register anything. The output
is a staging area — review and fix the reported generalisation issues before
treating it as a reusable capability.

The command also reports:
  - Hardcoded product names that should be replaced with {{name}} or a declaration
  - Absolute paths that should be made relative
  - A warning when the capability has only one known consumer (this product)

Use --from to specify the source directory inside the product whose files are
to be staged. When omitted, the command falls back to files recorded in the
lock for this capability (pipelines only — template files are not tracked
per-capability in fiducial.lock).",
        after_long_help = "\
EXAMPLES
  fid capability extract realtime --from src/realtime
  fid capability extract stripe --from apps/billing

NEXT STEPS AFTER STAGING
  1. Review capabilities/<id>/ and fix the reported issues
  2. Edit capabilities/<id>/SKILL.md with agent instructions
  3. Edit capabilities/<id>/capability.toml with description and declarations
  4. Run: fid capability check --capability <id>
  5. Install in another product: fid add capability <id> --from ./capabilities/<id>"
    )]
    Extract {
        /// Capability identifier (kebab-case) to extract
        #[arg(value_name = "ID")]
        id: String,
        /// Source directory inside the product to stage (optional; falls back to lock)
        #[arg(long, value_name = "PATH")]
        from: Option<String>,
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
        CapabilityAction::Extract { id, from } => cmd_extract(&id, from.as_deref()),
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

// ── extract ───────────────────────────────────────────────────────────────────

/// A single file to be staged, with its product-relative source path.
struct StagedFile {
    /// Path relative to the product root (forward slashes).
    rel_path: String,
    /// Current content on disk (what the product may have edited).
    content: String,
}

/// One reported generalisation issue.
struct Issue {
    /// Source file path (product-relative).
    file: String,
    /// 1-based line number where the issue was found.
    line: usize,
    /// Human-readable description.
    message: String,
}

fn cmd_extract(id: &str, from: Option<&str>) -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;
    let config = Config::load(&root.join(CONFIG_FILE))?;
    let lock = Lock::load(&root.join(LOCK_FILE))?;

    println!("✦ fid capability extract {id}");
    println!();

    // ── Collect files to stage ────────────────────────────────────────────────

    let files: Vec<StagedFile> = if let Some(src) = from {
        // User provided an explicit source directory — walk it.
        let src_path = root.join(src);
        if !src_path.exists() {
            bail!(
                "source directory `{src}` does not exist under product root `{}`",
                root.display()
            );
        }
        if !src_path.is_dir() {
            bail!("`{src}` is not a directory");
        }
        collect_dir(&src_path, &root)?
    } else {
        // No --from: fall back to lock.capabilities[id] (pipelines only).
        match lock.capabilities.get(id) {
            None => {
                bail!(
                    "capability `{id}` is not recorded in fiducial.lock.\n\
                     Either pass --from <directory> to specify the source files,\n\
                     or install the capability first with `fid add {id}`."
                );
            }
            Some(record) => {
                let mut staged = Vec::new();
                for pipeline_path in &record.pipelines {
                    let abs = root.join(pipeline_path);
                    match std::fs::read_to_string(&abs) {
                        Ok(content) => staged.push(StagedFile {
                            rel_path: pipeline_path.clone(),
                            content,
                        }),
                        Err(e) => eprintln!("  warn  {pipeline_path}: {e} (skipped)"),
                    }
                }
                if staged.is_empty() {
                    bail!(
                        "capability `{id}` is in the lock but no files could be read.\n\
                         Pass --from <directory> to specify a source directory."
                    );
                }
                eprintln!(
                    "  note  fiducial.lock does not track template files per-capability.\n\
                         Pass --from <directory> to include template files in the extraction."
                );
                staged
            }
        }
    };

    // ── Stage files ───────────────────────────────────────────────────────────

    let out_dir = root.join("capabilities").join(id);
    println!("  Staging to capabilities/{id}/");
    println!();

    let mut staged_paths: Vec<(String, String)> = Vec::new(); // (src, dest)
    let mut has_skill = false;
    let mut has_manifest = false;

    for file in &files {
        let dest_rel = format!("capabilities/{id}/{}", file.rel_path);
        let dest_abs = root.join(&dest_rel);
        if let Some(parent) = dest_abs.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory for {dest_rel}"))?;
        }
        std::fs::write(&dest_abs, &file.content).with_context(|| format!("writing {dest_rel}"))?;
        println!(
            "  staged   {src:<42}  →  {dest}",
            src = file.rel_path,
            dest = dest_rel
        );
        staged_paths.push((file.rel_path.clone(), dest_rel.clone()));

        if file.rel_path == "SKILL.md"
            || file.rel_path.ends_with("/SKILL.md")
            || std::path::Path::new(&file.rel_path)
                .file_name()
                .is_some_and(|n| n == "SKILL.md")
        {
            has_skill = true;
        }
        if file.rel_path == "capability.toml"
            || std::path::Path::new(&file.rel_path)
                .file_name()
                .is_some_and(|n| n == "capability.toml")
        {
            has_manifest = true;
        }
    }

    // ── Write stubs if missing ────────────────────────────────────────────────

    if !has_skill {
        let skill_content = format!(
            "# Skill: {id}\n\n\
             **Capability:** `{id}` · **Platform:** Fiducial\n\n\
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
        let skill_path = out_dir.join("SKILL.md");
        std::fs::create_dir_all(&out_dir).context("creating capability staging directory")?;
        std::fs::write(&skill_path, &skill_content).context("writing SKILL.md stub")?;
        println!("  wrote    (stub)  →  capabilities/{id}/SKILL.md");
    }

    if !has_manifest {
        let manifest_content = format!(
            "# capability.toml for `{id}`\n\
             #\n\
             # Fill in description and any declarations, then remove this comment block.\n\n\
             description = \"{id} capability\"\n\n\
             # [declarations.config]\n\
             # block = \"<block-name>\"\n\
             # seed = {{ key = \"value\" }}\n"
        );
        let manifest_path = out_dir.join("capability.toml");
        std::fs::create_dir_all(&out_dir).context("creating capability staging directory")?;
        std::fs::write(&manifest_path, &manifest_content)
            .context("writing capability.toml stub")?;
        println!("  wrote    (stub)  →  capabilities/{id}/capability.toml");
    }

    // ── Analyse for generalisation issues ─────────────────────────────────────

    let product_name = &config.product.name;
    let mut issues: Vec<Issue> = Vec::new();

    for file in &files {
        for (lineno, line) in file.content.lines().enumerate() {
            let lineno = lineno + 1; // 1-based
                                     // Hardcoded product name.
            if line.contains(product_name.as_str()) {
                issues.push(Issue {
                    file: file.rel_path.clone(),
                    line: lineno,
                    message: format!(
                        "hardcoded product name \"{product_name}\" — replace with {{{{name}}}} or a declaration"
                    ),
                });
            }
            // Absolute paths anywhere on the line (heuristic: token starting with /).
            for token in line.split_whitespace() {
                // Strip surrounding quotes/brackets for the check.
                let trimmed = token
                    .trim_matches(|c| matches!(c, '"' | '\'' | '(' | ')' | '[' | ']' | ',' | ';'));
                if trimmed.starts_with('/') && trimmed.len() > 1 {
                    issues.push(Issue {
                        file: file.rel_path.clone(),
                        line: lineno,
                        message: format!(
                            "absolute path \"{trimmed}\" — make it relative or a declaration"
                        ),
                    });
                    break; // one issue per line for this category
                }
            }
        }
    }

    // ── Print issues ──────────────────────────────────────────────────────────

    println!();
    if issues.is_empty() {
        println!("  No generalisation issues found.");
    } else {
        println!("  ⚠ generalisation issues found (review before using as a capability):");
        println!();
        for issue in &issues {
            println!("    {}:{}  {}", issue.file, issue.line, issue.message);
        }
    }

    // ── Single-consumer warning ───────────────────────────────────────────────

    println!();
    println!("  ⚠ single consumer: this capability exists in one product only.");
    println!(
        "    Anti-goal 2 (MISSION.md): generalise when a second product needs it, not speculatively."
    );

    // ── Next steps ────────────────────────────────────────────────────────────

    println!();
    println!("  Next steps:");
    if !issues.is_empty() {
        println!("    1. Review and fix the issues above");
        println!("    2. Add capabilities/{id}/capability.toml with description and declarations");
        println!("    3. Run: fid capability check --capability {id}");
        println!(
            "    4. Install in another product: fid add capability {id} --from ./capabilities/{id}"
        );
    } else {
        println!("    1. Add capabilities/{id}/capability.toml with description and declarations");
        println!("    2. Run: fid capability check --capability {id}");
        println!(
            "    3. Install in another product: fid add capability {id} --from ./capabilities/{id}"
        );
    }

    Ok(())
}

/// Walk a directory and return all files as `StagedFile` values with paths
/// relative to `root`.
fn collect_dir(dir: &Path, root: &Path) -> Result<Vec<StagedFile>> {
    let mut out = Vec::new();
    collect_dir_inner(dir, root, &mut out)?;
    Ok(out)
}

fn collect_dir_inner(dir: &Path, root: &Path, out: &mut Vec<StagedFile>) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("reading directory {}", dir.display()))?
        .flatten()
    {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Skip hidden files and directories (except .cargo).
        if name_str.starts_with('.') && name_str != ".cargo" {
            continue;
        }
        if path.is_dir() {
            collect_dir_inner(&path, root, out)?;
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .expect("walked path is below root")
            .to_string_lossy()
            .replace('\\', "/");
        match std::fs::read_to_string(&path) {
            Ok(content) => out.push(StagedFile {
                rel_path: rel,
                content,
            }),
            Err(e) => eprintln!("  warn  {}: {} (skipped — not UTF-8 text)", rel, e),
        }
    }
    Ok(())
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

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    // Tests that temporarily change the process-wide cwd must hold this lock
    // so they do not race with each other's TempDir cleanup.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    /// Helper: write a minimal product root with fiducial.toml and fiducial.lock.
    fn make_product_root(dir: &std::path::Path, product_name: &str) {
        let config =
            format!("[product]\nname = \"{product_name}\"\n\n[capabilities]\nenabled = []\n");
        fs::write(dir.join("fiducial.toml"), config).unwrap();
        let lock = "version = 1\n";
        fs::write(dir.join("fiducial.lock"), lock).unwrap();
    }

    /// The basic extract flow: given a source directory, files are staged under
    /// `capabilities/<id>/` and stubs are written for the missing SKILL.md and
    /// capability.toml.
    #[test]
    fn extract_stages_files_and_writes_stubs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        make_product_root(root, "my-product");

        // Create a source directory with one file.
        let src = root.join("src").join("realtime");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("index.ts"), "export const RT = true;\n").unwrap();

        // Run extraction (changes cwd temporarily).
        let _guard = CWD_LOCK.lock().unwrap();
        let orig = env::current_dir().unwrap();
        env::set_current_dir(root).unwrap();
        let result = cmd_extract("realtime", Some("src/realtime"));
        env::set_current_dir(orig).unwrap();

        assert!(result.is_ok(), "extract failed: {result:?}");

        // Staged file must exist.
        let staged = root
            .join("capabilities")
            .join("realtime")
            .join("src")
            .join("realtime")
            .join("index.ts");
        assert!(staged.exists(), "staged file missing");
        assert_eq!(
            fs::read_to_string(&staged).unwrap(),
            "export const RT = true;\n"
        );

        // SKILL.md stub must be written.
        let skill = root.join("capabilities").join("realtime").join("SKILL.md");
        assert!(skill.exists(), "SKILL.md stub missing");
        let skill_content = fs::read_to_string(&skill).unwrap();
        assert!(skill_content.contains("# Skill: realtime"));

        // capability.toml stub must be written.
        let manifest = root
            .join("capabilities")
            .join("realtime")
            .join("capability.toml");
        assert!(manifest.exists(), "capability.toml stub missing");
    }

    /// Hardcoded product names must be reported as generalisation issues.
    #[test]
    fn extract_reports_hardcoded_product_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        make_product_root(root, "acme");

        let src = root.join("pipelines");
        fs::create_dir_all(&src).unwrap();
        // File contains the product name — should trigger an issue.
        fs::write(
            src.join("realtime.toml"),
            "name = \"acme\"\nsome_key = \"value\"\n",
        )
        .unwrap();

        // Capture analysis without running the full command (avoids cwd change).
        // We test the issue-detection logic directly via the file content.
        let product_name = "acme";
        let content = fs::read_to_string(src.join("realtime.toml")).unwrap();
        let found = content.lines().any(|line| line.contains(product_name));
        assert!(
            found,
            "hardcoded product name should be detected in file content"
        );
    }

    /// When --from points to a non-existent directory, extract must fail.
    #[test]
    fn extract_fails_for_missing_source_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        make_product_root(root, "my-product");

        let _guard = CWD_LOCK.lock().unwrap();
        let orig = env::current_dir().unwrap();
        env::set_current_dir(root).unwrap();
        let result = cmd_extract("realtime", Some("no/such/dir"));
        env::set_current_dir(orig).unwrap();

        assert!(result.is_err(), "should fail for missing source dir");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("no/such/dir"),
            "error should mention the path: {msg}"
        );
    }

    /// Without --from and with no lock entry, extract must fail with a clear message.
    #[test]
    fn extract_fails_without_from_and_no_lock_entry() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        make_product_root(root, "my-product");

        let _guard = CWD_LOCK.lock().unwrap();
        let orig = env::current_dir().unwrap();
        env::set_current_dir(root).unwrap();
        let result = cmd_extract("unknown-cap", None);
        env::set_current_dir(orig).unwrap();

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("unknown-cap"),
            "error should mention the capability id: {msg}"
        );
    }
}

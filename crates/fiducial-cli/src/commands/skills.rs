//! `fid skills` — the AI skills this product's capabilities need the agent to load.
//!
//! See [`crate::agent_skills`] for what is enforced and why. This is the
//! command surface: `sync` makes the product match its capabilities, `check`
//! says whether it does and whether vendored skills are current upstream,
//! `list` shows what is enforced.

use anyhow::{bail, Result};
use clap::Subcommand;
use std::env;

use crate::agent_skills;
use crate::capability;
use crate::config::Config;

#[derive(Subcommand, Debug)]
pub enum SkillsCmd {
    /// Make the product's skills match its capabilities
    ///
    /// Writes what the gate enforces (`.fiducial/agent-skills.json`) and the
    /// skills block in `AGENTS.md`, and re-links the platform's own skills for
    /// the agents in `[skills] agents`. With `--install`, runs the `skills` CLI to
    /// install the skills the capabilities name and remove ones none needs.
    Sync {
        /// Run the `skills` CLI (`npx skills add` / `remove`; needs Node 22.20+)
        #[arg(long)]
        install: bool,
    },
    /// Say whether the skills match the capabilities, and are current upstream
    ///
    /// Exits 1 on any finding. Installed skills are compared with their source by
    /// `npx skills check`, which also catches a local edit (network).
    Check {
        /// Skip the upstream comparison (no network)
        #[arg(long)]
        offline: bool,
    },
    /// Show what is enforced, and for which files
    List {
        /// Every skill any built-in capability can name, as JSON (for CI)
        #[arg(long)]
        all: bool,
    },
}

pub fn run(cmd: SkillsCmd) -> Result<()> {
    // `--all` reads no product: it is the platform's own list.
    if let SkillsCmd::List { all: true } = cmd {
        let all: Vec<serde_json::Value> = agent_skills::known()
            .iter()
            .map(|a| {
                let caps: Vec<&str> = capability::builtins()
                    .iter()
                    .filter(|c| c.agent_skills.iter().any(|x| x.id == a.id))
                    .map(|c| c.id.as_str())
                    .collect();
                serde_json::json!({
                    "id": a.id,
                    "source": a.source,
                    "skills": a.skills,
                    "paths": a.paths,
                    "capabilities": caps,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&all)?);
        return Ok(());
    }
    let cwd = env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(crate::config::CONFIG_FILE))?;
    let enabled = cfg.capabilities.enabled.clone();

    match cmd {
        SkillsCmd::Sync { install } => {
            let agents = cfg.skills.agents();
            // The platform's own skills, for the agents the product now names.
            for id in &enabled {
                if let Some(def) = capability::find(id) {
                    capability::write_skill(&root, def, &cfg.product.name)?;
                }
            }
            let done = agent_skills::sync(&root, &enabled, &agents, install)?;
            if done.wrote.is_empty() && done.missing.is_empty() && done.surplus.is_empty() {
                println!("  ✓ skills already match the installed capabilities");
            }
            for w in &done.wrote {
                println!("  ✓ {w}");
            }
            if !done.missing.is_empty() {
                println!(
                    "  ! not installed: {}. Run `fid skills sync --install`, then commit skills-lock.json.",
                    done.missing.join(", ")
                );
            }
            if !done.surplus.is_empty() {
                println!(
                    "  ! no capability needs: {}. `fid skills sync --install` removes them.",
                    done.surplus.join(", ")
                );
            }
            Ok(())
        }
        SkillsCmd::List { .. } => {
            let needed = agent_skills::needed(&enabled);
            if needed.is_empty() {
                println!("  No installed capability declares an agent skill.");
                return Ok(());
            }
            for a in &needed {
                println!("  {}  [skills add {}]", a.id, a.source);
                println!("    load one of: {}", a.skills.join(", "));
                println!("    before editing: {}", a.paths.join(", "));
                println!("    why: {}", a.why);
            }
            Ok(())
        }
        SkillsCmd::Check { offline } => {
            let mut findings = agent_skills::problems(&root, &enabled, &cfg.skills.agents());
            if !offline
                && root.join(agent_skills::LOCK_FILE).exists()
                && !agent_skills::upstream_check(&root)?
            {
                findings.push(
                    "an installed skill is behind its source, or has been edited locally (`skills check` above says which); `npx skills update` brings it current"
                        .into(),
                );
            }
            if findings.is_empty() {
                println!("  ✓ agent skills match the installed capabilities");
                return Ok(());
            }
            for f in &findings {
                println!("  ✗ {f}");
            }
            bail!("{} agent-skill finding(s)", findings.len());
        }
    }
}

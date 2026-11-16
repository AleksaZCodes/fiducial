//! `fid skills` — the AI skills this product's capabilities need the agent to load.
//!
//! See [`crate::agent_skills`] for what is enforced and why. This is the
//! command surface: `sync` makes the product match its capabilities, `check`
//! says whether it does and whether vendored skills are current upstream,
//! `list` shows what is enforced.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use std::env;

use crate::agent_skills::{self, SKILLS_CLI};
use crate::capability::AgentSkillKind;
use crate::config::Config;

#[derive(Subcommand, Debug)]
pub enum SkillsCmd {
    /// Enable the skills the installed capabilities need, and no others
    ///
    /// Writes `.claude/settings.json` (plugins), `.fiducial/agent-skills.json`
    /// (what the gate enforces). With `--install`, also installs the skills a
    /// capability names that are vendored into the repository.
    Sync {
        /// Install vendored skills (runs `npx skills add`; needs Node 22.20+)
        #[arg(long)]
        install: bool,
    },
    /// Say whether the skills match the capabilities, and are current upstream
    ///
    /// Exits 1 on any finding. Vendored skills are compared with their source by
    /// `npx skills check` (network).
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
                serde_json::json!({
                    "id": a.id,
                    "kind": match a.kind {
                        AgentSkillKind::Plugin => "plugin",
                        AgentSkillKind::SkillsCli => "skills-cli",
                    },
                    "repo": a.repo,
                    "marketplace": a.marketplace,
                    "plugin": a.plugin,
                    "source": a.source,
                    "skill": a.skill,
                    "skills": a.skills,
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
            let done = agent_skills::sync(&root, &enabled, install)?;
            if done.wrote.is_empty() && done.missing.is_empty() {
                println!("  ✓ skills already match the installed capabilities");
            }
            for w in &done.wrote {
                println!("  ✓ wrote {w}");
            }
            for a in &done.missing {
                println!(
                    "  ! `{}` (from {}) is not installed. Run `fid skills sync --install`.",
                    a.skill, a.source
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
                println!(
                    "  {}  [{}]",
                    a.id,
                    match a.kind {
                        AgentSkillKind::Plugin => format!("plugin {}", a.plugin_key()),
                        AgentSkillKind::SkillsCli => format!("skills-cli {}", a.source),
                    }
                );
                println!("    load one of: {}", a.skills.join(", "));
                println!("    before editing: {}", a.paths.join(", "));
                println!("    why: {}", a.why);
            }
            Ok(())
        }
        SkillsCmd::Check { offline } => {
            let mut findings = agent_skills::problems(&root, &enabled);
            if !offline {
                let vendored = agent_skills::needed(&enabled)
                    .into_iter()
                    .any(|a| a.kind == AgentSkillKind::SkillsCli);
                if vendored && root.join("skills-lock.json").exists() {
                    let st = std::process::Command::new("npx")
                        .args(["--yes", SKILLS_CLI, "check"])
                        .current_dir(&root)
                        .status()
                        .context("running `npx skills check` (is Node 22.20 or newer on PATH?)")?;
                    if !st.success() {
                        findings.push(
                            "a vendored skill is behind its source (or edited locally); `npx skills update` brings it current"
                                .into(),
                        );
                    }
                }
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

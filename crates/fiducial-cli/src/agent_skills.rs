//! The AI skills a product's agents must load, chosen by the capabilities the
//! product has.
//!
//! # Standards, not a vendor's internals
//!
//! A capability names the vendor's skill for the technology it uses (shadcn for
//! components, Cloudflare for Workers) instead of copying guidance that would go
//! stale the day the vendor changes the technology. Everything about getting
//! those skills into a repository is the open ecosystem's own tooling:
//!
//! * **Format and location.** A skill is a folder with a `SKILL.md`, the Agent
//!   Skills format. Project skills live once in `.agents/skills/`, the shared
//!   directory Codex, Cursor, GitHub Copilot, Gemini CLI, OpenCode, Amp and others
//!   read directly; an agent with a directory of its own (Claude Code reads
//!   `.claude/skills/`) gets a link to that copy.
//! * **Install, update, remove, verify.** The `skills` CLI (`npx skills add`,
//!   `update`, `remove`, `check`), run exactly as its documentation shows, with its
//!   version pinned. It writes `skills-lock.json`, which is committed and records
//!   the upstream commit and a content hash per skill; `skills check` exits 1 when
//!   one is behind its source or has been edited locally, which is the CI gate.
//! * **Instructions every agent reads.** `AGENTS.md` is the open file coding
//!   agents load; a derived block in it says which skill to load before editing
//!   which files.
//!
//! What this module adds is only the part no standard covers: *which* skills a
//! product needs (from its capabilities), *when* one is required (by file), and a
//! check that the product has them.
//!
//! # Enforcement, in layers
//!
//! 1. **Every agent:** the `AGENTS.md` block, and `fid doctor` / `fid skills check`
//!    (a missing, surplus, stale or locally edited skill fails them), which CI runs.
//! 2. **Claude Code, additionally:** a `PreToolUse` hook ([`gate_from_stdin`])
//!    stops an `Edit`/`Write` to a covered file until one of the named skills has
//!    been loaded. Claude Code is the agent whose documented hooks allow a hard
//!    stop; another agent with an equivalent can be given the same adapter. It
//!    decides from two documented tool calls only: the `Skill` tool (its `skill`
//!    parameter) and a `Read` of the skill's `SKILL.md`, each recorded by a
//!    `PostToolUse` hook. It does not read the session transcript.
//!
//! The gate does not stop an edit made through a shell command: a path rule over
//! arbitrary shell is a guess, and `guard` exists to not guess at shell.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::capability::{self, AgentSkill};

/// The `skills` CLI, pinned. An unpinned `npx` fetch runs whatever was published
/// last, which is what the `no-unpinned-cli-fetch` guard rule exists to stop.
pub const SKILLS_CLI: &str = "skills@1.9.0";

/// The gate's input, derived.
pub const GATE_FILE: &str = ".fiducial/agent-skills.json";
/// The `skills` CLI's lock file, committed.
pub const LOCK_FILE: &str = "skills-lock.json";
/// Per-session records of skills loaded. Gitignored.
pub const STATE_DIR: &str = ".fiducial/state/skills";
/// The shared project skills directory.
pub const SHARED_DIR: &str = ".agents/skills";

const BEGIN: &str = "<!-- fid:begin agent-skills -->";
const END: &str = "<!-- fid:end agent-skills -->";

// ── Agents ───────────────────────────────────────────────────────────────────

/// `skills` CLI agent identifier → the directory it reads project skills from.
///
/// From the CLI's own `docs/agents.md`. Many agents share `.agents/skills/`, which
/// is why one canonical copy serves them without a link.
const AGENT_DIRS: &[(&str, &str)] = &[
    ("aider-desk", ".aider-desk/skills"),
    ("amp", ".agents/skills"),
    ("replit", ".agents/skills"),
    ("universal", ".agents/skills"),
    ("antigravity", ".agents/skills"),
    ("antigravity-cli", ".agents/skills"),
    ("astrbot", "data/skills"),
    ("autohand-code", ".autohand/skills"),
    ("augment", ".augment/skills"),
    ("bob", ".bob/skills"),
    ("claude-code", ".claude/skills"),
    ("openclaw", "skills"),
    ("cline", ".agents/skills"),
    ("codex", ".agents/skills"),
    ("dexto", ".agents/skills"),
    ("fx", ".agents/skills"),
    ("kimi-code-cli", ".agents/skills"),
    ("loaf", ".agents/skills"),
    ("pi", ".agents/skills"),
    ("sarvam-code", ".agents/skills"),
    ("warp", ".agents/skills"),
    ("zed", ".agents/skills"),
    ("codearts-agent", ".codeartsdoer/skills"),
    ("codebuddy", ".codebuddy/skills"),
    ("codemaker", ".codemaker/skills"),
    ("codestudio", ".codestudio/skills"),
    ("command-code", ".commandcode/skills"),
    ("continue", ".continue/skills"),
    ("cortex", ".cortex/skills"),
    ("crush", ".crush/skills"),
    ("cursor", ".agents/skills"),
    ("deepagents", ".agents/skills"),
    ("devin", ".devin/skills"),
    ("droid", ".agents/skills"),
    ("firebender", ".agents/skills"),
    ("forgecode", ".forge/skills"),
    ("gemini-cli", ".agents/skills"),
    ("github-copilot", ".agents/skills"),
    ("goose", ".goose/skills"),
    ("grok", ".grok/skills"),
    ("hermes-agent", ".hermes/skills"),
    ("inference-sh", ".inferencesh/skills"),
    ("jazz", ".jazz/skills"),
    ("junie", ".junie/skills"),
    ("iflow-cli", ".iflow/skills"),
    ("kilo", ".agents/skills"),
    ("kimchi", ".kimchi/skills"),
    ("kiro-cli", ".kiro/skills"),
    ("kode", ".kode/skills"),
    ("lingma", ".lingma/skills"),
    ("mcpjam", ".mcpjam/skills"),
    ("minimax-code", ".minimax/skills"),
    ("mistral-vibe", ".vibe/skills"),
    ("moxby", ".moxby/skills"),
    ("mux", ".mux/skills"),
    ("opencode", ".agents/skills"),
    ("openhands", ".openhands/skills"),
    ("ona", ".ona/skills"),
    ("posit-assistant", ".posit/assistant/skills"),
    ("qoder", ".qoder/skills"),
    ("qoder-cn", ".qoder/skills"),
    ("qwen-code", ".qwen/skills"),
    ("reasonix", ".reasonix/skills"),
    ("rovodev", ".rovodev/skills"),
    ("roo", ".roo/skills"),
    ("tabnine-cli", ".tabnine/agent/skills"),
    ("terramind", ".terramind/skills"),
    ("tinycloud", ".tinycloud/skills"),
    ("trae", ".trae/skills"),
    ("trae-cn", ".trae/skills"),
    ("windsurf", ".windsurf/skills"),
    ("zcode", ".zcode/skills"),
    ("zencoder", ".zencoder/skills"),
    ("zenflow", ".zencoder/skills"),
    ("neovate", ".neovate/skills"),
    ("pochi", ".pochi/skills"),
    ("promptscript", ".agents/skills"),
    ("adal", ".adal/skills"),
];

/// The project skills directory an agent reads, if the identifier is known.
pub fn agent_dir(id: &str) -> Option<&'static str> {
    AGENT_DIRS.iter().find(|(a, _)| *a == id).map(|(_, d)| *d)
}

/// Link (or, where links are not possible, copy) a skill in the shared directory
/// into the directories of the given agents that do not read the shared one.
///
/// Only for skills this platform writes itself (`fiducial-*`): a skill from the
/// `skills` CLI is linked by the CLI. The replaced entry is only ever a link or
/// copy of that same name.
pub fn link_for_agents(root: &Path, name: &str, agents: &[String]) -> Result<()> {
    assert!(
        name.starts_with("fiducial-"),
        "only the platform's own skills"
    );
    let canonical = root.join(SHARED_DIR).join(name);
    for agent in agents {
        let Some(dir) = agent_dir(agent) else {
            continue;
        };
        if dir == SHARED_DIR {
            continue;
        }
        let parent = root.join(dir);
        std::fs::create_dir_all(&parent)?;
        let at = parent.join(name);
        if at.symlink_metadata().is_ok() {
            if at.is_dir() && !at.is_symlink() {
                std::fs::remove_dir_all(&at)?;
            } else {
                std::fs::remove_file(&at)?;
            }
        }
        #[cfg(unix)]
        {
            let depth = dir.split('/').count();
            let target = format!("{}{SHARED_DIR}/{name}", "../".repeat(depth));
            std::os::unix::fs::symlink(&target, &at)
                .with_context(|| format!("linking {} → {target}", at.display()))?;
        }
        #[cfg(not(unix))]
        copy_dir(&canonical, &at)?;
    }
    let _ = canonical;
    Ok(())
}

#[cfg(not(unix))]
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let dest = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &dest)?;
        } else {
            std::fs::copy(e.path(), dest)?;
        }
    }
    Ok(())
}

// ── What a product needs ─────────────────────────────────────────────────────

/// The skills the given capabilities declare, one per `id`.
pub fn needed(enabled: &[String]) -> Vec<AgentSkill> {
    dedupe(
        enabled
            .iter()
            .filter_map(|id| capability::find(id))
            .flat_map(|c| c.agent_skills.iter().cloned()),
    )
}

/// Every skill any built-in capability can declare: what this platform could
/// have put in a product, and so what it may take out again. A skill in the
/// product's lock that is not in this set is the product's own and is left alone.
pub fn known() -> Vec<AgentSkill> {
    dedupe(
        capability::builtins()
            .iter()
            .flat_map(|c| c.agent_skills.iter().cloned()),
    )
}

fn dedupe(all: impl Iterator<Item = AgentSkill>) -> Vec<AgentSkill> {
    let mut seen = BTreeSet::new();
    let mut out: Vec<AgentSkill> = all.filter(|a| seen.insert(a.id.clone())).collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn names(skills: &[AgentSkill]) -> BTreeSet<String> {
    skills
        .iter()
        .flat_map(|a| a.skills.iter().cloned())
        .collect()
}

// ── The CLI's lock, and what is installed ────────────────────────────────────

/// The skill names `skills-lock.json` tracks.
pub fn tracked(root: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(root.join(LOCK_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["skills"].as_object().map(|m| m.keys().cloned().collect()))
        .unwrap_or_default()
}

fn installed(root: &Path, name: &str, lock: &BTreeSet<String>) -> bool {
    lock.contains(name) && root.join(SHARED_DIR).join(name).join("SKILL.md").exists()
}

fn run_skills_cli(root: &Path, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new("npx")
        .arg("--yes")
        .arg(SKILLS_CLI)
        .args(args)
        .current_dir(root)
        .status()
        .context("running the `skills` CLI through npx (needs Node 22.20 or newer on PATH)")?;
    if !status.success() {
        bail!("`npx {SKILLS_CLI} {}` failed", args.join(" "));
    }
    Ok(())
}

/// Run `skills check`: exit 1 means a skill is behind its source or edited locally.
pub fn upstream_check(root: &Path) -> Result<bool> {
    let status = std::process::Command::new("npx")
        .args(["--yes", SKILLS_CLI, "check"])
        .current_dir(root)
        .status()
        .context("running `skills check` through npx (needs Node 22.20 or newer on PATH)")?;
    Ok(status.success())
}

// ── What the platform writes ─────────────────────────────────────────────────

/// How to get a skill, said to the agent when it is missing.
pub const INSTALL_HINT: &str = "run `fid skills sync --install`, which runs the `skills` CLI";

/// The gate's input file.
pub fn render_gate(needed: &[AgentSkill]) -> String {
    let skills: Vec<Value> = needed
        .iter()
        .map(|a| {
            json!({
                "id": a.id,
                "skills": a.skills,
                "paths": a.paths,
                "why": a.why,
                "install": INSTALL_HINT,
            })
        })
        .collect();
    let doc = json!({
        "//": "Derived by `fid skills sync` from the installed capabilities. Do not edit.",
        "skills": skills,
    });
    format!("{}\n", serde_json::to_string_pretty(&doc).expect("json"))
}

/// The block in `AGENTS.md`: what every agent that reads the file is told.
pub fn render_agents_block(needed: &[AgentSkill]) -> String {
    let mut s = format!("{BEGIN}\n");
    s.push_str("## Skills to load first\n\n");
    if needed.is_empty() {
        s.push_str("No installed capability needs a vendor skill loaded before editing.\n");
    } else {
        s.push_str(
            "Load the named skill before you edit a file that matches. The skills are in \
             `.agents/skills/<name>/SKILL.md` (your agent's own skills directory links to it), \
             pinned in `skills-lock.json`. If one is missing, run `fid skills sync --install`.\n\n",
        );
        s.push_str("| Before editing | Load one of | Why |\n|---|---|---|\n");
        for a in needed {
            s.push_str(&format!(
                "| {} | {} | {} |\n",
                a.paths
                    .iter()
                    .map(|p| format!("`{p}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                a.skills
                    .iter()
                    .map(|n| format!("`{n}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                a.why
            ));
        }
    }
    s.push_str(END);
    s.push('\n');
    s
}

/// `existing` with the skills block replaced, or appended if it has none.
pub fn apply_agents_block(existing: &str, block: &str) -> String {
    match (existing.find(BEGIN), existing.find(END)) {
        (Some(a), Some(b)) if a < b => {
            let end = b + END.len();
            let end = if existing[end..].starts_with('\n') {
                end + 1
            } else {
                end
            };
            format!("{}{}{}", &existing[..a], block, &existing[end..])
        }
        _ => format!("{}\n\n{}", existing.trim_end_matches('\n'), block),
    }
}

fn current_block(existing: &str) -> Option<&str> {
    let a = existing.find(BEGIN)?;
    let b = existing.find(END)?;
    (a < b).then(|| &existing[a..b + END.len()])
}

/// What `fid skills sync` did.
#[derive(Debug, Default)]
pub struct Synced {
    /// Files written or changed.
    pub wrote: Vec<String>,
    /// Skills the product needs and does not have.
    pub missing: Vec<String>,
    /// Skills this platform installed that no capability needs any more.
    pub surplus: Vec<String>,
}

/// Bring the gate file and the `AGENTS.md` block in line with `enabled`; with
/// `install`, also run the `skills` CLI to install what is missing and remove
/// what is no longer needed.
pub fn sync(root: &Path, enabled: &[String], agents: &[String], install: bool) -> Result<Synced> {
    let needed = needed(enabled);
    let mut out = Synced::default();

    if write_if_changed(&root.join(GATE_FILE), &render_gate(&needed))? {
        out.wrote.push(GATE_FILE.into());
    }
    let agents_md = root.join("AGENTS.md");
    if let Ok(text) = std::fs::read_to_string(&agents_md) {
        let next = apply_agents_block(&text, &render_agents_block(&needed));
        if next != text {
            std::fs::write(&agents_md, &next)?;
            out.wrote.push("AGENTS.md".into());
            // A template the platform tracks, edited by the platform: re-record it,
            // as `fid add` does for `fiducial.toml`, or it reads as hand-modified.
            let lock_path = root.join("fiducial.lock");
            if lock_path.exists() {
                let mut lock = crate::lock::Lock::load(&lock_path)?;
                if lock.templates.contains_key("AGENTS.md") {
                    lock.rehash("AGENTS.md", next.as_bytes(), capability::PLATFORM_VERSION);
                    lock.save(&lock_path)?;
                }
            }
        }
    }

    let want = names(&needed);
    let lock = tracked(root);
    let ours = names(&known());
    out.missing = want
        .iter()
        .filter(|n| !installed(root, n, &lock))
        .cloned()
        .collect();
    out.surplus = ours
        .iter()
        .filter(|n| lock.contains(*n) && !want.contains(*n))
        .cloned()
        .collect();

    if install {
        let agent_args: Vec<&str> = agents.iter().map(String::as_str).collect();
        for a in &needed {
            let missing: Vec<&str> = a
                .skills
                .iter()
                .filter(|n| !installed(root, n, &lock))
                .map(String::as_str)
                .collect();
            if missing.is_empty() {
                continue;
            }
            let mut args = vec!["add", a.source.as_str(), "--skill"];
            args.extend(missing.iter().copied());
            args.push("--agent");
            args.extend(agent_args.iter().copied());
            args.push("-y");
            run_skills_cli(root, &args)?;
            out.wrote
                .push(format!("{} ← {}", missing.join(", "), a.source));
        }
        if !out.surplus.is_empty() {
            let mut args = vec!["remove"];
            args.extend(out.surplus.iter().map(String::as_str));
            args.push("-y");
            run_skills_cli(root, &args)?;
            out.wrote
                .push(format!("removed {}", out.surplus.join(", ")));
        }
        let lock = tracked(root);
        out.missing = want
            .iter()
            .filter(|n| !installed(root, n, &lock))
            .cloned()
            .collect();
        if !out.missing.is_empty() {
            bail!(
                "the `skills` CLI finished but {} is still not installed; the source may have renamed it \
                 (the weekly `scripts/verify-agent-skills.sh` checks this)",
                out.missing.join(", ")
            );
        }
        out.surplus.clear();
    }
    Ok(out)
}

fn write_if_changed(path: &Path, content: &str) -> Result<bool> {
    if std::fs::read_to_string(path).is_ok_and(|c| c == content) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

/// What is wrong, for `fid doctor` and `fid skills check`.
pub fn problems(root: &Path, enabled: &[String], agents: &[String]) -> Vec<String> {
    let needed = needed(enabled);
    let mut out = Vec::new();

    for a in agents {
        if agent_dir(a).is_none() {
            out.push(format!(
                "[skills] agents lists `{a}`, which the `skills` CLI does not know. \
                 Its identifiers are listed in the CLI's docs/agents.md."
            ));
        }
    }

    let lock = tracked(root);
    for a in &needed {
        for n in a.skills.iter().filter(|n| !installed(root, n, &lock)) {
            out.push(format!(
                "skill `{n}` (from {}) is not installed, and edits to {} are meant to wait for it. `fid skills sync --install`.",
                a.source,
                a.paths.join(", ")
            ));
        }
    }
    let want = names(&needed);
    for n in names(&known())
        .iter()
        .filter(|n| lock.contains(*n) && !want.contains(*n))
    {
        out.push(format!(
            "skill `{n}` is installed but no installed capability needs it. `fid skills sync --install` removes it."
        ));
    }
    if std::fs::read_to_string(root.join(GATE_FILE))
        .ok()
        .as_deref()
        != Some(render_gate(&needed).as_str())
    {
        out.push(format!(
            "{GATE_FILE} is stale or missing, so the skill gate enforces the wrong set. `fid skills sync` rewrites it."
        ));
    }
    if let Ok(text) = std::fs::read_to_string(root.join("AGENTS.md")) {
        if current_block(&text) != Some(render_agents_block(&needed).trim_end()) {
            out.push(
                "the skills block in AGENTS.md is missing or stale, so agents are told the wrong skills. `fid skills sync` rewrites it."
                    .into(),
            );
        }
    }
    // The platform's own skills, for every capability the product has, and a
    // link for every agent that reads a directory of its own.
    for id in enabled.iter().filter(|id| capability::find(id).is_some()) {
        let name = format!("fiducial-{id}");
        if !root.join(SHARED_DIR).join(&name).join("SKILL.md").exists() {
            out.push(format!(
                "capability `{id}` has no skill at {SHARED_DIR}/{name}/SKILL.md. `fid upgrade` writes it."
            ));
            continue;
        }
        for agent in agents {
            if let Some(dir) = agent_dir(agent).filter(|d| *d != SHARED_DIR) {
                if !root.join(dir).join(&name).join("SKILL.md").exists() {
                    out.push(format!(
                        "capability `{id}`'s skill is not linked for `{agent}` ({dir}/{name}). `fid skills sync` links it."
                    ));
                }
            }
        }
    }
    out
}

// ── The gate (Claude Code adapter) ───────────────────────────────────────────

/// What a hook is told. Only the fields the gate reads.
#[derive(Debug, Default, serde::Deserialize)]
pub struct HookInput {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: Value,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny(String),
}

/// Segment-wise glob: `**` matches any number of directories (including none),
/// `*` any run of characters inside one segment, `?` one character.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    fn seg(p: &[u8], s: &[u8]) -> bool {
        match (p.first(), s.first()) {
            (None, None) => true,
            (Some(b'*'), _) => seg(&p[1..], s) || (!s.is_empty() && seg(p, &s[1..])),
            (Some(b'?'), Some(_)) => seg(&p[1..], &s[1..]),
            (Some(a), Some(b)) if a == b => seg(&p[1..], &s[1..]),
            _ => false,
        }
    }
    fn go(p: &[&str], s: &[&str]) -> bool {
        match p.first() {
            None => s.is_empty(),
            Some(&"**") => (0..=s.len()).any(|k| go(&p[1..], &s[k..])),
            Some(first) => {
                !s.is_empty() && seg(first.as_bytes(), s[0].as_bytes()) && go(&p[1..], &s[1..])
            }
        }
    }
    let p: Vec<&str> = pattern.split('/').filter(|x| !x.is_empty()).collect();
    let s: Vec<&str> = path.split('/').filter(|x| !x.is_empty()).collect();
    go(&p, &s)
}

fn marker_name(skill: &str) -> String {
    skill.replace([':', '/'], "__")
}

/// Skills this session has loaded, from the records the `PostToolUse` hook wrote.
pub fn loaded(root: &Path, session_id: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if session_id.is_empty() {
        return out;
    }
    if let Ok(rd) = std::fs::read_dir(root.join(STATE_DIR).join(session_id)) {
        for e in rd.flatten() {
            if let Ok(n) = std::fs::read_to_string(e.path()) {
                out.insert(n.trim().to_string());
            }
        }
    }
    out
}

/// The skill a tool call loaded, from its documented inputs: the `Skill` tool's
/// `skill` parameter, or a `Read` of a skill's `SKILL.md`.
pub fn skill_loaded_by(input: &HookInput) -> Option<String> {
    match input.tool_name.as_str() {
        "Skill" => input.tool_input["skill"]
            .as_str()
            .map(|s| s.trim_start_matches('/').to_string()),
        "Read" => {
            let path = input.tool_input["file_path"].as_str()?;
            let parts: Vec<&str> = path.split('/').collect();
            let n = parts.len();
            (n >= 3 && parts[n - 1] == "SKILL.md" && parts[n - 3] == "skills")
                .then(|| parts[n - 2].to_string())
        }
        _ => None,
    }
}

fn project_root(input: &HookInput) -> Option<PathBuf> {
    let start = std::env::var("CLAUDE_PROJECT_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| (!input.cwd.is_empty()).then(|| input.cwd.clone()))
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;
    start
        .ancestors()
        .find(|d| d.join(GATE_FILE).exists())
        .map(Path::to_path_buf)
}

fn edited_path(input: &HookInput) -> Option<String> {
    ["file_path", "notebook_path", "path"]
        .iter()
        .find_map(|k| input.tool_input[k].as_str())
        .map(str::to_string)
}

/// Decide one tool call against the gate file.
pub fn decide(root: &Path, gate: &Value, input: &HookInput) -> Decision {
    if std::env::var("FID_SKILL_GATE").is_ok_and(|v| v == "off") {
        return Decision::Allow;
    }
    if !matches!(
        input.tool_name.as_str(),
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit"
    ) {
        return Decision::Allow;
    }
    let Some(file) = edited_path(input) else {
        return Decision::Allow;
    };
    let rel = match Path::new(&file).strip_prefix(root) {
        Ok(r) => r.to_string_lossy().to_string(),
        Err(_) => file.trim_start_matches("./").to_string(),
    };

    let mut blocked: Vec<String> = Vec::new();
    let mut have: Option<BTreeSet<String>> = None;
    for entry in gate["skills"].as_array().into_iter().flatten() {
        let paths: Vec<&str> = entry["paths"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if !paths.iter().any(|p| glob_match(p, &rel)) {
            continue;
        }
        let names: Vec<&str> = entry["skills"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let have = have.get_or_insert_with(|| loaded(root, &input.session_id));
        if names.iter().any(|n| have.contains(*n)) {
            continue;
        }
        blocked.push(format!(
            "`{rel}` is covered by the {} skills. Load one of {} (the Skill tool, or read its SKILL.md under .agents/skills/), then retry. {} If it is not installed: {}. (FID_SKILL_GATE=off turns this gate off for a session you have decided to run without it.)",
            entry["id"].as_str().unwrap_or("?"),
            names
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", "),
            entry["why"].as_str().unwrap_or(""),
            entry["install"].as_str().unwrap_or(INSTALL_HINT),
        ));
    }
    if blocked.is_empty() {
        Decision::Allow
    } else {
        Decision::Deny(blocked.join("\n"))
    }
}

/// `fid skill-gate`: the `PreToolUse` hook. Exit 2 blocks, with the reason on
/// stderr, which is what Claude Code shows the agent.
pub fn gate_from_stdin() -> Result<()> {
    let mut raw = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut raw)?;
    let input: HookInput = serde_json::from_str(&raw).unwrap_or_default();
    let Some(root) = project_root(&input) else {
        return Ok(()); // not a product with a gate
    };
    let gate: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(GATE_FILE)).context("reading the skill gate file")?,
    )
    .context("the skill gate file is not valid JSON; `fid skills sync` rewrites it")?;
    match decide(&root, &gate, &input) {
        Decision::Allow => Ok(()),
        Decision::Deny(why) => {
            eprintln!("{why}");
            std::process::exit(2);
        }
    }
}

/// `fid skill-mark`: the `PostToolUse` hook on `Skill` and `Read`. Records that
/// this session loaded a skill.
pub fn mark_from_stdin() -> Result<()> {
    let mut raw = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut raw)?;
    let input: HookInput = serde_json::from_str(&raw).unwrap_or_default();
    let (Some(root), Some(skill)) = (project_root(&input), skill_loaded_by(&input)) else {
        return Ok(());
    };
    if input.session_id.is_empty() {
        return Ok(());
    }
    let dir = root.join(STATE_DIR).join(&input.session_id);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(marker_name(&skill)), format!("{skill}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cloudflare() -> AgentSkill {
        AgentSkill {
            id: "cloudflare-workers".into(),
            source: "cloudflare/skills".into(),
            skills: vec!["wrangler".into(), "workers-best-practices".into()],
            paths: vec!["**/wrangler.jsonc".into(), "**/wrangler.toml".into()],
            why: "Wrangler config changes between releases.".into(),
        }
    }

    fn gate(skills: &[AgentSkill]) -> Value {
        serde_json::from_str(&render_gate(skills)).unwrap()
    }

    fn edit(path: &str, session: &str) -> HookInput {
        HookInput {
            session_id: session.into(),
            tool_name: "Edit".into(),
            tool_input: json!({ "file_path": path }),
            ..Default::default()
        }
    }

    fn mark(root: &Path, session: &str, tool: &str, input: Value) {
        let h = HookInput {
            session_id: session.into(),
            tool_name: tool.into(),
            tool_input: input,
            ..Default::default()
        };
        let skill = skill_loaded_by(&h).expect("a load");
        let dir = root.join(STATE_DIR).join(session);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(marker_name(&skill)), format!("{skill}\n")).unwrap();
    }

    /// Capabilities that need the same skill each declare it, and the platform
    /// keeps one entry per `id`. If two declarations of an id differed, one would
    /// be ignored without a word, so they may not differ.
    #[test]
    fn every_declaration_of_one_id_is_the_same_declaration() {
        let mut by_id: std::collections::BTreeMap<String, (String, AgentSkill)> =
            Default::default();
        for cap in capability::builtins() {
            for a in &cap.agent_skills {
                match by_id.get(&a.id) {
                    None => {
                        by_id.insert(a.id.clone(), (cap.id.clone(), a.clone()));
                    }
                    Some((first, seen)) => assert_eq!(
                        seen, a,
                        "`{}` declares agent skill `{}` differently from `{first}`; they must agree, \
                         because one entry is kept and the other is silently ignored",
                        cap.id, a.id
                    ),
                }
            }
        }
        assert!(
            by_id.len() >= 3,
            "the built-in capabilities name skills: {by_id:?}"
        );
    }

    #[test]
    fn globs_match_the_way_a_path_reads() {
        assert!(glob_match("**/wrangler.jsonc", "wrangler.jsonc"));
        assert!(glob_match("**/wrangler.jsonc", "apps/web/wrangler.jsonc"));
        assert!(!glob_match("**/wrangler.jsonc", "apps/web/wrangler.json"));
        assert!(glob_match(
            "**/components/ui/**",
            "apps/web/src/components/ui/button.tsx"
        ));
        assert!(!glob_match(
            "**/components/ui/**",
            "apps/web/src/components/card.tsx"
        ));
        assert!(glob_match("apps/*/wrangler.*", "apps/api/wrangler.toml"));
        assert!(!glob_match(
            "apps/*/wrangler.*",
            "apps/api/src/wrangler.toml"
        ));
        assert!(glob_match("a?c", "abc") && !glob_match("a?c", "ac"));
    }

    #[test]
    fn the_agent_table_covers_the_shared_group_and_claude() {
        assert_eq!(agent_dir("claude-code"), Some(".claude/skills"));
        for a in [
            "codex",
            "cursor",
            "github-copilot",
            "gemini-cli",
            "opencode",
            "amp",
        ] {
            assert_eq!(
                agent_dir(a),
                Some(SHARED_DIR),
                "{a} reads the shared directory"
            );
        }
        assert_eq!(agent_dir("not-an-agent"), None);
        for (id, dir) in AGENT_DIRS {
            assert!(dir.ends_with("skills"), "{id}: {dir}");
            assert!(!dir.starts_with('/') && !dir.contains(".."), "{id}: {dir}");
        }
        for a in crate::config::Skills::DEFAULT_AGENTS {
            assert!(
                agent_dir(a).is_some(),
                "the default names a known agent: {a}"
            );
        }
    }

    #[test]
    fn an_edit_to_a_covered_file_is_stopped_until_the_skill_is_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let g = gate(&[cloudflare()]);
        let Decision::Deny(why) = decide(tmp.path(), &g, &edit("apps/web/wrangler.jsonc", "s1"))
        else {
            panic!("an uncovered edit was allowed")
        };
        assert!(
            why.contains("`wrangler`") && why.contains("fid skills sync --install"),
            "{why}"
        );
        assert_eq!(
            decide(tmp.path(), &g, &edit("apps/web/src/page.tsx", "s1")),
            Decision::Allow
        );
        let mut read = edit("apps/web/wrangler.jsonc", "s1");
        read.tool_name = "Read".into();
        assert_eq!(decide(tmp.path(), &g, &read), Decision::Allow);
    }

    #[test]
    fn a_skill_is_loaded_by_the_skill_tool_or_by_reading_its_file_and_only_for_that_session() {
        let tmp = tempfile::tempdir().unwrap();
        let g = gate(&[cloudflare()]);

        // The Skill tool's own parameter.
        mark(tmp.path(), "s1", "Skill", json!({ "skill": "wrangler" }));
        assert_eq!(
            decide(tmp.path(), &g, &edit("wrangler.jsonc", "s1")),
            Decision::Allow
        );
        assert!(matches!(
            decide(tmp.path(), &g, &edit("wrangler.jsonc", "s2")),
            Decision::Deny(_)
        ));

        // Reading the SKILL.md, through the shared directory or an agent's link.
        for path in [
            "/repo/.agents/skills/workers-best-practices/SKILL.md",
            "/repo/.claude/skills/workers-best-practices/SKILL.md",
        ] {
            let tmp = tempfile::tempdir().unwrap();
            mark(tmp.path(), "s3", "Read", json!({ "file_path": path }));
            assert_eq!(
                decide(tmp.path(), &g, &edit("wrangler.toml", "s3")),
                Decision::Allow,
                "{path}"
            );
        }

        // A different skill, or a file that merely has the name, does not count.
        let tmp = tempfile::tempdir().unwrap();
        mark(tmp.path(), "s4", "Skill", json!({ "skill": "shadcn" }));
        assert!(matches!(
            decide(tmp.path(), &g, &edit("wrangler.toml", "s4")),
            Decision::Deny(_)
        ));
        let not = HookInput {
            tool_name: "Read".into(),
            tool_input: json!({ "file_path": "/repo/notes/SKILL.md" }),
            ..Default::default()
        };
        assert_eq!(skill_loaded_by(&not), None);
    }

    #[test]
    fn an_absolute_path_is_judged_relative_to_the_product() {
        let tmp = tempfile::tempdir().unwrap();
        let g = gate(&[cloudflare()]);
        let abs = tmp.path().join("apps/api/wrangler.toml");
        assert!(matches!(
            decide(tmp.path(), &g, &edit(&abs.to_string_lossy(), "s1")),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn the_agents_block_is_replaced_in_place_and_appended_once() {
        let block = render_agents_block(&[cloudflare()]);
        assert!(
            block.contains("`**/wrangler.jsonc`") && block.contains("`wrangler`"),
            "{block}"
        );
        let fresh = "# Product\n\nSome text.\n";
        let once = apply_agents_block(fresh, &block);
        assert!(once.starts_with(fresh.trim_end()) && once.contains(BEGIN));
        // Idempotent, and a changed block replaces rather than duplicates.
        assert_eq!(apply_agents_block(&once, &block), once);
        let none = apply_agents_block(&once, &render_agents_block(&[]));
        assert_eq!(none.matches(BEGIN).count(), 1);
        assert!(none.contains("No installed capability needs"), "{none}");
        assert!(none.starts_with(fresh.trim_end()));
    }

    #[test]
    fn the_lock_is_what_says_a_skill_is_installed() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(tracked(tmp.path()).is_empty());
        std::fs::write(
            tmp.path().join(LOCK_FILE),
            r#"{"version":2,"skills":{"wrangler":{"source":"cloudflare/skills"}}}"#,
        )
        .unwrap();
        let lock = tracked(tmp.path());
        assert!(lock.contains("wrangler"));
        // Tracked but the files are gone: not installed.
        assert!(!installed(tmp.path(), "wrangler", &lock));
        let dir = tmp.path().join(SHARED_DIR).join("wrangler");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: wrangler\n---\n").unwrap();
        assert!(installed(tmp.path(), "wrangler", &lock));
    }

    #[cfg(unix)]
    #[test]
    fn the_platforms_own_skill_is_linked_for_agents_with_a_directory_of_their_own() {
        let tmp = tempfile::tempdir().unwrap();
        let canon = tmp.path().join(SHARED_DIR).join("fiducial-x");
        std::fs::create_dir_all(&canon).unwrap();
        std::fs::write(canon.join("SKILL.md"), "---\nname: fiducial-x\n---\nbody\n").unwrap();
        let agents = vec![
            "claude-code".to_string(),
            "codex".to_string(),
            "kiro-cli".to_string(),
        ];
        link_for_agents(tmp.path(), "fiducial-x", &agents).unwrap();
        // Claude Code and Kiro get a link; Codex reads the shared directory and gets none.
        for dir in [".claude/skills", ".kiro/skills"] {
            let at = tmp.path().join(dir).join("fiducial-x");
            assert!(
                at.symlink_metadata().unwrap().file_type().is_symlink(),
                "{dir}"
            );
            assert!(
                std::fs::read_to_string(at.join("SKILL.md"))
                    .unwrap()
                    .contains("body"),
                "{dir}"
            );
        }
        // Re-linking is idempotent.
        link_for_agents(tmp.path(), "fiducial-x", &agents).unwrap();
        assert!(tmp
            .path()
            .join(".claude/skills/fiducial-x/SKILL.md")
            .exists());
    }
}

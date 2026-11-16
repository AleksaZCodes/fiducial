//! The AI skills a product's agent must load, chosen by the capabilities the
//! product has, and held to.
//!
//! # Why a capability *names* a skill instead of carrying one
//!
//! A capability says which technology it uses: shadcn components, Cloudflare
//! Workers. The vendor of that technology publishes the authoritative guidance
//! for it, and keeps it current. Copying that guidance into a capability's own
//! `SKILL.md` would be a second declaration of facts someone else owns, stale
//! the day the vendor ships a change. So a capability declares
//! `[[agent_skills]]` — which skill, from where, and for which files — and the
//! platform does four things with it:
//!
//! 1. **Enables only what is needed.** `.claude/settings.json` lists the plugins
//!    of the capabilities the product has, and no others. Removing a capability
//!    removes its plugin. A product with no Workers gets no Cloudflare skills.
//! 2. **Requires it.** A `PreToolUse` hook ([`gate`]) stops an edit to a file the
//!    skill covers until the session has loaded that skill. The agent is told
//!    which skill and why; it is not left to remember.
//! 3. **Keeps it fresh.** A plugin is updated by Claude Code; a vendored skill
//!    is compared with upstream by `npx skills check`, and CI verifies that every
//!    skill a capability names still exists (`scripts/verify-agent-skills.sh`).
//! 4. **Shows its work.** `.fiducial/agent-skills.json` is the gate's input and
//!    is derived, so a reader sees what is enforced without reading code.
//!
//! # What the gate can and cannot do
//!
//! It stops `Edit`, `Write`, `MultiEdit` and `NotebookEdit` on matching paths. It
//! does not stop an edit made through a shell command (`sed -i`, a redirect): a
//! path-matching rule over arbitrary shell is a guess, and `guard` already treats
//! guessing at shell as the thing it exists not to do. It also only runs in an
//! agent that supports Claude Code hooks; other agents read `AGENTS.md`.
//!
//! "Loaded" is decided from two sources, either of which is enough: a marker
//! written by a `PostToolUse` hook when the `Skill` tool runs, and the session
//! transcript. The marker exists because the transcript is written
//! asynchronously and may lag the call it records.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};

use crate::capability::{self, AgentSkill, AgentSkillKind};

/// The `skills` CLI, pinned. An unpinned `npx` fetch runs whatever was published
/// last; the `no-unpinned-cli-fetch` guard rule exists to stop that.
pub const SKILLS_CLI: &str = "skills@1.9.0";

/// The gate's input, derived.
pub const GATE_FILE: &str = ".fiducial/agent-skills.json";
/// Claude Code's project settings, where plugins are enabled.
pub const SETTINGS_FILE: &str = ".claude/settings.json";
/// Per-session records of skills loaded. Gitignored.
pub const STATE_DIR: &str = ".fiducial/state/skills";

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

/// Every skill any built-in capability can declare: what this platform *could*
/// have put in a product, and so what it may take out again. Anything else in a
/// product's settings is the product's own and is left alone.
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

// ── What the platform writes ─────────────────────────────────────────────────

/// How to get a skill, said to the agent when it is missing.
pub fn install_hint(a: &AgentSkill) -> String {
    match a.kind {
        AgentSkillKind::Plugin => format!(
            "run `fid skills sync` (it enables `{}` in .claude/settings.json), accept the plugin when Claude Code asks, or run `/plugin install {}`",
            a.plugin_key(),
            a.plugin_key()
        ),
        AgentSkillKind::SkillsCli => "run `fid skills sync --install`".to_string(),
    }
}

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
                "install": install_hint(a),
            })
        })
        .collect();
    let doc = json!({
        "//": "Derived by `fid skills sync` from the installed capabilities. Do not edit.",
        "skills": skills,
    });
    format!("{}\n", serde_json::to_string_pretty(&doc).expect("json"))
}

/// The platform's own entry, always present.
fn base_settings() -> Value {
    json!({
        "extraKnownMarketplaces": {
            "fiducial": { "source": { "source": "github", "repo": "AleksaZCodes/fiducial" } }
        },
        "enabledPlugins": { "fiducial@fiducial": true }
    })
}

fn marketplace_entry(a: &AgentSkill) -> Value {
    json!({ "source": { "source": "github", "repo": a.repo } })
}

/// `existing` with the plugins `needed` enabled, and the plugins this platform
/// could have enabled but no longer needs removed. Keys that are not ours are
/// kept exactly.
pub fn merge_settings(existing: Option<Value>, needed: &[AgentSkill]) -> Value {
    let mut doc = match existing {
        Some(Value::Object(m)) => Value::Object(m),
        _ => json!({}),
    };
    let base = base_settings();
    let obj = doc.as_object_mut().expect("object");
    let mut markets = take_map(obj, "extraKnownMarketplaces");
    let mut plugins = take_map(obj, "enabledPlugins");

    // The platform's own marketplace and plugin.
    for (k, v) in base["extraKnownMarketplaces"].as_object().expect("o") {
        markets.entry(k.clone()).or_insert_with(|| v.clone());
    }
    plugins.insert("fiducial@fiducial".into(), json!(true));
    let _ = &base;

    let wanted_keys: BTreeSet<String> = needed
        .iter()
        .filter(|a| a.kind == AgentSkillKind::Plugin)
        .map(AgentSkill::plugin_key)
        .collect();
    let wanted_markets: BTreeSet<&str> = needed
        .iter()
        .filter(|a| a.kind == AgentSkillKind::Plugin)
        .map(|a| a.marketplace.as_str())
        .collect();

    for a in known().iter().filter(|a| a.kind == AgentSkillKind::Plugin) {
        let key = a.plugin_key();
        if wanted_keys.contains(&key) {
            plugins.insert(key, json!(true));
            markets
                .entry(a.marketplace.clone())
                .or_insert_with(|| marketplace_entry(a));
        } else {
            plugins.remove(&key);
            // Only the marketplace entry this platform wrote, and only if no
            // other needed plugin comes from it.
            if !wanted_markets.contains(a.marketplace.as_str())
                && markets.get(&a.marketplace) == Some(&marketplace_entry(a))
            {
                markets.remove(&a.marketplace);
            }
        }
    }
    obj.insert("extraKnownMarketplaces".into(), Value::Object(markets));
    obj.insert("enabledPlugins".into(), Value::Object(plugins));
    doc
}

/// A top-level object key's map, removed from `obj` (empty if absent or not an object).
fn take_map(obj: &mut Map<String, Value>, key: &str) -> Map<String, Value> {
    match obj.remove(key) {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

fn pretty(v: &Value) -> String {
    format!("{}\n", serde_json::to_string_pretty(v).expect("json"))
}

/// What `fid skills sync` did.
#[derive(Debug, Default)]
pub struct Synced {
    /// Files written or changed.
    pub wrote: Vec<String>,
    /// `skills-cli` skills the product still has to install.
    pub missing: Vec<AgentSkill>,
}

fn skills_cli_present(root: &Path, a: &AgentSkill) -> bool {
    root.join(".claude/skills")
        .join(&a.skill)
        .join("SKILL.md")
        .exists()
}

/// Bring `.claude/settings.json` and the gate file in line with `enabled`, and
/// re-record the settings file in the lock (it is platform-owned, so a write that
/// the lock did not hear about would read as drift).
pub fn sync(root: &Path, enabled: &[String], install: bool) -> Result<Synced> {
    let needed = needed(enabled);
    let mut out = Synced::default();

    let settings_path = root.join(SETTINGS_FILE);
    let existing = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    let merged = pretty(&merge_settings(existing, &needed));
    if write_if_changed(&settings_path, &merged)? {
        out.wrote.push(SETTINGS_FILE.into());
        let lock_path = root.join("fiducial.lock");
        if lock_path.exists() {
            let mut lock = crate::lock::Lock::load(&lock_path)?;
            lock.rehash(
                SETTINGS_FILE,
                merged.as_bytes(),
                crate::capability::PLATFORM_VERSION,
            );
            lock.save(&lock_path)?;
        }
    }

    let gate = render_gate(&needed);
    if write_if_changed(&root.join(GATE_FILE), &gate)? {
        out.wrote.push(GATE_FILE.into());
    }

    for a in needed
        .iter()
        .filter(|a| a.kind == AgentSkillKind::SkillsCli)
    {
        if skills_cli_present(root, a) {
            continue;
        }
        if install {
            install_skills_cli(root, a)?;
            if !skills_cli_present(root, a) {
                bail!(
                    "`skills add {}` finished but `.claude/skills/{}/SKILL.md` is not there; \
                     the source's skill may have moved — `scripts/verify-agent-skills.sh` checks this",
                    a.source,
                    a.skill
                );
            }
            out.wrote.push(format!(".claude/skills/{}/", a.skill));
        } else {
            out.missing.push(a.clone());
        }
    }
    Ok(out)
}

fn install_skills_cli(root: &Path, a: &AgentSkill) -> Result<()> {
    let status = std::process::Command::new("npx")
        .args(["--yes", SKILLS_CLI, "add", &a.source, "--skill", &a.skill])
        .args(["--agent", "claude-code", "-y"])
        .current_dir(root)
        .status()
        .context("running `npx skills add` (is Node 22.20 or newer on PATH?)")?;
    if !status.success() {
        bail!(
            "`npx {SKILLS_CLI} add {} --skill {}` failed",
            a.source,
            a.skill
        );
    }
    Ok(())
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
pub fn problems(root: &Path, enabled: &[String]) -> Vec<String> {
    let needed = needed(enabled);
    let mut out = Vec::new();

    let settings: Option<Value> = std::fs::read_to_string(root.join(SETTINGS_FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let want = merge_settings(settings.clone(), &needed);
    if settings.as_ref() != Some(&want) {
        let mut missing: Vec<String> = needed
            .iter()
            .filter(|a| a.kind == AgentSkillKind::Plugin)
            .filter(|a| {
                settings
                    .as_ref()
                    .and_then(|s| s["enabledPlugins"][a.plugin_key()].as_bool())
                    != Some(true)
            })
            .map(AgentSkill::plugin_key)
            .collect();
        let extra: Vec<String> = known()
            .iter()
            .filter(|a| a.kind == AgentSkillKind::Plugin)
            .map(AgentSkill::plugin_key)
            .filter(|k| {
                settings
                    .as_ref()
                    .and_then(|s| s["enabledPlugins"][k.as_str()].as_bool())
                    == Some(true)
                    && !needed.iter().any(|a| &a.plugin_key() == k)
            })
            .collect();
        missing.sort();
        let mut msg = format!("{SETTINGS_FILE} does not match the installed capabilities");
        if !missing.is_empty() {
            msg.push_str(&format!("; not enabled: {}", missing.join(", ")));
        }
        if !extra.is_empty() {
            msg.push_str(&format!(
                "; enabled but no capability needs: {}",
                extra.join(", ")
            ));
        }
        msg.push_str(". `fid skills sync` fixes it.");
        out.push(msg);
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
    for a in needed
        .iter()
        .filter(|a| a.kind == AgentSkillKind::SkillsCli)
    {
        if !skills_cli_present(root, a) {
            out.push(format!(
                "skill `{}` (from {}) is not installed, and its gate will stop edits to {} until it is. `fid skills sync --install`.",
                a.skill,
                a.source,
                a.paths.join(", ")
            ));
        }
    }
    out
}

// ── The gate ─────────────────────────────────────────────────────────────────

/// What a hook is told. Only the fields the gate reads.
#[derive(Debug, Default, serde::Deserialize)]
pub struct HookInput {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub transcript_path: String,
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

/// The name of the skill a `Skill` tool call or a typed slash command loaded,
/// from one transcript line.
fn loaded_in_line(line: &str, out: &mut BTreeSet<String>) {
    let Ok(v) = serde_json::from_str::<Value>(line) else {
        return;
    };
    let content = &v["message"]["content"];
    if let Some(blocks) = content.as_array() {
        for b in blocks {
            if b["type"] == "tool_use" && b["name"] == "Skill" {
                if let Some(name) = b["input"]["skill"].as_str() {
                    out.insert(name.trim_start_matches('/').to_string());
                }
            }
            // A skill typed as `/name` arrives as a user turn that says so.
            if let Some(t) = b["text"].as_str() {
                command_names(t, out);
            }
        }
    } else if let Some(t) = content.as_str() {
        command_names(t, out);
    }
}

fn command_names(text: &str, out: &mut BTreeSet<String>) {
    let mut rest = text;
    while let Some(i) = rest.find("<command-name>") {
        rest = &rest[i + "<command-name>".len()..];
        if let Some(j) = rest.find("</command-name>") {
            out.insert(rest[..j].trim().trim_start_matches('/').to_string());
            rest = &rest[j..];
        }
    }
}

fn marker_name(skill: &str) -> String {
    skill.replace([':', '/'], "__")
}

/// Skills this session has loaded, from the marker and the transcript.
pub fn loaded(root: &Path, input: &HookInput) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if !input.session_id.is_empty() {
        let dir = root.join(STATE_DIR).join(&input.session_id);
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if let Ok(n) = std::fs::read_to_string(e.path()) {
                    out.insert(n.trim().to_string());
                }
            }
        }
    }
    if !input.transcript_path.is_empty() {
        if let Ok(text) = std::fs::read_to_string(&input.transcript_path) {
            for line in text.lines() {
                loaded_in_line(line, &mut out);
            }
        }
    }
    out
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
        let have = have.get_or_insert_with(|| loaded(root, input));
        if names.iter().any(|n| have.contains(*n)) {
            continue;
        }
        blocked.push(format!(
            "`{rel}` is covered by the {} skill. Load one of {} with the Skill tool, then retry. {} If it is not available: {}. (FID_SKILL_GATE=off turns this gate off for a session you have decided to run without it.)",
            entry["id"].as_str().unwrap_or("?"),
            names
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", "),
            entry["why"].as_str().unwrap_or(""),
            entry["install"].as_str().unwrap_or("run `fid skills sync`"),
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

/// `fid skill-mark`: the `PostToolUse` hook on `Skill`. Records that this session
/// loaded the skill, because the transcript can lag the call.
pub fn mark_from_stdin() -> Result<()> {
    let mut raw = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut raw)?;
    let input: HookInput = serde_json::from_str(&raw).unwrap_or_default();
    let (Some(root), Some(skill)) = (
        project_root(&input),
        input.tool_input["skill"].as_str().map(str::to_string),
    ) else {
        return Ok(());
    };
    if input.session_id.is_empty() {
        return Ok(());
    }
    let skill = skill.trim_start_matches('/').to_string();
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
            id: "cloudflare".into(),
            kind: AgentSkillKind::Plugin,
            marketplace: "cloudflare".into(),
            repo: "cloudflare/skills".into(),
            plugin: "cloudflare".into(),
            source: String::new(),
            skill: String::new(),
            skills: vec![
                "cloudflare:wrangler".into(),
                "cloudflare:workers-best-practices".into(),
            ],
            paths: vec!["**/wrangler.jsonc".into(), "**/wrangler.toml".into()],
            why: "Wrangler config changes between releases.".into(),
        }
    }

    fn gate(skills: &[AgentSkill]) -> Value {
        serde_json::from_str(&render_gate(skills)).unwrap()
    }

    fn edit(path: &str) -> HookInput {
        HookInput {
            tool_name: "Edit".into(),
            tool_input: json!({ "file_path": path }),
            ..Default::default()
        }
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
    fn an_edit_to_a_covered_file_is_stopped_until_the_skill_is_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let g = gate(&[cloudflare()]);
        let d = decide(tmp.path(), &g, &edit("apps/web/wrangler.jsonc"));
        let Decision::Deny(why) = d else {
            panic!("an uncovered edit was allowed")
        };
        assert!(
            why.contains("cloudflare:wrangler") && why.contains("Skill tool"),
            "{why}"
        );
        assert!(why.contains("fid skills sync"), "{why}");
        // An unrelated file is not the gate's business.
        assert_eq!(
            decide(tmp.path(), &g, &edit("apps/web/src/page.tsx")),
            Decision::Allow
        );
        // Neither is a read.
        let mut read = edit("apps/web/wrangler.jsonc");
        read.tool_name = "Read".into();
        assert_eq!(decide(tmp.path(), &g, &read), Decision::Allow);
    }

    #[test]
    fn loading_any_one_named_skill_opens_the_gate_by_marker_or_transcript() {
        let tmp = tempfile::tempdir().unwrap();
        let g = gate(&[cloudflare()]);
        let mut input = edit("wrangler.jsonc");
        input.session_id = "s1".into();

        // By marker.
        let dir = tmp.path().join(STATE_DIR).join("s1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(marker_name("cloudflare:workers-best-practices")),
            "cloudflare:workers-best-practices\n",
        )
        .unwrap();
        assert_eq!(decide(tmp.path(), &g, &input), Decision::Allow);
        std::fs::remove_file(dir.join(marker_name("cloudflare:workers-best-practices"))).unwrap();
        assert!(matches!(decide(tmp.path(), &g, &input), Decision::Deny(_)));

        // By transcript, in the exact shape Claude Code writes.
        let t = tmp.path().join("t.jsonl");
        std::fs::write(
            &t,
            "{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\n\
             {\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"id\":\"x\",\"name\":\"Skill\",\"input\":{\"skill\":\"cloudflare:wrangler\"}}]}}\n",
        )
        .unwrap();
        input.transcript_path = t.to_string_lossy().into();
        assert_eq!(decide(tmp.path(), &g, &input), Decision::Allow);

        // A different skill, or a different session's marker, does not count.
        std::fs::write(
            &t,
            "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"Skill\",\"input\":{\"skill\":\"shadcn\"}}]}}\n",
        )
        .unwrap();
        assert!(matches!(decide(tmp.path(), &g, &input), Decision::Deny(_)));

        // A skill the person typed as `/cloudflare:wrangler` counts too.
        std::fs::write(
            &t,
            "{\"type\":\"user\",\"message\":{\"content\":\"<command-name>/cloudflare:wrangler</command-name>\"}}\n",
        )
        .unwrap();
        assert_eq!(decide(tmp.path(), &g, &input), Decision::Allow);
    }

    #[test]
    fn an_absolute_path_is_judged_relative_to_the_product() {
        let tmp = tempfile::tempdir().unwrap();
        let g = gate(&[cloudflare()]);
        let abs = tmp.path().join("apps/api/wrangler.toml");
        assert!(matches!(
            decide(tmp.path(), &g, &edit(&abs.to_string_lossy())),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn settings_enable_what_is_needed_and_drop_what_is_not_but_keep_the_products_own() {
        let theirs = json!({
            "permissions": { "allow": ["Bash(git status)"] },
            "extraKnownMarketplaces": { "mine": { "source": { "source": "github", "repo": "me/mine" } } },
            "enabledPlugins": { "mine@mine": true, "cloudflare@cloudflare": true }
        });
        // Nothing needs Cloudflare: it goes, the product's own plugin stays.
        let off = merge_settings(Some(theirs.clone()), &[]);
        assert!(
            off["enabledPlugins"].get("cloudflare@cloudflare").is_none(),
            "{off}"
        );
        assert_eq!(off["enabledPlugins"]["mine@mine"], true);
        assert_eq!(off["enabledPlugins"]["fiducial@fiducial"], true);
        assert_eq!(off["permissions"]["allow"][0], "Bash(git status)");
        assert!(off["extraKnownMarketplaces"].get("mine").is_some());

        // A capability needs it: it comes back, with its marketplace.
        let on = merge_settings(Some(off), &[cloudflare()]);
        assert_eq!(on["enabledPlugins"]["cloudflare@cloudflare"], true);
        assert_eq!(
            on["extraKnownMarketplaces"]["cloudflare"]["source"]["repo"],
            "cloudflare/skills"
        );

        // Idempotent.
        assert_eq!(merge_settings(Some(on.clone()), &[cloudflare()]), on);
    }

    #[test]
    fn a_marketplace_the_product_customised_is_not_deleted() {
        let theirs = json!({
            "extraKnownMarketplaces": { "cloudflare": { "source": { "source": "github", "repo": "my-fork/skills" } } },
            "enabledPlugins": { "cloudflare@cloudflare": true }
        });
        let out = merge_settings(Some(theirs), &[]);
        assert!(out["enabledPlugins"].get("cloudflare@cloudflare").is_none());
        assert_eq!(
            out["extraKnownMarketplaces"]["cloudflare"]["source"]["repo"],
            "my-fork/skills"
        );
    }
}

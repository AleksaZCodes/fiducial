//! The AI skills a product's capabilities name, through the real binary: what is
//! required, where skills live, what `fid doctor` says, and the gate.
//!
//! The install itself is the `skills` CLI's and needs the network; it is covered
//! by `scripts/verify-agent-skills.sh`. Everything here is offline, and where a
//! test needs a skill to be installed it writes what the CLI writes: the skill's
//! folder under `.agents/skills/` and an entry in `skills-lock.json`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn fid() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fid"))
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(fid())
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .output()
        .expect("running fid")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn product(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    tmp.join("p")
}

fn add(root: &Path, args: &[&str]) {
    let mut a = vec!["add"];
    a.extend_from_slice(args);
    let out = run(root, &a);
    assert!(out.status.success(), "fid {a:?}: {}", text(&out));
}

fn json(root: &Path, rel: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(root.join(rel)).unwrap()).unwrap()
}

fn gate_ids(root: &Path) -> Vec<String> {
    json(root, ".fiducial/agent-skills.json")["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect()
}

/// What `npx skills add` leaves behind for a skill: its folder and its lock entry.
fn install_like_the_cli(root: &Path, names: &[&str]) {
    let mut entries = serde_json::Map::new();
    if let Ok(old) = std::fs::read_to_string(root.join("skills-lock.json")) {
        let v: serde_json::Value = serde_json::from_str(&old).unwrap();
        entries = v["skills"].as_object().cloned().unwrap_or_default();
    }
    for n in names {
        let dir = root.join(".agents/skills").join(n);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {n}\ndescription: x\n---\n"),
        )
        .unwrap();
        entries.insert(
            n.to_string(),
            serde_json::json!({ "source": "test/skills" }),
        );
    }
    std::fs::write(
        root.join("skills-lock.json"),
        serde_json::to_string_pretty(&serde_json::json!({ "version": 2, "skills": entries }))
            .unwrap(),
    )
    .unwrap();
}

/// Run a hook the way Claude Code does: JSON on stdin, project dir in the env.
fn hook(root: &Path, which: &str, input: serde_json::Value) -> Output {
    let mut child = Command::new(fid())
        .arg(which)
        .current_dir(root)
        .env("CLAUDE_PROJECT_DIR", root)
        .env_remove("FID_SKILL_GATE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn edit(root: &Path, session: &str, rel: &str) -> serde_json::Value {
    serde_json::json!({
        "session_id": session, "cwd": root,
        "tool_name": "Edit", "tool_input": { "file_path": root.join(rel) },
    })
}

#[test]
fn a_fresh_product_requires_nothing_and_each_capability_adds_what_it_names() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    assert!(gate_ids(&root).is_empty(), "nothing is required by default");
    // Nothing of any one agent's is enabled: the settings carry only the platform.
    let settings = json(&root, ".claude/settings.json");
    assert_eq!(
        settings["enabledPlugins"].as_object().unwrap().len(),
        1,
        "{settings}"
    );
    let agents_md = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert!(
        agents_md.contains("No installed capability needs a vendor skill"),
        "{agents_md}"
    );

    // Workers: Cloudflare's skills, by the standard CLI's names, and not a component skill.
    add(&root, &["app", "worker"]);
    assert_eq!(gate_ids(&root), ["cloudflare-workers"]);
    let agents_md = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert!(
        agents_md.contains("`wrangler`") && agents_md.contains("`**/wrangler.jsonc`"),
        "every agent that reads AGENTS.md is told: {agents_md}"
    );
    assert!(!agents_md.contains("shadcn"));

    // UI: shadcn too.
    add(&root, &["design"]);
    assert_eq!(gate_ids(&root), ["cloudflare-workers", "shadcn"]);

    // Doctor names exactly what is missing, and nothing else about skills is wrong.
    let doctor = text(&run(&root, &["doctor"]));
    for n in ["wrangler", "workers-best-practices", "shadcn"] {
        assert!(
            doctor.contains(&format!("skill `{n}`")) && doctor.contains("is not installed"),
            "{n}: {doctor}"
        );
    }
    assert!(!doctor.contains("is stale or missing"), "{doctor}");
    assert!(!doctor.contains("skills block in AGENTS.md"), "{doctor}");

    // Once the CLI has installed them, the skills are clean.
    install_like_the_cli(&root, &["wrangler", "workers-best-practices", "shadcn"]);
    let doctor = text(&run(&root, &["doctor"]));
    assert!(
        doctor.contains("agent skills match the installed capabilities"),
        "{doctor}"
    );
    assert!(run(&root, &["skills", "check", "--offline"])
        .status
        .success());
}

#[test]
fn a_capability_that_is_gone_leaves_its_skills_to_be_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    add(&root, &["app", "worker"]);
    install_like_the_cli(&root, &["wrangler", "workers-best-practices"]);
    assert!(run(&root, &["skills", "check", "--offline"])
        .status
        .success());

    // The capability is taken out of the product's declaration.
    let toml = root.join("fiducial.toml");
    let s = std::fs::read_to_string(&toml).unwrap();
    let from = s.find("[capabilities]").unwrap();
    let open = from + s[from..].find("enabled = [").unwrap();
    let close = open + s[open..].find("]\n").unwrap() + 1;
    assert!(
        s[open..close].contains("worker-cloudflare"),
        "setup: the capability was listed"
    );
    std::fs::write(&toml, format!("{}enabled = []{}", &s[..open], &s[close..])).unwrap();

    // Doctor says the gate and the AGENTS.md block are stale, and that the skills are surplus.
    let doctor = text(&run(&root, &["doctor"]));
    assert!(
        doctor.contains("skill `wrangler` is installed but no installed capability needs it"),
        "{doctor}"
    );
    assert!(doctor.contains("is stale or missing"), "{doctor}");
    // Sync (offline) settles the gate and the block; removing the files is the CLI's, with --install.
    let out = run(&root, &["skills", "sync"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(gate_ids(&root).is_empty());
    assert!(std::fs::read_to_string(root.join("AGENTS.md"))
        .unwrap()
        .contains("No installed capability needs"));
    let said = text(&out);
    assert!(
        said.contains("no capability needs:")
            && said.contains("wrangler")
            && said.contains("workers-best-practices"),
        "{said}"
    );
}

#[test]
fn skills_the_product_installed_itself_are_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    // A skill this platform has never heard of, tracked in the lock.
    install_like_the_cli(&root, &["my-own-skill"]);
    let doctor = text(&run(&root, &["doctor"]));
    assert!(!doctor.contains("my-own-skill"), "{doctor}");
    let out = run(&root, &["skills", "sync"]);
    assert!(!text(&out).contains("my-own-skill"), "{}", text(&out));
}

#[test]
fn the_gate_stops_an_edit_to_a_covered_file_until_a_named_skill_is_loaded() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    add(&root, &["app", "worker"]);

    // Blocked: exit 2, the reason on stderr names the skills, the why and the way to get them.
    let out = hook(
        &root,
        "skill-gate",
        edit(&root, "s1", "apps/worker/wrangler.jsonc"),
    );
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let why = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        why.contains("`wrangler`") && why.contains("fid skills sync --install"),
        "{why}"
    );

    // A file the skills do not cover is not the gate's business.
    let out = hook(
        &root,
        "skill-gate",
        edit(&root, "s1", "apps/worker/src/index.ts"),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));

    // Loading one through the Skill tool (the PostToolUse hook records it) opens it, for that session only.
    let loaded = serde_json::json!({
        "session_id": "s1", "cwd": root, "tool_name": "Skill", "tool_input": { "skill": "wrangler" },
    });
    assert!(hook(&root, "skill-mark", loaded).status.success());
    assert_eq!(
        hook(
            &root,
            "skill-gate",
            edit(&root, "s1", "apps/worker/wrangler.jsonc")
        )
        .status
        .code(),
        Some(0)
    );
    assert_eq!(
        hook(
            &root,
            "skill-gate",
            edit(&root, "s2", "apps/worker/wrangler.jsonc")
        )
        .status
        .code(),
        Some(2)
    );

    // So does reading the skill's file, which is how an agent without a Skill tool loads one.
    let read = serde_json::json!({
        "session_id": "s3", "cwd": root, "tool_name": "Read",
        "tool_input": { "file_path": root.join(".agents/skills/workers-best-practices/SKILL.md") },
    });
    assert!(hook(&root, "skill-mark", read).status.success());
    assert_eq!(
        hook(
            &root,
            "skill-gate",
            edit(&root, "s3", "apps/worker/wrangler.jsonc")
        )
        .status
        .code(),
        Some(0)
    );

    // The record is local state, never committed.
    assert!(std::fs::read_to_string(root.join(".gitignore"))
        .unwrap()
        .contains(".fiducial/state/"));
}

#[test]
fn a_product_with_no_gate_is_never_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let out = hook(&root, "skill-gate", edit(&root, "s1", "wrangler.jsonc"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let bare = tmp.path().join("bare");
    std::fs::create_dir_all(&bare).unwrap();
    let out = hook(&bare, "skill-gate", edit(&bare, "s1", "wrangler.jsonc"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
}

#[cfg(unix)]
#[test]
fn capability_instructions_are_an_agent_skill_in_the_shared_directory_linked_per_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    add(&root, &["design"]);

    // One copy, in the directory Codex, Cursor, Copilot, Gemini CLI and OpenCode read.
    let canonical = root.join(".agents/skills/fiducial-design/SKILL.md");
    let skill = std::fs::read_to_string(&canonical).unwrap();
    assert!(
        skill.starts_with("---\nname: fiducial-design\ndescription: \""),
        "{skill}"
    );
    assert!(
        skill.contains("Use when working on this product's design capability"),
        "{skill}"
    );
    // Claude Code reads its own directory, which links to it.
    let link = root.join(".claude/skills/fiducial-design");
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(
        std::fs::read_to_string(link.join("SKILL.md")).unwrap(),
        skill
    );

    // The older layouts are gone, and a file someone wrote is left.
    assert!(!root.join(".fiducial/skills/design.md").exists());
    let legacy = root.join(".claude/skills/design.md");
    std::fs::write(
        &legacy,
        "---\nname: fiducial-design\ndescription: old\n---\n",
    )
    .unwrap();
    assert!(run(&root, &["upgrade"]).status.success());
    assert!(
        !legacy.exists(),
        "the platform's own old pointer is cleaned up"
    );
    let theirs = root.join(".claude/skills/i18n.md");
    std::fs::write(&theirs, "my own notes\n").unwrap();
    add(&root, &["i18n"]);
    assert!(theirs.exists(), "a file someone wrote is theirs");
}

#[cfg(unix)]
#[test]
fn the_agents_a_product_names_decide_where_skills_are_linked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    add(&root, &["design"]);
    assert!(
        root.join(".claude/skills/fiducial-design").exists(),
        "the default includes Claude Code"
    );

    // A team on Codex and Kiro only: the shared directory serves Codex, Kiro gets a link.
    let toml = root.join("fiducial.toml");
    let s = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(
        &toml,
        format!("{s}\n[skills]\nagents = [\"codex\", \"kiro-cli\"]\n"),
    )
    .unwrap();
    let out = run(&root, &["skills", "sync"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(root.join(".kiro/skills/fiducial-design/SKILL.md").exists());
    assert!(root
        .join(".agents/skills/fiducial-design/SKILL.md")
        .exists());

    // A name the `skills` CLI does not know is refused by doctor, not silently dropped.
    let s = std::fs::read_to_string(&toml)
        .unwrap()
        .replace("\"kiro-cli\"", "\"kiro-clii\"");
    std::fs::write(&toml, s).unwrap();
    let doctor = text(&run(&root, &["doctor"]));
    assert!(
        doctor.contains("`kiro-clii`, which the `skills` CLI does not know"),
        "{doctor}"
    );
}

//! The AI skills a product's capabilities name, through the real binary: which
//! plugins are enabled, what the gate enforces, and what `fid doctor` says.
//!
//! The `skills` CLI install and the upstream freshness check need the network and
//! are covered by `scripts/verify-agent-skills.sh`. Everything here is offline.

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
        "session_id": session,
        "cwd": root,
        "tool_name": "Edit",
        "tool_input": { "file_path": root.join(rel) },
    })
}

fn plugins(root: &Path) -> Vec<String> {
    let mut v: Vec<String> = json(root, ".claude/settings.json")["enabledPlugins"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    v.sort();
    v
}

#[test]
fn a_fresh_product_enables_only_the_platform_and_a_capability_adds_what_it_names() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    assert_eq!(
        plugins(&root),
        ["fiducial@fiducial"],
        "nothing else by default"
    );
    assert_eq!(
        json(&root, ".fiducial/agent-skills.json")["skills"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Workers: the Cloudflare plugin, and not a component skill.
    add(&root, &["app", "worker"]);
    assert_eq!(
        plugins(&root),
        ["cloudflare@cloudflare", "fiducial@fiducial"]
    );
    let gate = json(&root, ".fiducial/agent-skills.json");
    let ids: Vec<&str> = gate["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["cloudflare"], "{gate}");
    let market = &json(&root, ".claude/settings.json")["extraKnownMarketplaces"]["cloudflare"];
    assert_eq!(market["source"]["repo"], "cloudflare/skills");

    // UI: shadcn is required too, and is vendored, so doctor says it is missing.
    add(&root, &["design"]);
    let ids: Vec<String> = json(&root, ".fiducial/agent-skills.json")["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["cloudflare", "shadcn"]);
    let doctor = text(&run(&root, &["doctor"]));
    assert!(
        doctor.contains("skill `shadcn` (from shadcn/ui) is not installed"),
        "{doctor}"
    );
    // …and it is the only thing wrong with the skills: settings and gate are current.
    assert!(
        !doctor.contains("does not match the installed capabilities"),
        "{doctor}"
    );
    assert!(!doctor.contains("is stale or missing"), "{doctor}");
    // The platform's own file, rewritten by sync, is not reported as drift.
    assert!(
        doctor.contains("no platform-owned file has been modified"),
        "{doctor}"
    );
}

#[test]
fn a_capability_that_is_gone_takes_its_plugin_with_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    add(&root, &["app", "worker"]);
    assert!(plugins(&root).contains(&"cloudflare@cloudflare".to_string()));

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
    let no_worker = format!("{}enabled = []{}", &s[..open], &s[close..]);
    std::fs::write(&toml, no_worker).unwrap();

    // Doctor says the plugin is no longer called for; sync takes it out.
    let doctor = text(&run(&root, &["doctor"]));
    assert!(
        doctor.contains("enabled but no capability needs: cloudflare@cloudflare"),
        "{doctor}"
    );
    assert!(run(&root, &["skills", "sync"]).status.success());
    assert_eq!(plugins(&root), ["fiducial@fiducial"]);
    let settled = text(&run(&root, &["doctor"]));
    assert!(
        settled.contains("agent skills match the installed capabilities"),
        "{settled}"
    );
    assert!(
        !settled.contains("enabled but no capability needs"),
        "{settled}"
    );
}

#[test]
fn the_gate_stops_an_edit_to_a_covered_file_until_the_skill_is_loaded() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    add(&root, &["app", "worker"]);

    // Blocked: exit 2, and the reason, on stderr, names the skill and the why.
    let out = hook(
        &root,
        "skill-gate",
        edit(&root, "s1", "apps/worker/wrangler.jsonc"),
    );
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let why = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        why.contains("cloudflare:wrangler") && why.contains("Skill tool"),
        "{why}"
    );

    // A file the skill does not cover is not the gate's business.
    let out = hook(
        &root,
        "skill-gate",
        edit(&root, "s1", "apps/worker/src/index.ts"),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));

    // Loading the skill (the PostToolUse hook records it) opens it, for that session only.
    let mark = serde_json::json!({
        "session_id": "s1", "cwd": root, "tool_name": "Skill",
        "tool_input": { "skill": "cloudflare:wrangler" },
    });
    assert!(hook(&root, "skill-mark", mark).status.success());
    let out = hook(
        &root,
        "skill-gate",
        edit(&root, "s1", "apps/worker/wrangler.jsonc"),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let out = hook(
        &root,
        "skill-gate",
        edit(&root, "s2", "apps/worker/wrangler.jsonc"),
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "another session is not let through"
    );

    // The record is the product's local state, never committed.
    let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(ignore.contains(".fiducial/state/"), "{ignore}");
}

#[test]
fn a_product_with_no_gate_is_never_blocked() {
    // Outside a product, or in one whose capabilities name no skill.
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let out = hook(&root, "skill-gate", edit(&root, "s1", "wrangler.jsonc"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let bare = tmp.path().join("bare");
    std::fs::create_dir_all(&bare).unwrap();
    let out = hook(&bare, "skill-gate", edit(&bare, "s1", "wrangler.jsonc"));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
}

#[test]
fn capability_instructions_land_where_claude_code_discovers_skills() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    add(&root, &["design"]);
    // A skill is a folder with a SKILL.md; a flat `.md` is not a place it looks.
    let skill =
        std::fs::read_to_string(root.join(".claude/skills/fiducial-design/SKILL.md")).unwrap();
    assert!(
        skill.starts_with("---\nname: fiducial-design\ndescription: \""),
        "{skill}"
    );
    assert!(
        skill.contains("Use when working on this product's design capability"),
        "{skill}"
    );
    assert!(skill.contains(".fiducial/skills/design.md"), "{skill}");
    assert!(
        !root.join(".claude/skills/design.md").exists(),
        "the flat pointer is gone"
    );

    // An older product's flat pointer is removed on the next write, and only if it is the pointer.
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

//! End-to-end tests for `fid dash` — the workbench view.
//!
//! Drives the real binary through `fid new` → `add eda` → `derive` → `dash`,
//! the path a user takes. Asserts against the **JSON** output rather than the
//! rendered text: the text is a presentation of those facts, and a test that
//! greps prose breaks when a label is reworded while saying nothing about
//! whether the number was right.
//!
//! The one property every test here defends is that dash is a *view*: it reads
//! the repository and reports it, holds no state, and never writes.

use std::{path::Path, process::Command};

fn fid_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_fid"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(fid_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to invoke fid")
}

fn scaffold(tmp: &Path) -> std::path::PathBuf {
    let out = run(tmp, &["new", "demo"]);
    assert!(out.status.success(), "fid new failed: {out:?}");
    tmp.join("demo")
}

/// `fid dash --json`, parsed.
fn dash(root: &Path) -> serde_json::Value {
    let out = run(root, &["dash", "--json"]);
    assert!(
        out.status.success(),
        "fid dash failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("dash --json must emit valid JSON")
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

// ── The view renders at all ───────────────────────────────────────────────────

#[test]
fn dash_renders_every_section_for_a_bare_product() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let d = dash(&root);

    for section in [
        "product",
        "git",
        "roadmap",
        "decisions",
        "ci",
        "graph",
        "freshness",
    ] {
        assert!(d.get(section).is_some(), "missing section: {section}");
    }
    assert_eq!(d["product"]["name"], "demo");
    assert_eq!(d["product"]["version"], "0.1.0");
    assert_eq!(d["product"]["spine_enabled"], false);
}

#[test]
fn dash_writes_nothing_and_leaves_the_repo_identical() {
    // The defining property: the repository is the database, dash is a view. A
    // view that writes has a store, and a store has to be invalidated.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());
    assert!(run(&root, &["derive"]).status.success());

    let before = snapshot(&root);
    assert!(run(&root, &["dash"]).status.success());
    assert!(run(&root, &["dash", "--json"]).status.success());
    assert!(run(&root, &["dash", "--section", "freshness"])
        .status
        .success());
    let after = snapshot(&root);

    assert_eq!(before, after, "dash must not modify the repository");
}

/// Every file under `root` (excluding `.git`) with its content hash.
fn snapshot(root: &Path) -> Vec<(String, u64)> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, u64)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if p.is_dir() {
                walk(&p, base, out);
            } else if let Ok(bytes) = std::fs::read(&p) {
                use std::hash::{DefaultHasher, Hash, Hasher};
                let mut h = DefaultHasher::new();
                bytes.hash(&mut h);
                let rel = p.strip_prefix(base).unwrap().display().to_string();
                out.push((rel, h.finish()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

// ── Absence is reported, not fatal ────────────────────────────────────────────

#[test]
fn a_product_with_no_roadmap_or_decisions_still_renders() {
    // Refusing to render because a young product has no roadmap would make the
    // view useless exactly when it is most wanted.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let d = dash(&root);

    assert!(d["roadmap"]["source"].is_null(), "no roadmap file yet");
    assert_eq!(d["roadmap"]["done"], 0);
    assert!(d["decisions"]["source"].is_null(), "no decisions yet");
    assert_eq!(d["decisions"]["count"], 0);
    assert_eq!(d["graph"]["pipelines"].as_array().unwrap().len(), 0);

    // And the text view says so rather than printing an empty heading.
    let out = run(&root, &["dash", "--section", "roadmap"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("no roadmap found"), "got: {text}");
}

// ── Roadmap ───────────────────────────────────────────────────────────────────

#[test]
fn roadmap_progress_is_counted_from_status_markers() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        "ROADMAP.md",
        "# Roadmap\n\
         | 1 | Board bring-up | ✅ |\n\
         | 2 | Enclosure | ✅ |\n\
         | 3 | Firmware uplink | 🟡 |\n\
         | 4 | Field trial | ⬜ |\n\
         - [x] pick a connector\n\
         - [ ] order a panel\n\
         \n\
         This paragraph has no marker and must not be counted.\n",
    );
    let d = dash(&root);
    assert_eq!(d["roadmap"]["source"], "ROADMAP.md");
    assert_eq!(d["roadmap"]["done"], 3, "two ✅ plus one [x]");
    assert_eq!(d["roadmap"]["in_progress"], 1);
    assert_eq!(d["roadmap"]["todo"], 2, "one ⬜ plus one [ ]");

    // The in-progress rows are what a dashboard is opened to see.
    let active = d["roadmap"]["active"].as_array().unwrap();
    assert_eq!(active.len(), 1);
    assert!(
        active[0].as_str().unwrap().contains("Firmware uplink"),
        "active row should name the work: {active:?}"
    );
}

#[test]
fn prose_without_markers_is_not_roadmap_progress() {
    // A roadmap that is mostly narrative should report zero tracked items, not
    // invent progress from sentences.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        "ROADMAP.md",
        "# Roadmap\n\nWe intend to ship the enclosure and then the firmware.\n",
    );
    let d = dash(&root);
    assert_eq!(d["roadmap"]["source"], "ROADMAP.md");
    assert_eq!(d["roadmap"]["done"], 0);
    assert_eq!(d["roadmap"]["in_progress"], 0);
    assert_eq!(d["roadmap"]["todo"], 0);
}

#[test]
fn phases_md_is_accepted_as_a_roadmap() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(&root, "PHASES.md", "# Phases\n| 1 | thing | ✅ |\n");
    assert_eq!(dash(&root)["roadmap"]["source"], "PHASES.md");
}

// ── Decisions ─────────────────────────────────────────────────────────────────

#[test]
fn decisions_are_listed_newest_first_with_their_titles() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        "docs/specs/2026-01-15-outline.md",
        "# Board outline frozen at 100 x 60 mm\n",
    );
    write(
        &root,
        "docs/specs/2026-04-02-gasket.md",
        "# Gasket material: TPU 95A\n",
    );
    write(&root, "docs/specs/notes.txt", "not a decision");

    let d = dash(&root);
    assert_eq!(d["decisions"]["source"], "docs/specs");
    assert_eq!(d["decisions"]["count"], 2, ".txt must not be counted");

    let recent = d["decisions"]["recent"].as_array().unwrap();
    // Filenames are date-prefixed by the append-only rule, so name order is
    // chronological order.
    assert_eq!(recent[0]["date"], "2026-04-02");
    assert_eq!(recent[0]["title"], "Gasket material: TPU 95A");
    assert_eq!(recent[1]["date"], "2026-01-15");
}

#[test]
fn a_decision_without_a_dated_filename_still_appears() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(&root, "docs/specs/undated.md", "# Something we decided\n");
    let d = dash(&root);
    assert_eq!(d["decisions"]["count"], 1);
    assert!(d["decisions"]["recent"][0]["date"].is_null());
    assert_eq!(d["decisions"]["recent"][0]["title"], "Something we decided");
}

// ── CI ────────────────────────────────────────────────────────────────────────

#[test]
fn ci_reports_declared_workflows_and_whether_any_guards_freshness() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    // The scaffold ships a review workflow that does not check freshness, so a
    // stale artifact would reach main. Dash should say so — that is the finding.
    let d = dash(&root);
    let workflows = d["ci"]["workflows"].as_array().unwrap();
    assert!(!workflows.is_empty(), "scaffold ships a workflow");
    assert!(
        workflows.iter().all(|w| w["checks_freshness"] == false),
        "scaffold has no freshness guard"
    );
    let ci_out = run(&root, &["dash", "--section", "ci"]);
    let text = String::from_utf8_lossy(&ci_out.stdout);
    assert!(
        text.contains("no workflow runs `fid derive --check`"),
        "the gap should be called out: {text}"
    );

    // Adding one flips it.
    write(
        &root,
        ".github/workflows/derive.yml",
        "name: Derive\non:\n  push:\n    branches: [main]\njobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n      - run: fid derive --check\n",
    );
    let d = dash(&root);
    let w: Vec<_> = d["ci"]["workflows"].as_array().unwrap().to_vec();
    let derive = w.iter().find(|w| w["name"] == "Derive").expect("Derive");
    assert_eq!(derive["checks_freshness"], true);
    assert!(
        derive["triggers"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("push")),
        "triggers: {:?}",
        derive["triggers"]
    );
}

#[test]
fn dash_makes_no_network_calls_for_ci() {
    // CI is read from the workflow files, so the view renders identically with
    // no network and no credentials. A dashboard that needs a token to draw is
    // one that stops working when you most want it.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let mut cmd = Command::new(fid_bin());
    cmd.args(["dash", "--json"])
        .current_dir(&root)
        // Point every proxy at a black hole: any HTTP attempt would fail.
        .env("http_proxy", "http://127.0.0.1:1")
        .env("https_proxy", "http://127.0.0.1:1")
        .env("ALL_PROXY", "http://127.0.0.1:1")
        .env("GH_TOKEN", "")
        .env("GITHUB_TOKEN", "");
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "dash must render without network: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let d: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(d["ci"]["workflows"].is_array());
}

// ── Graph ─────────────────────────────────────────────────────────────────────

#[test]
fn the_graph_section_matches_fid_graph() {
    // Both read the same declaration through the same module, so they cannot
    // disagree — which they did when each parsed pipelines/ for itself.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());

    let d = dash(&root);
    let dash_names: Vec<String> = d["graph"]["pipelines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();

    let graph: serde_json::Value =
        serde_json::from_slice(&run(&root, &["graph", "--format", "json"]).stdout).unwrap();
    let graph_names: Vec<String> = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["kind"] == "pipeline")
        .map(|n| n["label"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(dash_names, graph_names);
    assert!(dash_names.contains(&"enclosure".to_string()));
}

#[test]
fn a_malformed_pipeline_fails_dash_the_way_it_fails_derive() {
    // The divergence the shared module removed: `graph` used to label an
    // unparseable pipeline "unknown" while `derive` errored. A pipeline that
    // cannot be run must not be reported as if it could.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(&root, "pipelines/broken.toml", "name = \"x\"\n");

    for cmd in [
        vec!["dash", "--json"],
        vec!["graph", "--format", "json"],
        vec!["derive"],
    ] {
        let out = run(&root, &cmd);
        assert!(
            !out.status.success(),
            "{cmd:?} should fail on a malformed pipeline"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("broken.toml"),
            "{cmd:?} should name the file: {stderr}"
        );
    }
}

// ── Freshness ─────────────────────────────────────────────────────────────────

#[test]
fn freshness_recomputes_hashes_rather_than_trusting_the_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());
    assert!(run(&root, &["derive"]).status.success());

    let d = dash(&root);
    let f = &d["freshness"];
    assert!(f["artifacts_fresh"].as_u64().unwrap() >= 5);
    assert_eq!(f["artifacts_stale"], 0);
    assert_eq!(f["problems"].as_array().unwrap().len(), 0);

    // Tamper with one artifact: dash must notice, because it hashes the file.
    let gasket = root.join("enclosure/gasket.stl");
    let mut bytes = std::fs::read(&gasket).unwrap();
    bytes.extend_from_slice(b"tampered");
    std::fs::write(&gasket, bytes).unwrap();

    let d = dash(&root);
    assert_eq!(d["freshness"]["artifacts_stale"], 1);
    let problems = d["freshness"]["problems"].as_array().unwrap();
    assert!(
        problems[0].as_str().unwrap().contains("gasket.stl"),
        "should name the file: {problems:?}"
    );

    // Deleting it is a different finding from changing it.
    std::fs::remove_file(&gasket).unwrap();
    let d = dash(&root);
    assert_eq!(d["freshness"]["artifacts_stale"], 0);
    assert_eq!(d["freshness"]["artifacts_missing"], 1);
}

#[test]
fn an_output_never_derived_is_distinguished_from_a_stale_one() {
    // Visible only by comparing declared outputs against the lock, which is why
    // freshness reads the pipelines and not just the lock.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());

    let d = dash(&root);
    assert_eq!(d["freshness"]["artifacts_fresh"], 0);
    assert!(
        d["freshness"]["artifacts_untracked"].as_u64().unwrap() >= 5,
        "nothing has been derived yet: {}",
        d["freshness"]
    );
    let problems = d["freshness"]["problems"].as_array().unwrap();
    assert!(problems
        .iter()
        .any(|p| p.as_str().unwrap().contains("never derived")));
}

#[test]
fn a_hand_edited_template_is_reported_as_drift() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert_eq!(dash(&root)["freshness"]["templates_modified"], 0);

    let mission = root.join("MISSION.md");
    let mut text = std::fs::read_to_string(&mission).unwrap();
    text.push_str("\nEdited by hand.\n");
    std::fs::write(&mission, text).unwrap();

    assert_eq!(
        dash(&root)["freshness"]["templates_modified"],
        1,
        "a hand-edited template is drift `fid upgrade` will have to merge around"
    );
}

// ── Git ───────────────────────────────────────────────────────────────────────

#[test]
fn git_reports_branch_head_and_working_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let d = dash(&root);
    // `fid new` runs `git init` but makes no commit, so HEAD is unborn: a real
    // repository with no branch to resolve. Reporting that as "not a git
    // repository" is what `is_repo` exists to prevent.
    assert_eq!(d["git"]["is_repo"], true, "fid new runs git init");
    assert!(d["git"]["commit"].is_null(), "no commit yet");
    let out = run(&root, &["dash", "--section", "git"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("no commits yet") && !text.contains("not a git repository"),
        "an unborn HEAD is still a repository: {text}"
    );

    // No upstream on a fresh scaffold, and that is normal rather than an error.
    assert!(d["git"]["ahead"].is_null());
    assert!(d["git"]["behind"].is_null());

    write(&root, "notes.md", "hello");
    assert!(dash(&root)["git"]["dirty_files"].as_u64().unwrap() >= 1);
}

#[test]
fn a_product_outside_git_still_renders() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    std::fs::remove_dir_all(root.join(".git")).unwrap();

    let d = dash(&root);
    assert_eq!(d["git"]["is_repo"], false);
    assert!(d["git"]["branch"].is_null());
    assert_eq!(d["git"]["dirty_files"], 0);
    // The rest of the view is unaffected.
    assert_eq!(d["product"]["name"], "demo");
}

// ── Interface ─────────────────────────────────────────────────────────────────

#[test]
fn each_section_can_be_rendered_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    for section in [
        "product",
        "git",
        "roadmap",
        "decisions",
        "ci",
        "graph",
        "freshness",
    ] {
        let out = run(&root, &["dash", "--section", section]);
        assert!(out.status.success(), "--section {section} failed");
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.trim().is_empty(),
            "--section {section} printed nothing"
        );
    }
}

#[test]
fn an_unknown_section_lists_the_valid_ones() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = run(&root, &["dash", "--section", "sprockets"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("freshness") && stderr.contains("roadmap"),
        "got: {stderr}"
    );
}

#[test]
fn dash_reports_problems_without_failing() {
    // Dash is a view, so it exits 0 even when what it shows is bad. Failing on
    // findings is `fid derive --check` and `fid doctor`; a dashboard that exits
    // non-zero cannot be run casually.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());

    let out = run(&root, &["dash"]);
    assert!(out.status.success(), "dash must exit 0 despite findings");
    let d = dash(&root);
    assert!(
        !d["freshness"]["problems"].as_array().unwrap().is_empty(),
        "there should be findings to report"
    );
    // …whereas derive --check is the command that fails on them.
    assert!(!run(&root, &["derive", "--check"]).status.success());
}

#[test]
fn dash_outside_a_product_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["dash"]);
    assert!(!out.status.success(), "there is no product here");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("fiducial.toml"),
        "should name what is missing: {stderr}"
    );
}

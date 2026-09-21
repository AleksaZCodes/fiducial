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

/// A product with the full set of foundations.
///
/// `--full` rather than the default, because these tests are about `fid dash`
/// — a view over roadmap, decisions, pipelines and freshness — and want a
/// product that actually has those things to render. The minimal scaffold is
/// the subject of `new.rs`'s own tests, not of this file.
fn scaffold(tmp: &Path) -> std::path::PathBuf {
    let out = run(tmp, &["new", "demo", "--full"]);
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
    // A scaffold now ships a ROADMAP.md, so remove it to get the empty case
    // this test is actually about.
    std::fs::remove_file(root.join("ROADMAP.md")).unwrap();
    let d = dash(&root);

    assert!(d["roadmap"]["source"].is_null(), "no roadmap file yet");
    assert_eq!(d["roadmap"]["done"], 0);
    assert!(d["decisions"]["source"].is_null(), "no decisions yet");
    assert_eq!(d["decisions"]["count"], 0);
    // A scaffold ships the i18n, design and thesis pipelines — `fid new`
    // creates a product that is already localized, already has a design system,
    // and already has somewhere to state what it claims. So "no pipelines" is
    // not a state a new product passes through. This test is about an absent
    // roadmap, not about that.
    assert_eq!(d["graph"]["pipelines"].as_array().unwrap().len(), 3);

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
    // Only when there is no ROADMAP.md — see the precedence test below.
    std::fs::remove_file(root.join("ROADMAP.md")).unwrap();
    write(&root, "SHIPPED.md", "# Phases\n| 1 | thing | ✅ |\n");
    assert_eq!(dash(&root)["roadmap"]["source"], "SHIPPED.md");
}

/// `ROADMAP.md` wins over `SHIPPED.md` when a repository keeps both.
///
/// They hold different things — the roadmap is what is *intended*, PHASES is
/// what was *built* — and a dashboard is forward-looking, so the roadmap is the
/// right one to read. Exactly one file is read, so the two can never be counted
/// together.
#[test]
fn roadmap_wins_over_phases_when_both_exist() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(&root, "ROADMAP.md", "# Roadmap\n| next thing | ⬜ |\n");
    write(
        &root,
        "SHIPPED.md",
        "# Phases\n| 1 | built | ✅ |\n| 2 | built | ✅ |\n",
    );

    let d = dash(&root);
    assert_eq!(d["roadmap"]["source"], "ROADMAP.md");
    // If both were read, done would be 2 rather than 0.
    assert_eq!(d["roadmap"]["done"], 0, "only one file is counted");
    assert_eq!(d["roadmap"]["todo"], 1);
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

    // The scaffold ships a CI workflow that runs `fid derive --check`, so a
    // fresh product is guarded from its first commit.
    //
    // This assertion used to read the other way — it asserted that a scaffold
    // had NO freshness guard, and that dash reported the gap. That was an
    // accurate description of a defect: `fid new` generated a product in
    // precisely the state its own dashboard flags as unsafe, and every product
    // ever scaffolded inherited it. Phase 18 scaffolds the workflow instead, so
    // the test now asserts the fixed behaviour.
    let d = dash(&root);
    let workflows = d["ci"]["workflows"].as_array().unwrap();
    assert!(!workflows.is_empty(), "scaffold ships a workflow");
    assert!(
        workflows.iter().any(|w| w["checks_freshness"] == true),
        "scaffold ships a freshness guard: {workflows:?}"
    );
    let ci_out = run(&root, &["dash", "--section", "ci"]);
    let text = String::from_utf8_lossy(&ci_out.stdout);
    assert!(
        !text.contains("no workflow runs `fid derive --check`"),
        "a scaffolded product is guarded, so the gap must not be reported: {text}"
    );
    assert!(
        text.contains("checks artifact freshness"),
        "the guard should be visible in the CI section: {text}"
    );

    // The negative case still has to work, and it needs its own fixture now: a
    // scaffold ships exactly one workflow and that one does gate. If
    // `checks_freshness` were hardcoded true the dashboard would be useless, so
    // a workflow that does not run the gate has to be reported as not running
    // it.
    //
    // This used to lean on the scaffolded Claude review workflow, which no
    // longer exists — an incidental fixture for a property that is not about
    // reviews at all.
    std::fs::write(
        root.join(".github/workflows/nightly.yml"),
        "name: Nightly\n\non:\n  schedule:\n    - cron: '0 3 * * *'\n\njobs:\n  \
         build:\n    name: build\n    runs-on: ubuntu-latest\n    steps:\n      \
         - run: echo no gate here\n",
    )
    .unwrap();

    let d = dash(&root);
    let workflows = d["ci"]["workflows"].as_array().unwrap();
    assert!(
        workflows
            .iter()
            .any(|w| w["name"] == "Nightly" && w["checks_freshness"] == false),
        "a workflow without the gate must be reported as not having it: {workflows:?}"
    );

    // Freshness is detected per workflow, not inferred once for the repository.
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
    // One: the scaffold's own generated messages, which `fid new` derives so a
    // product does not fail the `fid derive --check` in its own CI on the
    // first commit. Every `eda` output is still untracked, which is the
    // distinction under test.
    assert_eq!(d["freshness"]["artifacts_fresh"], 1);
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

// ── Regressions ───────────────────────────────────────────────────────────────
//
// Each of these reproduces something dash got wrong. Two were crashes; three
// were worse — it reported a confident number that was false. A view an agent
// reads has to be right or say nothing, so they are pinned here.

#[test]
fn a_non_ascii_decision_filename_does_not_crash_the_dashboard() {
    // `&name[..10]` panics when byte 10 lands inside a multi-byte character,
    // so one decision file with a non-English name took down every section.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(&root, "docs/specs/启动决定-2026.md", "# 启动决定\n");
    write(&root, "docs/specs/2026-01-02-ok.md", "# Dated and fine\n");

    let d = dash(&root);
    assert_eq!(d["decisions"]["count"], 2);
    let recent = d["decisions"]["recent"].as_array().unwrap();
    // The undated one still appears, with no date rather than a crash.
    assert!(recent.iter().any(|r| r["date"].is_null()));
    assert!(recent.iter().any(|r| r["date"] == "2026-01-02"));
}

#[test]
fn a_long_roadmap_row_with_multibyte_text_does_not_crash() {
    // `String::truncate(93)` panics mid-character. An em-dash in a long row was
    // enough — and this project writes em-dashes everywhere.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let long = "x".repeat(91);
    write(
        &root,
        "ROADMAP.md",
        &format!("# Roadmap\n| 2 | {long}—ship it, eventually, after the review | 🟡 |\n"),
    );

    let d = dash(&root);
    assert_eq!(d["roadmap"]["in_progress"], 1);
    let active = d["roadmap"]["active"].as_array().unwrap();
    assert!(
        active[0].as_str().unwrap().ends_with("..."),
        "should truncate"
    );
}

#[test]
fn a_legend_line_is_not_counted_as_roadmap_progress() {
    // "Legend: ✅ done · 🟡 in progress · ⬜ not started" is a normal thing to
    // write, and it used to score as one completed item.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        "ROADMAP.md",
        "# Roadmap\n\n\
         Legend: ✅ done · 🟡 in progress · ⬜ not started\n\n\
         | 1 | Real item | ✅ |\n",
    );

    let d = dash(&root);
    assert_eq!(d["roadmap"]["done"], 1, "the legend is not an item");
    assert_eq!(d["roadmap"]["in_progress"], 0);
    assert_eq!(d["roadmap"]["todo"], 0);
}

/// A blockquote is commentary about the roadmap, not an item in it.
///
/// The legend guard above only catches a line carrying *several* markers.
/// Fiducial's own ROADMAP.md opens with "Every ⬜ item below now carries a
/// …" — one marker, so it counted as a to-do, and it surfaced as `next` the
/// moment no item was in progress to outrank it. `fid dash --section roadmap`
/// is what AGENTS.md sends every agent to for what comes next, so it answered
/// that question with a sentence about the file's format.
#[test]
fn a_blockquote_about_the_roadmap_is_not_an_item_in_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        "ROADMAP.md",
        "# Roadmap\n\n\
         > Every ⬜ item below carries a \"Where it lands\" paragraph.\n\n\
         ### Real item ⬜\n",
    );

    let d = dash(&root);
    assert_eq!(d["roadmap"]["todo"], 1, "only the heading is an item");
    assert_eq!(d["roadmap"]["next"], "Real item");
}

#[test]
fn two_markers_meaning_the_same_thing_are_still_one_item() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(&root, "ROADMAP.md", "# Roadmap\n- [x] shipped ✅\n");
    assert_eq!(dash(&root)["roadmap"]["done"], 1);
}

#[test]
fn a_commented_out_freshness_check_does_not_count_as_one() {
    // The inversion that mattered most: dash's most useful finding is "no
    // workflow guards artifact freshness", and a comment mentioning
    // `fid derive --check` used to flip it to "guarded".
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        ".github/workflows/manual.yml",
        "name: Manual only\n\
         # someone should run fid derive --check here one day\n\
         on:\n  workflow_dispatch:\n\
         jobs:\n  note:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo hi\n",
    );

    let d = dash(&root);
    let w = d["ci"]["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["name"] == "Manual only")
        .expect("Manual only");
    assert_eq!(
        w["checks_freshness"], false,
        "a comment is not a freshness check"
    );
}

#[test]
fn job_names_are_not_mistaken_for_triggers() {
    // `push` and `schedule` are plausible job names, and substring-searching
    // the file reported both as triggers of a manual-only workflow.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        ".github/workflows/deploy.yml",
        "name: Deploy\n\
         on:\n  workflow_dispatch:\n\
         jobs:\n\
         \x20 push:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo a\n\
         \x20 schedule:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo b\n",
    );

    let d = dash(&root);
    let w = d["ci"]["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["name"] == "Deploy")
        .expect("Deploy");
    assert_eq!(
        w["triggers"],
        serde_json::json!(["workflow_dispatch"]),
        "job names are not triggers"
    );
}

#[test]
fn every_shape_of_the_on_key_is_read() {
    // GitHub accepts three forms, and `on` is a YAML 1.1 boolean so the key is
    // sometimes quoted. All four appear in real repositories.
    let cases: [(&str, &str, serde_json::Value); 4] = [
        (
            "scalar.yml",
            "name: Scalar\non: push\njobs: {}\n",
            serde_json::json!(["push"]),
        ),
        (
            "flow.yml",
            "name: Flow\non: [push, pull_request]\njobs: {}\n",
            serde_json::json!(["push", "pull_request"]),
        ),
        (
            "block.yml",
            "name: Block\non:\n  push:\n    branches: [main]\n  pull_request:\njobs: {}\n",
            serde_json::json!(["push", "pull_request"]),
        ),
        (
            "quoted.yml",
            "name: Quoted\n\"on\":\n  schedule:\n    - cron: '0 0 * * *'\njobs: {}\n",
            serde_json::json!(["schedule"]),
        ),
    ];

    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    for (file, body, _) in &cases {
        write(&root, &format!(".github/workflows/{file}"), body);
    }

    let d = dash(&root);
    let workflows = d["ci"]["workflows"].as_array().unwrap();
    for (file, _, expected) in &cases {
        let w = workflows
            .iter()
            .find(|w| w["file"] == *file)
            .unwrap_or_else(|| panic!("missing {file}"));
        assert_eq!(&w["triggers"], expected, "{file} triggers");
    }
}

#[test]
fn a_triggers_options_are_not_read_as_triggers() {
    // `branches:` is a property of the `push` trigger, not another trigger.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        ".github/workflows/deep.yml",
        "name: Deep\non:\n  push:\n    branches:\n      - main\n      - dev\n    tags: ['v*']\njobs: {}\n",
    );
    let d = dash(&root);
    let w = d["ci"]["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["name"] == "Deep")
        .unwrap();
    assert_eq!(w["triggers"], serde_json::json!(["push"]));
}

#[test]
fn text_output_carries_no_escape_codes_when_redirected() {
    // Output captured by a pipe is not a terminal, and styling it writes escape
    // codes into whatever reads it.
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = run(&root, &["dash"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains('\x1b'),
        "redirected output must be plain text: {text:?}"
    );
    assert!(text.contains("Product"), "headings still present");
}

// ── Propagation ───────────────────────────────────────────────────────────────

/// `fid upgrade` installs scaffold templates the platform added after a product
/// was created.
///
/// A 3-way merge only updates files already in the lock, so before Phase 18 a
/// new template reached only products scaffolded afterwards. The workflow that
/// gates artifact freshness was added in Phase 18 and would have arrived in zero
/// existing products.
#[test]
fn upgrade_installs_templates_added_since_the_product_was_scaffolded() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let workflow = root.join(".github/workflows/ci.yml");
    assert!(workflow.is_file(), "scaffold ships the CI workflow");

    // Simulate a product scaffolded before the template existed: remove the file
    // and drop it from the lock.
    std::fs::remove_file(&workflow).unwrap();
    let lock_path = root.join("fiducial.lock");
    let lock_text = std::fs::read_to_string(&lock_path).unwrap();
    let mut lock: toml::Value = toml::from_str(&lock_text).unwrap();
    lock.get_mut("templates")
        .and_then(|t| t.as_table_mut())
        .expect("lock has a templates table")
        .remove(".github/workflows/ci.yml")
        .expect("the workflow was tracked");
    std::fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();

    // A dry run reports the addition and writes nothing.
    let out = run(&root, &["upgrade", "--dry-run"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("would add: .github/workflows/ci.yml"),
        "dry run should report the addition: {text}"
    );
    assert!(!workflow.exists(), "a dry run writes nothing");

    // The real run installs it.
    let out = run(&root, &["upgrade"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("added: .github/workflows/ci.yml"),
        "upgrade should install it: {text}"
    );
    assert!(workflow.is_file(), "the workflow now exists");
    assert!(
        std::fs::read_to_string(&workflow)
            .unwrap()
            .contains("fid derive --check"),
        "the installed workflow gates artifact freshness"
    );

    // And the product is clean afterwards — the lock tracks what was written.
    let out = run(&root, &["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("clean"), "doctor after upgrade: {text}");
}

/// An untracked file already on disk is never clobbered by the new-template
/// path — it is reported, and the author reconciles it.
#[test]
fn upgrade_does_not_clobber_an_untracked_file_at_a_template_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let workflow = root.join(".github/workflows/ci.yml");
    let lock_path = root.join("fiducial.lock");
    let lock_text = std::fs::read_to_string(&lock_path).unwrap();
    let mut lock: toml::Value = toml::from_str(&lock_text).unwrap();
    lock.get_mut("templates")
        .and_then(|t| t.as_table_mut())
        .unwrap()
        .remove(".github/workflows/ci.yml");
    std::fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();

    // The author's own version of that file.
    std::fs::write(&workflow, "name: MINE\n").unwrap();

    let out = run(&root, &["upgrade"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("untracked") && text.contains("leaving it alone"),
        "should report rather than overwrite: {text}"
    );
    assert_eq!(
        std::fs::read_to_string(&workflow).unwrap(),
        "name: MINE\n",
        "the author's file is untouched"
    );
}

/// `fid upgrade` renames templates the platform has moved, and removes the old
/// file — because a subagent's filename *is* its identity, so a leftover
/// `.claude/agents/design.md` keeps claiming the colliding name that the rename
/// existed to free.
#[test]
fn upgrade_renames_a_moved_template_and_removes_the_old_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let new_path = root.join(".claude/agents/fiducial-design.md");
    let old_path = root.join(".claude/agents/design.md");
    assert!(new_path.is_file(), "scaffold ships the namespaced name");

    // Rebuild the pre-rename state: the old path, tracked in the lock.
    let content = std::fs::read_to_string(&new_path).unwrap();
    std::fs::write(&old_path, &content).unwrap();
    std::fs::remove_file(&new_path).unwrap();

    let lock_path = root.join("fiducial.lock");
    let lock_text = std::fs::read_to_string(&lock_path).unwrap();
    let mut lock: toml::Value = toml::from_str(&lock_text).unwrap();
    let templates = lock
        .get_mut("templates")
        .and_then(|t| t.as_table_mut())
        .unwrap();
    let record = templates
        .remove(".claude/agents/fiducial-design.md")
        .expect("tracked");
    templates.insert(".claude/agents/design.md".into(), record);
    std::fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();

    let out = run(&root, &["upgrade"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("renamed: .claude/agents/design.md"),
        "upgrade should report the rename: {text}"
    );

    assert!(new_path.is_file(), "the namespaced file now exists");
    assert!(
        !old_path.exists(),
        "the colliding file must be gone — leaving it defeats the rename"
    );

    let out = run(&root, &["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("clean"), "doctor after rename: {text}");
}

/// A locally modified file at a renamed path is never deleted. Silently
/// discarding someone's edits to fix a naming problem is worse than the naming
/// problem.
#[test]
fn upgrade_keeps_a_locally_modified_file_at_a_renamed_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let new_path = root.join(".claude/agents/fiducial-review.md");
    let old_path = root.join(".claude/agents/review.md");

    // Pre-rename state, but the product edited its copy.
    std::fs::write(&old_path, "---\nname: review\n---\n\nMy own edits.\n").unwrap();
    std::fs::remove_file(&new_path).unwrap();

    let lock_path = root.join("fiducial.lock");
    let lock_text = std::fs::read_to_string(&lock_path).unwrap();
    let mut lock: toml::Value = toml::from_str(&lock_text).unwrap();
    let templates = lock
        .get_mut("templates")
        .and_then(|t| t.as_table_mut())
        .unwrap();
    let record = templates
        .remove(".claude/agents/fiducial-review.md")
        .expect("tracked");
    templates.insert(".claude/agents/review.md".into(), record);
    std::fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();

    let out = run(&root, &["upgrade"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("locally modified"),
        "should say why it kept the file: {text}"
    );

    assert!(new_path.is_file(), "the namespaced file is installed");
    assert_eq!(
        std::fs::read_to_string(&old_path).unwrap(),
        "---\nname: review\n---\n\nMy own edits.\n",
        "local edits must survive"
    );
}

/// A repository that gates freshness by something other than `fid derive`.
///
/// This is the platform's own situation, and dash used to get it exactly
/// backwards: it equated "gated" with "runs `fid derive --check`" and reported
/// this repository as ungated while nine gates ran on every commit. The
/// protocol vectors and the terminal captures are gated by Rust tests on
/// purpose — a gate that runs through the tool it gates is blind exactly where
/// it matters — so the answer cannot be pattern-matched and is declared.
#[test]
fn a_declared_gate_counts_as_a_freshness_check() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    write(
        &root,
        ".github/workflows/gates.yml",
        "name: Gates\n\
         on:\n  push:\n\
         jobs:\n  t:\n    runs-on: ubuntu-latest\n    steps:\n\
         - run: cargo test -p demo-protocol --test vectors\n",
    );

    // Before declaring it, the gate is invisible — the workflow runs no
    // `fid derive --check`.
    let before = dash(&root);
    let w = |d: &serde_json::Value| {
        d["ci"]["workflows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["name"] == "Gates")
            .cloned()
            .expect("Gates workflow")
    };
    assert_eq!(
        w(&before)["checks_freshness"],
        false,
        "an undeclared command is not yet a known gate"
    );

    let config = root.join("fiducial.toml");
    let text = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        format!("{text}\n[freshness]\ngates = [\"cargo test -p demo-protocol --test vectors\"]\n"),
    )
    .unwrap();

    let after = dash(&root);
    let gates = w(&after);
    assert_eq!(
        gates["checks_freshness"], true,
        "a declared gate the workflow runs is a freshness check"
    );
    assert_eq!(
        gates["gates"][0], "cargo test -p demo-protocol --test vectors",
        "dash names which gate it matched, so the claim can be audited"
    );
}

/// A gate declared and then never run is the more dangerous drift.
///
/// It reads as protection in `fiducial.toml` whether or not any workflow runs
/// it — the same shape as the commented-out `fid derive --check` that dash
/// already refuses to count.
#[test]
fn a_declared_gate_no_workflow_runs_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let config = root.join("fiducial.toml");
    let text = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        format!("{text}\n[freshness]\ngates = [\"cargo test --test nobody-runs-this\"]\n"),
    )
    .unwrap();

    let d = dash(&root);
    let unrun = d["ci"]["gates_declared_but_unrun"]
        .as_array()
        .expect("gates_declared_but_unrun");
    assert_eq!(
        unrun.len(),
        1,
        "a declared gate nothing runs must be reported: {unrun:?}"
    );
    assert_eq!(unrun[0], "cargo test --test nobody-runs-this");
}

/// `fid derive --check` still counts without being declared.
///
/// It is what a scaffolded product uses, and requiring every product to
/// restate it in `[freshness]` would be the second declaration that block
/// exists to avoid.
#[test]
fn fid_derive_check_is_recognised_without_being_declared() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    write(
        &root,
        ".github/workflows/derive.yml",
        "name: Derive\n\
         on:\n  push:\n\
         jobs:\n  t:\n    runs-on: ubuntu-latest\n    steps:\n\
         - run: fid derive --check\n",
    );

    let d = dash(&root);
    let w = d["ci"]["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["name"] == "Derive")
        .expect("Derive");
    assert_eq!(w["checks_freshness"], true);
    assert_eq!(w["gates"][0], "fid derive --check");
}

//! End-to-end: a capability that is not compiled into `fid`.
//!
//! Roadmap item 3. *"We can add that later, easily" is only true once shipping
//! a capability does not require releasing the CLI* — so these install
//! capabilities the binary has never heard of, from a directory and from a git
//! repository, and assert they are first-class afterwards.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fid() -> PathBuf {
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("fid")
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

const SKILL: &str = "# Skill: stripe

Payments through Stripe, under the same discipline as first-party code: the
catalogue is declared once and the accessors are derived from it.

## Usage

Declare products in `billing/catalogue.json` and re-derive. Never hand-edit the
generated module — change the declaration instead.
";

/// A capability directory that exists nowhere in the binary.
fn write_capability(dir: &Path, manifest: Option<&str>) {
    std::fs::create_dir_all(dir.join("declarations/billing")).unwrap();
    std::fs::create_dir_all(dir.join("pipelines")).unwrap();
    std::fs::write(dir.join("SKILL.md"), SKILL).unwrap();
    if let Some(m) = manifest {
        std::fs::write(dir.join("capability.toml"), m).unwrap();
    }
    std::fs::write(
        dir.join("declarations/billing/catalogue.json"),
        "{\"products\":[]}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("pipelines/stripe.toml"),
        "name = \"stripe\"\nexecutor = \"noop\"\ninputs = [\"billing/catalogue.json\"]\noutputs = [\"src/generated/billing.ts\"]\n",
    )
    .unwrap();
}

fn scaffold(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    tmp.join("p")
}

fn dash(root: &Path) -> serde_json::Value {
    let out = run(root, &["dash", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    serde_json::from_str(&text(&out)).expect("dash --json")
}

fn lock(root: &Path) -> toml::Value {
    toml::from_str(&std::fs::read_to_string(root.join("fiducial.lock")).unwrap()).unwrap()
}

// ── A directory source ────────────────────────────────────────────────────────

#[test]
fn a_capability_the_binary_has_never_heard_of_installs_and_is_first_class() {
    let tmp = tempfile::tempdir().unwrap();
    let caps = tmp.path().join("caps/stripe");
    write_capability(
        &caps,
        Some("description = \"Stripe billing\"\nguard_rules = [\"no-unpinned-cli-fetch\"]\nrequires_adapters = [\"database\"]\n"),
    );
    let root = scaffold(tmp.path());

    let out = run(
        &root,
        &[
            "add",
            "capability",
            "stripe",
            "--from",
            caps.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", text(&out));

    // Its files land where the taxonomy says they should.
    assert!(root.join("billing/catalogue.json").is_file());
    assert!(root.join("pipelines/stripe.toml").is_file());
    assert!(root.join(".fiducial/skills/stripe.md").is_file());

    // Its pipeline is a pipeline — `fid derive` picks it up with no extra step,
    // which is the property that separates one from a copied file.
    let graph = text(&run(&root, &["graph"]));
    assert!(graph.contains("stripe"), "{graph}");

    // Its declaration and its adapter requirement are reported like a
    // built-in's. Before the lock recorded capabilities, every consumer looked
    // in the built-in registry and an external capability was invisible.
    let d = dash(&root);
    let decls = d["taxonomy"]["declarations"].as_array().unwrap();
    assert!(
        decls
            .iter()
            .any(|x| x["name"] == "billing/catalogue.json" && x["capability"] == "stripe"),
        "{decls:?}"
    );
    let database = d["taxonomy"]["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["contract"] == "database")
        .unwrap();
    assert_eq!(database["required_by"][0], "stripe");

    // And `fid doctor` holds the product to it.
    let doctor = run(&root, &["doctor"]);
    assert!(!doctor.status.success());
    assert!(
        text(&doctor).contains("capability `stripe` requires the `database` adapter"),
        "{}",
        text(&doctor)
    );
}

#[test]
fn the_source_is_pinned_in_the_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let caps = tmp.path().join("caps/stripe");
    write_capability(&caps, Some("description = \"Stripe billing\"\n"));
    let root = scaffold(tmp.path());
    assert!(run(
        &root,
        &[
            "add",
            "capability",
            "stripe",
            "--from",
            caps.to_str().unwrap()
        ]
    )
    .status
    .success());

    let lock = lock(&root);
    let record = &lock["capabilities"]["stripe"];
    assert!(
        record["source"].as_str().unwrap().starts_with("path:"),
        "{record:?}"
    );
    assert_eq!(record["description"].as_str(), Some("Stripe billing"));
    assert_eq!(
        record["declarations"][0].as_str(),
        Some("billing/catalogue.json")
    );
    assert_eq!(
        record["pipelines"][0].as_str(),
        Some("pipelines/stripe.toml")
    );
    // A hash over what it installed, so tampering at the source is visible even
    // for a `path:` source, which has no revision of its own.
    assert_eq!(record["hash"].as_str().unwrap().len(), 64);
}

/// Third-party content is held to the rules first-party content is.
#[test]
fn a_capability_that_does_not_conform_is_not_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let caps = tmp.path().join("caps/stripe");
    write_capability(&caps, None);
    // Too short to teach an agent anything — the one hard rule of §3.3.
    std::fs::write(caps.join("SKILL.md"), "# stripe\n").unwrap();
    let root = scaffold(tmp.path());

    let out = run(
        &root,
        &[
            "add",
            "capability",
            "stripe",
            "--from",
            caps.to_str().unwrap(),
        ],
    );
    assert!(!out.status.success());
    assert!(text(&out).contains("SKILL.md"), "{}", text(&out));
    assert!(
        !root.join("billing/catalogue.json").exists(),
        "a refused capability must install nothing"
    );
}

#[test]
fn the_id_must_match_the_source() {
    let tmp = tempfile::tempdir().unwrap();
    let caps = tmp.path().join("caps/billing");
    write_capability(&caps, Some("description = \"d\"\n"));
    let root = scaffold(tmp.path());

    // Otherwise `fiducial.toml` records a capability that is not what it says.
    let out = run(
        &root,
        &[
            "add",
            "capability",
            "stripe",
            "--from",
            caps.to_str().unwrap(),
        ],
    );
    assert!(!out.status.success());
    assert!(text(&out).contains("provides `billing`"), "{}", text(&out));
}

// ── A git source ──────────────────────────────────────────────────────────────

fn git(cwd: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("running git")
}

/// A repository holding one capability among others, tagged.
fn capability_repo(tmp: &Path) -> (PathBuf, String) {
    let repo = tmp.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    assert!(git(&repo, &["init", "-q", "--initial-branch=main"])
        .status
        .success());
    write_capability(
        &repo.join("caps/stripe"),
        Some("description = \"Stripe billing\"\n"),
    );
    assert!(git(&repo, &["add", "-A"]).status.success());
    assert!(git(
        &repo,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "feat: stripe capability",
        ],
    )
    .status
    .success());
    assert!(git(&repo, &["tag", "v1.0.0"]).status.success());
    let sha = String::from_utf8_lossy(&git(&repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    (repo, sha)
}

#[test]
fn a_capability_installs_from_a_git_repository_and_pins_the_commit() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, sha) = capability_repo(tmp.path());
    let root = scaffold(tmp.path());

    let spec = format!("git:{}#v1.0.0::caps/stripe", repo.display());
    let out = run(&root, &["add", "capability", "stripe", "--from", &spec]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(root.join("billing/catalogue.json").is_file());

    // Pinned at the commit, never the tag. `#main` is a question whose answer
    // changes, and a lock that records the question pins nothing.
    let record = lock(&root)["capabilities"]["stripe"].clone();
    let source = record["source"].as_str().unwrap();
    assert!(source.starts_with("git:"), "{source}");
    assert!(
        source.contains(&sha[..12]),
        "expected the commit {} in {source}",
        &sha[..12]
    );
    assert!(!source.contains("v1.0.0"), "a tag is not a pin: {source}");
}

#[test]
fn a_revision_that_does_not_exist_fails_rather_than_installing_the_default_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, _) = capability_repo(tmp.path());
    let root = scaffold(tmp.path());

    let spec = format!("git:{}#v9.9.9::caps/stripe", repo.display());
    let out = run(&root, &["add", "capability", "stripe", "--from", &spec]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(!root.join("billing/catalogue.json").exists());
}

#[test]
fn a_subdirectory_that_does_not_exist_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, _) = capability_repo(tmp.path());
    let root = scaffold(tmp.path());

    let spec = format!("git:{}#v1.0.0::caps/nope", repo.display());
    let out = run(&root, &["add", "capability", "stripe", "--from", &spec]);
    assert!(!out.status.success());
    assert!(text(&out).contains("caps/nope"), "{}", text(&out));
}

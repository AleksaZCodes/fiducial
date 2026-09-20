//! End-to-end: the capability taxonomy, through the real binary.
//!
//! Spec: `docs/specs/2026-09-14-capability-taxonomy.md`. The unit tests in
//! `src/capability.rs` and `src/adapter.rs` cover the registry's own
//! conformance. These cover what a product actually experiences: installing a
//! capability that introduces a declaration and a pipeline, and selecting an
//! adapter that does or does not exist.

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

fn scaffold(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    tmp.join("p")
}

fn dash(root: &Path) -> serde_json::Value {
    let out = run(root, &["dash", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    serde_json::from_str(&text(&out)).expect("dash --json")
}

fn set_adapters(root: &Path, block: &str) {
    let path = root.join("fiducial.toml");
    let mut config = std::fs::read_to_string(&path).unwrap();
    config.push_str("\n[adapters]\n");
    config.push_str(block);
    std::fs::write(&path, config).unwrap();
}

// ── Declarations and pipelines ────────────────────────────────────────────────

#[test]
fn installing_a_capability_declares_its_facts_and_wires_its_pipelines() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());

    let d = dash(&root);
    let decls = d["taxonomy"]["declarations"].as_array().unwrap();

    let board = decls
        .iter()
        .find(|x| x["name"] == "board/board.interface.json")
        .expect("eda declares the board interface");
    assert_eq!(board["capability"], "eda");
    assert_eq!(board["kind"], "file");
    assert_eq!(board["present"], true);

    // i18n's block is a declaration, not a special case in `patch_config`.
    // Found by capability rather than by being the first config-block in the
    // list: a scaffold installs more than one capability now, and "the first
    // block" was silently asserting install order.
    let block = decls
        .iter()
        .find(|x| x["capability"] == "i18n" && x["kind"] == "config-block")
        .expect("i18n declares a fiducial.toml block");
    assert_eq!(block["present"], true);

    // And `design-system.md` is a *file* declaration, not a block. The kind
    // used to be guessed from whether the name contained a slash, which got
    // every root-level declaration wrong.
    let ds = decls
        .iter()
        .find(|x| x["name"] == "design-system.md")
        .expect("design declares design-system.md");
    assert_eq!(ds["kind"], "file");
    assert_eq!(ds["capability"], "design");
    assert_eq!(ds["present"], true);

    // Both of eda's pipelines are wired and runnable, which is the property
    // that distinguishes a pipeline from a file that was copied in.
    let pipelines = d["graph"]["pipelines"].as_array().unwrap();
    assert!(
        pipelines.len() >= 3,
        "expected i18n + eda's two: {pipelines:?}"
    );
    assert!(run(&root, &["derive"]).status.success());
}

/// A declaration a pipeline reads and nobody wrote is reported *before*
/// `fid derive` hits it.
#[test]
fn a_missing_declaration_is_visible_without_running_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    std::fs::remove_file(root.join("messages/sr.json")).unwrap();

    let d = dash(&root);
    let missing = d["taxonomy"]["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "messages/sr.json")
        .expect("the declaration is still declared");
    assert_eq!(missing["present"], false);
}

// ── Adapters ──────────────────────────────────────────────────────────────────

#[test]
fn the_no_op_is_a_real_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    set_adapters(&root, "errors = \"none\"\nstorage = \"none\"\n");

    let out = run(&root, &["doctor"]);
    let t = text(&out);
    assert!(
        !t.contains("[adapters]:"),
        "`none` must be a working selection, not a complaint:\n{t}"
    );

    let d = dash(&root);
    let errors = d["taxonomy"]["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["contract"] == "errors")
        .expect("errors is a contract");
    assert_eq!(errors["selected"], "none");
    assert!(errors["problem"].is_null());
}

/// The lesson from the guard rules, applied to vendors: a selectable name is a
/// promise. Naming one nothing implements has to fail, and has to say so
/// differently from a typo.
#[test]
fn a_vendor_nothing_implements_is_refused_and_distinguished_from_a_typo() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    set_adapters(&root, "database = \"neon\"\n");

    let out = run(&root, &["doctor"]);
    assert!(!out.status.success(), "an unbacked vendor must fail doctor");
    let t = text(&out);
    assert!(t.contains("nothing implements yet"), "{t}");
    assert!(
        t.contains("\"none\""),
        "the message must name the way out:\n{t}"
    );

    let tmp2 = tempfile::tempdir().unwrap();
    let root2 = scaffold(tmp2.path());
    set_adapters(&root2, "database = \"supabse\"\n");
    let out = run(&root2, &["doctor"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("not a known implementation"),
        "a typo must not read as a roadmap item:\n{}",
        text(&out)
    );
}

#[test]
fn a_contract_that_does_not_exist_is_named_in_both_views() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    set_adapters(&root, "blockchain = \"ethereum\"\n");

    assert!(!run(&root, &["doctor"]).status.success());

    // Absence is a finding: the dash listing walks the known contracts, so an
    // unknown key would otherwise be in neither the listing nor the selection.
    let d = dash(&root);
    let found = d["taxonomy"]["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["contract"] == "blockchain")
        .expect("an unknown contract must still appear");
    assert!(found["problem"].is_string());
}

#[test]
fn every_contract_is_listed_whether_selected_or_not() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let d = dash(&root);
    let adapters = d["taxonomy"]["adapters"].as_array().unwrap();
    // A literal, not `CONTRACTS.len()` — this crate has no lib target, so the
    // test reaches the registry through the binary. Bumping it is the point:
    // adding a contract has to be acknowledged here, the same tripwire
    // `REAL_VENDORS` is for vendors in the generated-factory suite.
    assert_eq!(
        adapters.len(),
        11,
        "every contract is a fact about the product"
    );
    for a in adapters {
        assert!(a["selected"].is_null(), "a fresh product selects none");
        assert!(a["problem"].is_null());
    }
}

/// `fid capability list --all` is the only place a contract is discoverable.
#[test]
fn contracts_are_discoverable_and_honest_about_what_works() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let out = run(&root, &["capability", "list", "--all"]);
    assert!(out.status.success(), "{}", text(&out));
    let t = text(&out);
    assert!(t.contains("ADAPTER CONTRACTS"), "{t}");
    assert!(t.contains("selectable: none"), "{t}");
    // d1 and r2 shipped as the first real vendors (Cloudflare adapter set);
    // supabase and supabase-storage ship now — all four are selectable.
    assert!(
        t.contains("selectable: none, d1, supabase") && t.contains("planned: neon"),
        "database: d1 and supabase selectable, neon still planned:\n{t}"
    );
    assert!(
        t.contains("selectable: none, r2, supabase-storage") && t.contains("planned: s3"),
        "storage: r2 and supabase-storage selectable, s3 still planned:\n{t}"
    );
    // turnstile and cloudflare-queues followed the same seam.
    assert!(
        t.contains("selectable: none, turnstile") && t.contains("planned: recaptcha"),
        "turnstile moved from planned to selectable:\n{t}"
    );
    assert!(
        t.contains("selectable: none, cloudflare-queues") && t.contains("planned: sqs"),
        "cloudflare-queues moved from planned to selectable:\n{t}"
    );
    // auth is new this round — supabase ships selectable from day one.
    assert!(
        t.contains("selectable: none, supabase") && t.contains("planned: clerk"),
        "auth's supabase should be selectable, clerk still planned:\n{t}"
    );
}

#[test]
fn capability_list_says_what_each_capability_contributes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());

    let t = text(&run(&root, &["capability", "list"]));
    assert!(t.contains("declares board/board.interface.json"), "{t}");
    assert!(t.contains("derives via eda.toml, enclosure.toml"), "{t}");
    assert!(t.contains("seeds a fiducial.toml block"), "{t}");
}

#[test]
fn capability_check_passes_on_the_built_in_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["add", "eda"]).status.success());

    let out = run(&root, &["capability", "check"]);
    assert!(out.status.success(), "{}", text(&out));
}

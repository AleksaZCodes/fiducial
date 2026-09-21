//! End-to-end: ownership, and what `fid rebaseline` will and will not accept.
//!
//! `src/ownership.rs` unit-tests the classification itself. These cover the
//! behaviour a product actually sees — that editing your own file is not an
//! error, that editing the platform's still is, and that accepting a fork
//! survives into `fid doctor`.
//!
//! The case that motivated all of it: fon carried 32 `fid doctor` issues, 26 of
//! them edits to files whose own template says *"It is product-owned — edit it
//! freely."* Its CI runs `fid doctor` with `continue-on-error` as a result,
//! which costs every real finding too.

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

fn scaffold(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "demo", "--full"]);
    assert!(out.status.success(), "fid new failed: {out:?}");
    tmp.join("demo")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn editing_a_file_you_own_is_not_a_doctor_issue() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    std::fs::write(
        root.join("MISSION.md"),
        "# demo\n\nEverything about why this exists, in my own words.\n",
    )
    .unwrap();
    std::fs::write(
        root.join("messages/en.json"),
        "{\"meta\":{\"description\":\"Mine.\"}}\n",
    )
    .unwrap();

    let out = run(&root, &["doctor"]);
    let t = text(&out);
    assert!(
        !t.contains("✗ MISSION.md") && !t.contains("✗ messages/en.json"),
        "the template tells you to edit these:\n{t}"
    );
    assert!(
        t.contains("product-owned file(s) edited — expected, not drift"),
        "they should still be counted:\n{t}"
    );
}

#[test]
fn editing_a_file_the_platform_owns_is_still_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    // A derive script a pipeline invokes. Editing it forks platform code, and
    // `fid upgrade` will replace it — which is the whole reason to warn.
    let script = root.join("scripts/build-design-system.mjs");
    let mut body = std::fs::read_to_string(&script).unwrap();
    body.push_str("\n// my change\n");
    std::fs::write(&script, body).unwrap();

    let out = run(&root, &["doctor"]);
    let t = text(&out);
    assert!(t.contains("scripts/build-design-system.mjs"), "{t}");
    assert!(t.contains("platform-owned"), "{t}");
    assert!(
        t.contains("will be merged over"),
        "the warning must say what is at stake:\n{t}"
    );
}

#[test]
fn rebaseline_refuses_a_file_the_product_owns() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    std::fs::write(root.join("README.md"), "# mine\n").unwrap();

    let out = run(&root, &["rebaseline", "README.md"]);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(t.contains("product-owned"), "{t}");
    assert!(
        t.contains("merge base"),
        "the refusal has to say why, or it reads as an arbitrary rule:\n{t}"
    );
}

#[test]
fn accepting_a_fork_stops_doctor_reporting_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let script = root.join("scripts/derive-logo.mjs");
    let mut body = std::fs::read_to_string(&script).unwrap();
    body.push_str("\n// a fork I meant\n");
    std::fs::write(&script, body).unwrap();

    assert!(text(&run(&root, &["doctor"])).contains("scripts/derive-logo.mjs"));

    let out = run(&root, &["rebaseline", "scripts/derive-logo.mjs"]);
    assert!(out.status.success(), "{}", text(&out));

    let t = text(&run(&root, &["doctor"]));
    assert!(
        !t.contains("✗ scripts/derive-logo.mjs"),
        "accepted forks stop being news:\n{t}"
    );
}

#[test]
fn rebaseline_with_no_arguments_lists_without_changing_anything() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let script = root.join("scripts/derive-logo.mjs");
    let mut body = std::fs::read_to_string(&script).unwrap();
    body.push_str("\n// unaccepted\n");
    std::fs::write(&script, body).unwrap();

    let before = std::fs::read_to_string(root.join("fiducial.lock")).unwrap();
    let out = run(&root, &["rebaseline"]);
    assert!(out.status.success());
    let t = text(&out);
    assert!(t.contains("scripts/derive-logo.mjs"), "{t}");
    assert!(t.contains("There is no --all"), "{t}");

    let after = std::fs::read_to_string(root.join("fiducial.lock")).unwrap();
    assert_eq!(before, after, "listing must not write");

    // And it is still reported, because listing is not accepting.
    assert!(text(&run(&root, &["doctor"])).contains("scripts/derive-logo.mjs"));
}

#[test]
fn an_untracked_path_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = run(&root, &["rebaseline", "not-a-template.txt"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("not a template this product tracks"));
}

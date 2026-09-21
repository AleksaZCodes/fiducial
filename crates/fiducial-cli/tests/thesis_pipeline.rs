//! End-to-end: `fid new` → `fid thesis set` → `fid derive` → the gate.
//!
//! The unit tests in `src/thesis.rs` cover parsing, supersession and rendering;
//! `src/commands/thesis.rs` covers the append semantics. These cover the two
//! properties that only appear on a real scaffold, through the real binary —
//! and both were bugs when this pipeline was first wired.
//!
//! 1. **A product with no thesis is not blocked.** `fid derive --check` must
//!    pass on a scaffold that has not decided what it claims, because that is
//!    the state every product starts in.
//! 2. **An inapplicable output is not written.** The executor iterated its
//!    declared outputs and wrote all of them, so a product with no `apps/web`
//!    got three conjured directories and a `thesis.ts` that `fid derive` then
//!    reported as "no longer derived but still exists" — describing a file it
//!    had just created itself.

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
    let out = run(tmp, &["new", "demo"]);
    assert!(out.status.success(), "fid new failed: {out:?}");
    tmp.join("demo")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn a_scaffold_ships_a_thesis_file_and_its_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    assert!(root.join("thesis.toml").is_file());
    assert!(root.join("pipelines/thesis.toml").is_file());

    // And the scaffolded file declares nothing, on purpose: it is a page of
    // questions. A scaffolded placeholder claim would be a lie the product
    // starts life holding.
    let raw = std::fs::read_to_string(root.join("thesis.toml")).unwrap();
    assert!(
        !raw.lines()
            .any(|l| l.trim_start().starts_with("[[thesis]]")),
        "the scaffold must not declare a placeholder thesis:\n{raw}"
    );
}

#[test]
fn a_product_with_no_thesis_still_passes_the_gate() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let out = run(&root, &["derive", "--check"]);
    assert!(
        out.status.success(),
        "a product that has not decided what it claims must not be blocked:\n{}\n{}",
        stdout(&out),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(!root.join("PITCH.md").exists(), "nothing to pitch yet");
}

#[test]
fn declaring_a_claim_starts_deriving_the_pitch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let out = run(
        &root,
        &[
            "thesis",
            "set",
            "No unverified alert ever reaches a responder.",
        ],
    );
    assert!(out.status.success(), "{out:?}");

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", stdout(&out));

    let pitch = std::fs::read_to_string(root.join("PITCH.md")).unwrap();
    assert!(
        pitch.contains("**No unverified alert ever reaches a responder.**"),
        "{pitch}"
    );

    // And it is now gated like any other artifact.
    assert!(run(&root, &["derive", "--check"]).status.success());
}

#[test]
fn the_typescript_module_waits_for_an_app_instead_of_conjuring_one() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    run(&root, &["thesis", "set", "Ship it."]);

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", stdout(&out));

    let ts = root.join("apps/web/src/generated/thesis.ts");
    assert!(
        !ts.exists(),
        "an output the pipeline declares inapplicable must not be written"
    );
    assert!(
        !stdout(&out).contains("no longer derived"),
        "and derive must not report a file it just created:\n{}",
        stdout(&out)
    );

    // Give the product a real app and the module starts deriving, with no
    // further ceremony — the same rule `fid-design` uses for its stylesheet.
    std::fs::write(
        root.join("apps/web/package.json"),
        "{\"name\":\"web\",\"private\":true}\n",
    )
    .unwrap();

    assert!(run(&root, &["derive"]).status.success());
    let module = std::fs::read_to_string(&ts).expect("thesis.ts should derive once an app exists");
    assert!(module.contains("claim: \"Ship it.\""), "{module}");
    assert!(module.contains("as const"), "{module}");
}

#[test]
fn a_sharpened_claim_supersedes_and_the_pitch_follows() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    run(&root, &["thesis", "set", "Detection should be cheap."]);

    // Replacing a thesis without saying why is refused.
    let refused = run(&root, &["thesis", "set", "Trust is the objection."]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("has to say why"));

    let out = run(
        &root,
        &[
            "thesis",
            "set",
            "Trust is the objection.",
            "--because",
            "Cheap was never the objection.",
        ],
    );
    assert!(out.status.success(), "{out:?}");

    run(&root, &["derive"]);
    let pitch = std::fs::read_to_string(root.join("PITCH.md")).unwrap();
    assert!(pitch.contains("**Trust is the objection.**"), "{pitch}");
    assert!(
        !pitch.contains("Detection should be cheap"),
        "the pitch follows the current claim, not the history:\n{pitch}"
    );

    // The superseded wording survives in the declaration, which is the whole
    // reason the file is append-only.
    let log = run(&root, &["thesis", "log"]);
    let text = stdout(&log);
    assert!(text.contains("Detection should be cheap."), "{text}");
    assert!(text.contains("[superseded]"), "{text}");
    assert!(text.contains("Cheap was never the objection."), "{text}");
}

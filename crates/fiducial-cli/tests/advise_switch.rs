//! Advisory review is part of a product's procedure, and it can be switched off
//! at any point by a declared line in `fiducial.toml`.
//!
//! Three properties, because each fails differently:
//!
//! 1. A scaffolded product has the switch, on. "It should be there" is only
//!    true if a fresh `fid new` carries it.
//! 2. `--no-advise` sets the starting value and does not make a one-time
//!    decision: editing the line flips review either way later.
//! 3. Switching it off asks nothing, and changes nothing a build reads.

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
        .env("FID_ADVISE_BIN", "/nonexistent/fid-advise")
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

fn config(root: &Path) -> String {
    std::fs::read_to_string(root.join("fiducial.toml")).unwrap()
}

#[test]
fn a_new_product_has_review_on_and_the_pipeline_asks_for_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");

    let cfg = config(&root);
    assert!(cfg.contains("[advise]"), "{cfg}");
    assert!(cfg.contains("enabled = true"), "{cfg}");

    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    assert!(ci.contains("fid advise"), "{ci}");
    assert!(
        ci.contains("continue-on-error: true"),
        "advice must never gate a merge: {ci}"
    );

    let agents = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert!(agents.contains("fid advise"), "{agents}");
}

#[test]
fn no_advise_sets_the_starting_value_and_the_line_flips_it_back() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = run(tmp.path(), &["new", "q", "--no-advise"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("q");

    assert!(
        config(&root).contains("enabled = false"),
        "{}",
        config(&root)
    );
    let off = run(&root, &["advise"]);
    assert!(off.status.success(), "{}", text(&off));
    assert!(text(&off).contains("switched off"), "{}", text(&off));

    let on = config(&root).replace("enabled = false", "enabled = true");
    std::fs::write(root.join("fiducial.toml"), on).unwrap();
    let back = run(&root, &["advise"]);
    assert!(back.status.success(), "{}", text(&back));
    assert!(
        !text(&back).contains("switched off"),
        "turning it back on must turn review back on: {}",
        text(&back)
    );
}

#[test]
fn switching_review_off_changes_nothing_a_build_reads() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");

    let before = run(&root, &["derive", "--check"]);
    assert!(before.status.success(), "{}", text(&before));

    let cfg = config(&root).replace("enabled = true", "enabled = false");
    std::fs::write(root.join("fiducial.toml"), cfg).unwrap();

    let after = run(&root, &["derive", "--check"]);
    assert!(after.status.success(), "{}", text(&after));
    let status = run(&root, &["advise", "status"]);
    assert!(text(&status).contains("switched off"), "{}", text(&status));
}

#[test]
fn a_product_without_the_block_still_has_review_on() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");

    let cfg = config(&root);
    let start = cfg.find("[advise]").unwrap();
    let end = cfg.find("[guard]").unwrap();
    std::fs::write(
        root.join("fiducial.toml"),
        format!("{}{}", &cfg[..start], &cfg[end..]),
    )
    .unwrap();

    let out = run(&root, &["advise"]);
    assert!(
        !text(&out).contains("switched off"),
        "a product that predates the block must not be silently opted out: {}",
        text(&out)
    );
}

//! The `design` pipeline through the real binary: what derive says when the
//! declaration breaks one of its rules.

use std::{path::Path, process::Command};

fn fid(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fid"))
        .args(args)
        .current_dir(dir)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to invoke fid")
}

fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A designer wants softer captions, derive refuses the contrast, and the
/// minimum is lowered instead of the colour (the paper's round 4, case R8).
/// Derive refuses that too — and says why, where it used to say only that
/// the file "does not parse".
#[test]
fn a_contrast_minimum_lowered_under_the_floor_fails_saying_why() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(fid(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    let out = fid(&root, &["add", "capability", "design"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(fid(&root, &["derive"]).status.success());

    let path = root.join("design-system.md");
    let md = std::fs::read_to_string(&path).unwrap();
    let from = r#"{ fg = "muted-foreground",   bg = "background", min = 4.5,"#;
    assert!(md.contains(from), "the seed's pair moved");
    std::fs::write(
        &path,
        md.replace(
            from,
            r#"{ fg = "muted-foreground",   bg = "background", min = 3.0,"#,
        ),
    )
    .unwrap();
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success(), "a lowered minimum passed");
    assert!(
        text(&out)
            .contains("muted-foreground on background has min = 3 — under the 4.5 WCAG AA floor"),
        "{}",
        text(&out)
    );
}

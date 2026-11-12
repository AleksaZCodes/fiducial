//! End-to-end: `fid add cms` → `fid derive`.
//!
//! The `cms` capability had no test, and a fresh product with it installed
//! could not derive at all. `derive-cms.mjs` imports `parseToml` from
//! `derive-content.mjs`, which was a script with no main-module guard: the
//! import ran the whole content derivation, which reads `content/**` — a path
//! the `cms` pipeline never declared as an input, so the derive sandbox
//! refused the read and the pipeline failed.
//!
//! `seo` and `brand` had hit the same script and each moved to a separate
//! parser. This fixes it where it lives: importing the content deriver no
//! longer runs it.

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

fn cms_product(tmp: &Path) -> PathBuf {
    let out = run(
        tmp,
        &["new", "p", "--locales", "en,sr", "--default-locale", "en"],
    );
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.join("p");
    for capability in ["content", "cms"] {
        let out = run(&root, &["add", "capability", capability]);
        assert!(out.status.success(), "{capability}: {}", text(&out));
    }
    root
}

#[test]
fn a_fresh_product_with_the_cms_derives() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let config =
        std::fs::read_to_string(root.join("tools/cms/config.yml")).expect("the editor config");
    assert!(config.contains("posts"), "{config}");

    let out = run(&root, &["derive", "--check"]);
    assert!(out.status.success(), "{}", text(&out));
}

/// The class, not the instance: importing the content deriver for its parser
/// must not derive content.
#[test]
fn importing_the_content_deriver_has_no_side_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let artifact = root.join("apps/web/src/generated/content.ts");
    std::fs::write(&artifact, "// marker\n").unwrap();

    let out = Command::new("node")
        .args([
            "--input-type=module",
            "-e",
            "import { parseToml } from './scripts/derive-content.mjs'; if (typeof parseToml !== 'function') process.exit(2);",
        ])
        .current_dir(&root)
        .output()
        .expect("running node");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        std::fs::read_to_string(&artifact).unwrap(),
        "// marker\n",
        "importing the deriver rewrote its artifact"
    );
}

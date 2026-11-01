//! A fresh product with every software capability derives and checks clean,
//! before anyone edits a file.
//!
//! It did not: two pipelines claimed the sitemap, deploy needed a vendor no
//! one had written, the sample post existed in one of two languages, and the
//! home page had no title. Each was the platform failing on files it wrote
//! itself — found while setting up the paper's study.

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

#[test]
fn every_software_capability_derives_and_checks_clean_out_of_the_box() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("skipped: node is not installed (content and seo pipelines need it)");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let out = fid(tmp.path(), &["new", "--locales", "en,sr", "w"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("w");
    for cap in [
        "i18n",
        "content",
        "brand",
        "seo",
        "legal",
        "design",
        "adapters",
        "deploy",
        "migrations",
        "identity",
    ] {
        let out = fid(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }
    let out = fid(&root, &["derive"]);
    assert!(
        out.status.success(),
        "a fresh product failed derive:\n{}",
        text(&out)
    );
    let out = fid(&root, &["derive", "--check"]);
    assert!(
        out.status.success(),
        "a fresh product failed --check:\n{}",
        text(&out)
    );
    assert!(
        root.join("content/posts/sr").is_dir(),
        "the sample post was not given to sr"
    );
    assert!(
        !root.join("public/sitemap.xml").exists(),
        "brand wrote a second sitemap beside seo's"
    );
}

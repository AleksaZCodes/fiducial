//! `[apps.<name>]` through the real binary: `fid doctor` holds a declared app to
//! what can be built, and to its framework's own safety requirements.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(PathBuf::from(env!("CARGO_BIN_EXE_fid")))
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

fn product(tmp: &Path, apps: &str) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    let toml = root.join("fiducial.toml");
    let mut s = std::fs::read_to_string(&toml).unwrap();
    s.push_str(&format!("\n{apps}\n"));
    std::fs::write(&toml, s).unwrap();
    root
}

#[test]
fn an_intended_framework_fails_doctor_with_what_is_selectable() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), "[apps.web]\nframework = \"react-router\"\n");
    let out = text(&run(&root, &["doctor"]));
    assert!(
        out.contains("names an intended choice that nothing builds yet"),
        "{out}"
    );
    assert!(
        out.contains("Selectable today: next, sveltekit, astro"),
        "{out}"
    );
}

#[test]
fn a_combination_that_does_not_fit_fails_doctor_with_the_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(
        tmp.path(),
        "[apps.web]\nframework = \"next\"\n[deploy]\nshape = \"sveltekit\"\n",
    );
    let out = text(&run(&root, &["doctor"]));
    assert!(out.contains("the app deploys as \"nextjs\""), "{out}");
}

#[test]
fn an_astro_app_without_its_headers_file_fails_doctor() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), "[apps.web]\nframework = \"astro\"\n");
    assert!(run(&root, &["add", "design"]).status.success());
    assert!(run(&root, &["add", "app", "astro"]).status.success());

    let out = run(&root, &["doctor"]);
    assert!(
        !text(&out).contains("Astro app"),
        "a fresh Astro product was reported unsafe: {}",
        text(&out)
    );

    std::fs::remove_file(root.join("apps/web/public/_headers")).unwrap();
    let out = text(&run(&root, &["doctor"]));
    assert!(
        out.contains("Astro app has no `apps/web/public/_headers`"),
        "{out}"
    );
}

#[test]
fn a_product_with_no_apps_block_is_read_as_before() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let out = text(&run(&tmp.path().join("p"), &["doctor"]));
    assert!(!out.contains("[apps"), "{out}");
}

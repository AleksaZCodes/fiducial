//! End-to-end tests for `fid docs` — the prose freshness gate.
//!
//! The property under test is not "the hash changes". It is the pair that
//! makes the gate usable at all:
//!
//! 1. It **fires** when the source a paragraph describes actually moves.
//! 2. It **stays quiet** when something unrelated in the same file moves.
//!
//! Only the first is obvious. The second is what stops it becoming noise, and
//! a gate that cries wolf is worse than no gate — it trains the reader to
//! accept without reading, which is the exact failure it exists to prevent.

use std::{path::Path, process::Command};

fn fid_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_fid"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(fid_bin())
        .args(args)
        .current_dir(dir)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to invoke fid")
}

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

/// A product with one annotated paragraph describing one symbol.
fn product(tmp: &Path) -> std::path::PathBuf {
    let out = run(tmp, &["new", "demo"]);
    assert!(out.status.success(), "fid new failed: {}", text(&out));
    let root = tmp.join("demo");

    write(
        &root,
        "src/registry.rs",
        "//! A registry.\n\
         \n\
         pub static VENDORS: &[&str] = &[\n    \"none\",\n];\n\
         \n\
         pub fn unrelated() -> u8 {\n    1\n}\n",
    );
    write(
        &root,
        "docs/notes.md",
        "# Notes\n\n\
         <!-- fid:describes src/registry.rs#pub static VENDORS -->\n\n\
         Only `none` can be selected today.\n\n\
         <!-- fid:end-describes -->\n",
    );
    root
}

#[test]
fn a_block_starts_unaccepted_because_nobody_has_read_it_yet() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());

    let out = run(&root, &["docs", "--check"]);
    assert!(!out.status.success(), "a never-read block must not pass");
    assert!(
        text(&out).contains("never accepted"),
        "the reason must say nobody has read it: {}",
        text(&out)
    );
}

#[test]
fn accepting_makes_the_check_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());

    assert!(run(&root, &["docs", "--accept"]).status.success());
    let out = run(&root, &["docs", "--check"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(root.join("docs/prose.lock").exists(), "lock is written");
}

#[test]
fn changing_the_described_symbol_fails_the_check() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    run(&root, &["docs", "--accept"]);

    // Exactly the change that made the original sentence wrong: a vendor is
    // promoted, and the paragraph saying "only `none`" is now a lie.
    let src = root.join("src/registry.rs");
    let body = std::fs::read_to_string(&src).unwrap();
    std::fs::write(
        &src,
        body.replace("\"none\",", "\"none\",\n    \"resend\","),
    )
    .unwrap();

    let out = run(&root, &["docs", "--check"]);
    assert!(!out.status.success(), "a moved source must fail the gate");
    let t = text(&out);
    assert!(t.contains("docs/notes.md"), "names the document: {t}");
    assert!(
        t.contains("src/registry.rs#pub static VENDORS"),
        "names the source that moved: {t}"
    );
}

#[test]
fn an_unrelated_change_in_the_same_file_does_not_fire() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    run(&root, &["docs", "--accept"]);

    // The property that keeps this gate worth having. `#pub static VENDORS`
    // narrows the watch to one balanced block, so a paragraph about the
    // registry is not woken by an edit elsewhere in the file.
    let src = root.join("src/registry.rs");
    let body = std::fs::read_to_string(&src).unwrap();
    std::fs::write(&src, body.replace("    1\n", "    2\n")).unwrap();

    let out = run(&root, &["docs", "--check"]);
    assert!(
        out.status.success(),
        "an unrelated edit must not fire the gate: {}",
        text(&out)
    );
}

#[test]
fn a_renamed_symbol_is_reported_as_the_staleness_it_is() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    run(&root, &["docs", "--accept"]);

    let src = root.join("src/registry.rs");
    let body = std::fs::read_to_string(&src).unwrap();
    std::fs::write(
        &src,
        body.replace("pub static VENDORS", "pub static PROVIDERS"),
    )
    .unwrap();

    let out = run(&root, &["docs", "--check"]);
    assert!(!out.status.success(), "a vanished symbol must fail");
    assert!(
        text(&out).contains("has no"),
        "the message must say the symbol is gone rather than hashing the \
         whole file and passing: {}",
        text(&out)
    );
}

#[test]
fn a_product_with_no_annotations_is_not_a_finding() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "bare"]);
    assert!(out.status.success());

    // Opt-in per block. Annotating everything is the ceremony that kills the
    // mechanism; a document with no markers is not a failure.
    let out = run(&tmp.path().join("bare"), &["docs", "--check"]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn an_unclosed_marker_is_an_error_rather_than_a_silently_ignored_block() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    write(
        &root,
        "docs/broken.md",
        "<!-- fid:describes src/registry.rs -->\nprose with no close\n",
    );

    let out = run(&root, &["docs", "--check"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("never closed"), "{}", text(&out));
}

//! `fid add` edits `fiducial.toml`; it does not re-serialise it.
//!
//! `fiducial.toml` is the one file in a product where a human writes down the
//! judgments nothing can derive — which KV id is load-bearing, why a Worker
//! name matches by declaration rather than coincidence, what an artbox
//! measures. Principle 1b says a judgment that cannot be derived is still
//! declared, and in practice those declarations are the comments.
//!
//! `patch_config` used to read the file into the typed `Config` and write
//! `toml::to_string_pretty` back. That round-trip has nowhere to put a comment,
//! so **every `fid add` silently deleted the entire authored commentary** and
//! reflowed the tables besides. On 2026-09-23 one `fid add seo` destroyed 27
//! comment lines in a product. Every value survived, nothing failed, and the
//! loss is invisible unless you diff a file you did not expect to change.
//!
//! These tests are the assertion that was missing.

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

fn config(root: &Path) -> String {
    std::fs::read_to_string(root.join("fiducial.toml")).expect("reading fiducial.toml")
}

/// A comment of each kind a real product carries, plus an inline table and a
/// hand-ordered block — the shapes a typed round-trip destroys.
const AUTHORED: &str = r#"
# The namespace holding the spot count. The id is load-bearing in the worst
# way: point a deploy at a different one and the signups are silently gone
# rather than missing.
[deploy]
# The Worker name is the one that already exists. A generated default would
# have been the product name, which happens to match — but matching by
# coincidence is not the same as being declared.
name = "upoznaj-biznis"
vars = { SPOTS_DEFAULT = "14" }

# The artbox is the *cap box of the letterforms* — U left edge to B right
# edge, cap line down to the U's baseline overshoot.
[raster]
source_dir = "apps/web/static/brand"
"#;

fn product_with_authored_config(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    let path = root.join("fiducial.toml");
    let mut body = std::fs::read_to_string(&path).unwrap();
    body.push_str(AUTHORED);
    std::fs::write(&path, body).unwrap();
    root
}

/// Every comment line present before `fid add` is present after it.
#[test]
fn authored_comments_survive_fid_add() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_authored_config(tmp.path());

    let before: Vec<String> = config(&root)
        .lines()
        .filter(|l| l.trim_start().starts_with('#'))
        .map(str::to_string)
        .collect();
    assert!(before.len() > 5, "fixture carries comments: {before:?}");

    let out = run(&root, &["add", "capability", "i18n"]);
    assert!(out.status.success(), "fid add: {}", text(&out));

    let after = config(&root);
    let missing: Vec<&String> = before.iter().filter(|l| !after.contains(*l)).collect();
    assert!(
        missing.is_empty(),
        "`fid add` deleted authored comments:\n{missing:#?}\n\nresulting file:\n{after}"
    );
}

/// …and they still survive when several capabilities are added in turn.
///
/// One `fid add` losing nothing is not the property. Products install
/// capabilities one at a time over months, so the loss has to be absent from
/// every step, not just the first.
#[test]
fn comments_survive_repeated_adds() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_authored_config(tmp.path());

    let before: Vec<String> = config(&root)
        .lines()
        .filter(|l| l.trim_start().starts_with('#'))
        .map(str::to_string)
        .collect();

    for capability in ["i18n", "content", "brand"] {
        let out = run(&root, &["add", "capability", capability]);
        assert!(out.status.success(), "fid add {capability}: {}", text(&out));
    }

    let after = config(&root);
    let missing: Vec<&String> = before.iter().filter(|l| !after.contains(*l)).collect();
    assert!(missing.is_empty(), "lost after three adds:\n{missing:#?}");
}

/// Values a human wrote keep the shape they were written in.
///
/// The round-trip rewrote `vars = { … }` as a `[deploy.vars]` table and
/// re-sorted every block alphabetically. Both are legal TOML and neither is
/// what the file said.
#[test]
fn hand_written_formatting_is_not_reflowed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_authored_config(tmp.path());

    assert!(run(&root, &["add", "capability", "i18n"]).status.success());

    let after = config(&root);
    assert!(
        after.contains(r#"vars = { SPOTS_DEFAULT = "14" }"#),
        "inline table was expanded into a block:\n{after}"
    );
    assert!(
        !after.contains("[deploy.vars]"),
        "inline table was expanded into a block:\n{after}"
    );
}

/// The header names the last command to touch the file — one header, not a pile.
///
/// Leading trivia in a TOML document attaches to the first *item*, not to the
/// root table, so an earlier fix that rewrote the root's prefix found nothing
/// and prepended a fresh header on every run. Three adds left three headers,
/// two of them lying about which command wrote the file.
#[test]
fn the_header_is_replaced_not_stacked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_authored_config(tmp.path());

    for capability in ["i18n", "content", "brand"] {
        assert!(run(&root, &["add", "capability", capability])
            .status
            .success());
    }

    let after = config(&root);
    let headers = after
        .lines()
        .filter(|l| l.starts_with("# fiducial.toml — updated by"))
        .count();
    assert_eq!(headers, 1, "expected exactly one header:\n{after}");
    assert!(
        after
            .lines()
            .next()
            .is_some_and(|l| l.contains("fid add brand")),
        "the header names the most recent command:\n{after}"
    );
}

/// An array written one-per-line stays one-per-line.
#[test]
fn a_multiline_array_keeps_its_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_authored_config(tmp.path());

    assert!(run(&root, &["add", "capability", "i18n"]).status.success());
    assert!(run(&root, &["add", "capability", "content"])
        .status
        .success());

    let after = config(&root);
    assert!(
        after.contains("enabled = [\n    \"i18n\",\n    \"content\",\n]"),
        "capabilities.enabled should be one per line:\n{after}"
    );
}

/// The file `fid add` writes is still valid TOML that `fid` itself can read.
///
/// Format preservation is worth nothing if it preserves the format of a broken
/// file, so this asserts against the binary rather than against a parser.
#[test]
fn the_patched_config_is_still_readable_by_fid() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_authored_config(tmp.path());

    assert!(run(&root, &["add", "capability", "i18n"]).status.success());

    let out = run(&root, &["dash", "--json"]);
    assert!(out.status.success(), "fid dash: {}", text(&out));
    let parsed: Result<serde_json::Value, _> =
        serde_json::from_str(&String::from_utf8_lossy(&out.stdout));
    assert!(parsed.is_ok(), "dash did not return JSON: {}", text(&out));
}

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
    PathBuf::from(env!("CARGO_BIN_EXE_fid"))
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

/// Everything the interface says is editable, in every language, and settings
/// that are the same in every language are one file.
///
/// The editor used to cover only the markdown collections, so a button label
/// or a link meant opening a data file. `messages/<locale>.json` (one entry per
/// language) and `settings.toml` are now a Settings group of file forms, read
/// from the files, and `settings.toml` is exported typed from the content module.
#[test]
fn the_editor_covers_interface_text_and_settings() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    std::fs::write(
        root.join("settings.toml"),
        "survey_url = \"https://example.test/s\"\n\n[credits]\nmaker = \"Name\"\n",
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let config = std::fs::read_to_string(root.join("tools/cms/config.yml")).unwrap();

    assert!(config.contains("name: settings"), "{config}");
    assert!(config.contains("file: messages/en.json"), "{config}");
    assert!(config.contains("file: messages/sr.json"), "{config}");
    assert!(config.contains("file: settings.toml"), "{config}");
    assert!(config.contains("name: survey_url"), "{config}");
    assert!(config.contains("name: maker"), "{config}");

    // The same file, typed, from the generated content module.
    let module = std::fs::read_to_string(root.join("apps/web/src/generated/content.ts")).unwrap();
    assert!(module.contains("export const settings"), "{module}");
    assert!(module.contains("https://example.test/s"), "{module}");

    let out = run(&root, &["derive", "--check"]);
    assert!(out.status.success(), "{}", text(&out));
}

/// Partial dates and lists of small records, from a product that needed both.
/// A key that is not declared inside a record fails the derive, as an
/// undeclared field does.
#[test]
fn partial_dates_and_object_lists_are_checked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    let content = root.join("content.toml");
    let declared = std::fs::read_to_string(&content).unwrap();
    std::fs::write(
        &content,
        format!(
            "{declared}\n[collections.works]\nkind = \"collection\"\nschema = {{ title = \"string\", started = \"partial-date\", links = \"object[]\" }}\nbody = \"none\"\nroute = \"\"\n\n[collections.works.objects]\nlinks = {{ label = \"string\", url = \"string\" }}\n"
        ),
    )
    .unwrap();
    let entry = |extra: &str| {
        format!(
            "---\ntitle: A\nstarted: 2026-03\nlinks:\n  - label: Home\n    url: https://example.test\n{extra}---\n"
        )
    };
    for locale in ["en", "sr"] {
        let dir = root.join("content/works").join(locale);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), entry("")).unwrap();
    }
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));

    // An undeclared key in a record is refused.
    let bad = "---\ntitle: A\nstarted: \"2026\"\nlinks:\n  - label: Home\n    url: https://example.test\n    extra: no\n---\n";
    std::fs::write(root.join("content/works/en/a.md"), bad).unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "an undeclared record key must fail");
    assert!(
        text(&out).contains("is not declared in content.toml"),
        "{}",
        text(&out)
    );
}

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
    let config = std::fs::read_to_string(root.join("apps/cms/keystatic.config.ts"))
        .expect("the editor config");
    // One editable collection per locale, paths from the repository root.
    assert!(config.contains("posts_en"), "{config}");
    assert!(config.contains("posts_sr"), "{config}");
    assert!(config.contains("\"content/posts/en/*\""), "{config}");
    // Plain `.md`, so Keystatic and Tina edit the same files.
    assert!(config.contains("extension: \"md\""), "{config}");
    let tina = std::fs::read_to_string(root.join("apps/cms/tina/config.ts")).unwrap();
    assert!(tina.contains("posts_en"), "{tina}");
    // No Decap anywhere: not the config, not the proxy, not the CDN script.
    assert!(!root.join("tools/cms").exists());

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

/// The editor is a choice, and it changes the installed packages and nothing
/// else: both configs are always derived and both edit the same files.
#[test]
fn the_editor_is_chosen_by_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    let pkg = |root: &Path| std::fs::read_to_string(root.join("apps/cms/package.json")).unwrap();

    assert!(run(&root, &["derive"]).status.success());
    let keystatic = pkg(&root);
    assert!(keystatic.contains("@keystatic/next"), "{keystatic}");
    assert!(!keystatic.contains("tinacms"), "{keystatic}");

    let toml = root.join("fiducial.toml");
    let declared = std::fs::read_to_string(&toml).unwrap();
    assert!(declared.contains("editor = \"keystatic\""), "{declared}");
    std::fs::write(
        &toml,
        declared.replace("editor = \"keystatic\"", "editor = \"tina\""),
    )
    .unwrap();
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let tina = pkg(&root);
    assert!(tina.contains("tinacms"), "{tina}");
    assert!(!tina.contains("@keystatic"), "{tina}");

    std::fs::write(
        &toml,
        declared.replace("editor = \"keystatic\"", "editor = \"decap\""),
    )
    .unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "an unknown editor must fail");
    assert!(text(&out).contains("is not one of"), "{}", text(&out));
}

/// Everything the interface says is editable, in every language, and settings
/// that are the same in every language are one file.
///
/// The editor used to cover only the markdown collections, so a button label
/// or a link meant opening a data file. `messages/<locale>.json` (one entry per
/// language) and `settings.json` are in a Settings group of file forms, read
/// from the files, and `settings.json` is exported typed from the content module.
#[test]
fn the_editor_covers_interface_text_and_settings() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    std::fs::write(
        root.join("settings.json"),
        "{ \"survey_url\": \"https://example.test/s\", \"credits\": { \"maker\": \"Name\" } }",
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let config = std::fs::read_to_string(root.join("apps/cms/keystatic.config.ts")).unwrap();

    assert!(config.contains("settings: singleton"), "{config}");
    assert!(config.contains("\"messages/en\""), "{config}");
    assert!(config.contains("\"messages/sr\""), "{config}");
    assert!(config.contains("\"settings\""), "{config}");
    assert!(config.contains("\"survey_url\""), "{config}");
    assert!(config.contains("\"maker\""), "{config}");

    // The same file, typed, from the generated content module.
    let module = std::fs::read_to_string(root.join("apps/web/src/generated/content.ts")).unwrap();
    assert!(module.contains("export const settings"), "{module}");
    assert!(module.contains("https://example.test/s"), "{module}");

    let out = run(&root, &["derive", "--check"]);
    assert!(out.status.success(), "{}", text(&out));

    // The old TOML file is refused with the way out, not silently ignored.
    std::fs::write(root.join("settings.toml"), "a = \"b\"\n").unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "a leftover settings.toml must fail");
    assert!(text(&out).contains("settings.json"), "{}", text(&out));
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
    let config = std::fs::read_to_string(root.join("apps/cms/keystatic.config.ts")).unwrap();
    assert!(config.contains("fields.array(fields.object("), "{config}");

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

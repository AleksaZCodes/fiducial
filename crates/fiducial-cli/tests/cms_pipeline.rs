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
    assert!(config.contains("survey_url: fields.text"), "{config}");
    assert!(config.contains("maker: fields.text"), "{config}");

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

/// Append a collection to `content.toml` and write its entry in every locale.
fn add_collection(root: &Path, decl: &str, slug: &str, en: &str, sr: &str) {
    let content = root.join("content.toml");
    let declared = std::fs::read_to_string(&content).unwrap();
    std::fs::write(&content, format!("{declared}\n{decl}\n")).unwrap();
    let name = decl
        .lines()
        .find_map(|l| l.strip_prefix("[collections.")?.strip_suffix(']'))
        .unwrap();
    for (locale, text) in [("en", en), ("sr", sr)] {
        let dir = root.join("content").join(name).join(locale);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{slug}.md")), text).unwrap();
    }
}

fn append(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    let before = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{before}\n{text}\n")).unwrap();
}

/// A collection with no body is still `.md` in the editor. Keystatic writes
/// `.yaml` for a frontmatter-only entry unless the body is `emptyContent`, and
/// the collection then shows up empty while every file is fine.
#[test]
fn a_collection_without_a_body_stays_markdown() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    let entry = "---\ntitle: A\n---\n";
    add_collection(
        &root,
        "[collections.links]\nkind = \"collection\"\nschema = { title = \"string\" }\nbody = \"none\"\nroute = \"\"",
        "a",
        entry,
        entry,
    );
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let config = std::fs::read_to_string(root.join("apps/cms/keystatic.config.ts")).unwrap();
    assert!(
        config.contains("fields.emptyContent({ extension: \"md\" })"),
        "{config}"
    );
    assert!(!config.contains("\"yaml\""), "{config}");
}

/// `[cms.selects]` keys are quoted ("collection.field"). The TOML reader used
/// to skip quoted keys without a word, so no select, hint or relation ever
/// reached the editor. A value outside the choices is refused, because the
/// editor would show the default and overwrite it on the next save; a choice
/// among numbers is a number field, because a select stores text.
#[test]
fn selects_reach_the_editor_and_guard_the_content() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    let entry = |status: &str| format!("---\ntitle: A\nstatus: {status}\ntier: 2\n---\n");
    add_collection(
        &root,
        "[collections.works]\nkind = \"collection\"\nschema = { title = \"string\", status = \"string\", tier = \"number\" }\nbody = \"none\"\nroute = \"\"",
        "a",
        &entry("active"),
        &entry("active"),
    );
    append(
        &root,
        "fiducial.toml",
        "[cms.selects]\n\"works.status\" = [\"active\", \"parked\"] # a comment\n\"works.tier\" = [1, 2, 3]\n\n[cms.hints]\n\"works.title\" = \"What it is called\"",
    );
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let config = std::fs::read_to_string(root.join("apps/cms/keystatic.config.ts")).unwrap();
    assert!(config.contains("fields.select("), "{config}");
    assert!(config.contains("value: \"parked\""), "{config}");
    assert!(config.contains("One of: 1, 2, 3"), "{config}");
    assert!(config.contains("What it is called"), "{config}");

    std::fs::write(root.join("content/works/en/a.md"), entry("bogus")).unwrap();
    std::fs::write(root.join("content/works/sr/a.md"), entry("bogus")).unwrap();
    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "a value outside the select must fail"
    );
    assert!(
        text(&out).contains("not one of [cms.selects]"),
        "{}",
        text(&out)
    );
}

/// An editor saves a JSON file from its form, so a key the form cannot hold is
/// deleted on the first save. It fails the derive, naming the file and key.
#[test]
fn a_json_key_the_editor_would_drop_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    std::fs::write(root.join("settings.json"), "{ \"nav.home\": \"/\" }").unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "a dotted key must fail");
    assert!(
        text(&out).contains("settings.json: `nav.home`"),
        "{}",
        text(&out)
    );

    std::fs::write(root.join("settings.json"), "{ \"video\": null }").unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "a null must fail");
    assert!(text(&out).contains("is null"), "{}", text(&out));
}

/// A date is a day whoever wrote it: Tina saves midnight UTC as a timestamp,
/// and that reads as the day. And it is one fact in every language: copies that
/// disagree fail, because each editor keeps a language in its own file.
#[test]
fn dates_are_days_and_agree_across_locales() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    let decl = "[collections.events]\nkind = \"collection\"\nschema = { title = \"string\", date = \"date\" }\nbody = \"none\"\nroute = \"\"";
    add_collection(
        &root,
        decl,
        "a",
        "---\ntitle: A\ndate: 2026-02-03T00:00:00.000Z\n---\n",
        "---\ntitle: А\ndate: 2026-02-03\n---\n",
    );
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let module = std::fs::read_to_string(root.join("apps/web/src/generated/content.ts")).unwrap();
    assert!(module.contains("\"date\": \"2026-02-03\""), "{module}");
    assert!(!module.contains("T00:00"), "{module}");

    std::fs::write(
        root.join("content/events/sr/a.md"),
        "---\ntitle: А\ndate: 2026-02-04\n---\n",
    )
    .unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "dates that disagree must fail");
    assert!(
        text(&out).contains("differs between locales"),
        "{}",
        text(&out)
    );

    // A time of day is not cut off: that could move the date.
    std::fs::write(
        root.join("content/events/sr/a.md"),
        "---\ntitle: А\ndate: 2026-02-03T23:00:00.000Z\n---\n",
    )
    .unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "a time of day must fail");
    assert!(text(&out).contains("has a time of day"), "{}", text(&out));
}

/// A picture inside a list of records is referenced. It was not, so
/// `media:push --prune` would have deleted every gallery image from the bucket.
#[test]
fn gallery_images_are_referenced_media() {
    let tmp = tempfile::tempdir().unwrap();
    let root = cms_product(tmp.path());
    let entry = "---\ntitle: A\ngallery:\n  - image: site/works/a/one.png\n    caption: One\n---\n";
    add_collection(
        &root,
        "[collections.works]\nkind = \"collection\"\nschema = { title = \"string\", gallery = \"object[]\" }\nbody = \"none\"\nroute = \"\"\n\n[collections.works.objects]\ngallery = { image = \"media\", caption = \"string\" }",
        "a",
        entry,
        entry,
    );
    let out = Command::new("node")
        .args([
            "--input-type=module",
            "-e",
            "import { referencedKeys } from './scripts/media-keys.mjs'; console.log([...referencedKeys()].join(','));",
        ])
        .current_dir(&root)
        .output()
        .expect("running node");
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("site/works/a/one.png"),
        "{}",
        text(&out)
    );
}

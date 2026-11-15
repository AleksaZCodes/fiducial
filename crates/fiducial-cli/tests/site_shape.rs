//! `fid new <name> --shape site`: a localized website in one command.
//!
//! What can be checked without a network: the tree the shape writes derives,
//! passes `--check`, gives `fid doctor` nothing to say, and is wired — the plain
//! root layout is gone, the locale routes and their words are there, and the
//! deploy config is the app's own. `scripts/verify-web-next.sh` installs,
//! builds and dry-runs the deploy of the same tree.

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

fn site(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "site", "--shape", "site"]);
    assert!(out.status.success(), "{}", text(&out));
    tmp.join("site")
}

#[test]
fn a_site_is_born_derived_checked_and_quiet() {
    let tmp = tempfile::tempdir().unwrap();
    let root = site(tmp.path());

    let out = run(&root, &["derive", "--check"]);
    assert!(out.status.success(), "{}", text(&out));

    // Nothing to upgrade and nothing modified, on a tree written a second ago.
    let doctor = text(&run(&root, &["doctor"]));
    assert!(!doctor.contains("upstream template updated"), "{doctor}");
    assert!(
        doctor.contains("no platform-owned file has been modified"),
        "{doctor}"
    );
}

#[test]
fn the_site_replaces_the_plain_app_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = site(tmp.path());
    let web = root.join("apps/web");

    // Two `<html>`s cannot coexist: the plain root layout and page are gone,
    // and the locale-aware ones are there.
    assert!(!web.join("src/app/layout.tsx").exists());
    assert!(!web.join("src/app/page.tsx").exists());
    for page in [
        "src/app/[locale]/layout.tsx",
        "src/app/[locale]/page.tsx",
        "src/app/[locale]/[...path]/page.tsx",
        "src/app/[locale]/legal/[page]/page.tsx",
        "src/lib/metadata.ts",
        "src/generated/fonts.ts",
    ] {
        assert!(web.join(page).exists(), "missing {page}");
    }
    // The URL rules are the site's, not the empty default.
    let routing = std::fs::read_to_string(web.join("src/routing.ts")).unwrap();
    assert!(routing.contains("defaultLocale"), "{routing}");
    let lock = std::fs::read_to_string(root.join("fiducial.lock")).unwrap();
    assert!(!lock.contains("[templates.\"apps/web/src/app/layout.tsx\"]"));

    // What the pages import is installed.
    let pkg = std::fs::read_to_string(web.join("package.json")).unwrap();
    for dep in ["\"marked\"", "\"@opennextjs/cloudflare\"", "\"wrangler\""] {
        assert!(pkg.contains(dep), "{dep} missing: {pkg}");
    }
    assert!(
        pkg.contains("\"deploy\": \"opennextjs-cloudflare build"),
        "{pkg}"
    );
}

#[test]
fn the_words_the_pages_print_exist_in_every_locale() {
    let tmp = tempfile::tempdir().unwrap();
    let root = site(tmp.path());
    for locale in ["en", "sr"] {
        let raw = std::fs::read_to_string(root.join(format!("messages/{locale}.json"))).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        for key in ["readMore", "notFound", "latest"] {
            assert!(json["site"][key].is_string(), "{locale}: site.{key}\n{raw}");
        }
        assert!(
            json["press"]["angle"]["origin"].is_string(),
            "{locale}\n{raw}"
        );
    }
    let sr = std::fs::read_to_string(root.join("messages/sr.json")).unwrap();
    assert!(
        sr.contains("Pročitaj više"),
        "sr got the English seed: {sr}"
    );
}

#[test]
fn the_app_is_the_worker() {
    let tmp = tempfile::tempdir().unwrap();
    let root = site(tmp.path());
    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("shape = \"nextjs\""), "{config}");
    let wrangler = std::fs::read_to_string(root.join("apps/web/wrangler.jsonc")).unwrap();
    assert!(wrangler.contains("\".open-next/worker.js\""), "{wrangler}");
    assert!(
        !wrangler.contains("\"vars\""),
        "an empty vars block: {wrangler}"
    );
    assert!(!root.join("apps/worker/wrangler.toml").exists());
}

#[test]
fn press_added_later_gets_its_pages_and_every_locale() {
    let tmp = tempfile::tempdir().unwrap();
    let root = site(tmp.path());
    let out = run(&root, &["add", "capability", "press"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(root
        .join("apps/web/src/app/[locale]/press/page.tsx")
        .exists());
    // The press room was seeded in English only, and a two-locale product
    // failed its first derive on files the platform wrote.
    assert!(root.join("press/docs/sr/boilerplate.md").exists());
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
    let seo = std::fs::read_to_string(root.join("apps/web/src/generated/seo.ts")).unwrap();
    assert!(
        seo.contains("\"page/press\""),
        "the press room is not in the route table"
    );
}

#[test]
fn seeding_locales_keeps_the_config_comments() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(
        tmp.path(),
        &["new", "p", "--locales", "en,sr", "--default-locale", "en"],
    );
    assert!(out.status.success(), "{}", text(&out));
    let config = std::fs::read_to_string(tmp.path().join("p/fiducial.toml")).unwrap();
    assert!(
        config.lines().filter(|l| l.starts_with('#')).count() > 10,
        "{config}"
    );
    assert!(config.contains("locales = [\"en\", \"sr\"]"), "{config}");
}

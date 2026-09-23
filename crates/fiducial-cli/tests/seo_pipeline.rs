//! End-to-end: `fid add seo` → `fid derive` → the sitemap.
//!
//! The `seo` capability shipped with no test at all, and it showed. Three
//! separate defects were sitting in it, each one invisible to the product that
//! it was written against and fatal to any other:
//!
//!   1. The deriver hard-coded Next.js's `public/…` output paths while the
//!      pipeline *declared* them, so the documented one-line change for a
//!      SvelteKit product ("`public` → `static`") left the script writing
//!      somewhere the pipeline did not guard.
//!   2. It imported `parseToml` from `derive-content.mjs`, a pipeline executor
//!      with no main-module guard — so running THIS pipeline re-derived the
//!      whole content model as an import side effect, writing a file it never
//!      declared as an output.
//!   3. The two-pipelines-one-sitemap guard matched a full Next.js path, so it
//!      went quiet in exactly the products whose paths differ — the ones that
//!      had to edit the pipeline, and therefore the ones most likely to leave
//!      both owners in place.
//!
//! Each test below is one of those, plus the declaration that replaced the
//! script's hard-coded table of one product's four routes.

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

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
}

fn write(root: &Path, rel: &str, body: &str) {
    std::fs::write(root.join(rel), body).unwrap_or_else(|e| panic!("writing {rel}: {e}"));
}

/// Replace the first non-comment line matching `needle`, or drop it when `to`
/// is None.
///
/// Comments are skipped, and only the first match is touched. Both matter:
/// these pipeline files discuss their own paths in prose, so a helper that
/// rewrote every hit turned a comment into a bare array element and produced
/// TOML that failed to parse — a fixture bug that reads exactly like the
/// product bug under test.
fn edit_line(root: &Path, rel: &str, needle: &str, to: Option<&str>) {
    let body = read(root, rel);
    let mut out = String::new();
    let mut hit = false;
    for line in body.lines() {
        if !hit && !line.trim_start().starts_with('#') && line.contains(needle) {
            hit = true;
            if let Some(replacement) = to {
                out.push_str(replacement);
                out.push('\n');
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    assert!(hit, "no line containing `{needle}` in {rel}:\n{body}");
    write(root, rel, &out);
}

/// The home page's title and description, which `[seo] pages = ["/"]` resolves
/// to by convention. The scaffolded catalogues do not carry them.
fn seed_home_meta(root: &Path) {
    for locale in ["en", "sr"] {
        let rel = format!("messages/{locale}.json");
        let body = read(root, &rel);
        let mut value: serde_json::Value = serde_json::from_str(&body).expect("catalogue is JSON");
        value["meta"] = serde_json::json!({
            "title": format!("Title {locale}"),
            "description": format!("Description {locale}"),
        });
        write(
            root,
            &rel,
            &format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
        );
    }
}

/// A scaffolded product with `seo` and everything it requires installed.
///
/// The sitemap is handed over from `brand` here, because that is the documented
/// adoption step and every test below is about what happens afterwards. The one
/// test that is about the handover itself puts the line back.
fn seo_product(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    for capability in ["i18n", "content", "brand", "seo"] {
        let out = run(&root, &["add", "capability", capability]);
        assert!(out.status.success(), "fid add {capability}: {}", text(&out));
    }
    seed_home_meta(&root);
    seed_every_locales_posts(&root);
    edit_line(&root, "pipelines/brand.toml", "sitemap.xml", None);
    root
}

/// The scaffold seeds `content/posts/en/hello.md` and nothing for the second
/// locale, so the `content` pipeline fails before `seo` ever runs — a missing
/// translation is a missing artifact, which is the platform working as
/// intended and not what these tests are about. Mirroring the entry is the
/// fixture's job.
fn seed_every_locales_posts(root: &Path) {
    let dir = root.join("content/posts");
    let from = dir.join("en");
    let entries: Vec<_> = std::fs::read_dir(&from)
        .expect("the scaffold seeds content/posts/en")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    for locale in ["sr"] {
        let to = dir.join(locale);
        std::fs::create_dir_all(&to).unwrap();
        for entry in &entries {
            let name = entry.file_name().expect("a file name");
            std::fs::copy(entry, to.join(name)).unwrap();
        }
    }
}

// ── The dependency it never declared ────────────────────────────────────────

/// Every URL the sitemap emits starts with `[brand] domain`, and the deriver
/// reads its own pipeline with a parser `brand` ships. Installing `seo` without
/// it used to fail on a missing import, one layer away from the real cause.
#[test]
fn seo_refuses_to_install_without_brand() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    for capability in ["i18n", "content"] {
        assert!(
            run(&root, &["add", "capability", capability])
                .status
                .success(),
            "fid add {capability}"
        );
    }

    let out = run(&root, &["add", "capability", "seo"]);
    assert!(
        !out.status.success(),
        "install should refuse: {}",
        text(&out)
    );
    let said = text(&out);
    assert!(
        said.contains("brand"),
        "names the missing capability: {said}"
    );
}

// ── The derivation ──────────────────────────────────────────────────────────

#[test]
fn derive_writes_the_three_declared_artifacts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));

    let sitemap = read(&root, "apps/web/public/sitemap.xml");
    assert!(sitemap.contains("<urlset"), "{sitemap}");
    assert!(
        sitemap.contains("xmlns:xhtml"),
        "alternates need the xhtml namespace: {sitemap}"
    );
    assert!(
        sitemap.contains(r#"hreflang="x-default""#),
        "x-default names the default locale's URL: {sitemap}"
    );

    let ts = read(&root, "apps/web/src/generated/seo.ts");
    assert!(ts.contains("export const seoRoutes"), "{ts}");
    // `pages = ["/"]`, a bare path, still resolves its keys by convention.
    assert!(
        ts.contains("Title en"),
        "home title from the catalogue: {ts}"
    );

    let json = read(&root, "apps/web/src/generated/seo.json");
    let json: serde_json::Value = serde_json::from_str(&json).expect("seo.json is JSON");
    assert!(json["routes"].as_array().is_some_and(|r| !r.is_empty()));
}

/// Every page exists once per language, with the default locale unprefixed.
#[test]
fn every_route_is_emitted_in_every_locale() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let json = read(&root, "apps/web/src/generated/seo.json");
    let json: serde_json::Value = serde_json::from_str(&json).unwrap();
    let default = json["defaultLocale"].as_str().expect("a default locale");
    let routes = json["routes"].as_array().unwrap();

    let home: Vec<_> = routes
        .iter()
        .filter(|r| r["key"] == "page/")
        .map(|r| (r["locale"].as_str().unwrap(), r["path"].as_str().unwrap()))
        .collect();
    assert_eq!(home.len(), 2, "one home per locale: {home:?}");

    // The default locale owns the bare path; the other carries its prefix.
    for (locale, path) in &home {
        if *locale == default {
            assert_eq!(*path, "/", "the default locale is unprefixed: {home:?}");
        } else {
            assert_eq!(
                *path,
                &format!("/{locale}"),
                "a non-default locale is prefixed: {home:?}"
            );
        }
    }
}

// ── The output paths are the pipeline's declaration ─────────────────────────

/// Moving the outputs is a one-line edit to the pipeline, and sufficient.
///
/// This is the SvelteKit case. The deriver used to hard-code `public/…`, so
/// this edit produced a sitemap at a path the pipeline did not declare: the
/// declared one was simply missing, and `fid derive --check` failed pointing at
/// neither the cause nor the fix.
#[test]
fn outputs_land_where_the_pipeline_declares_them() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());

    edit_line(
        &root,
        "pipelines/seo.toml",
        "apps/web/public/sitemap.xml",
        Some(r#"  "apps/web/static/sitemap.xml","#),
    );

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));

    assert!(
        root.join("apps/web/static/sitemap.xml").exists(),
        "the declared path is the one written: {}",
        text(&out)
    );
    assert!(
        !root.join("apps/web/public/sitemap.xml").exists(),
        "and nothing is left at the old one"
    );

    assert!(
        run(&root, &["derive", "--check"]).status.success(),
        "the gate agrees with the declaration"
    );
}

/// An output the deriver does not produce is named, not silently skipped.
#[test]
fn a_missing_declared_output_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());

    edit_line(&root, "pipelines/seo.toml", "sitemap.xml", None);

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "should refuse: {}", text(&out));
    let said = text(&out);
    assert!(
        said.contains("sitemap.xml"),
        "says which artifact has no declared path: {said}"
    );
}

// ── No undeclared writes ────────────────────────────────────────────────────

/// Running the `seo` pipeline writes what `seo` declares, and nothing else.
///
/// It used to also rewrite `generated/content.ts`, because it imported one
/// helper from `derive-content.mjs` — a pipeline executor whose body runs on
/// import. The artifact was never stale, so nothing failed; the bug was a file
/// changing under a pipeline that never claimed it.
#[test]
fn the_seo_pipeline_does_not_write_another_pipelines_artifact() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let content = root.join("apps/web/src/generated/content.ts");
    let before = std::fs::read_to_string(&content).ok();

    // A marker that survives only if nothing rewrites the file.
    if before.is_some() {
        let marked = format!("// marker\n{}", before.clone().unwrap());
        std::fs::write(&content, &marked).unwrap();
    }

    let out = Command::new("node")
        .arg("scripts/derive-seo.mjs")
        .current_dir(&root)
        .output()
        .expect("running the deriver directly");
    assert!(
        out.status.success(),
        "deriver: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    if before.is_some() {
        let after = std::fs::read_to_string(&content).unwrap();
        assert!(
            after.starts_with("// marker"),
            "content.ts was rewritten by the seo pipeline"
        );
    }
}

// ── The handover off `brand` ────────────────────────────────────────────────

/// Two pipelines owning one artifact is refused whatever directory they spell.
///
/// The guard used to match the literal `apps/web/public/sitemap.xml`, so it was
/// silent for a product whose pipelines say `static/` — which is every product
/// that had to edit the paths, and therefore the population most likely to have
/// left both owners in place.
#[test]
fn both_pipelines_owning_the_sitemap_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());

    // Hand it back to brand, at a path that is not the Next.js one.
    edit_line(
        &root,
        "pipelines/seo.toml",
        "apps/web/public/sitemap.xml",
        Some(r#"  "apps/web/static/sitemap.xml","#),
    );
    let brand = read(&root, "pipelines/brand.toml");
    let brand = brand.replacen(
        "outputs  = [",
        "outputs  = [\n  \"apps/web/static/sitemap.xml\",",
        1,
    );
    write(&root, "pipelines/brand.toml", &brand);

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "should refuse: {}", text(&out));
    let said = text(&out);
    assert!(
        said.contains("sitemap.xml") && said.contains("brand.toml"),
        "names the file and the pipeline to edit: {said}"
    );
}

// ── The pages declaration ───────────────────────────────────────────────────

/// A page declares the catalogue keys its title and description live under.
///
/// The deriver used to hold a table of four paths with one product's key
/// spellings baked in. Any other page set hit "no title is declared for it",
/// which named the wrong problem: the page had a title, under a key this script
/// had never heard of.
#[test]
fn a_page_can_declare_its_own_message_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());

    for locale in ["en", "sr"] {
        let rel = format!("messages/{locale}.json");
        let mut value: serde_json::Value = serde_json::from_str(&read(&root, &rel)).unwrap();
        value["workshop"] = serde_json::json!({
            "heading": format!("Workshop {locale}"),
            "blurb": format!("Blurb {locale}"),
        });
        write(
            &root,
            &rel,
            &format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
        );
    }

    edit_line(
        &root,
        "fiducial.toml",
        "pages = ",
        Some(
            r#"pages = [
    { path = "/", title = "meta.title", description = "meta.description" },
    { path = "/workshop", title = "workshop.heading", description = "workshop.blurb" },
]"#,
        ),
    );

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));

    let sitemap = read(&root, "apps/web/public/sitemap.xml");
    assert!(sitemap.contains("/workshop"), "{sitemap}");
    let ts = read(&root, "apps/web/src/generated/seo.ts");
    assert!(ts.contains("Workshop en"), "{ts}");
}

/// A bare path the convention does not cover says how to declare it.
#[test]
fn an_unknown_bare_page_names_the_fix() {
    let tmp = tempfile::tempdir().unwrap();
    let root = seo_product(tmp.path());

    edit_line(
        &root,
        "fiducial.toml",
        "pages = ",
        Some(r#"pages = ["/workshop"]"#),
    );

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "should refuse: {}", text(&out));
    let said = text(&out);
    assert!(
        said.contains("/workshop") && said.contains("title ="),
        "shows the declaration to write: {said}"
    );
}

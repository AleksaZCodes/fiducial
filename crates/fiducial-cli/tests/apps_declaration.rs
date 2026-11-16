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
fn an_intended_choice_fails_doctor_with_what_is_selectable() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(
        tmp.path(),
        "[apps.web]\nframework = \"next\"\neditor = \"pages\"\n",
    );
    let out = text(&run(&root, &["doctor"]));
    assert!(
        out.contains("names an intended choice that nothing builds yet"),
        "{out}"
    );
    assert!(out.contains("Selectable today: keystatic, tina"), "{out}");
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

#[test]
fn a_react_router_app_without_its_headers_module_fails_doctor() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), "[apps.web]\nframework = \"react-router\"\n");
    assert!(run(&root, &["add", "design"]).status.success());
    assert!(run(&root, &["add", "app", "router"]).status.success());
    let out = text(&run(&root, &["doctor"]));
    assert!(
        !out.contains("React Router app"),
        "a fresh product was reported unsafe: {out}"
    );

    std::fs::write(
        root.join("apps/web/app/security.ts"),
        "export const responseHeaders = [];\n",
    )
    .unwrap();
    let out = text(&run(&root, &["doctor"]));
    assert!(out.contains("React Router app is missing"), "{out}");
}

#[test]
fn a_tanstack_start_app_without_its_headers_module_fails_doctor() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), "[apps.web]\nframework = \"tanstack-start\"\n");
    assert!(run(&root, &["add", "design"]).status.success());
    assert!(run(&root, &["add", "app", "start"]).status.success());
    let out = text(&run(&root, &["doctor"]));
    assert!(
        !out.contains("TanStack Start app"),
        "a fresh product was reported unsafe: {out}"
    );

    std::fs::write(
        root.join("apps/web/src/security.ts"),
        "export const responseHeaders = [];\n",
    )
    .unwrap();
    let out = text(&run(&root, &["doctor"]));
    assert!(out.contains("TanStack Start app is missing"), "{out}");
}

#[test]
fn an_api_with_an_orm_over_a_database_passes_doctor() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(
        tmp.path(),
        "[adapters]\ndatabase = \"d1\"\n[apps.api]\nframework = \"hono\"\norm = \"drizzle\"\n",
    );
    let out = text(&run(&root, &["doctor"]));
    assert!(
        out.contains("[apps]: 1 app(s), every choice buildable"),
        "{out}"
    );
}

#[test]
fn an_orm_over_no_database_fails_doctor_naming_the_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(
        tmp.path(),
        "[apps.api]\nframework = \"hono\"\norm = \"drizzle\"\n",
    );
    let out = text(&run(&root, &["doctor"]));
    assert!(
        out.contains("a layer over the `database` contract"),
        "{out}"
    );
}

#[test]
fn a_capability_can_add_dependencies_to_another_apps_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    for args in [
        &["add", "app", "api"][..],
        &["add", "capability", "migrations"],
        &["add", "capability", "orm-drizzle"],
    ] {
        let out = run(&root, args);
        assert!(out.status.success(), "{args:?}: {}", text(&out));
    }
    let api = std::fs::read_to_string(root.join("apps/api/package.json")).unwrap();
    assert!(api.contains("\"drizzle-orm\""), "{api}");
    assert!(api.contains("\"drizzle-kit\""), "{api}");
    assert!(
        api.contains("\"db:generate\": \"drizzle-kit generate\""),
        "{api}"
    );
    // Parses, with the dependency in its own section.
    let v: serde_json::Value = serde_json::from_str(&api).unwrap();
    assert!(v["dependencies"]["drizzle-orm"].is_string(), "{api}");
    assert!(v["devDependencies"]["drizzle-kit"].is_string(), "{api}");
    assert!(
        !root.join("apps/web/package.json").exists(),
        "the web app's manifest must not appear or change"
    );
}

/// The React Router template is a full-stack app, not a server-rendered page: a
/// loader to read, an action to write, a form that posts to it, a server-only
/// module, and a resource route. The real behaviour is proved by
/// `scripts/verify-web-router.sh`; this keeps the pieces from being deleted.
#[test]
fn the_react_router_template_is_full_stack() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "design"]).status.success());
    assert!(run(&root, &["add", "app", "router"]).status.success());
    let read = |p: &str| std::fs::read_to_string(root.join("apps/web").join(p)).unwrap();

    let home = read("app/routes/home.tsx");
    assert!(home.contains("export function loader"), "no loader");
    assert!(home.contains("export async function action"), "no action");
    assert!(
        home.contains("<Form method=\"post\">"),
        "no form posting to the action"
    );
    assert!(
        home.contains("status: 400"),
        "the action does not refuse bad input"
    );
    assert!(
        home.contains("greeting.server"),
        "the action does not use the server-only module"
    );
    assert!(root.join("apps/web/app/lib/greeting.server.ts").exists());
    assert!(
        read("app/routes.ts").contains("routes/healthz.ts"),
        "no resource route"
    );
    assert!(read("app/routes/healthz.ts").contains("export function loader"));

    // The framework's baseline: a React older than v8 supports would let an
    // install resolve one that React Router 8 rejects.
    let pkg = read("package.json");
    assert!(pkg.contains("\"react\": \"^19.2.8\""), "{pkg}");
    assert!(pkg.contains("\"react-router\": \"^8."), "{pkg}");
}

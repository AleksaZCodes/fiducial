//! A Next.js product, freshly scaffolded, is one its author can build.
//!
//! Every assertion here is a defect that shipped. A product created on
//! 2026-10-13 with `fid new` + `fid add design` + `fid add app next` reported
//! three upgrades pending on a tree nothing had touched, and its first
//! `next build` failed on: a missing `@/lib/utils`, Radix imports with no
//! Radix installed, `lucide-react` imported and not declared, no Tailwind and
//! no PostCSS config for the `@import "tailwindcss"` its stylesheet opens with,
//! a `components.json` naming a `tailwind.config.ts` that does not exist, and
//! a board page importing a package nothing declared.
//!
//! These checks need no network. The build itself — `pnpm install` and
//! `next build` on the real output — is `scripts/verify-web-next.sh`, run by
//! the `web-next-scaffold` CI job, because generated code asserted as text is
//! not tested.

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

fn scaffold(dir: &Path, app_args: &[&str]) -> PathBuf {
    assert!(run(dir, &["new", "p"]).status.success(), "fid new");
    let root = dir.join("p");
    for args in [
        &["add", "design"][..],
        app_args,
        &["add", "i18n"],
        &["add", "brand"],
    ] {
        let out = run(&root, args);
        assert!(out.status.success(), "fid {args:?}: {}", text(&out));
    }
    root
}

fn web_files(root: &Path) -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if let Ok(s) = std::fs::read_to_string(&p) {
                out.push((p, s));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("apps/web"), &mut out);
    out
}

#[test]
fn a_fresh_next_product_reports_nothing_to_upgrade() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path(), &["add", "app", "next"]);
    let out = run(&root, &["doctor"]);
    let report = text(&out);
    assert!(
        !report.contains("migration pending"),
        "the templates fid just wrote already contain every migration: {report}"
    );
    assert!(
        !report.contains("upstream template updated"),
        "nothing upstream changed since a second ago: {report}"
    );
}

#[test]
fn nothing_in_a_fresh_next_app_names_radix() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path(), &["add", "app", "next"]);
    for (path, body) in web_files(&root) {
        for code_line in body.lines().filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//") || t.starts_with('*') || t.starts_with("/*"))
        }) {
            assert!(
                !code_line.contains("radix-ui") && !code_line.contains("@radix-ui/"),
                "{}: Fiducial's React primitives are Base UI — `{code_line}`",
                path.display()
            );
        }
    }
    let components: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("apps/web/components.json")).unwrap(),
    )
    .unwrap();
    assert!(
        components["style"].as_str().unwrap().starts_with("base-"),
        "a `base-*` style is what makes `npx shadcn add` write Base UI: {components}"
    );
    assert_eq!(
        components["tailwind"]["config"], "",
        "Tailwind 4 has no config file; naming one sends the shadcn CLI looking for it"
    );
}

#[test]
fn every_import_the_app_makes_is_declared_or_shipped() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path(), &["add", "app", "next"]);
    let web = root.join("apps/web");
    assert!(
        web.join("src/lib/utils.ts").exists(),
        "@/lib/utils is imported by every ui component"
    );
    assert!(
        web.join("postcss.config.mjs").exists(),
        "`@import \"tailwindcss\"` needs the PostCSS plugin"
    );
    assert!(
        !web.join("src/app/board/page.tsx").exists(),
        "the PCB viewer page belongs to `eda`, not to every website"
    );

    let pkg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(web.join("package.json")).unwrap()).unwrap();
    let declared = |name: &str| {
        pkg["dependencies"].get(name).is_some() || pkg["devDependencies"].get(name).is_some()
    };
    for (path, body) in web_files(&root) {
        if !matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts" | "tsx")
        ) {
            continue;
        }
        for line in body.lines() {
            let Some(from) = line
                .trim_start()
                .strip_prefix("import ")
                .and_then(|l| l.split(" from ").nth(1))
            else {
                continue;
            };
            let spec = from
                .trim()
                .trim_end_matches(';')
                .trim_matches(|c| c == '"' || c == '\'');
            if spec.starts_with('.')
                || spec.starts_with("@/")
                || spec.starts_with("next")
                || spec.starts_with("react")
            {
                continue;
            }
            let package = if spec.starts_with('@') {
                spec.splitn(3, '/').take(2).collect::<Vec<_>>().join("/")
            } else {
                spec.split('/').next().unwrap().to_string()
            };
            assert!(
                declared(&package),
                "{} imports `{spec}`, and apps/web/package.json does not declare `{package}`",
                path.display()
            );
        }
    }
    assert!(
        declared("tailwindcss") && declared("@tailwindcss/postcss") && declared("@types/react-dom"),
        "{pkg}"
    );
    let workspace = std::fs::read_to_string(root.join("pnpm-workspace.yaml"))
        .expect("pnpm 11 refuses every script until Biome's install script is decided");
    assert!(
        workspace.contains("allowBuilds") && workspace.contains("\"apps/*\""),
        "{workspace}"
    );
    assert!(
        !web.join("pnpm-workspace.yaml").exists(),
        "a second workspace root inside apps/web makes pnpm install there separately"
    );
    let biome = std::fs::read_to_string(root.join("biome.json"))
        .expect("without a Biome config, `pnpm lint` runs on Biome's defaults and fails (#99)");
    assert!(
        biome.contains("\"indentWidth\": 2") && biome.contains("**/generated/**"),
        "{biome}"
    );
}

/// A product that is already a pnpm workspace keeps its own root file.
///
/// fon's root `pnpm-workspace.yaml` lists its packages and the install scripts
/// it has vetted (esbuild, workerd). Writing the template over it would drop
/// both — so `fid add` keeps a file the platform did not write.
#[test]
fn an_existing_root_workspace_is_kept() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    let own = "packages:\n  - \"apps/*\"\nallowBuilds:\n  esbuild: true\n";
    std::fs::write(root.join("pnpm-workspace.yaml"), own).unwrap();
    assert!(run(&root, &["add", "design"]).status.success());
    let out = run(&root, &["add", "app", "next"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        std::fs::read_to_string(root.join("pnpm-workspace.yaml")).unwrap(),
        own
    );
}

#[test]
fn the_chosen_icon_library_is_the_one_imported() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path(), &["add", "app", "next", "--icons", "hugeicons"]);
    let web = root.join("apps/web");
    let components = std::fs::read_to_string(web.join("components.json")).unwrap();
    assert!(
        components.contains("\"iconLibrary\": \"hugeicons\""),
        "{components}"
    );

    let menu = std::fs::read_to_string(web.join("src/components/ui/dropdown-menu.tsx")).unwrap();
    assert!(
        !menu.contains("IconPlaceholder"),
        "no placeholder reaches a product"
    );
    assert!(
        menu.contains("from \"@hugeicons/core-free-icons\""),
        "{menu}"
    );
    assert!(menu.contains("<HugeiconsIcon icon={Tick02Icon}"), "{menu}");
    assert!(!menu.contains("lucide"), "{menu}");

    let pkg = std::fs::read_to_string(web.join("package.json")).unwrap();
    assert!(
        pkg.contains("\"@hugeicons/react\"") && pkg.contains("\"@hugeicons/core-free-icons\""),
        "{pkg}"
    );
    assert!(!pkg.contains("lucide-react"), "{pkg}");

    let out = run(&root, &["doctor"]);
    assert!(
        !text(&out).contains("upstream template updated"),
        "a rewritten icon is the product's library, not drift: {}",
        text(&out)
    );
}

#[test]
fn icons_default_to_lucide_and_reject_unknown_libraries() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path(), &["add", "app", "next"]);
    let menu =
        std::fs::read_to_string(root.join("apps/web/src/components/ui/dropdown-menu.tsx")).unwrap();
    assert!(menu.contains("from \"lucide-react\""), "{menu}");

    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "q"]).status.success());
    let root = tmp.path().join("q");
    assert!(run(&root, &["add", "design"]).status.success());
    let out = run(&root, &["add", "app", "next", "--icons", "radix"]);
    assert!(
        !out.status.success(),
        "an unsupported library is refused, not defaulted"
    );
}

#[test]
fn next_without_design_is_refused_with_the_fix() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let out = run(&tmp.path().join("p"), &["add", "app", "next"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("design"), "{}", text(&out));
}

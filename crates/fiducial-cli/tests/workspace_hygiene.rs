//! Workspace hygiene — principle 1 applied to the platform's own manifests.
//!
//! The platform's thesis is "declare each fact once". Before Phase 18 the
//! repository that states that rule declared `serde`'s version in five separate
//! manifests, `serde_json`'s in four and `sha2`'s in three, and the TypeScript
//! version in eleven `package.json` files — where they had already drifted into
//! three different answers.
//!
//! These tests make that class of drift impossible to reintroduce silently.
//! They are cheap, they read only files already in the repository, and they fail
//! with the exact file and line to change.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The workspace root, derived from this crate's manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("fiducial-cli lives two levels below the workspace root")
        .to_path_buf()
}

/// Every `Cargo.toml` under `crates/`.
fn crate_manifests() -> Vec<PathBuf> {
    let crates_dir = workspace_root().join("crates");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&crates_dir)
        .expect("crates/ is readable")
        .filter_map(|entry| {
            let path = entry.ok()?.path().join("Cargo.toml");
            path.is_file().then_some(path)
        })
        .collect();
    found.sort();
    assert!(
        found.len() >= 10,
        "expected the full crate set, found {} — has the layout moved?",
        found.len()
    );
    found
}

/// A dependency line that pins a version literal, e.g. `serde = "1"` or
/// `serde = {{ version = "1", ... }}`.
///
/// Returns the dependency name when the line declares a version literal, and
/// `None` for inherited (`workspace = true`) or non-dependency lines.
fn version_literal(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let (name, rest) = trimmed.split_once('=')?;
    let name = name.trim();

    // `version.workspace = true` and friends are package metadata, not deps.
    if name.contains('.') {
        return None;
    }
    // Only dependency names — reject `name = "fid"`, `path = "src/main.rs"`.
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }

    let rest = rest.trim();
    let declares_version = if rest.starts_with('"') {
        // `dep = "1.2"` — a bare version literal.
        true
    } else {
        // `dep = { version = "1.2", ... }` — a version key inside a table.
        rest.starts_with('{') && rest.contains("version")
    };

    declares_version.then_some(name)
}

/// Keys that legitimately take a string on the right-hand side but are not
/// dependencies. `version` itself appears in `[package]` as `version = "0.1.0"`.
const NON_DEPENDENCY_KEYS: &[&str] = &[
    "name",
    "version",
    "description",
    "path",
    "edition",
    "license",
    "repository",
    "homepage",
    "readme",
    "documentation",
    "rust-version",
];

/// No crate manifest may pin a dependency version. Versions are declared once,
/// in `[workspace.dependencies]`, and inherited with `workspace = true`.
#[test]
fn crate_manifests_declare_no_version_literals() {
    let mut offenders: Vec<String> = Vec::new();

    for manifest in crate_manifests() {
        let text = std::fs::read_to_string(&manifest).expect("manifest is readable");
        let name = manifest
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();

        // Only lines inside a dependency table can declare a dependency.
        let mut in_dependency_table = false;
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_dependency_table = trimmed.contains("dependencies");
                continue;
            }
            if !in_dependency_table {
                continue;
            }
            if let Some(dep) = version_literal(line) {
                if NON_DEPENDENCY_KEYS.contains(&dep) {
                    continue;
                }
                offenders.push(format!(
                    "  crates/{name}/Cargo.toml:{} — `{dep}` pins a version here",
                    index + 1
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "dependency versions must be declared once in [workspace.dependencies] \
         and inherited with `{{ workspace = true }}`.\n\
         Principle 1: a value used in two places belongs in one declared location.\n\n\
         {}\n",
        offenders.join("\n")
    );
}

/// Every crate inherits the same package metadata from `[workspace.package]`.
///
/// `fiducial-sim` shipped in Phase 17 without `authors` or `rust-version`, which
/// meant it silently carried no MSRV while every sibling declared 1.82.
#[test]
fn crate_manifests_inherit_all_package_metadata() {
    const INHERITED: &[&str] = &[
        "version",
        "edition",
        "license",
        "repository",
        "homepage",
        "authors",
        "rust-version",
    ];

    let mut missing: Vec<String> = Vec::new();

    for manifest in crate_manifests() {
        let text = std::fs::read_to_string(&manifest).expect("manifest is readable");
        let name = manifest
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();

        for key in INHERITED {
            if !text.contains(&format!("{key}.workspace")) {
                missing.push(format!(
                    "  crates/{name}/Cargo.toml — missing `{key}.workspace = true`"
                ));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "every crate inherits its package metadata from [workspace.package]:\n\n{}\n",
        missing.join("\n")
    );
}

/// Every published crate declares `keywords` and `categories`, because both are
/// what makes it findable on crates.io. `fiducial-sim` shipped without either.
#[test]
fn published_crates_declare_keywords_and_categories() {
    let mut missing: Vec<String> = Vec::new();

    for manifest in crate_manifests() {
        let text = std::fs::read_to_string(&manifest).expect("manifest is readable");
        let name = manifest
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();

        for key in ["keywords", "categories"] {
            if !text.lines().any(|l| l.trim_start().starts_with(key)) {
                missing.push(format!("  crates/{name}/Cargo.toml — missing `{key}`"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "every crate is published, and keywords/categories are how it is found:\n\n{}\n",
        missing.join("\n")
    );
}

/// Every crate ships a README. crates.io renders it as the crate's front page;
/// without one the page is a bare description line. All thirteen crates were
/// missing this before Phase 18.
#[test]
fn every_crate_ships_a_readme() {
    let mut missing: Vec<String> = Vec::new();

    for manifest in crate_manifests() {
        let dir = manifest.parent().expect("manifest has a parent");
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        if !dir.join("README.md").is_file() {
            missing.push(format!("  crates/{name}/README.md — does not exist"));
        }
    }

    assert!(
        missing.is_empty(),
        "every crate ships a README.md — it is the crates.io front page:\n\n{}\n",
        missing.join("\n")
    );
}

/// The TypeScript version is declared once, in the pnpm catalog, and every
/// package references it as `catalog:`.
///
/// Before Phase 18 it was written into eleven `package.json` files as three
/// different answers — `^7.0.2`, `^7.0.0` and `^5.0.0` — while one version was
/// actually installed. That is the drift this platform exists to delete.
#[test]
fn js_packages_pin_no_shared_dependency_versions() {
    let root = workspace_root();
    let packages_dir = root.join("packages");

    // Collect every dependency declaration across every package.
    let mut declarations: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

    let entries = std::fs::read_dir(&packages_dir).expect("packages/ is readable");
    for entry in entries.flatten() {
        let manifest = entry.path().join("package.json");
        if !manifest.is_file() {
            continue;
        }
        let package_name = entry.file_name().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&manifest).expect("package.json is readable");
        let parsed: serde_json::Value =
            serde_json::from_str(&text).expect("package.json is valid JSON");

        // `peerDependencies` are deliberately broad ranges expressing what a
        // consumer must supply ("react >=18"), not a version this repository
        // installs. Two packages declaring the same peer range is correct, and
        // a catalog entry cannot express it — so they are not drift.
        for field in ["dependencies", "devDependencies"] {
            let Some(table) = parsed.get(field).and_then(|v| v.as_object()) else {
                continue;
            };
            for (dep, spec) in table {
                let Some(spec) = spec.as_str() else { continue };
                // Workspace-internal and catalog references are already single
                // declarations — they are the fix, not the problem.
                if spec.starts_with("workspace:") || spec.starts_with("catalog:") {
                    continue;
                }
                declarations
                    .entry(dep.clone())
                    .or_default()
                    .push((package_name.clone(), spec.to_string()));
            }
        }
    }

    // A dependency named by two or more *distinct packages* must come from the
    // catalog. One package naming it twice (a dev pin plus a peer range) is not
    // duplication across declarations.
    let shared: Vec<String> = declarations
        .iter()
        .filter(|(_, sites)| {
            let distinct: std::collections::BTreeSet<&String> =
                sites.iter().map(|(pkg, _)| pkg).collect();
            distinct.len() > 1
        })
        .map(|(dep, sites)| {
            let detail = sites
                .iter()
                .map(|(pkg, spec)| format!("      packages/{pkg}: {spec}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "  `{dep}` is declared by {} packages:\n{detail}",
                sites.len()
            )
        })
        .collect();

    assert!(
        shared.is_empty(),
        "a dependency used by two packages is declared once, in the `catalog:` \
         block of pnpm-workspace.yaml, and referenced as `\"catalog:\"`.\n\
         Principle 1 — and these are exactly where versions drift apart.\n\n{}\n",
        shared.join("\n")
    );
}

/// Every glob in `pnpm-workspace.yaml` resolves to a directory that exists.
///
/// The file listed `apps/*`, `workbench` and `cli`; none of the three had ever
/// existed at the repository root. A stale glob is silent — pnpm matches nothing
/// and says nothing — so it survives until someone creates the directory and is
/// surprised by what it picks up.
#[test]
fn pnpm_workspace_globs_all_resolve() {
    let root = workspace_root();
    let text =
        std::fs::read_to_string(root.join("pnpm-workspace.yaml")).expect("pnpm-workspace.yaml");

    let mut stale: Vec<String> = Vec::new();
    let mut in_packages = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("packages:") {
            in_packages = true;
            continue;
        }
        // Any other top-level key ends the packages list.
        if in_packages && !line.starts_with(' ') && !line.starts_with('-') && !trimmed.is_empty() {
            in_packages = false;
        }
        if !in_packages || !trimmed.starts_with('-') {
            continue;
        }

        let glob = trimmed
            .trim_start_matches('-')
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if glob.is_empty() {
            continue;
        }

        // `packages/*` resolves if `packages/` exists and is non-empty.
        let base = glob.trim_end_matches("/*");
        let candidate = root.join(base);
        let resolves = if glob.ends_with("/*") {
            candidate.is_dir()
                && std::fs::read_dir(&candidate)
                    .map(|mut d| d.any(|e| e.map(|e| e.path().is_dir()).unwrap_or(false)))
                    .unwrap_or(false)
        } else {
            candidate.is_dir()
        };

        if !resolves {
            stale.push(format!("  `{glob}` matches no directory"));
        }
    }

    assert!(
        stale.is_empty(),
        "every pnpm workspace glob resolves to a real directory — a stale glob \
         matches nothing and reports nothing:\n\n{}\n",
        stale.join("\n")
    );
}

/// Every crate and package directory is named in the layout tree of the agent
/// context files.
///
/// `CLAUDE.md` carried a hand-maintained tree that had gone stale twice over —
/// it was missing `fiducial-ota`, `fiducial-sim` and `packages/realtime`. The
/// file even documented its own unreliability, telling readers to run
/// `ls crates packages` rather than trust it. A disclaimer is not a fix: agents
/// read that tree to decide what exists, and a crate missing from it is a crate
/// they will not use or will rebuild.
///
/// Checking is cheaper than disclaiming, so this test does the checking.
#[test]
fn agent_context_layout_tree_names_every_crate_and_package() {
    let root = workspace_root();

    let mut expected: Vec<String> = Vec::new();
    for dir in ["crates", "packages"] {
        let entries = std::fs::read_dir(root.join(dir)).expect("directory is readable");
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            expected.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    expected.sort();

    let mut missing: Vec<String> = Vec::new();
    for doc in ["CLAUDE.md", "AGENTS.md"] {
        let path = root.join(doc);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // Not every context file has to exist.
        };
        // Only check a file that actually draws a layout tree.
        if !text.contains("├──") {
            continue;
        }
        for name in &expected {
            if !text.contains(name.as_str()) {
                missing.push(format!("  {doc} — does not mention `{name}`"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "the layout tree in the agent context files must name every crate and \
         package. Agents read it to decide what already exists, and anything \
         absent gets rebuilt:\n\n{}\n",
        missing.join("\n")
    );
}

/// No tracked file lives inside a build-output directory.
///
/// Phase 21b committed nine files from `packages/ui-svelte/.svelte-check/` —
/// generated type scratch, rewritten on every `pnpm typecheck`. Committing a
/// derived artifact is the rule this repository spends a whole phase enforcing
/// everywhere else, and it still happened, because turning on a new tool
/// introduced a new output directory nobody had thought to ignore.
///
/// That is the general shape of the problem: the `.gitignore` is a list someone
/// maintains by remembering, and the next tool will have a different scratch
/// directory. So this checks the property instead.
#[test]
fn no_build_output_is_tracked() {
    // Directories that only ever hold generated or vendored content.
    const BUILD_DIRS: &[&str] = &[
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        ".nuxt",
        ".svelte-kit",
        ".svelte-check",
        ".turbo",
        ".wrangler",
        ".vitepress",
        ".astro",
        ".vercel",
        ".output",
        ".cache",
        "coverage",
        "storybook-static",
        "playwright-report",
        "test-results",
        "__pycache__",
    ];

    let root = workspace_root();
    let output = std::process::Command::new("git")
        .args(["ls-files"])
        .current_dir(&root)
        .output();

    // Not a git checkout (a packaged crate, say) — nothing to check.
    let Ok(output) = output else { return };
    if !output.status.success() {
        return;
    }

    let tracked = String::from_utf8_lossy(&output.stdout);
    let mut offenders: Vec<String> = Vec::new();

    for path in tracked.lines() {
        for segment in path.split('/') {
            if BUILD_DIRS.contains(&segment) {
                offenders.push(format!("  {path} — inside `{segment}/`"));
                break;
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "generated output is tracked in git. Add the directory to .gitignore and \
         `git rm --cached` it — a derived artifact is never committed by hand:\n\n{}\n",
        offenders.join("\n")
    );
}

/// The principles restated in the scaffolded `AGENTS.md` match the platform's.
///
/// Phase 22 put the principle headings in two files: `MISSION.md`, where they are
/// authored, and `templates/AGENTS.md.tmpl`, where products inherit them. That is
/// a second declaration of one fact — the exact thing principle 1 forbids —
/// introduced while writing principle 1c.
///
/// The right fix is to *generate* the template section from `MISSION.md`, which is
/// roadmap item 29 (context sync). Until that exists, this gate makes the
/// duplication safe: adding, removing or renaming a principle in one file without
/// the other fails the build.
///
/// Recorded rather than quietly tolerated, because a known duplication with a
/// test is a different thing from an unknown one without.
#[test]
fn scaffolded_principles_match_the_platform_mission() {
    let root = workspace_root();

    let mission = std::fs::read_to_string(root.join("MISSION.md")).expect("MISSION.md");
    let template =
        std::fs::read_to_string(root.join("crates/fiducial-cli/templates/AGENTS.md.tmpl"))
            .expect("AGENTS.md.tmpl");

    /// The identifier of each principle heading: `**1c · A user-visible…` → `1c`.
    fn principle_ids(text: &str) -> Vec<String> {
        text.lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("**")?;
                let (id, _) = rest.split_once(" · ")?;
                // Ids look like `1`, `1b`, `5b`, `7`.
                let looks_like_id = id.chars().next().is_some_and(|c| c.is_ascii_digit())
                    && id.len() <= 2
                    && id.chars().all(|c| c.is_ascii_alphanumeric());
                looks_like_id.then(|| id.to_string())
            })
            .collect()
    }

    let authored = principle_ids(&mission);
    let inherited = principle_ids(&template);

    assert!(
        !authored.is_empty(),
        "no principles found in MISSION.md — has the heading format changed?"
    );

    let missing: Vec<&String> = authored
        .iter()
        .filter(|id| !inherited.contains(id))
        .collect();
    let extra: Vec<&String> = inherited
        .iter()
        .filter(|id| !authored.contains(id))
        .collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "the principles a scaffolded product inherits have drifted from MISSION.md.\n\
         Every product's AGENTS.md restates these, so a principle missing there is a \
         principle that product never learns.\n\n\
         in MISSION.md but not the template: {missing:?}\n\
         in the template but not MISSION.md: {extra:?}\n\n\
         Authored:  {authored:?}\n\
         Inherited: {inherited:?}\n"
    );
}

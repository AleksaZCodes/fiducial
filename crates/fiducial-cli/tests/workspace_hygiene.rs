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

/// The layout tree in `AGENTS.md` is generated, not checked.
///
/// This used to be a test that read `crates/` and `packages/` and asserted
/// every member appeared in the hand-written tree. It caught drift, and the
/// roadmap's objection to it is the whole reason context sync exists:
/// **a test that fails after the fact is detection, not sync.** Someone still
/// had to edit the tree, and until they did the build was red.
///
/// The tree is now produced by `fid context` from the workspace manifests, so
/// what is left to check is that it *is* generated — a file that quietly lost
/// its markers would go back to being hand-written and stale, with nothing
/// saying so.
#[test]
fn agent_context_is_generated_rather_than_asserted() {
    let root = workspace_root();
    let text = std::fs::read_to_string(root.join("AGENTS.md")).expect("reading AGENTS.md");

    for block in ["layout", "commands", "capabilities", "skills"] {
        assert!(
            text.contains(&format!("<!-- fid:begin {block} -->")),
            "AGENTS.md no longer generates its `{block}` block — it is a fact \
             about this repository, and hand-written it goes stale. Run \
             `fid context` and keep the markers."
        );
    }
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

// ── The record-keeping split ──────────────────────────────────────────────────

/// `SHIPPED.md` looks backwards only.
///
/// `docs/specs/2026-09-14-phases-renamed-to-shipped.md` states that the
/// next-items prose *"was removed from `PHASES.md` precisely so it would hold
/// only the record"* — and the same commit that wrote that sentence put a
/// `## Current phase: … — next is …` line back into the renamed file, where it
/// then had to be hand-edited on every merge for three phases.
///
/// A spec asserting something is not the same as anything checking it. This
/// checks it.
#[test]
fn shipped_does_not_state_what_is_next() {
    let root = workspace_root();
    let text = std::fs::read_to_string(root.join("SHIPPED.md")).expect("reading SHIPPED.md");

    let offenders: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with('#'))
        .filter(|l| {
            let l = l.to_lowercase();
            l.contains("current phase") || l.contains("next is") || l.contains("up next")
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "SHIPPED.md records what was built; what is next belongs to ROADMAP.md, \
         whose order and markers already say it and which `fid dash` derives from.\n\
         Offending heading(s): {offenders:?}"
    );
}

/// No document sends a reader to `SHIPPED.md` for what is next.
///
/// The narrower `shipped_does_not_state_what_is_next` checks headings *inside*
/// that file, and passed while `README.md`, `docs/guides/README.md` and the
/// design agent all still described it as "build order and current state". A
/// file that holds only the record is not much use if three other files say it
/// holds the plan.
#[test]
fn nothing_points_at_shipped_for_what_is_next() {
    let root = workspace_root();
    // The specs are the append-only history: they record what *was* true on the
    // day they were written, and correcting them would be the edit this
    // project's fourth rule forbids.
    let skip = ["docs/specs/", "/target/", "node_modules/", "CHANGELOG"];

    let mut offenders: Vec<String> = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let shown = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if skip.iter().any(|s| shown.contains(s)) || shown.starts_with(".git/") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (i, line) in text.lines().enumerate() {
                let l = line.to_lowercase();
                if !l.contains("shipped.md") {
                    continue;
                }
                // "looks backwards only … what is next is in ROADMAP.md" is the
                // correction, not the offence.
                if l.contains("roadmap.md") {
                    continue;
                }
                if [
                    "build order",
                    "current state",
                    "current phase",
                    "what's next",
                    "what is next",
                ]
                .iter()
                .any(|phrase| l.contains(phrase))
                {
                    offenders.push(format!("{shown}:{}: {}", i + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "`SHIPPED.md` holds the record; `ROADMAP.md` holds the order of work and \
         what is next.\n{}",
        offenders.join("\n")
    );
}

/// The ordered roadmap items: every `###` heading under *Order of work*.
///
/// Scoped by section rather than by a leading digit. The items used to be
/// `### 1 ·`, `### 2 ·` — ordinals sitting next to `SHIPPED.md`'s phase numbers,
/// so the same work answered to both "item 3" and "Phase 24". They are named
/// now, which means a check keyed to a digit would quietly match nothing and
/// pass forever. This one did, for exactly as long as it took to notice.
fn roadmap_items(text: &str) -> Vec<&str> {
    text.lines()
        .skip_while(|l| !l.starts_with("## Order of work"))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .filter(|l| l.starts_with("### "))
        .collect()
}

/// Every ordered roadmap item carries a status marker.
///
/// `fid dash` counts ⬜ 🟡 ✅ and ignores unmarked prose — deliberately, so a
/// sentence is not counted as an item. The cost is that an *item* without a
/// marker is invisible: six carried none, and the dashboard reported the
/// platform's own roadmap as "5 done, 0 to do".
#[test]
fn every_roadmap_item_carries_a_marker() {
    let root = workspace_root();
    let text = std::fs::read_to_string(root.join("ROADMAP.md")).expect("reading ROADMAP.md");

    let items = roadmap_items(&text);
    assert!(
        !items.is_empty(),
        "no roadmap items found under `## Order of work` — a check that matches \
         nothing passes forever"
    );

    let unmarked: Vec<&&str> = items
        .iter()
        .filter(|l| !l.contains('⬜') && !l.contains('🟡') && !l.contains('✅'))
        .collect();

    assert!(
        unmarked.is_empty(),
        "a roadmap item with no marker is invisible to `fid dash`, which then \
         reports the roadmap as finished.\nUnmarked: {unmarked:?}"
    );
}

/// There is exactly one numbering, and it is `SHIPPED.md`'s.
///
/// A roadmap item carrying its own number puts a second set of ordinals beside
/// the phase numbers, so one piece of work answers to two names. `ROADMAP.md`
/// decided against that — *"items have names, not numbers"* — and kept the
/// ordinals anyway, which is how the confusion outlived the decision.
#[test]
fn roadmap_items_are_named_not_numbered() {
    let root = workspace_root();
    let text = std::fs::read_to_string(root.join("ROADMAP.md")).expect("reading ROADMAP.md");

    let items = roadmap_items(&text);
    let numbered: Vec<&&str> = items
        .iter()
        .filter(|l| {
            l.trim_start_matches("### ")
                .starts_with(|c: char| c.is_ascii_digit())
        })
        .collect();

    assert!(
        numbered.is_empty(),
        "roadmap items are named; phases are numbered. An ordinal here is a \
         second numbering for the same work.\n{numbered:?}"
    );
}

/// A phase that implements a roadmap item says which one.
///
/// This sentence is the only link between the two files, and the thing that
/// stops them being two competing trackers. Checked from Phase 22 on, which is
/// where the roadmap took over ordering — earlier phases predate it and were
/// ordered by the design spec's own build order.
#[test]
fn a_phase_built_from_the_roadmap_names_its_item() {
    let root = workspace_root();
    let text = std::fs::read_to_string(root.join("SHIPPED.md")).expect("reading SHIPPED.md");

    let lines: Vec<&str> = text.lines().collect();
    let mut missing: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("**Phase ") else {
            continue;
        };
        let number: u32 = rest
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        if number < 22 {
            continue;
        }
        // The blockquote under the heading is where a phase says what it is.
        let names_item = lines[i..(i + 6).min(lines.len())]
            .iter()
            .any(|l| l.contains("roadmap item"));
        if !names_item {
            missing.push((*line).to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "a phase from Phase 22 on implements a roadmap item and must name it — \
         that sentence is the only link between ROADMAP.md and SHIPPED.md.\n{}",
        missing.join("\n")
    );
}

/// The release scripts invoke no tool the repository does not declare.
///
/// `jq` is preinstalled on GitHub's ubuntu runners and on no machine by
/// guarantee, so three scripts under `scripts/` used it for years while CI
/// stayed green. The gap surfaced the one place it must not: a maintainer
/// running `backfill-tags.sh` by hand to repair a release got
/// `jq: command not found` after it had printed nothing.
///
/// They read JSON through `scripts/lib/json.sh` (`node -e`) now, and Node *is*
/// declared — `.nvmrc`, `engines`, `packageManager`. This keeps the next
/// convenient one-liner from quietly reintroducing the same class of gap.
///
/// It checks invocations, not prose: `scripts/lib/json.sh` explains at length
/// why jq is gone, and a naive substring search would fail on its own rationale.
#[test]
fn release_scripts_invoke_no_undeclared_tool() {
    // Tools CI installs but a clone is not guaranteed to have. `curl`, `git`,
    // `sed` and `node` are not here: the first three are POSIX-ubiquitous and
    // the fourth this repository pins.
    const UNDECLARED: &[&str] = &["jq", "yq", "http", "xmllint"];

    let scripts_dir = workspace_root().join("scripts");
    let mut offences: Vec<String> = Vec::new();

    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![scripts_dir.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("scripts/ is readable") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "sh") {
                files.push(path);
            }
        }
    }
    files.sort();
    assert!(
        files.len() >= 4,
        "expected the release scripts, found {} — has scripts/ moved?",
        files.len()
    );

    for path in &files {
        let text = std::fs::read_to_string(path).expect("script is readable");
        for (n, line) in text.lines().enumerate() {
            // A comment is where the reasoning lives, and reasoning names the
            // tool it rejects. Only what the shell would run counts.
            let code = match line.split_once('#') {
                Some((before, _)) if !before.trim().is_empty() => before,
                Some(_) => continue,
                None => line,
            };
            for tool in UNDECLARED {
                // Word-bounded: `jqueue=1` is not an invocation of `jq`.
                let invoked = code
                    .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                    .any(|word| word == *tool);
                if invoked {
                    offences.push(format!(
                        "  {}:{}  invokes `{tool}`\n      {}",
                        path.strip_prefix(workspace_root())
                            .unwrap_or(path)
                            .display(),
                        n + 1,
                        code.trim()
                    ));
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "release scripts may only invoke tools this repository declares.\n\
         These are present on GitHub's runners and not on a fresh clone:\n\n{}\n\n\
         Read JSON through `scripts/lib/json.sh` instead — it uses `node -e`,\n\
         and Node is pinned by `.nvmrc`, `engines` and `packageManager`.\n",
        offences.join("\n")
    );
}

/// The release workflow can recover from a run that never happened.
///
/// `release.yml` serializes on one concurrency group, and GitHub keeps at most
/// one *pending* run per group — so a newly queued run evicts the one already
/// waiting, which never creates a job at all. On 2026-09-16 that silently cost
/// the publish of `@fiducial/adapters@0.3.0`: main declared it, npm did not
/// have it, and the `if: always()` verify step never ran, because `always()`
/// covers a failed run and not one cancelled before it starts.
///
/// The fix is not a wider concurrency group — that would let two publishes
/// race. It is that **something always comes back**: every step re-derives what
/// to do from the registries, so a scheduled run repairs whatever a dropped run
/// missed, and eviction costs latency instead of a version nothing has.
///
/// This holds the `schedule:` trigger in place. Delete it and the failure above
/// is silently reintroduced, with nothing to notice it — which is exactly how
/// it happened the first time.
#[test]
fn the_release_workflow_runs_on_a_schedule_not_only_on_merges() {
    let path = workspace_root().join(".github/workflows/release.yml");
    let text = std::fs::read_to_string(&path).expect("release.yml is readable");

    // Only what the workflow declares, not what its comments discuss: the
    // rationale below `concurrency:` names every one of these words.
    let declared: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        declared.contains("schedule:") && declared.contains("cron:"),
        "`.github/workflows/release.yml` must declare a `schedule:` trigger.\n\n\
         It serializes on one concurrency group, and GitHub evicts a queued run\n\
         when another queues behind it — an evicted run creates no job, so the\n\
         `if: always()` verify step cannot report it either. A periodic run is\n\
         what makes that eviction cost latency rather than a lost publish.\n"
    );

    // The verify step is the other half: convergence is only useful if
    // something still says when the registry disagrees with the repository.
    assert!(
        declared.contains("verify-published.sh"),
        "`release.yml` must still run `scripts/verify-published.sh` — a release\n\
         that re-derives its work needs something that reports the result.\n"
    );
}

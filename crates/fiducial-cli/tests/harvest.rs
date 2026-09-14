//! End-to-end tests for `fid harvest`.
//!
//! The properties that matter are not "does it classify a `.tsx` file correctly"
//! — that is a unit test in the module. They are the safety properties: that the
//! command never touches the product's source tree, never stages a secret, and
//! never consumes its own output.

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
        .output()
        .expect("running fid")
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

/// A donor with one file of each interesting kind.
fn donor(root: &Path) -> PathBuf {
    let donor = root.join("donor");
    std::fs::create_dir_all(&donor).unwrap();

    write(
        &donor,
        "package.json",
        r#"{"dependencies":{"react":"^19"}}"#,
    );
    write(
        &donor,
        "src/styles/tokens.css",
        ":root {\n  --primary: #3355ff;\n  --radius: 8px;\n}\n",
    );
    write(
        &donor,
        "src/rules.ts",
        "export function priceFor(units: number) {\n  return units < 10 ? units * 5 : units * 4;\n}\n",
    );
    write(
        &donor,
        "src/components/Hero.tsx",
        "import React from 'react';\nexport const Hero = () => <h1>Hi</h1>;\n",
    );
    write(
        &donor,
        "index.html",
        "<html><body><h1>Landing</h1></body></html>\n",
    );
    write(
        &donor,
        "docs/why-postgres.md",
        "# Why Postgres\n\nBecause.\n",
    );
    write(&donor, ".github/workflows/deploy.yml", "name: Deploy\n");
    write(&donor, "src/rules.test.ts", "test('prices', () => {});\n");
    write(&donor, "public/logo.svg", "<svg/>\n");

    // Never-inventoried noise.
    write(&donor, "pnpm-lock.yaml", "lockfileVersion: 9\n");
    write(&donor, ".env", "SUPABASE_SERVICE_KEY=super-secret-value\n");
    write(&donor, "debug.log", "noise\n");

    // Never-walked directories.
    write(
        &donor,
        "node_modules/left-pad/index.js",
        "module.exports = 1;\n",
    );
    write(&donor, "dist/bundle.js", "console.log(1);\n");

    donor
}

fn product(root: &Path) -> PathBuf {
    let out = run(root, &["new", "target-product"]);
    assert!(out.status.success(), "fid new failed");
    root.join("target-product")
}

// ── Safety ───────────────────────────────────────────────────────────────────

/// The product's own source tree is never written to. This is the property the
/// whole design rests on: harvest stages, it does not import.
#[test]
fn harvest_writes_only_inside_the_staging_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    // Fingerprint everything outside harvest/ before the run.
    let before = fingerprint(&prod);

    let out = run(
        &prod,
        &["harvest", src.to_str().unwrap(), "--name", "donor"],
    );
    assert!(
        out.status.success(),
        "harvest failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let after = fingerprint(&prod);
    assert_eq!(
        before, after,
        "harvest modified files outside harvest/ — it must only stage"
    );
    assert!(prod.join("harvest/donor/SURVEY.md").is_file());
    assert!(prod.join("harvest/donor/harvest.toml").is_file());
}

/// Secrets and lockfiles are never inventoried and never staged.
///
/// `ops` is the highest-risk category in any donor, and an `.env` staged into a
/// directory the user may later commit is the worst possible outcome for a tool
/// whose entire purpose is to read someone else's repository.
#[test]
fn secrets_and_lockfiles_are_never_inventoried_or_staged() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    run(
        &prod,
        &["harvest", src.to_str().unwrap(), "--name", "donor"],
    );

    let inventory = std::fs::read_to_string(prod.join("harvest/donor/harvest.toml")).unwrap();
    for forbidden in [".env", "pnpm-lock.yaml", "debug.log"] {
        assert!(
            !inventory.contains(forbidden),
            "`{forbidden}` must not be inventoried"
        );
    }

    // And the secret value never reaches disk anywhere under the staging tree.
    let staged = walk_text(&prod.join("harvest"));
    assert!(
        !staged.contains("super-secret-value"),
        "a secret was staged into harvest/"
    );
}

/// Build output and dependencies are never walked.
#[test]
fn build_output_and_dependencies_are_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    run(
        &prod,
        &["harvest", src.to_str().unwrap(), "--name", "donor"],
    );
    let inventory = std::fs::read_to_string(prod.join("harvest/donor/harvest.toml")).unwrap();

    assert!(!inventory.contains("node_modules"), "walked node_modules");
    assert!(!inventory.contains("dist/"), "walked dist");
}

/// Harvesting a directory into itself is refused: the walk would consume its own
/// output and the staged copies would be indistinguishable from the source.
#[test]
fn harvesting_into_the_source_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());

    let out = Command::new(fid())
        .args(["harvest", ".", "--into", "."])
        .current_dir(&src)
        .output()
        .unwrap();

    assert!(!out.status.success(), "should have refused");
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(
        text.contains("inside the source"),
        "should explain the overlap: {text}"
    );
}

// ── Classification ───────────────────────────────────────────────────────────

/// Every kind present in the donor is found, and the donor's stack is detected.
#[test]
fn the_survey_finds_each_kind_and_detects_the_stack() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    let out = run(
        &prod,
        &[
            "harvest",
            src.to_str().unwrap(),
            "--name",
            "donor",
            "--json",
        ],
    );
    let json = String::from_utf8_lossy(&out.stdout);

    for kind in ["logic", "ui", "theme", "art", "principle", "ops", "test"] {
        assert!(
            json.contains(&format!("\"{kind}\"")),
            "missing kind: {kind}"
        );
    }
    assert!(json.contains("React"), "React not detected: {json}");

    // The rule file is logic; the component is UI; the token file is theme.
    assert!(json.contains("src/rules.ts"));
    assert!(json.contains("src/components/Hero.tsx"));
    assert!(json.contains("src/styles/tokens.css"));
}

/// An HTML file is UI. A landing page is the commonest thing anyone wants to
/// reuse, and it *is* its markup.
#[test]
fn markup_is_inventoried_as_ui() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    run(
        &prod,
        &["harvest", src.to_str().unwrap(), "--name", "donor"],
    );
    let inventory = std::fs::read_to_string(prod.join("harvest/donor/harvest.toml")).unwrap();

    let block = inventory
        .split("[[asset]]")
        .find(|b| b.contains("index.html"))
        .expect("index.html inventoried");
    assert!(
        block.contains("kind = \"ui\""),
        "html should be ui: {block}"
    );
}

/// Every classification states the evidence for it, so a wrong guess is visible
/// rather than authoritative.
#[test]
fn every_asset_records_why_it_was_classified() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    run(
        &prod,
        &["harvest", src.to_str().unwrap(), "--name", "donor"],
    );
    let inventory = std::fs::read_to_string(prod.join("harvest/donor/harvest.toml")).unwrap();

    for block in inventory.split("[[asset]]").skip(1) {
        assert!(
            block.contains("reason = "),
            "asset without a reason: {block}"
        );
        let reason_line = block
            .lines()
            .find(|l| l.starts_with("reason = "))
            .expect("reason line");
        assert!(
            reason_line.len() > "reason = \"\"".len(),
            "empty reason: {reason_line}"
        );
    }
}

// ── Usability ────────────────────────────────────────────────────────────────

/// A missing source fails with a message that says what to pass instead.
#[test]
fn a_missing_source_explains_itself() {
    let tmp = tempfile::tempdir().unwrap();
    let prod = product(tmp.path());

    let out = run(&prod, &["harvest", "/no/such/path"]);
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("does not exist"), "{text}");
    assert!(
        text.contains("repository or folder"),
        "should say what to pass: {text}"
    );
}

/// The survey names the staging rule explicitly. The single most important thing
/// a reader can take away is that none of this is wired in.
#[test]
fn the_survey_states_that_nothing_is_wired_in() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    run(
        &prod,
        &["harvest", src.to_str().unwrap(), "--name", "donor"],
    );
    let survey = std::fs::read_to_string(prod.join("harvest/donor/SURVEY.md")).unwrap();

    assert!(survey.contains("Nothing here is wired into this product"));
    assert!(survey.contains("staging area"));
    assert!(
        survey.contains("/fiducial:harvest"),
        "should point at the skill that does the judgment half"
    );
}

/// The product stays healthy after a harvest — staging must not disturb the lock
/// or the template integrity `fid doctor` checks.
#[test]
fn doctor_stays_clean_after_a_harvest() {
    let tmp = tempfile::tempdir().unwrap();
    let src = donor(tmp.path());
    let prod = product(tmp.path());

    run(
        &prod,
        &["harvest", src.to_str().unwrap(), "--name", "donor"],
    );

    let out = run(&prod, &["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "doctor failed: {text}");
    assert!(text.contains("clean"), "doctor after harvest: {text}");
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Path + content of every file outside `harvest/`.
fn fingerprint(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out.sort();
    out
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .to_string();
        if rel.starts_with("harvest") || rel.starts_with(".git") {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, out);
        } else {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            out.push((rel, content));
        }
    }
}

/// Every readable byte under a directory, concatenated.
fn walk_text(dir: &Path) -> String {
    let mut out = String::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.push_str(&walk_text(&path));
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            out.push_str(&text);
        }
    }
    out
}

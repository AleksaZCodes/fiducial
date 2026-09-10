//! `fid dash --portfolio` — workbench v1 end-to-end tests.
//!
//! Scaffolds two product repos, writes a `fiducial.portfolio` manifest in a
//! parent directory, and asserts that `fid dash --portfolio --json` returns
//! both products aggregated.
//!
//! "Done when: workbench v1 multi-repo view" — Phase 17.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn fid_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_fid"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(fid_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to invoke fid")
}

fn scaffold_product(parent: &Path, name: &str) -> PathBuf {
    let out = run(parent, &["new", name]);
    assert!(
        out.status.success(),
        "fid new {name} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    parent.join(name)
}

/// Write a portfolio manifest in `dir` pointing at the given product dirs.
fn write_portfolio(dir: &Path, products: &[(&str, &Path)]) {
    let mut toml = String::new();
    for (name, path) in products {
        toml.push_str("[[products]]\n");
        toml.push_str(&format!("name = \"{name}\"\n"));
        toml.push_str(&format!("path = \"{}\"\n\n", path.display()));
    }
    fs::write(dir.join("fiducial.portfolio"), toml).expect("write portfolio manifest");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn portfolio_aggregates_two_products() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let p1 = scaffold_product(root, "alpha");
    let p2 = scaffold_product(root, "beta");

    write_portfolio(root, &[("Alpha", &p1), ("Beta", &p2)]);

    let out = run(root, &["dash", "--portfolio", "--json"]);
    assert!(
        out.status.success(),
        "fid dash --portfolio failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("portfolio --json must emit valid JSON");

    let products = v["products"].as_array().expect("products must be an array");
    assert_eq!(
        products.len(),
        2,
        "expected 2 products, got {}",
        products.len()
    );

    let names: Vec<&str> = products
        .iter()
        .map(|p| p["name"].as_str().expect("product name must be a string"))
        .collect();
    assert!(names.contains(&"Alpha"), "missing Alpha in {names:?}");
    assert!(names.contains(&"Beta"), "missing Beta in {names:?}");
}

#[test]
fn portfolio_entry_contains_dash_data() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let p1 = scaffold_product(root, "gamma");
    write_portfolio(root, &[("Gamma", &p1)]);

    let out = run(root, &["dash", "--portfolio", "--json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    let entry = &v["products"][0];
    assert!(
        entry["dash"].is_object(),
        "dash data should be present for a valid product"
    );
    assert!(
        entry["error"].is_null(),
        "error should be null for a valid product"
    );
    // The embedded dash must have the expected top-level sections.
    for section in [
        "product",
        "git",
        "roadmap",
        "decisions",
        "ci",
        "graph",
        "freshness",
    ] {
        assert!(
            entry["dash"].get(section).is_some(),
            "missing dash section: {section}"
        );
    }
    assert_eq!(entry["dash"]["product"]["name"], "gamma");
}

#[test]
fn portfolio_reports_error_for_missing_product_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let missing = root.join("does_not_exist");
    write_portfolio(root, &[("Ghost", &missing)]);

    let out = run(root, &["dash", "--portfolio", "--json"]);
    // The command itself succeeds (portfolio is a view — it reports errors, does
    // not fail on them), but the entry has an error and a null dash field.
    assert!(
        out.status.success(),
        "fid dash --portfolio should succeed even with a missing product"
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let entry = &v["products"][0];
    assert!(
        entry["dash"].is_null(),
        "dash should be null for a missing product"
    );
    assert!(
        !entry["error"].is_null(),
        "error should be non-null for a missing product"
    );
}

#[test]
fn portfolio_missing_manifest_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    // No fiducial.portfolio written — expect non-zero exit.
    let out = run(tmp.path(), &["dash", "--portfolio"]);
    assert!(
        !out.status.success(),
        "fid dash --portfolio should fail when no manifest exists"
    );
}

#[test]
fn portfolio_is_a_view_it_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let p1 = scaffold_product(root, "delta");
    write_portfolio(root, &[("Delta", &p1)]);

    // Capture the filesystem state before and after.
    fn dir_entries(dir: &Path) -> Vec<String> {
        let mut entries = Vec::new();
        for e in walkdir::WalkDir::new(dir).sort_by_file_name() {
            let e = e.unwrap();
            entries.push(e.path().display().to_string());
        }
        entries
    }

    let before = dir_entries(root);
    let out = run(root, &["dash", "--portfolio", "--json"]);
    assert!(out.status.success());
    let after = dir_entries(root);

    assert_eq!(
        before, after,
        "fid dash --portfolio must not write any files"
    );
}

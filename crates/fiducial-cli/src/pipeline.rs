//! Pipeline declarations — the single place `pipelines/*.toml` is read.
//!
//! `fid derive`, `fid graph`, and `fid dash` all need to know which pipelines a
//! product declares and what each one produces. That fact is declared once, in
//! `pipelines/*.toml`, so it is read once, here.
//!
//! It was previously read twice — `derive` parsed strictly and failed loudly on
//! a malformed file, while `graph` swallowed the error and labelled the pipeline
//! `"unknown"`. Two commands therefore disagreed about the same file, and the
//! one that lied was the one meant to explain the build. One reader, one answer.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::Config;

/// A pipeline as declared in `pipelines/<name>.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pipeline {
    /// Pipeline name, unique within a product.
    pub name: String,
    /// Executor that runs it: `cargo-test`, `shell`, `fid-validate`, `fid-mesh`.
    pub executor: String,
    /// Arguments passed to the executor.
    #[serde(default)]
    pub args: Vec<String>,
    /// Paths this pipeline writes, relative to the product root. Tracked in
    /// `fiducial.lock`.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Directory to run in, relative to the product root.
    #[serde(default)]
    pub working_dir: Option<String>,
}

/// The built-in `types` pipeline, present when `[spine] enabled = true`.
///
/// Declared in code rather than in a template file because it is not a product
/// decision: enabling the spine *is* the declaration, and a product cannot
/// meaningfully redefine what regenerating its Rust-derived types means.
pub fn builtin_types_pipeline() -> Pipeline {
    Pipeline {
        name: "types".into(),
        executor: "cargo-test".into(),
        args: vec![
            "--features".into(),
            "ts".into(),
            "-p".into(),
            "fiducial-wasm".into(),
        ],
        outputs: vec!["packages/wasm-bridge/src/generated.ts".into()],
        working_dir: None,
    }
}

/// Every pipeline a product declares, built-ins first, then `pipelines/*.toml`
/// in filename order.
///
/// A name already taken is skipped rather than overriding, so a product cannot
/// shadow a built-in by accident. A malformed file is an error: a pipeline that
/// cannot be parsed cannot be run, and reporting it as an unnamed placeholder
/// would hide that from whoever is trying to understand the build.
pub fn discover(root: &Path, config: &Config) -> Result<Vec<Pipeline>> {
    let mut pipelines = Vec::new();

    if config.spine.enabled {
        pipelines.push(builtin_types_pipeline());
    }

    let dir = root.join("pipelines");
    if dir.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
            .context("reading pipelines/")?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .collect();
        entries.sort();

        for path in entries {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let p: Pipeline =
                toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
            if !pipelines.iter().any(|e: &Pipeline| e.name == p.name) {
                pipelines.push(p);
            }
        }
    }

    Ok(pipelines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn product(spine: bool) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let cfg =
            format!("[product]\nname = \"t\"\nversion = \"0.1.0\"\n\n[spine]\nenabled = {spine}\n");
        std::fs::write(tmp.path().join("fiducial.toml"), cfg).unwrap();
        tmp
    }

    fn write_pipeline(root: &Path, file: &str, body: &str) {
        let dir = root.join("pipelines");
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join(file)).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    fn load(tmp: &tempfile::TempDir) -> Result<Vec<Pipeline>> {
        let config = Config::load(&tmp.path().join("fiducial.toml")).unwrap();
        discover(tmp.path(), &config)
    }

    #[test]
    fn a_product_with_nothing_declared_has_no_pipelines() {
        assert!(load(&product(false)).unwrap().is_empty());
    }

    #[test]
    fn enabling_the_spine_adds_the_builtin_types_pipeline() {
        let p = load(&product(true)).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "types");
        assert_eq!(p[0].outputs, ["packages/wasm-bridge/src/generated.ts"]);
    }

    #[test]
    fn declared_pipelines_are_read_in_filename_order() {
        let tmp = product(false);
        write_pipeline(
            tmp.path(),
            "b.toml",
            "name = \"beta\"\nexecutor = \"shell\"\nargs = [\"true\"]\n",
        );
        write_pipeline(
            tmp.path(),
            "a.toml",
            "name = \"alpha\"\nexecutor = \"shell\"\nargs = [\"true\"]\n",
        );
        let names: Vec<_> = load(&tmp).unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn a_declared_pipeline_cannot_shadow_a_builtin() {
        let tmp = product(true);
        write_pipeline(
            tmp.path(),
            "types.toml",
            "name = \"types\"\nexecutor = \"shell\"\nargs = [\"false\"]\noutputs = [\"x\"]\n",
        );
        let p = load(&tmp).unwrap();
        assert_eq!(p.len(), 1, "the built-in must win");
        assert_eq!(p[0].executor, "cargo-test");
    }

    #[test]
    fn a_malformed_pipeline_is_an_error_naming_the_file() {
        // The divergence this module exists to remove: `graph` used to report a
        // malformed pipeline as "unknown" while `derive` failed. A pipeline that
        // cannot be parsed cannot be run, so both must say so.
        let tmp = product(false);
        write_pipeline(tmp.path(), "broken.toml", "name = \"x\"\n");
        let err = load(&tmp).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("broken.toml"), "must name the file: {msg}");
    }

    #[test]
    fn non_toml_files_in_the_pipelines_directory_are_ignored() {
        let tmp = product(false);
        write_pipeline(tmp.path(), "notes.md", "not a pipeline");
        write_pipeline(
            tmp.path(),
            "real.toml",
            "name = \"real\"\nexecutor = \"shell\"\nargs = [\"true\"]\n",
        );
        let p = load(&tmp).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "real");
    }

    #[test]
    fn outputs_and_working_dir_default_to_empty() {
        let tmp = product(false);
        write_pipeline(
            tmp.path(),
            "p.toml",
            "name = \"p\"\nexecutor = \"shell\"\nargs = [\"true\"]\n",
        );
        let p = load(&tmp).unwrap();
        assert!(p[0].outputs.is_empty());
        assert!(p[0].working_dir.is_none());
    }
}

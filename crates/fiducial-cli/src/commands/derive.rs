//! `fid derive [--check] [--pipeline <name>]` — run product pipelines.
//!
//! Discovers pipelines in `pipelines/*.toml` at the product root.
//! The built-in "types" pipeline runs `cargo test --features ts -p fiducial-wasm`
//! when `[spine] enabled = true` in `fiducial.toml`.
//!
//! `--check` mode: re-hashes outputs against `fiducial.lock [artifacts]` and
//! exits non-zero if any artifact is stale or missing. CI uses this.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use fiducial_eda::validate as validate_board_interface;

use crate::{
    config::{Config, CONFIG_FILE},
    lock::{sha256_hex, Lock, LOCK_FILE},
};

// ── Pipeline declaration (pipelines/*.toml) ───────────────────────────────────

#[derive(Debug, Deserialize)]
struct PipelineToml {
    name: String,
    executor: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    outputs: Vec<String>,
    #[serde(default)]
    working_dir: Option<String>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(check: bool, pipeline_filter: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let config_path = root.join(CONFIG_FILE);
    let lock_path = root.join(LOCK_FILE);

    let config = Config::load(&config_path)?;
    let mut lock = if lock_path.exists() {
        Lock::load(&lock_path)?
    } else {
        Lock::new()
    };

    let pipelines = discover_pipelines(&root, &config)?;

    let to_run: Vec<&PipelineToml> = match &pipeline_filter {
        Some(name) => {
            let found: Vec<_> = pipelines.iter().filter(|p| &p.name == name).collect();
            if found.is_empty() {
                bail!("no pipeline named `{name}` found in {}", root.display());
            }
            found
        }
        None => pipelines.iter().collect(),
    };

    if to_run.is_empty() {
        println!("✦ fid derive — no pipelines declared");
        println!("  Declare pipelines in `pipelines/*.toml` or enable [spine] in fiducial.toml.");
        return Ok(());
    }

    if check {
        run_check(&to_run, &lock, &root)
    } else {
        run_derive(&to_run, &root, &mut lock, &lock_path)
    }
}

// ── Derive (write) mode ───────────────────────────────────────────────────────

fn run_derive(
    pipelines: &[&PipelineToml],
    root: &Path,
    lock: &mut Lock,
    lock_path: &Path,
) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let mut any_error = false;

    for pipeline in pipelines {
        print!("  ▶ {} ({})", pipeline.name, pipeline.executor);

        let working_dir = match &pipeline.working_dir {
            Some(rel) => root.join(rel),
            None => root.to_path_buf(),
        };

        match run_pipeline_command(pipeline, &working_dir) {
            Ok(_) => {
                println!(" ✓");
                for out in &pipeline.outputs {
                    let abs = root.join(out);
                    match std::fs::read(&abs) {
                        Ok(content) => {
                            lock.record_artifact(out, &content, &pipeline.name, version);
                        }
                        Err(e) => {
                            eprintln!("  ⚠ could not read output `{out}`: {e}");
                            any_error = true;
                        }
                    }
                }
            }
            Err(e) => {
                println!(" ✗");
                eprintln!("  error: {e}");
                any_error = true;
            }
        }
    }

    lock.save(lock_path)?;

    if any_error {
        bail!("one or more pipelines failed");
    }
    println!("\n✦ fid derive complete — fiducial.lock updated");
    Ok(())
}

// ── Check mode ────────────────────────────────────────────────────────────────

fn run_check(pipelines: &[&PipelineToml], lock: &Lock, root: &Path) -> Result<()> {
    let mut issues: Vec<String> = Vec::new();

    for pipeline in pipelines {
        for out in &pipeline.outputs {
            let abs = root.join(out);
            match lock.artifacts.get(out.as_str()) {
                None => {
                    issues.push(format!(
                        "  {out}: not in fiducial.lock (run `fid derive` first)"
                    ));
                }
                Some(record) => match std::fs::read(&abs) {
                    Err(e) => {
                        issues.push(format!("  {out}: missing — {e}"));
                    }
                    Ok(content) => {
                        let actual = sha256_hex(&content);
                        if actual != record.hash {
                            issues.push(format!(
                                "  {out}: stale (lock:{} file:{}) — run `fid derive`",
                                &record.hash[..8],
                                &actual[..8],
                            ));
                        }
                    }
                },
            }
        }
    }

    if issues.is_empty() {
        println!("✦ fid derive --check — all artifacts fresh");
        Ok(())
    } else {
        eprintln!("✗ fid derive --check failed:\n{}", issues.join("\n"));
        bail!("stale artifacts detected");
    }
}

// ── Pipeline discovery ────────────────────────────────────────────────────────

fn discover_pipelines(root: &Path, config: &Config) -> Result<Vec<PipelineToml>> {
    let mut pipelines = Vec::new();

    if config.spine.enabled {
        pipelines.push(builtin_types_pipeline());
    }

    let pipelines_dir = root.join("pipelines");
    if pipelines_dir.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&pipelines_dir)
            .context("reading pipelines/")?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .collect();
        entries.sort();

        for path in entries {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let p: PipelineToml =
                toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
            if !pipelines
                .iter()
                .any(|existing: &PipelineToml| existing.name == p.name)
            {
                pipelines.push(p);
            }
        }
    }

    Ok(pipelines)
}

fn builtin_types_pipeline() -> PipelineToml {
    PipelineToml {
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

// ── Built-in fid-validate executor ───────────────────────────────────────────

/// Validate each output listed in the pipeline against its declared schema.
///
/// Supported schema names (pipeline `args[0]`):
///   - `board-interface` — validates JSON against the `BoardInterface` schema
///
/// Does not run any external process; validates in-process using `fiducial-eda`.
fn run_fid_validate(pipeline: &PipelineToml, working_dir: &Path) -> Result<()> {
    let schema = pipeline
        .args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("board-interface");

    match schema {
        "board-interface" => {
            for out in &pipeline.outputs {
                let abs = working_dir.join(out);
                let json =
                    std::fs::read_to_string(&abs).with_context(|| format!("reading {out}"))?;
                validate_board_interface(&json).map_err(|e| anyhow::anyhow!("{out}: {e}"))?;
            }
            Ok(())
        }
        other => bail!("fid-validate: unknown schema `{other}` (supported: board-interface)"),
    }
}

// ── Command execution ─────────────────────────────────────────────────────────

fn run_pipeline_command(pipeline: &PipelineToml, working_dir: &Path) -> Result<()> {
    let (program, base_args, extra_args): (&str, Vec<&str>, Vec<&str>) = match pipeline
        .executor
        .as_str()
    {
        "cargo-test" => (
            "cargo",
            vec!["test"],
            pipeline.args.iter().map(|s| s.as_str()).collect(),
        ),
        "shell" => {
            if pipeline.args.is_empty() {
                bail!("shell executor requires at least one arg (the command)");
            }
            (
                pipeline.args[0].as_str(),
                pipeline.args[1..].iter().map(|s| s.as_str()).collect(),
                Vec::new(),
            )
        }
        "fid-validate" => return run_fid_validate(pipeline, working_dir),
        other => bail!("unknown executor `{other}` (supported: cargo-test, shell, fid-validate)"),
    };

    let status = Command::new(program)
        .args(&base_args)
        .args(&extra_args)
        .current_dir(working_dir)
        .status()
        .with_context(|| format!("spawning `{program}`"))?;

    if !status.success() {
        bail!("pipeline `{}` exited with {}", pipeline.name, status);
    }
    Ok(())
}

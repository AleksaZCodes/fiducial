//! `fid derive [--check]` — run pipelines.
//!
//! Phase 4 deliverable. The stub is here so help text and shell completion are
//! correct from day one, and so CI workflows can reference `fid derive --check`
//! before the implementation is wired up.

use anyhow::Result;

pub fn run(check: bool, pipeline: Option<String>) -> Result<()> {
    let scope = match &pipeline {
        Some(p) => format!("pipeline `{p}`"),
        None => "all pipelines".into(),
    };
    let mode = if check { " (--check)" } else { "" };
    println!(
        "✦ fid derive{mode} — {scope}\n\n\
         Pipeline execution is coming in Phase 4.\n\
         Run `cargo build` / `wasm-pack build` / `cargo check` directly for now.\n\n\
         Track progress: https://github.com/AleksaZCodes/fiducial"
    );
    Ok(())
}

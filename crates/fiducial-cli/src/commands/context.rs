//! `fid context` — regenerate the derivable parts of agent context.

use anyhow::{Context as _, Result};
use clap::CommandFactory;

use crate::{
    config::Config,
    context::{render_file, CONTEXT_FILES},
};

/// Regenerate, or check, every marked block in this repository's context files.
pub fn run(check: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;
    // A repository, not necessarily a product: the platform itself has context
    // files and deliberately declares no pipelines.
    let root = Config::find_root(&cwd).unwrap_or(cwd);

    let command = crate::Cli::command();

    println!(
        "✦ fid context{} — {}",
        if check { " --check" } else { "" },
        root.display()
    );
    println!();

    let mut stale: Vec<String> = Vec::new();
    let mut touched = 0usize;

    for rel in CONTEXT_FILES {
        let Some(rendered) = render_file(&root, rel, &command)? else {
            continue;
        };
        touched += 1;
        let names: Vec<&str> = rendered.blocks.iter().map(|b| b.name()).collect();

        if rendered.fresh {
            println!("  ✓ {:<12} {}", rendered.path, names.join(", "));
            continue;
        }
        if check {
            println!("  ✗ {:<12} {} — stale", rendered.path, names.join(", "));
            stale.push(rendered.path.clone());
        } else {
            std::fs::write(root.join(&rendered.path), &rendered.content)
                .with_context(|| format!("writing {}", rendered.path))?;
            println!("  ↻ {:<12} {}", rendered.path, names.join(", "));
        }
    }

    println!();
    if touched == 0 {
        println!(
            "No generated blocks found. Mark one with:\n\n  \
             <!-- fid:begin layout -->\n  <!-- fid:end layout -->\n"
        );
        return Ok(());
    }

    if !stale.is_empty() {
        anyhow::bail!(
            "{} file(s) out of date with what they describe — run `fid context`:\n  {}",
            stale.len(),
            stale.join("\n  ")
        );
    }
    println!(
        "✦ fid context{}: agent context matches the code",
        if check { " --check" } else { " complete" }
    );
    Ok(())
}

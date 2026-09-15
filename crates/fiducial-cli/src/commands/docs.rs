//! `fid docs` — documentation freshness, including the parts nothing can generate.
//!
//! Two kinds of documentation rot, and this command reports both because a
//! reader asking "is the documentation current?" does not care which mechanism
//! answers:
//!
//! - A **generated block** drifting from what it renders. Owned by
//!   `fid context`, delegated to here so there is one question to ask.
//! - A **prose block** drifting from what it describes. Owned by
//!   `crates/fiducial-cli/src/prose.rs`, and the reason this command exists —
//!   nothing was watching prose at all, which is how `AGENTS.md` claimed every
//!   adapter contract implemented only `none` for six vendors after it stopped
//!   being true.

use anyhow::{Context as _, Result};

use crate::{
    config::Config,
    context::CONTEXT_FILES,
    prose::{self, ProseLock},
};

/// Documents scanned for prose blocks.
///
/// The context files plus everything under `docs/`. A prose block is opt-in per
/// block, so this list is "where to look", not "what must be annotated" — a
/// document with no markers is not a finding. Annotating everything would be
/// the ceremony that kills the mechanism; annotate the paragraph that would be
/// expensive to have wrong.
fn documents(root: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = CONTEXT_FILES.iter().map(|s| s.to_string()).collect();
    out.push("ROADMAP.md".to_string());
    out.push("ARCHITECTURE.md".to_string());
    out.push("STACK.md".to_string());

    let mut stack = vec![root.join("docs")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.display().to_string());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn run(check: bool, accept: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd).unwrap_or(cwd);

    println!(
        "✦ fid docs{} — {}",
        if check {
            " --check"
        } else if accept {
            " --accept"
        } else {
            ""
        },
        root.display()
    );
    println!();

    let docs = documents(&root);
    let report = prose::check(&root, &docs)?;
    let prose::Report {
        stale,
        current,
        total: watched,
    } = report;

    if accept {
        // Accepting records the hash of every watched source as it stands now.
        // It is the one place a human or an agent says "I have read this
        // paragraph against the thing it describes", so it is deliberately a
        // separate verb rather than something `--check` does when it fails.
        let mut lock = ProseLock::load(&root)?;
        let newly = stale.len();
        lock.accepted = current;
        lock.save(&root)?;
        println!("  ↻ {} prose block(s) accepted", newly);
        println!();
        println!("✦ fid docs --accept: {watched} block(s) now recorded as read");
        return Ok(());
    }

    if watched == 0 {
        println!("  No prose blocks declared. Mark one with:");
        println!();
        println!("    <!-- fid:describes path/to/source.rs#Symbol -->");
        println!("    …the paragraph that would be wrong if that source moved…");
        println!("    <!-- fid:end-describes -->");
        println!();
        return Ok(());
    }

    for s in &stale {
        println!(
            "  ✗ {}:{}  describes `{}`",
            s.block.document, s.block.line, s.block.target
        );
        println!("      {}", s.reason);
    }

    if stale.is_empty() {
        println!("  ✓ {watched} prose block(s) current with what they describe");
        println!();
        println!("✦ fid docs{}: documentation matches the code", {
            if check {
                " --check"
            } else {
                ""
            }
        });
        return Ok(());
    }

    println!();
    if check {
        anyhow::bail!(
            "{} of {watched} prose block(s) describe something that has changed.\n\n\
             This is not a formatting failure. Re-read each paragraph above against \
             the source it names,\nfix what is now wrong, then record that you did:\n\n  \
             fid docs --accept\n\n\
             Accepting without reading is the one way to make this gate worthless.",
            stale.len()
        );
    }
    println!(
        "{} of {watched} block(s) need re-reading. Then: fid docs --accept",
        stale.len()
    );
    Ok(())
}

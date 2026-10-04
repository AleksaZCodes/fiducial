//! `fid rebaseline` — accept a deliberate fork of a platform-owned file.
//!
//! # What re-baselining is, and what it is not
//!
//! `fiducial.lock` records, for every scaffolded file, the content the platform
//! wrote. That record is a **merge base**: `fid upgrade` uses it to tell an
//! upstream change apart from a local one and fold the two together. Next to
//! it is the file's hash, which `fid doctor` compares. Re-baselining moves the
//! hash to the file as it stands now — declaring "this version is mine, stop
//! treating the difference as news" — and leaves the base alone, so the next
//! upgrade still merges into the fork instead of over it.
//!
//! That is a coherent thing to want for exactly one category of file.
//!
//! - For a **product-owned** file it is destructive and never right. The base
//!   must stay at the original scaffold, because that is the only thing that
//!   makes a 3-way merge possible; overwrite it and the next `fid upgrade`
//!   cannot tell your paragraph from the platform's. `fid doctor` no longer
//!   reports these at all, so there is nothing to silence. This command refuses
//!   them.
//!
//! - For a **platform-owned** file it is the honest way to record a fork you
//!   meant. The first product has six: a `ci.yml` with its own pin, a customised
//!   `doodle.tsx`, three forked derive scripts. Each one is real — `fid
//!   upgrade` will merge over them — and each is intentional. Re-baselining
//!   says so, in the lock, where the next upgrade can see it.
//!
//! # Why it takes explicit paths and has no `--all`
//!
//! Re-baselining is how you stop hearing about a divergence, and the whole
//! value of the warning is that it fires before `fid upgrade` overwrites work.
//! A flag that silences every one of them at once converts a list of real
//! findings into silence in a single keystroke, which is the same failure as
//! `continue-on-error` and reached faster.
//!
//! So each path is named. Six is not many, and each deserves the half-second.

use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::{lock::Lock, ownership};

pub fn run(paths: &[String]) -> Result<()> {
    let root = std::env::current_dir().context("reading the working directory")?;
    let lock_path = root.join("fiducial.lock");
    let mut lock = Lock::load(&lock_path).with_context(|| {
        format!(
            "reading {}. Is this a scaffolded product?",
            lock_path.display()
        )
    })?;

    if paths.is_empty() {
        return list(&root, &lock);
    }

    let mut accepted = Vec::new();
    for raw in paths {
        let rel = raw.trim_start_matches("./").replace('\\', "/");

        let record = lock.templates.get(&rel).ok_or_else(|| {
            anyhow::anyhow!(
                "`{rel}` is not a template this product tracks.\n\
                 Run `fid rebaseline` with no arguments to see what can be accepted."
            )
        })?;

        if ownership::of(&rel).is_product() {
            bail!(
                "`{rel}` is product-owned — editing it is the intended use, and \
                 `fid doctor` does not report it.\n\n  \
                 Re-baselining it would replace the merge base `fid upgrade` needs to \
                 fold an upstream\n  change into your edits, so the next upgrade could \
                 no longer tell the two apart.\n  Nothing here needs accepting."
            );
        }

        let abs = root.join(&rel);
        let content = std::fs::read(&abs).with_context(|| {
            format!(
                "reading {rel}. A deleted file cannot be re-baselined — there is no \
                 content to record. `fid upgrade` restores it."
            )
        })?;

        if crate::lock::sha256_hex(&content) == record.hash {
            println!("  · {rel} already matches the lock — nothing to accept");
            continue;
        }

        // Only the hash moves: it is what `fid doctor` compares, so the fork
        // stops being reported. The merge base stays what the platform
        // shipped. Recording *your* content as the base — as this once did —
        // made the next upgrade see no local change and take upstream whole:
        // a product's CI fork lost its hardware and web jobs to a
        // "merged cleanly". With the platform's base, an upstream change is
        // folded into your version, or conflicts where both touched a line.
        // The source version stays too: `fid doctor` reads it to decide
        // whether upstream has moved since, a question this has not answered.
        let hash = crate::lock::sha256_hex(&content);
        if let Some(record) = lock.templates.get_mut(&rel) {
            record.hash = hash;
        }
        accepted.push(rel);
    }

    if accepted.is_empty() {
        println!("\n  Nothing changed.");
        return Ok(());
    }

    lock.save(&lock_path)
        .with_context(|| format!("writing {}", lock_path.display()))?;

    println!("✦ fid rebaseline — accepted {} fork(s)", accepted.len());
    println!();
    for rel in &accepted {
        println!("  ✓ {rel}");
    }
    println!();
    println!("  Recorded in fiducial.lock as yours. `fid doctor` stops reporting them,");
    println!("  and `fid upgrade` now merges upstream changes into your version rather");
    println!("  than over it.");
    println!();
    Ok(())
}

/// With no arguments: what could be accepted, and the two ways to resolve it.
fn list(root: &Path, lock: &Lock) -> Result<()> {
    let forks: Vec<(String, String)> = lock
        .verify(root)
        .into_iter()
        .filter_map(|(path, problem)| {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            (!ownership::of(&rel).is_product()).then_some((rel, problem))
        })
        .collect();

    println!("✦ fid rebaseline — {}", root.display());
    println!();

    if forks.is_empty() {
        println!("  No platform-owned file differs from what the platform wrote.");
        println!();
        println!("  Edits to your own files are not listed here and never need accepting —");
        println!("  `fid doctor` counts them and moves on.");
        println!();
        return Ok(());
    }

    println!(
        "  {} platform-owned file(s) differ from what the platform wrote.",
        forks.len()
    );
    println!("  `fid upgrade` will merge over each one unless you accept it.");
    println!();
    for (rel, problem) in &forks {
        println!("    {rel}");
        println!("      {problem}");
    }
    println!();
    println!("  Two ways out, per file:");
    println!();
    println!("    fid rebaseline <path>   keep your version — record it as the new base");
    println!("    fid upgrade             take the platform's, merging where it can");
    println!();
    println!("  There is no --all. Each of these is a real divergence that `fid upgrade`");
    println!("  is about to act on, and silencing the list in one keystroke is how a");
    println!("  warning stops being one.");
    println!();
    Ok(())
}

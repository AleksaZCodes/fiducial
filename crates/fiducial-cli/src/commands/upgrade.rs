//! `fid upgrade` — propagate platform updates into this product.
//!
//! Phase 4 deliverable. Stub present for help text and CI reference.

use anyhow::Result;

pub fn run(dry_run: bool, portfolio: bool) -> Result<()> {
    let mut flags = Vec::new();
    if dry_run {
        flags.push("--dry-run");
    }
    if portfolio {
        flags.push("--portfolio");
    }
    let flag_str = if flags.is_empty() {
        String::new()
    } else {
        format!(" {}", flags.join(" "))
    };
    println!(
        "✦ fid upgrade{flag_str}\n\n\
         Template propagation (3-way merge) and codemod application are coming in Phase 4.\n\n\
         Track progress: https://github.com/AleksaZCodes/fiducial"
    );
    Ok(())
}

//! `fid design` — is the published gallery the design system this product declares?
//!
//! # What this command is, and is not
//!
//! It does **not** upload. The upload is `/design-sync`, which runs in an agent
//! with the user's claude.ai authorization; a CLI cannot hold that credential
//! and pretending otherwise would put a `push` in the help text that never
//! pushes.
//!
//! What a CLI *can* own is the half that goes wrong silently: whether the
//! bundle about to be published is the design system this repository declares,
//! and whether every page in it will actually appear once it arrives.
//!
//! Two checks, and both have a failure mode that is invisible until someone
//! opens Claude Design and sees the wrong thing:
//!
//! 1. **The gallery is stale.** `fid derive` regenerates `tokens.css` from
//!    `design-system.md`; the gallery is rebuilt by a separate `node` script.
//!    Run the first without the second and the published swatches describe a
//!    palette the product no longer has — while every other gate stays green,
//!    because the app builds fine either way.
//!
//! 2. **A page has no card marker.** The Design System pane builds its index
//!    from each preview's first-line `<!-- @dsCard group="…" -->` comment. A
//!    page without one uploads successfully and then simply is not there, which
//!    reads as "the sync dropped a file" rather than "the file was malformed".
//!
//! `--plan` prints the write set for `/design-sync` to hand to `finalize_plan`,
//! so the paths come from the directory rather than from someone retyping them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::config::Config;

/// Where the gallery build script writes, by convention.
const GALLERY_DIR: &str = "design-system";

/// One preview page and the section it declares itself into.
struct Card {
    path: String,
    group: Option<String>,
}

/// Read the `group` out of a first-line `<!-- @dsCard group="…" -->` marker.
///
/// First line only, deliberately: that is where the pane looks, and a marker
/// further down is a marker that does not work. Finding it anywhere would make
/// this check pass on a file the app then ignores.
fn card_group(content: &str) -> Option<String> {
    let first = content.lines().next()?.trim();
    let rest = first.strip_prefix("<!-- @dsCard")?;
    let start = rest.find("group=\"")? + "group=\"".len();
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_string())
}

fn collect(dir: &Path, root: &Path, out: &mut Vec<Card>) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, root, out)?;
        } else if path.extension().is_some_and(|e| e == "html") {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(Card {
                path: rel,
                group: card_group(&content),
            });
        }
    }
    Ok(())
}

pub fn run(check: bool, plan: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let gallery = root.join(GALLERY_DIR);

    if !gallery.is_dir() {
        bail!(
            "no `{GALLERY_DIR}/` in this product.\n  \
             It is written by `node scripts/build-design-system.mjs`, which the \
             `design` capability installs."
        );
    }

    let mut cards = Vec::new();
    collect(&gallery, &gallery, &mut cards)?;

    let mut issues: Vec<String> = Vec::new();

    // ── 1 · is the gallery built from the tokens the product declares? ───────
    //
    // The gallery copies the derived stylesheet beside its previews. If that
    // copy has drifted from the real one, the gallery was built before the last
    // `fid derive` and every swatch in it is describing an older palette.
    let derived = root.join("apps/web/src/app/tokens.css");
    let copied = gallery.join("tokens.css");
    let token_state = match (
        std::fs::read_to_string(&derived),
        std::fs::read_to_string(&copied),
    ) {
        (Ok(a), Ok(b)) if a == b => "fresh",
        (Ok(_), Ok(_)) => {
            issues.push(format!(
                "{GALLERY_DIR}/tokens.css differs from the derived stylesheet — \
                 the gallery was built before the last `fid derive`\n      \
                 rebuild it: node scripts/build-design-system.mjs"
            ));
            "stale"
        }
        (Err(_), _) => "no derived stylesheet yet (run `fid derive`)",
        (_, Err(_)) => {
            issues.push(format!("{GALLERY_DIR}/tokens.css is missing"));
            "missing"
        }
    };

    // ── 2 · will every page actually appear? ────────────────────────────────
    for card in &cards {
        if card.group.is_none() {
            issues.push(format!(
                "{GALLERY_DIR}/{}: no `<!-- @dsCard group=\"…\" -->` on line 1 — \
                 it uploads and then does not appear",
                card.path
            ));
        }
    }

    if plan {
        // The write set, from the directory rather than from someone retyping
        // it. `/design-sync` hands this to `finalize_plan`.
        let paths: Vec<String> = std::iter::once("tokens.css".to_string())
            .chain(cards.iter().map(|c| c.path.clone()))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "localDir": gallery.to_string_lossy(),
                "writes": paths,
                "deletes": [],
            }))?
        );
        return Ok(());
    }

    println!("✦ fid design — {}", root.display());
    println!();

    let mut by_group: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for c in &cards {
        by_group
            .entry(c.group.as_deref().unwrap_or("(no marker)"))
            .or_default()
            .push(&c.path);
    }
    for (group, paths) in &by_group {
        println!("  {group}");
        for p in paths {
            println!("    · {p}");
        }
    }
    println!();
    println!("  {} card(s), tokens {token_state}", cards.len());

    if issues.is_empty() {
        println!("  ✓ the gallery matches the declaration and every page carries a card marker");
        println!();
        println!("  Publish it with `/design-sync` — `fid design --plan` prints the write set.");
        return Ok(());
    }

    println!();
    for i in &issues {
        println!("  ✗ {i}");
    }
    if check {
        bail!("{} design gallery issue(s)", issues.len());
    }
    Ok(())
}

/// Path for tests and callers that want the gallery location without the IO.
#[allow(dead_code)]
pub fn gallery_dir(root: &Path) -> PathBuf {
    root.join(GALLERY_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_on_the_first_line_is_read() {
        let html = "<!-- @dsCard group=\"Foundations\" -->\n<!doctype html>\n";
        assert_eq!(card_group(html).as_deref(), Some("Foundations"));
    }

    #[test]
    fn a_marker_below_the_first_line_does_not_count() {
        // The pane reads line 1. A marker on line 2 is a marker that does not
        // work, and a check that accepted it would pass a page the app ignores.
        let html = "<!doctype html>\n<!-- @dsCard group=\"Foundations\" -->\n";
        assert!(card_group(html).is_none());
    }

    #[test]
    fn a_page_with_no_marker_is_reported_not_guessed() {
        assert!(card_group("<!doctype html>\n").is_none());
        // Nor does a marker without a group become one.
        assert!(card_group("<!-- @dsCard -->\n").is_none());
    }

    #[test]
    fn leading_whitespace_is_tolerated() {
        assert_eq!(
            card_group("  <!-- @dsCard group=\"Components\" -->\n").as_deref(),
            Some("Components")
        );
    }
}

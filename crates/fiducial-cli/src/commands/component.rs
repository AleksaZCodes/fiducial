//! `fid add component <name> [--framework react|svelte]`
//!
//! Copies a UI component from the embedded registry into the product's
//! `apps/web/src/components/ui/` directory.
//!
//! Components become product-owned after copy — no runtime package dep.
//! `fid upgrade` tracks them via `fiducial.lock` and offers upstream changes
//! through 3-way merge, same as scaffolded template files.

use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::{
    config::Config,
    lock::{Lock, LOCK_FILE},
};

pub fn run(name: &str, framework: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let lock_path = root.join(LOCK_FILE);

    let mut lock = if lock_path.exists() {
        Lock::load(&lock_path)?
    } else {
        Lock::new()
    };

    let version = env!("CARGO_PKG_VERSION");

    match framework {
        "react" => install_react(name, &root, &mut lock, version)?,
        "svelte" => install_svelte(name, &root, &mut lock, version)?,
        other => bail!("unknown framework `{other}` — supported: react, svelte"),
    }

    lock.save(&lock_path)?;
    println!("✦ fid add component {name} ({framework}) installed");
    println!(
        "  → apps/web/src/components/ui/{}",
        component_filename(name, framework)
    );
    if name == "dialog" && framework == "react" {
        println!(
            "\n  Dialog uses @base-ui-components/react for accessibility.\n  \
             Install it:\n    pnpm add @base-ui-components/react --filter @<product>/web"
        );
    }
    Ok(())
}

fn component_filename(name: &str, framework: &str) -> String {
    match framework {
        "svelte" => format!("{}.svelte", capitalise(name)),
        _ => format!("{name}.tsx"),
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn install_react(name: &str, root: &Path, lock: &mut Lock, version: &str) -> Result<()> {
    let (tsx, css) = react_source(name)?;

    let ui_dir = root.join("apps/web/src/components/ui");
    std::fs::create_dir_all(&ui_dir).context("creating components/ui")?;

    let tsx_path = ui_dir.join(format!("{name}.tsx"));
    let tsx_rel = format!("apps/web/src/components/ui/{name}.tsx");
    std::fs::write(&tsx_path, tsx).with_context(|| format!("writing {}", tsx_path.display()))?;
    lock.record(&tsx_rel, tsx.as_bytes(), version);

    let css_path = ui_dir.join(format!("{name}.css"));
    let css_rel = format!("apps/web/src/components/ui/{name}.css");
    std::fs::write(&css_path, css).with_context(|| format!("writing {}", css_path.display()))?;
    lock.record(&css_rel, css.as_bytes(), version);

    Ok(())
}

fn install_svelte(name: &str, root: &Path, lock: &mut Lock, version: &str) -> Result<()> {
    let src = svelte_source(name)?;
    let cap_name = capitalise(name);

    let ui_dir = root.join("apps/web/src/components/ui");
    std::fs::create_dir_all(&ui_dir).context("creating components/ui")?;

    let path = ui_dir.join(format!("{cap_name}.svelte"));
    let rel = format!("apps/web/src/components/ui/{cap_name}.svelte");
    std::fs::write(&path, src).with_context(|| format!("writing {}", path.display()))?;
    lock.record(&rel, src.as_bytes(), version);

    Ok(())
}

// ── Embedded component sources ────────────────────────────────────────────────

fn react_source(name: &str) -> Result<(&'static str, &'static str)> {
    match name {
        "button" => Ok((
            include_str!("../../components/react/button.tsx"),
            include_str!("../../components/react/button.css"),
        )),
        "card" => Ok((
            include_str!("../../components/react/card.tsx"),
            include_str!("../../components/react/card.css"),
        )),
        "badge" => Ok((
            include_str!("../../components/react/badge.tsx"),
            include_str!("../../components/react/badge.css"),
        )),
        "dialog" => Ok((
            include_str!("../../components/react/dialog.tsx"),
            include_str!("../../components/react/dialog.css"),
        )),
        other => bail!(
            "unknown component `{other}`\n  \
             Available: button, card, badge, dialog\n  \
             Run `fid add component --help` for details."
        ),
    }
}

fn svelte_source(name: &str) -> Result<&'static str> {
    match name {
        "button" => Ok(include_str!("../../components/svelte/Button.svelte")),
        "card" => Ok(include_str!("../../components/svelte/Card.svelte")),
        "badge" => Ok(include_str!("../../components/svelte/Badge.svelte")),
        "dialog" => Ok(include_str!("../../components/svelte/Dialog.svelte")),
        other => bail!(
            "unknown component `{other}`\n  \
             Available: button, card, badge, dialog\n  \
             Run `fid add component --help` for details."
        ),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn react_source_returns_all_components() {
        for name in ["button", "card", "badge", "dialog"] {
            let (tsx, css) = react_source(name).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!tsx.is_empty(), "{name}.tsx is empty");
            assert!(!css.is_empty(), "{name}.css is empty");
        }
    }

    #[test]
    fn svelte_source_returns_all_components() {
        for name in ["button", "card", "badge", "dialog"] {
            let src = svelte_source(name).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!src.is_empty(), "{name}.svelte is empty");
        }
    }

    #[test]
    fn unknown_component_errors() {
        assert!(react_source("nonexistent").is_err());
        assert!(svelte_source("nonexistent").is_err());
    }

    #[test]
    fn capitalise_works() {
        assert_eq!(capitalise("button"), "Button");
        assert_eq!(capitalise("dialog"), "Dialog");
        assert_eq!(capitalise(""), "");
    }
}

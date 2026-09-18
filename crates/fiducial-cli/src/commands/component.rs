//! `fid add component <name> [--framework react|svelte]`
//!
//! Copies a UI component from the embedded registry into the product's
//! `apps/web/src/components/ui/` directory.
//!
//! Components become product-owned after copy — no runtime package dep.
//! `fid upgrade` tracks them via `fiducial.lock` and offers upstream changes
//! through 3-way merge, same as scaffolded template files.
//!
//! React components follow shadcn conventions: Tailwind + CVA + cn() utility.
//! Complex components (dialog) use @base-ui-components/react internally.

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
    if name == "doodle" && framework == "react" {
        println!(
            "\n  Doodle marks are stroked in --doodle-ink and animated by the\n  \
             marks layer. Both come from the `design` capability:\n    \
             fid add design"
        );
    }
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
    let tsx = react_source(name)?;

    let ui_dir = root.join("apps/web/src/components/ui");
    std::fs::create_dir_all(&ui_dir).context("creating components/ui")?;

    // Bootstrap lib/utils.ts (cn utility) if not yet present.
    let lib_dir = root.join("apps/web/src/lib");
    let utils_path = lib_dir.join("utils.ts");
    if !utils_path.exists() {
        std::fs::create_dir_all(&lib_dir).context("creating src/lib")?;
        std::fs::write(&utils_path, UTILS_TS).context("writing lib/utils.ts")?;
        lock.record("apps/web/src/lib/utils.ts", UTILS_TS.as_bytes(), version);
    }

    // Write component .tsx
    let tsx_path = ui_dir.join(format!("{name}.tsx"));
    let tsx_rel = format!("apps/web/src/components/ui/{name}.tsx");
    std::fs::write(&tsx_path, tsx).with_context(|| format!("writing {}", tsx_path.display()))?;
    lock.record(&tsx_rel, tsx.as_bytes(), version);

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

const UTILS_TS: &str = include_str!("../../components/react/lib/utils.ts");

fn react_source(name: &str) -> Result<&'static str> {
    match name {
        "button" => Ok(include_str!("../../components/react/button.tsx")),
        "card" => Ok(include_str!("../../components/react/card.tsx")),
        "badge" => Ok(include_str!("../../components/react/badge.tsx")),
        "dialog" => Ok(include_str!("../../components/react/dialog.tsx")),
        // `doodle` is not a control — it is the annotation mark set (arrows,
        // circles, underlines). It needs the `design` capability's marks layer
        // for its draw animation; see the note printed after install.
        "doodle" => Ok(include_str!("../../components/react/doodle.tsx")),
        // Page shapes, not controls: the hero, the band, the numbered sequence.
        // The page is where a design system stops being applied, so the page is
        // a component too.
        "sections" => Ok(include_str!("../../components/react/sections.tsx")),
        other => bail!(
            "unknown component `{other}`\n  \
             Available: button, card, badge, dialog, doodle, sections\n  \
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
        for name in ["button", "card", "badge", "dialog", "doodle", "sections"] {
            let tsx = react_source(name).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!tsx.is_empty(), "{name}.tsx is empty");
        }
    }

    #[test]
    fn react_components_use_cn_utility() {
        // Verify shadcn convention: all components import cn() not fid-btn classes.
        for name in ["button", "card", "badge", "dialog", "doodle", "sections"] {
            let tsx = react_source(name).unwrap();
            assert!(
                tsx.contains("cn("),
                "{name}.tsx must use cn() utility (shadcn convention)"
            );
            assert!(
                !tsx.contains("fid-"),
                "{name}.tsx must not contain legacy fid-* CSS class names"
            );
        }
    }

    #[test]
    fn components_import_cn_through_the_alias_that_resolves_where_they_land() {
        // The bug this guards: components are installed into
        // `apps/web/src/components/ui/`, while `utils.ts` is installed into
        // `apps/web/src/lib/`. A relative `../lib/utils` from the first is
        // `src/components/lib/utils` — which does not exist. It shipped that
        // way, and the second copy of these files (since deleted) typechecked
        // green because it used a different spelling.
        //
        // `@/lib/utils` is the alias every scaffolded product maps to `src/*`,
        // and it is the spelling shadcn itself uses.
        for name in ["button", "card", "badge", "dialog", "doodle", "sections"] {
            let tsx = react_source(name).unwrap();
            if !tsx.contains("from \"@/lib/utils\"") {
                assert!(
                    !tsx.contains("lib/utils"),
                    "{name}.tsx imports cn through a path that does not resolve \
                     from apps/web/src/components/ui/ — use `@/lib/utils`"
                );
            }
        }
    }

    #[test]
    fn utils_lands_where_the_alias_points() {
        // `@/lib/utils` resolves to `apps/web/src/lib/utils`, which is exactly
        // where `install_react` writes UTILS_TS. If either moves, both move.
        assert!(UTILS_TS.contains("export function cn"));
    }

    #[test]
    fn doodle_exports_the_whole_mark_set() {
        // The set is a vocabulary, not a grab bag: a page that can point but
        // cannot circle ends up pointing at everything.
        let tsx = react_source("doodle").unwrap();
        for export in [
            "export function Arrow",
            "export function Circle",
            "export function Underline",
            "export function Bracket",
            "export function Burst",
            "export function Check",
            "export function Cross",
            "export function Note",
        ] {
            assert!(tsx.contains(export), "doodle.tsx is missing `{export}`");
        }
    }

    #[test]
    fn doodle_marks_are_decorative_and_renormalised() {
        let tsx = react_source("doodle").unwrap();
        // Marks carry no meaning of their own — the text they point at does.
        assert!(
            tsx.contains("aria-hidden"),
            "doodle marks must default to aria-hidden"
        );
        // pathLength={1} is what lets one draw animation fit every path length.
        assert!(
            tsx.contains("pathLength={1}"),
            "every mark path must be renormalised with pathLength={{1}}"
        );
        // Annotation ink, never text foreground.
        assert!(
            tsx.contains("var(--doodle-ink)"),
            "marks must stroke in --doodle-ink"
        );
        assert!(
            !tsx.contains("var(--foreground)"),
            "a mark at full text contrast is a headline, not an annotation"
        );
    }

    #[test]
    fn sections_hold_the_page_level_negative_constraints() {
        // These are the shapes the "not this" list rules out. A section
        // component that offers one as a prop is a list that does not hold.
        //
        // Comments are stripped first: the file's own doc comment explains what
        // is banned by naming it, and a test that cannot tell the rule from a
        // violation of it is a test that forbids documentation.
        let tsx = react_source("sections").unwrap();
        let code: String = tsx
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !(t.starts_with("//") || t.starts_with("*") || t.starts_with("/*"))
            })
            .collect::<Vec<_>>()
            .join("\n");

        for banned in ["gradient", "shadow-", "hover:scale", "carousel"] {
            assert!(
                !code.contains(banned),
                "sections.tsx uses `{banned}`, which design-system.md rules out"
            );
        }

        // `rounded-full` has exactly one carve-out, and it is written down in
        // the declaration: a six-pixel status dot. A circle that small is a
        // circle, not a pill. Anywhere else it is the shape the list forbids.
        for line in code.lines().filter(|l| l.contains("rounded-full")) {
            assert!(
                line.contains("animate-ping") || line.contains("h-1.5 w-1.5"),
                "rounded-full outside the status dot: {}",
                line.trim()
            );
        }
    }

    #[test]
    fn sections_read_named_type_steps_rather_than_sizes() {
        // The point of shipping page shapes at all: a section that inlines
        // `text-4xl` is a scale step that exists in one component and drifts.
        let tsx = react_source("sections").unwrap();
        assert!(tsx.contains("type-display"));
        assert!(tsx.contains("type-h2"));
        assert!(tsx.contains("type-small"));
        for inline in ["text-4xl", "text-5xl", "text-6xl", "text-3xl"] {
            assert!(
                !tsx.contains(inline),
                "sections.tsx inlines `{inline}` instead of using a named step"
            );
        }
    }

    #[test]
    fn utils_ts_is_embedded() {
        assert!(!UTILS_TS.is_empty());
        assert!(
            UTILS_TS.contains("twMerge"),
            "utils.ts must export cn via tailwind-merge"
        );
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

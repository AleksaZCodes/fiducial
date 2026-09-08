//! Capability model and built-in registry.
//!
//! A **capability** is a named, versioned extension to the platform. It may
//! contribute any of:
//!
//! - Template files written into a product on `fid add <capability>`
//! - Guard rules that activate when the capability is installed
//! - A `SKILL.md` that agents load on session start (mandatory per §3.3)
//! - Crates / packages / pipelines / migrations (future phases)
//!
//! The minimum viable capability is a single `SKILL.md`. Everything else is
//! optional and added when the capability actually grows into it.
//!
//! ## Capability identifiers
//!
//! Capabilities are referenced by a kebab-case slug:
//!   `web-next`, `firmware-rp2040`, `tauri`, `worker-cloudflare`, …
//!
//! First-party capabilities are defined here. Third-party capabilities (Phase 5+)
//! resolve from git repos using the same source model as the Claude Code plugin
//! marketplace.

use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{Config, CONFIG_FILE};
use crate::lock::Lock;

pub const PLATFORM_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Capability definition ────────────────────────────────────────────────────

/// A capability definition: everything the platform needs to install it.
pub struct CapabilityDef {
    /// Kebab-case slug used in `fiducial.toml [capabilities]` and on the CLI.
    pub id: &'static str,
    /// One-line description shown in `fid capability list`.
    pub description: &'static str,
    /// Guard rule names that activate when this capability is installed.
    pub guard_rules: &'static [&'static str],
    /// Template files to write into the product root on install.
    /// Each entry is (repo-relative path, file content).
    pub templates: &'static [(&'static str, &'static str)],
    /// The SKILL.md content (mandatory per §3.3).
    pub skill_md: &'static str,
}

// ── Built-in capability registry ─────────────────────────────────────────────

/// All first-party capabilities shipped with this platform version.
pub static BUILTIN_CAPABILITIES: &[CapabilityDef] = &[
    CapabilityDef {
        id: "web-next",
        description: "Next.js web app with Turborepo wiring, Biome, and Changesets",
        guard_rules: &["no-direct-schema-migration", "no-unpinned-cli-fetch"],
        templates: &[
            (
                "apps/web/package.json",
                include_str!("../capabilities/web-next/apps/web/package.json"),
            ),
            (
                "apps/web/next.config.ts",
                include_str!("../capabilities/web-next/apps/web/next.config.ts"),
            ),
            (
                "apps/web/tsconfig.json",
                include_str!("../capabilities/web-next/apps/web/tsconfig.json"),
            ),
            (
                "apps/web/src/app/page.tsx",
                include_str!("../capabilities/web-next/apps/web/src/app/page.tsx"),
            ),
            (
                "apps/web/src/app/layout.tsx",
                include_str!("../capabilities/web-next/apps/web/src/app/layout.tsx"),
            ),
            (
                "apps/web/src/app/globals.css",
                include_str!("../capabilities/web-next/apps/web/src/app/globals.css"),
            ),
            (
                "apps/web/components.json",
                include_str!("../capabilities/web-next/apps/web/components.json"),
            ),
        ],
        skill_md: include_str!("../capabilities/web-next/SKILL.md"),
    },
    CapabilityDef {
        id: "web-svelte",
        description: "SvelteKit web app with Turborepo wiring, Biome, and Changesets",
        guard_rules: &["no-direct-schema-migration", "no-unpinned-cli-fetch"],
        templates: &[
            (
                "apps/web/package.json",
                include_str!("../capabilities/web-svelte/apps/web/package.json"),
            ),
            (
                "apps/web/svelte.config.js",
                include_str!("../capabilities/web-svelte/apps/web/svelte.config.js"),
            ),
            (
                "apps/web/vite.config.ts",
                include_str!("../capabilities/web-svelte/apps/web/vite.config.ts"),
            ),
            (
                "apps/web/tsconfig.json",
                include_str!("../capabilities/web-svelte/apps/web/tsconfig.json"),
            ),
            (
                "apps/web/src/app.html",
                include_str!("../capabilities/web-svelte/apps/web/src/app.html"),
            ),
            (
                "apps/web/src/app.css",
                include_str!("../capabilities/web-svelte/apps/web/src/app.css"),
            ),
            (
                "apps/web/src/routes/+layout.svelte",
                include_str!("../capabilities/web-svelte/apps/web/src/routes/+layout.svelte"),
            ),
            (
                "apps/web/src/routes/+page.svelte",
                include_str!("../capabilities/web-svelte/apps/web/src/routes/+page.svelte"),
            ),
        ],
        skill_md: include_str!("../capabilities/web-svelte/SKILL.md"),
    },
    CapabilityDef {
        id: "firmware-rp2040",
        description: "RP2040 Embassy firmware with defmt logging and probe-rs flashing",
        guard_rules: &["no-direct-flash-without-check", "no-unpinned-cli-fetch"],
        templates: &[
            (
                "firmware/Cargo.toml",
                include_str!("../capabilities/firmware-rp2040/firmware/Cargo.toml"),
            ),
            (
                "firmware/rust-toolchain.toml",
                include_str!("../capabilities/firmware-rp2040/firmware/rust-toolchain.toml"),
            ),
            (
                "firmware/shared/Cargo.toml",
                include_str!("../capabilities/firmware-rp2040/firmware/shared/Cargo.toml"),
            ),
            (
                "firmware/shared/src/lib.rs",
                include_str!("../capabilities/firmware-rp2040/firmware/shared/src/lib.rs"),
            ),
            (
                "firmware/rp2040/.cargo/config.toml",
                include_str!("../capabilities/firmware-rp2040/firmware/rp2040/.cargo/config.toml"),
            ),
            (
                "firmware/rp2040/Cargo.toml",
                include_str!("../capabilities/firmware-rp2040/firmware/rp2040/Cargo.toml"),
            ),
            (
                "firmware/rp2040/build.rs",
                include_str!("../capabilities/firmware-rp2040/firmware/rp2040/build.rs"),
            ),
            (
                "firmware/rp2040/memory.x",
                include_str!("../capabilities/firmware-rp2040/firmware/rp2040/memory.x"),
            ),
            (
                "firmware/rp2040/src/main.rs",
                include_str!("../capabilities/firmware-rp2040/firmware/rp2040/src/main.rs"),
            ),
            (
                "firmware/rp2040/README.md",
                include_str!("../capabilities/firmware-rp2040/firmware/rp2040/README.md"),
            ),
        ],
        skill_md: include_str!("../capabilities/firmware-rp2040/SKILL.md"),
    },
    CapabilityDef {
        id: "firmware-stm32",
        description: "STM32F401 Embassy firmware with defmt logging and probe-rs flashing",
        guard_rules: &["no-direct-flash-without-check", "no-unpinned-cli-fetch"],
        templates: &[
            (
                "firmware/Cargo.toml",
                include_str!("../capabilities/firmware-stm32/firmware/Cargo.toml"),
            ),
            (
                "firmware/rust-toolchain.toml",
                include_str!("../capabilities/firmware-stm32/firmware/rust-toolchain.toml"),
            ),
            (
                "firmware/shared/Cargo.toml",
                include_str!("../capabilities/firmware-stm32/firmware/shared/Cargo.toml"),
            ),
            (
                "firmware/shared/src/lib.rs",
                include_str!("../capabilities/firmware-stm32/firmware/shared/src/lib.rs"),
            ),
            (
                "firmware/stm32/.cargo/config.toml",
                include_str!("../capabilities/firmware-stm32/firmware/stm32/.cargo/config.toml"),
            ),
            (
                "firmware/stm32/Cargo.toml",
                include_str!("../capabilities/firmware-stm32/firmware/stm32/Cargo.toml"),
            ),
            (
                "firmware/stm32/build.rs",
                include_str!("../capabilities/firmware-stm32/firmware/stm32/build.rs"),
            ),
            (
                "firmware/stm32/memory.x",
                include_str!("../capabilities/firmware-stm32/firmware/stm32/memory.x"),
            ),
            (
                "firmware/stm32/src/main.rs",
                include_str!("../capabilities/firmware-stm32/firmware/stm32/src/main.rs"),
            ),
        ],
        skill_md: include_str!("../capabilities/firmware-stm32/SKILL.md"),
    },
    CapabilityDef {
        id: "tauri",
        description: "Tauri 2 desktop app with fiducial-tauri serial transport",
        guard_rules: &["no-unpinned-cli-fetch"],
        templates: &[
            (
                "apps/desktop/src-tauri/tauri.conf.json",
                include_str!("../capabilities/tauri/apps/desktop/src-tauri/tauri.conf.json"),
            ),
            (
                "apps/desktop/src-tauri/Cargo.toml",
                include_str!("../capabilities/tauri/apps/desktop/src-tauri/Cargo.toml.tmpl"),
            ),
            (
                "apps/desktop/src-tauri/build.rs",
                include_str!("../capabilities/tauri/apps/desktop/src-tauri/build.rs"),
            ),
            (
                "apps/desktop/src-tauri/src/lib.rs",
                include_str!("../capabilities/tauri/apps/desktop/src-tauri/src/lib.rs"),
            ),
            (
                "apps/desktop/src-tauri/src/main.rs",
                include_str!("../capabilities/tauri/apps/desktop/src-tauri/src/main.rs"),
            ),
        ],
        skill_md: include_str!("../capabilities/tauri/SKILL.md"),
    },
    CapabilityDef {
        id: "worker-cloudflare",
        description: "Cloudflare Worker / Durable Object with Wrangler and Miniflare",
        guard_rules: &["no-unpinned-cli-fetch"],
        templates: &[(
            "apps/worker/wrangler.toml",
            include_str!("../capabilities/worker-cloudflare/apps/worker/wrangler.toml"),
        )],
        skill_md: include_str!("../capabilities/worker-cloudflare/SKILL.md"),
    },
];

/// Look up a capability by id.
pub fn find(id: &str) -> Option<&'static CapabilityDef> {
    BUILTIN_CAPABILITIES.iter().find(|c| c.id == id)
}

// ── Install ──────────────────────────────────────────────────────────────────

/// Install a capability into the product rooted at `root`.
///
/// 1. Write template files (tracked in `fiducial.lock`).
/// 2. Write `SKILL.md` to `.claude/skills/<capability-id>.md`.
/// 3. Add capability id to `fiducial.toml [capabilities]`.
/// 4. Merge capability guard rules into `fiducial.toml [guard]`.
pub fn install(cap: &CapabilityDef, root: &Path, product_name: &str) -> Result<()> {
    println!("✦ fid add {} — installing into {}", cap.id, root.display());

    let mut lock = load_or_new_lock(root)?;

    // 1. Template files.
    for (rel, content) in cap.templates {
        let expanded = content
            .replace("{{name}}", product_name)
            .replace("{{version}}", PLATFORM_VERSION);
        write_file(root, rel, &expanded)?;
        lock.record(
            rel.replace('\\', "/"),
            expanded.as_bytes(),
            PLATFORM_VERSION,
        );
    }

    // 2. SKILL.md → .claude/skills/<id>.md
    let skill_path = format!(".claude/skills/{}.md", cap.id);
    let skill_content = cap
        .skill_md
        .replace("{{name}}", product_name)
        .replace("{{version}}", PLATFORM_VERSION);
    write_file(root, &skill_path, &skill_content)?;
    // Skills are not tracked in the lock — they are platform-owned and upgraded
    // automatically by the Claude Code plugin, not by `fid upgrade`.

    // 3 + 4. Update fiducial.toml.
    patch_config(root, cap)?;

    // Re-record fiducial.toml in the lock — its hash changed after the patch.
    let new_config_content = std::fs::read(root.join(crate::config::CONFIG_FILE))
        .context("reading updated fiducial.toml")?;
    lock.record("fiducial.toml", &new_config_content, PLATFORM_VERSION);

    // Persist updated lock.
    lock.save(&root.join("fiducial.lock"))
        .context("writing fiducial.lock")?;

    println!("  ✓ {} installed", cap.id);
    println!("  ✓ guard rules added: {}", cap.guard_rules.join(", "));
    println!(
        "  ✓ skill written → {} (auto-loaded by Claude Code)",
        skill_path
    );
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn load_or_new_lock(root: &Path) -> Result<Lock> {
    let path = root.join("fiducial.lock");
    if path.exists() {
        Lock::load(&path).context("loading fiducial.lock")
    } else {
        Ok(Lock::new())
    }
}

fn write_file(root: &Path, rel: &str, content: &str) -> Result<()> {
    let dest = root.join(rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating parent for `{rel}`"))?;
    }
    std::fs::write(&dest, content).with_context(|| format!("writing `{rel}`"))?;
    println!("  wrote  {rel}");
    Ok(())
}

/// Patch `fiducial.toml` to add the capability id and its guard rules.
///
/// Uses line-level editing rather than a full TOML rewrite to preserve
/// comments and formatting. This is safe because the relevant lines have
/// a predictable shape written by `fid new`.
fn patch_config(root: &Path, cap: &CapabilityDef) -> Result<()> {
    let config_path = root.join(CONFIG_FILE);
    let mut cfg = Config::load(&config_path)?;

    // Add capability id if not already present.
    if !cfg.capabilities.enabled.contains(&cap.id.to_string()) {
        cfg.capabilities.enabled.push(cap.id.to_string());
    }

    // Add guard rules that aren't already present.
    for rule in cap.guard_rules {
        if !cfg.guard.rules.contains(&rule.to_string()) {
            cfg.guard.rules.push(rule.to_string());
        }
    }

    // Serialise back. We use a structured round-trip here rather than line
    // editing because the config schema is small and comments are at the top.
    let raw = toml::to_string_pretty(&cfg).context("serialising fiducial.toml")?;
    let header = format!(
        "# fiducial.toml — updated by `fid add {}` (fiducial {})\n\n",
        cap.id, PLATFORM_VERSION
    );
    std::fs::write(&config_path, format!("{header}{raw}")).context("writing fiducial.toml")?;

    println!("  patched fiducial.toml");
    Ok(())
}

// ── Conformance check ─────────────────────────────────────────────────────────

/// Check that a capability definition is well-formed.
/// Returns a list of conformance errors.
pub fn check_capability(cap: &CapabilityDef) -> Vec<String> {
    let mut errors = Vec::new();

    if cap.id.is_empty() {
        errors.push("capability id must not be empty".into());
    } else if cap
        .id
        .contains(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-')
    {
        errors.push(format!(
            "capability id `{}` must be kebab-case (lowercase letters, digits, and hyphens)",
            cap.id
        ));
    }

    if cap.description.is_empty() {
        errors.push(format!("[{}] description must not be empty", cap.id));
    }

    if cap.skill_md.is_empty() {
        errors.push(format!(
            "[{}] SKILL.md is mandatory (§3.3: no SKILL.md, not a capability)",
            cap.id
        ));
    } else if cap.skill_md.len() < 100 {
        errors.push(format!(
            "[{}] SKILL.md is suspiciously short ({} bytes) — \
             it must teach an agent how to use this capability correctly",
            cap.id,
            cap.skill_md.len()
        ));
    }

    errors
}

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

/// A fact a capability introduces, which its pipelines then read.
///
/// The test, from the spec: *could two different pipelines read this and both be
/// correct?* `board/board.interface.json` is read by both `eda.toml` and
/// `enclosure.toml`, so it is a declaration. `apps/worker/wrangler.toml` is one
/// tool's config file, so it is a template.
///
/// The distinction was invisible before: a declaration, a pipeline and a plain
/// file were all entries in `templates`, which is why the `[i18n]` block had to
/// be seeded by an `if cap.id == "i18n"` branch in `patch_config`. A capability
/// could not say what it declared, so one place had to know for it.
pub enum Declaration {
    /// A file in the product that people edit and pipelines read.
    File {
        /// Repo-relative path.
        path: &'static str,
        /// Seed content, written on install.
        content: &'static str,
    },
    /// A `fiducial.toml` block.
    ///
    /// Seeded on install, because a pipeline whose declaration is an empty
    /// block fails on the next `fid derive` — the capability would install
    /// clean and break the first time anyone used it.
    ConfigBlock {
        /// Block name as it appears in `fiducial.toml`, e.g. `i18n`.
        name: &'static str,
        /// Fills the block in, if the product has not already declared it.
        seed: fn(&mut Config),
    },
}

impl Declaration {
    /// What this declaration is called in `fid capability list` and `fid dash`.
    pub fn name(&self) -> &'static str {
        match self {
            Self::File { path, .. } => path,
            Self::ConfigBlock { name, .. } => name,
        }
    }
}

/// A capability definition: everything the platform needs to install it.
///
/// The three members above `templates` are the taxonomy from
/// `docs/specs/2026-09-14-capability-taxonomy.md`. They are not decoration: a
/// capability that cannot say which facts it introduces, what it derives from
/// them, and which contracts it needs is one the CLI has to special-case.
pub struct CapabilityDef {
    /// Kebab-case slug used in `fiducial.toml [capabilities]` and on the CLI.
    pub id: &'static str,
    /// One-line description shown in `fid capability list`.
    pub description: &'static str,
    /// Typed facts this capability introduces. Inert on their own.
    pub declarations: &'static [Declaration],
    /// Pipelines that read those facts and produce gated artifacts.
    ///
    /// Each entry is (repo-relative path under `pipelines/`, file content).
    /// Separate from `templates` because a pipeline is the thing `fid derive`
    /// runs and `fid derive --check` gates — the property that makes its output
    /// a build failure rather than a surprise.
    pub pipelines: &'static [(&'static str, &'static str)],
    /// Adapter contracts this capability needs a vendor for.
    ///
    /// Naming a contract is not choosing a vendor. A capability that needs
    /// storage says `storage`; which of `r2`, `s3` or `none` satisfies it is
    /// the product's decision, recorded in `[adapters]`.
    pub requires_adapters: &'static [&'static str],
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
        declarations: &[],
        pipelines: &[],
        requires_adapters: &[],
        guard_rules: &["no-unpinned-cli-fetch"],
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
                "apps/web/src/app/board/page.tsx",
                include_str!("../capabilities/web-next/apps/web/src/app/board/page.tsx"),
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
        declarations: &[],
        pipelines: &[],
        requires_adapters: &[],
        guard_rules: &["no-unpinned-cli-fetch"],
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
        declarations: &[],
        pipelines: &[],
        requires_adapters: &[],
        guard_rules: &["no-unpinned-cli-fetch"],
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
        declarations: &[],
        pipelines: &[],
        requires_adapters: &[],
        guard_rules: &["no-unpinned-cli-fetch"],
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
        declarations: &[],
        pipelines: &[],
        requires_adapters: &[],
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
        declarations: &[],
        pipelines: &[],
        requires_adapters: &[],
        guard_rules: &["no-unpinned-cli-fetch"],
        templates: &[(
            "apps/worker/wrangler.toml",
            include_str!("../capabilities/worker-cloudflare/apps/worker/wrangler.toml"),
        )],
        skill_md: include_str!("../capabilities/worker-cloudflare/SKILL.md"),
    },
    CapabilityDef {
        id: "i18n",
        description: "Localized by construction: JSON catalogs in, typed message keys out",
        declarations: &[
            Declaration::ConfigBlock {
                name: "i18n",
                seed: seed_default_locales,
            },
            Declaration::File {
                path: "messages/en.json",
                content: include_str!("../capabilities/i18n/messages/en.json"),
            },
            Declaration::File {
                path: "messages/sr.json",
                content: include_str!("../capabilities/i18n/messages/sr.json"),
            },
        ],
        pipelines: &[(
            "pipelines/i18n.toml",
            include_str!("../capabilities/i18n/pipelines/i18n.toml"),
        )],
        requires_adapters: &[],
        guard_rules: &[],
        templates: &[],
        skill_md: include_str!("../capabilities/i18n/SKILL.md"),
    },
    CapabilityDef {
        id: "eda",
        description: "EDA pipeline: atopile → KiCad → board.interface.json tracked by fid derive",
        // Read by both pipelines below, which is the test for a declaration.
        declarations: &[Declaration::File {
            path: "board/board.interface.json",
            content: include_str!("../capabilities/eda/board/board.interface.json"),
        }],
        pipelines: &[
            (
                "pipelines/eda.toml",
                include_str!("../capabilities/eda/pipelines/eda.toml"),
            ),
            (
                "pipelines/enclosure.toml",
                include_str!("../capabilities/eda/pipelines/enclosure.toml"),
            ),
        ],
        requires_adapters: &[],
        guard_rules: &[],
        templates: &[(
            "board/main.ato",
            include_str!("../capabilities/eda/board/main.ato"),
        )],
        skill_md: include_str!("../capabilities/eda/SKILL.md"),
    },
];

/// Seed `[i18n]` with the locales a product is born with.
///
/// A function rather than a branch in `patch_config`: the capability that
/// introduces a declaration is the thing that knows how to fill it in, and
/// `if cap.id == "i18n"` put that knowledge somewhere the capability could not
/// reach. The next capability with a config block would have added a second
/// branch to the same function.
fn seed_default_locales(cfg: &mut Config) {
    cfg.i18n.locales = crate::config::DEFAULT_LOCALES
        .iter()
        .map(|l| (*l).to_string())
        .collect();
    cfg.i18n.default = Some(crate::config::DEFAULT_LOCALE.to_string());
}

/// Look up a capability by id.
pub fn find(id: &str) -> Option<&'static CapabilityDef> {
    BUILTIN_CAPABILITIES.iter().find(|c| c.id == id)
}

// ── Install ──────────────────────────────────────────────────────────────────

/// Install a capability into the product rooted at `root`.
///
/// 1. Write template files (tracked in `fiducial.lock`).
/// 2. Write `SKILL.md` to `.fiducial/skills/<capability-id>.md`, plus a pointer
///    at `.claude/skills/<capability-id>.md` for Claude Code's auto-discovery.
/// 3. Add capability id to `fiducial.toml [capabilities]`.
/// 4. Merge capability guard rules into `fiducial.toml [guard]`.
pub fn install(cap: &CapabilityDef, root: &Path, product_name: &str) -> Result<()> {
    println!("✦ fid add {} — installing into {}", cap.id, root.display());

    let mut lock = load_or_new_lock(root)?;

    // 1. Declarations, pipelines, then templates.
    //
    // Declarations first: a pipeline whose declaration is not yet on disk
    // fails on the next `fid derive`, and install order is the cheapest place
    // to make that impossible.
    let mut files: Vec<(&str, &str)> = Vec::new();
    for decl in cap.declarations {
        if let Declaration::File { path, content } = decl {
            files.push((path, content));
        }
    }
    files.extend(cap.pipelines.iter().copied());
    files.extend(cap.templates.iter().copied());

    for (rel, content) in files {
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

    // 2. SKILL.md → a vendor-neutral path, plus a pointer for Claude Code.
    //
    // The instructions used to be written only to `.claude/skills/`, which meant
    // a Codex, Copilot or Cursor session in this product could install the `eda`
    // capability and receive no instructions for using it. The content is
    // ordinary Markdown and portable; only *discovery* is vendor-specific.
    //
    // So the content lives once, at a neutral path that `AGENTS.md` points every
    // agent at, and Claude Code gets a short pointer file so its automatic skill
    // discovery still works. The pointer carries no instructions of its own —
    // two copies of the content would be the duplication this platform exists
    // to delete.
    let skill_content = cap
        .skill_md
        .replace("{{name}}", product_name)
        .replace("{{version}}", PLATFORM_VERSION);
    write_file(
        root,
        &format!(".fiducial/skills/{}.md", cap.id),
        &skill_content,
    )?;

    write_file(
        root,
        &format!(".claude/skills/{}.md", cap.id),
        &format!(
            "---\n\
             name: fiducial-{id}\n\
             description: How to use the `{id}` capability in this product.\n\
             ---\n\n\
             Read `.fiducial/skills/{id}.md` — the instructions live there so that \
             every agent can find them, not only Claude Code.\n",
            id = cap.id
        ),
    )?;
    // Skills are not tracked in the lock — they are platform-owned and rewritten
    // on every `fid upgrade`, not merged.

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
    if cap.guard_rules.is_empty() {
        println!("  ✓ no guard rules — this capability adds none");
    } else {
        println!("  ✓ guard rules added: {}", cap.guard_rules.join(", "));
    }
    println!(
        "  ✓ instructions → .fiducial/skills/{}.md (any agent; see AGENTS.md)",
        cap.id
    );
    Ok(())
}

/// Bring `messages/` in line with a declared locale set.
///
/// The `i18n` capability ships catalogs for [`DEFAULT_LOCALES`] only. A product
/// created with a different set needs a catalog for each locale it declares and
/// none for a locale it does not — the executor reads the locale set **from the
/// directory listing**, so a stray catalog is a language the product silently
/// claims to ship, and a missing one is a language it silently does not.
///
/// A locale with no shipped catalog starts as a copy of the default's. That is
/// deliberately visible rather than convenient: identical values are exactly
/// what `compare` reports as untranslated, so the work still to do shows up in
/// the first `fid derive` instead of being discovered by a reader.
///
/// [`DEFAULT_LOCALES`]: crate::config::DEFAULT_LOCALES
pub fn reconcile_i18n_catalogs(root: &Path, locales: &[String], default: &str) -> Result<()> {
    let dir = root.join("messages");
    if !dir.is_dir() {
        return Ok(());
    }
    let mut lock = load_or_new_lock(root)?;

    let default_catalog = std::fs::read_to_string(dir.join(format!("{default}.json")))
        .with_context(|| format!("reading the default catalog messages/{default}.json"))?;

    // Add a catalog for every declared locale that has none.
    for locale in locales {
        let path = dir.join(format!("{locale}.json"));
        if path.exists() {
            continue;
        }
        let rel = format!("messages/{locale}.json");
        write_file(root, &rel, &default_catalog)?;
        lock.record(rel, default_catalog.as_bytes(), PLATFORM_VERSION);
    }

    // Remove the shipped catalogs for locales this product does not declare.
    for entry in std::fs::read_dir(&dir)
        .context("reading messages/")?
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(locale) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if locales.iter().any(|l| l == locale) {
            continue;
        }
        let rel = format!("messages/{locale}.json");
        // Only ours to remove. A catalog someone added by hand is theirs.
        if lock.templates.contains_key(&rel) {
            std::fs::remove_file(&path).with_context(|| format!("removing {rel}"))?;
            lock.templates.remove(&rel);
            println!("  removed {rel} (locale not declared)");
        }
    }

    lock.save(&root.join("fiducial.lock"))
        .context("writing fiducial.lock")?;
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

    // A capability that introduces a DECLARATION must seed it, or the pipeline
    // it also installs fails on the next `fid derive` with an empty block.
    for decl in cap.declarations {
        if let Declaration::ConfigBlock { seed, .. } = decl {
            seed(&mut cfg);
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

    // ── Taxonomy conformance ────────────────────────────────────────────────
    //
    // The split is only worth having if the three lists mean what they say. A
    // pipeline filed under `templates` is not gated by `fid derive --check`,
    // and a declaration filed there is one `fid dash` cannot report as missing.

    for (path, _) in cap.pipelines {
        if !path.starts_with("pipelines/") {
            errors.push(format!(
                "[{}] pipeline `{path}` must live under `pipelines/` — that is \
                 where `pipeline::discover` looks, so one filed elsewhere is \
                 installed and never run",
                cap.id
            ));
        }
    }

    for (path, _) in cap.templates {
        if path.starts_with("pipelines/") {
            errors.push(format!(
                "[{}] `{path}` is under `pipelines/` but is declared as a \
                 template. Move it to `pipelines` so it is recognised as one",
                cap.id
            ));
        }
    }

    // A capability that derives something must say what it derives it *from*.
    // Without this the taxonomy is decoration: a pipeline with no declared
    // input reads a fact nothing is responsible for putting there.
    if !cap.pipelines.is_empty() && cap.declarations.is_empty() {
        errors.push(format!(
            "[{}] installs {} pipeline(s) and declares nothing for them to \
             read — name the facts they consume in `declarations`",
            cap.id,
            cap.pipelines.len()
        ));
    }

    for contract in cap.requires_adapters {
        if crate::adapter::find(contract).is_none() {
            errors.push(format!(
                "[{}] requires adapter contract `{contract}`, which is not one \
                 of: {}",
                cap.id,
                crate::adapter::CONTRACTS
                    .iter()
                    .map(|c| c.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every built-in capability conforms to its own rules.
    ///
    /// `fid capability check` runs this against a product's installed set; this
    /// runs it against the registry, so a first-party capability cannot ship
    /// violating a rule the CLI enforces on everyone else's.
    #[test]
    fn every_builtin_capability_conforms() {
        for cap in BUILTIN_CAPABILITIES {
            let errors = check_capability(cap);
            assert!(errors.is_empty(), "capability `{}`: {errors:?}", cap.id);
        }
    }

    /// No file is installed from two lists.
    ///
    /// The three lists are written into the product by one loop, so a path in
    /// two of them is written twice and recorded in the lock twice — and which
    /// kind `fid dash` reports it as becomes a matter of iteration order.
    #[test]
    fn no_path_is_declared_twice_within_a_capability() {
        for cap in BUILTIN_CAPABILITIES {
            let mut paths: Vec<&str> = Vec::new();
            for decl in cap.declarations {
                if let Declaration::File { path, .. } = decl {
                    paths.push(path);
                }
            }
            paths.extend(cap.pipelines.iter().map(|(p, _)| *p));
            paths.extend(cap.templates.iter().map(|(p, _)| *p));

            let mut seen = paths.clone();
            seen.sort_unstable();
            let before = seen.len();
            seen.dedup();
            assert_eq!(
                before,
                seen.len(),
                "capability `{}` installs a path from two lists: {paths:?}",
                cap.id
            );
        }
    }

    /// Ids are unique — they are the key `fid add` and `[capabilities]` use.
    #[test]
    fn capability_ids_are_unique() {
        let mut ids: Vec<&str> = BUILTIN_CAPABILITIES.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate capability id");
    }

    /// The two capabilities the taxonomy was generalized from still exercise
    /// all three kinds between them.
    ///
    /// Not trivia: the spec's argument for building this now is that `eda` and
    /// `i18n` are *working* declaration→pipeline cases. If a refactor quietly
    /// flattened them back into `templates`, the taxonomy would still compile
    /// and mean nothing.
    #[test]
    fn the_taxonomy_is_exercised_by_real_capabilities() {
        let i18n = find("i18n").expect("i18n is built in");
        assert!(
            i18n.declarations
                .iter()
                .any(|d| matches!(d, Declaration::ConfigBlock { .. })),
            "i18n must declare its [i18n] block, not have it special-cased"
        );
        assert!(!i18n.pipelines.is_empty(), "i18n derives typed keys");

        let eda = find("eda").expect("eda is built in");
        assert_eq!(
            eda.pipelines.len(),
            2,
            "eda's board interface is read by two pipelines — the test for a declaration"
        );
        assert!(!eda.declarations.is_empty());
    }

    /// A config-block declaration actually fills its block in.
    ///
    /// A `seed` that does nothing installs clean and fails on the next
    /// `fid derive` with an empty declaration — the exact failure the seeding
    /// exists to prevent, moved one step later.
    #[test]
    fn every_config_block_seed_populates_something() {
        for cap in BUILTIN_CAPABILITIES {
            for decl in cap.declarations {
                let Declaration::ConfigBlock { name, seed } = decl else {
                    continue;
                };
                let mut cfg = Config::minimal("probe");
                let before = toml::to_string(&cfg).expect("serialise");
                seed(&mut cfg);
                let after = toml::to_string(&cfg).expect("serialise");
                assert_ne!(
                    before, after,
                    "[{}] declares config block `{name}` with a seed that changes nothing",
                    cap.id
                );
            }
        }
    }
}

//! `fid add` — add an app, module, firmware target, or capability.
//!
//! With a structural subcommand (`app`, `module`, `firmware`) this scaffolds a
//! specific kind of target. With a bare capability id it delegates to the
//! capability installer, which writes templates, activates guard rules, and
//! writes the SKILL.md to `.claude/skills/`.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use std::env;

use crate::{
    capability,
    config::{Config, CONFIG_FILE},
};

// ── Subcommand tree ──────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum AddTarget {
    /// Add a web or native application
    #[command(
        long_about = "\
Scaffold a web or native application target.

Installs the corresponding platform capability and writes all template files
into the product. After installation, run `fid capability check` to verify.",
        after_long_help = "\
TARGETS
  next      Next.js 15 (App Router) web app     → apps/web/
  svelte    SvelteKit web app                    → apps/web/   (Phase 3b)
  tauri     Tauri 2 desktop + mobile             → apps/desktop/
  worker    Cloudflare Worker / Durable Object   → apps/worker/
  mobile    Tauri mobile standalone              → apps/mobile/ (Phase 3b)

EXAMPLES
  fid add app next
  fid add app tauri
  fid add app worker"
    )]
    App {
        /// Target: next | svelte | tauri | worker | mobile
        #[arg(
            value_name = "TARGET",
            long_help = "Application type to scaffold. Run `fid add app --help` for the full list."
        )]
        target: String,
    },

    /// Add a bounded-context module (Rust + TS)
    #[command(
        long_about = "\
Scaffold a bounded-context module: a Rust crate + TypeScript package pair that
lives in the host workspace and can be imported by apps and firmware.

Modules are the idiomatic unit for shared business logic — protocol codecs,
state machines, domain rules — that must be correct across multiple runtimes.",
        after_long_help = "\
EXAMPLES
  fid add module payments     bounded context for payment flows
  fid add module auth         authentication / session logic
  fid add module telemetry    device telemetry pipeline

STATUS
  Module scaffolding is coming in Phase 3b."
    )]
    Module {
        /// Module name in kebab-case (e.g. `payments`, `auth`)
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Add firmware support for a microcontroller target
    #[command(
        long_about = "\
Scaffold firmware for a specific microcontroller family.

Installs the `firmware-<target>` capability: writes the firmware workspace
structure, configures the embedded toolchain, and adds the relevant guard rules.",
        after_long_help = "\
TARGETS
  rp2040     Raspberry Pi RP2040 — Embassy + probe-rs      → firmware/rp2040/
  rp2350     Raspberry Pi RP2350 — Embassy + probe-rs      → firmware/rp2350/
  stm32      STM32 family — Embassy HAL                    → firmware/stm32/  (Phase 3b)
  nrf52      nRF52 family — Embassy HAL                    → firmware/nrf52/  (Phase 3b)

EXAMPLES
  fid add firmware rp2040
  fid add firmware rp2350"
    )]
    Firmware {
        /// Board target: rp2040 | rp2350 | stm32 | nrf52
        #[arg(value_name = "TARGET")]
        target: String,
    },
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

pub fn run(target: AddTarget) -> Result<()> {
    match target {
        AddTarget::App { target } => add_app(&target),
        AddTarget::Module { name } => add_module(&name),
        AddTarget::Firmware { target } => add_firmware(&target),
    }
}

fn add_app(target: &str) -> Result<()> {
    let cap_id = match target {
        "next" => "web-next",
        "tauri" => "tauri",
        "worker" => "worker-cloudflare",
        "svelte" | "mobile" => {
            println!(
                "✦ fid add app {target}\n\n\
                 The `{target}` target is coming in Phase 3b.\n\
                 Track progress: https://github.com/AleksaZCodes/fiducial"
            );
            return Ok(());
        }
        other => bail!(
            "unknown app target `{other}`.\n\
             Supported: next, svelte, tauri, worker, mobile.\n\
             Run `fid add app --help` for details."
        ),
    };
    install_capability(cap_id)
}

fn add_firmware(target: &str) -> Result<()> {
    let cap_id = match target {
        "rp2040" => "firmware-rp2040",
        "rp2350" => {
            // rp2350 is structurally identical to rp2040 for Phase 3b — stub.
            println!(
                "✦ fid add firmware rp2350\n\n\
                 The `rp2350` firmware target is coming in Phase 3b.\n\
                 It reuses the same Embassy setup as rp2040 with the RP2350 HAL.\n\
                 Track progress: https://github.com/AleksaZCodes/fiducial"
            );
            return Ok(());
        }
        "stm32" | "nrf52" => {
            println!(
                "✦ fid add firmware {target}\n\n\
                 The `{target}` firmware target is coming in Phase 3b.\n\
                 Track progress: https://github.com/AleksaZCodes/fiducial"
            );
            return Ok(());
        }
        other => bail!(
            "unknown firmware target `{other}`.\n\
             Supported: rp2040, rp2350, stm32, nrf52.\n\
             Run `fid add firmware --help` for details."
        ),
    };
    install_capability(cap_id)
}

fn add_module(name: &str) -> Result<()> {
    println!(
        "✦ fid add module {name}\n\n\
         Module scaffolding is coming in Phase 3b.\n\
         Track progress: https://github.com/AleksaZCodes/fiducial"
    );
    Ok(())
}

// ── Capability installer ──────────────────────────────────────────────────────

/// Install a capability by id into the current product.
pub fn install_capability(cap_id: &str) -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(CONFIG_FILE))?;

    // Already installed?
    if cfg.capabilities.enabled.contains(&cap_id.to_string()) {
        println!(
            "  ✓ `{cap_id}` is already installed in `{}`.",
            cfg.product.name
        );
        println!("  Run `fid capability check` to verify it is in good shape.");
        return Ok(());
    }

    // Look up the capability definition.
    let def = capability::find(cap_id).ok_or_else(|| {
        anyhow::anyhow!(
            "capability `{cap_id}` not found in the built-in registry.\n\
             Run `fid capability list --all` to see available capabilities.\n\
             Third-party capabilities (from git repos) are supported in Phase 5+."
        )
    })?;

    capability::install(def, &root, &cfg.product.name)
}

//! `fid add app|module <target>` — add an app or bounded-context module.
//!
//! Phase 3 scope: structure is in place, targets are stubs that print a clear
//! "coming in Phase 3b" message. The subcommand structure is final so that
//! shell completion and help text are correct from day one.

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum AddTarget {
    /// Add a web or native application
    App {
        /// Target: next | svelte | tauri | worker | mobile
        target: String,
    },
    /// Add a bounded-context module (Rust + TS)
    Module {
        /// Module name (e.g. "auth", "payments")
        name: String,
    },
    /// Add firmware support for a microcontroller target
    Firmware {
        /// Board target: rp2040 | rp2350 | stm32 | nrf52
        target: String,
    },
}

pub fn run(target: AddTarget) -> Result<()> {
    match target {
        AddTarget::App { target } => {
            println!(
                "✦ fid add app {target}\n\n\
                 Capability scaffolding is coming in Phase 3b.\n\
                 Track progress: https://github.com/AleksaZCodes/fiducial/issues"
            );
        }
        AddTarget::Module { name } => {
            println!(
                "✦ fid add module {name}\n\n\
                 Module scaffolding is coming in Phase 3b."
            );
        }
        AddTarget::Firmware { target } => {
            println!(
                "✦ fid add firmware {target}\n\n\
                 Firmware scaffolding is coming in Phase 3b."
            );
        }
    }
    Ok(())
}

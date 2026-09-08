//! **`fiducial-firmware-shared`** — target-agnostic firmware helpers.
//!
//! Everything in this crate compiles for every target in the CI matrix:
//! `thumbv6m-none-eabi`, `thumbv7em-none-eabihf`, and (for `cargo check`)
//! `x86_64`. It does not depend on any HAL; HAL dependencies go in the
//! target-specific crates (`firmware/rp2040`, `firmware/stm32`, …).
//!
//! # Re-exports
//!
//! Re-exports selected items from `fiducial-core` so firmware apps only
//! need one `firmware-shared` dependency for both the core types and the
//! firmware-specific helpers.

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

// Re-export core types so firmware apps have a single import path.
pub use fiducial_core::{version, DeviceId, VERSION};

/// LoRa physical-layer helpers (EU868 defaults).
///
/// Enable with `features = ["lora"]` in your firmware's Cargo.toml.
#[cfg(feature = "lora")]
pub mod lora {
    pub use lora_modulation::{Bandwidth, CodingRate, SpreadingFactor};

    /// EU868 uplink channel frequencies (Hz) — channels 0–2 (mandatory).
    pub const EU868_CHANNELS: &[u32] = &[868_100_000, 868_300_000, 868_500_000];

    /// EU868 DR0 — SF12 / BW125 — maximum range, minimum data rate.
    pub const EU868_DR0: (SpreadingFactor, Bandwidth) =
        (SpreadingFactor::_12, Bandwidth::_125KHz);

    /// EU868 DR5 — SF7 / BW125 — minimum range, maximum data rate.
    pub const EU868_DR5: (SpreadingFactor, Bandwidth) =
        (SpreadingFactor::_7, Bandwidth::_125KHz);
}

/// Blink timing constants — the baseline all LED indicators follow.
///
/// Using named constants instead of magic numbers means the pattern is
/// searchable and can be changed in one place.
pub mod blink {
    /// Single short blink — "I am alive" heartbeat. 50 ms on.
    pub const HEARTBEAT_ON_MS: u64 = 50;
    /// Gap after a heartbeat blink. 950 ms off → 1 Hz total.
    pub const HEARTBEAT_OFF_MS: u64 = 950;

    /// Fast double-blink on error: 100 ms on / 100 ms off / 100 ms on / 700 ms off.
    pub const ERROR_ON_MS: u64 = 100;
    /// Gap between the two error pulses.
    pub const ERROR_GAP_MS: u64 = 100;
    /// Silence after a double-blink error pattern.
    pub const ERROR_SILENCE_MS: u64 = 700;
}

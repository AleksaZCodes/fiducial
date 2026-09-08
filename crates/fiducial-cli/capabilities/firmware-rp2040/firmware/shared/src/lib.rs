//! Target-agnostic firmware helpers for {{name}}.
//!
//! Compiles for every embedded target — no HAL deps here.
//! Enable the `lora` feature for LoRa PHY channel/datarate constants.
#![no_std]

pub use fiducial_core::{version, DeviceId, VERSION};
pub use fiducial_protocol::{Frame, FrameDecoder, FrameEncoder};

/// Blink timing constants — the baseline all LED indicators follow.
pub mod blink {
    /// Heartbeat — 50 ms on.
    pub const HEARTBEAT_ON_MS: u64 = 50;
    /// Heartbeat — 950 ms off → 1 Hz total.
    pub const HEARTBEAT_OFF_MS: u64 = 950;
    /// Error double-blink: 100 ms on.
    pub const ERROR_ON_MS: u64 = 100;
    /// Gap between error pulses.
    pub const ERROR_GAP_MS: u64 = 100;
    /// Silence after error pattern.
    pub const ERROR_SILENCE_MS: u64 = 700;
}

/// LoRa PHY helpers (EU868 defaults).
///
/// Enable with `features = ["lora"]` in your firmware Cargo.toml.
#[cfg(feature = "lora")]
pub mod lora {
    pub use lora_modulation::{Bandwidth, CodingRate, SpreadingFactor};

    /// EU868 uplink channels (Hz) — channels 0–2 (mandatory).
    pub const EU868_CHANNELS: &[u32] = &[868_100_000, 868_300_000, 868_500_000];

    /// EU868 DR0 — SF12 / BW125 — maximum range, minimum data rate.
    pub const EU868_DR0: (SpreadingFactor, Bandwidth) =
        (SpreadingFactor::_12, Bandwidth::_125KHz);

    /// EU868 DR5 — SF7 / BW125 — minimum range, maximum data rate.
    pub const EU868_DR5: (SpreadingFactor, Bandwidth) =
        (SpreadingFactor::_7, Bandwidth::_125KHz);
}

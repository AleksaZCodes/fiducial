//! **`fiducial-core`** — the `no_std` spine shared across every runtime.
//!
//! This crate is the reference point everything else aligns against — firmware,
//! browser (WASM), desktop (Tauri), edge (Workers), and host tooling all depend
//! on the same compiled artefact.
//!
//! # `no_std` discipline
//!
//! The crate is unconditionally `#![no_std]`. Embedded targets use it with no
//! features; WASM and host enable `--features std` (which also enables `alloc`).
//! CI compiles this crate for **four targets every commit**:
//!
//! | Target | Triple | Notes |
//! |---|---|---|
//! | Host | `x86_64-unknown-linux-gnu` | tests run here |
//! | WASM | `wasm32-unknown-unknown` | browser + edge |
//! | RP2040 | `thumbv6m-none-eabi` | Cortex-M0+ |
//! | STM32 | `thumbv7em-none-eabihf` | Cortex-M4F |
//!
//! If it stops compiling for any target, CI fails the day it breaks — not the
//! day it is needed.
//!
//! # Contents (Phase 2 — vertical slice)
//!
//! - [`version()`] — platform version string, injected by Cargo
//! - [`DeviceId`] — 8-byte opaque device identifier

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

// Make std/alloc available in test runs (the test harness itself uses std).
#[cfg(test)]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

// ── Version ─────────────────────────────────────────────────────────────────

/// Platform version string, injected by Cargo at build time.
///
/// Returns the same string on every target — firmware, WASM, desktop, edge.
/// The canonical source is `Cargo.toml`; nothing else declares the version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns [`VERSION`].
///
/// Exists as a function (rather than only a constant) so wasm-bindgen can
/// export it to JavaScript without a manual JS wrapper.
#[inline]
pub fn version() -> &'static str {
    VERSION
}

// ── DeviceId ─────────────────────────────────────────────────────────────────

/// An 8-byte opaque device identifier.
///
/// On embedded targets this is read from a chip-specific UID register or OTP
/// flash region during boot and stored in a static. On host and WASM it is
/// generated (or supplied externally).
///
/// The all-zero value is the **uninitialized sentinel** and is treated as
/// absent by every layer that consumes it. [`DeviceId::is_zero`] tests for it.
///
/// # Wire format
///
/// Transmitted as 8 raw bytes, big-endian, with no framing. Small enough to
/// fit in a BLE advertisement payload alongside a name and RSSI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeviceId([u8; 8]);

impl DeviceId {
    /// The uninitialized sentinel — all bytes zero.
    pub const ZERO: DeviceId = DeviceId([0u8; 8]);

    /// Construct from a raw byte array.
    #[inline]
    pub const fn new(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// Return the raw bytes.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }

    /// `true` when all bytes are zero — the uninitialized sentinel.
    ///
    /// A device that has not stored its UID yet returns `true` here.
    /// Every layer that receives a `DeviceId` should gate on this before
    /// registering or associating the device.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        let b = &self.0;
        b[0] == 0
            && b[1] == 0
            && b[2] == 0
            && b[3] == 0
            && b[4] == 0
            && b[5] == 0
            && b[6] == 0
            && b[7] == 0
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────
//
// Tests run on the host target (x86_64) where std is available.
// The library code itself uses none of std — only the test harness does.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_nonempty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn version_constant_matches_fn() {
        assert_eq!(version(), VERSION);
    }

    #[test]
    fn device_id_zero_sentinel() {
        assert!(DeviceId::ZERO.is_zero());
        assert!(DeviceId::new([0u8; 8]).is_zero());
    }

    #[test]
    fn device_id_nonzero() {
        let id = DeviceId::new([1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(!id.is_zero());
        assert_eq!(id.as_bytes(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn device_id_roundtrip() {
        let bytes = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
        let id = DeviceId::new(bytes);
        assert_eq!(*id.as_bytes(), bytes);
    }

    #[test]
    fn device_id_copy() {
        let a = DeviceId::new([1; 8]);
        let b = a; // Copy
        assert_eq!(a, b);
    }
}

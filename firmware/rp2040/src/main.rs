//! **fiducial-rp2040** — Phase 2 vertical-slice blink demo.
//!
//! Proves the core claim: one function (`version()`) declared in `fiducial-core`
//! runs identically in a browser (WASM), in Tauri (native Rust), and on this
//! RP2040 (Cortex-M0+, no_std) — **from one crate, with no forking**.
//!
//! # What this does
//!
//! 1. Reads `fiducial_core::version()` — the same string the browser and
//!    desktop log — and emits it over RTT via `defmt`.
//! 2. Blinks the Pico's onboard LED (GPIO 25) using the timing constants from
//!    `fiducial_firmware_shared::blink`, so all future firmware inherits one
//!    canonical blink pattern.
//!
//! # Flashing
//!
//! ```sh
//! # From firmware/rp2040/ :
//! cargo run --release
//! # probe-rs flashes and attaches; defmt output appears in the terminal.
//! ```
//!
//! Connect a Raspberry Pi Debug Probe (or a second Pico as picoprobe) via SWD.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::{Duration, Timer};
use fiducial_firmware_shared::{blink, version, DeviceId};
use {defmt_rtt as _, panic_probe as _};

/// Hard-coded demo ID — replaced by a UID register read in a real product.
const DEMO_ID: DeviceId = DeviceId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    defmt::info!(
        "fiducial-core {} on RP2040 — device id {:?}",
        version(),
        DEMO_ID.as_bytes(),
    );

    // Onboard LED is GPIO 25 on the Raspberry Pi Pico.
    let mut led = Output::new(p.PIN_25, Level::Low);

    loop {
        // Heartbeat: 50 ms on, 950 ms off (1 Hz).
        led.set_high();
        Timer::after(Duration::from_millis(blink::HEARTBEAT_ON_MS)).await;
        led.set_low();
        Timer::after(Duration::from_millis(blink::HEARTBEAT_OFF_MS)).await;
    }
}

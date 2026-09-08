//! **fiducial-stm32** — STM32F401 Embassy blink demo.
//!
//! Proves `fiducial-core` compiles for Cortex-M4F (`thumbv7em-none-eabihf`).
//! Blinks PC13 — the onboard LED on the STM32F401 blackpill v2 board (active-low).
//!
//! # Flashing
//!
//! ```sh
//! # From firmware/stm32/ :
//! cargo run --release
//! # probe-rs flashes and attaches; defmt output appears in the terminal.
//! ```
//!
//! Connect a probe (ST-Link, J-Link, or a DAPLink adapter) via SWD.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use fiducial_firmware_shared::{blink, version, DeviceId};
use {defmt_rtt as _, panic_probe as _};

/// Hard-coded demo ID — replace with a UID register read in a real product.
const DEMO_ID: DeviceId = DeviceId::new([0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    defmt::info!(
        "fiducial-core {} on STM32F401 — device id {:?}",
        version(),
        DEMO_ID.as_bytes(),
    );

    // PC13 is the onboard LED on the STM32F401 blackpill v2 (active-low).
    let mut led = Output::new(p.PC13, Level::High, Speed::Low);

    loop {
        led.set_low();  // on
        Timer::after(Duration::from_millis(blink::HEARTBEAT_ON_MS)).await;
        led.set_high(); // off
        Timer::after(Duration::from_millis(blink::HEARTBEAT_OFF_MS)).await;
    }
}

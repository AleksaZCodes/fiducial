//! STM32F401 firmware for {{name}}.
//! Blinks PC13 (blackpill v2 onboard LED, active-low) and logs via defmt/RTT.
//!
//! Flash: `cargo run --release` (from firmware/stm32/)
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use firmware_shared::{blink, version, DeviceId};
use {defmt_rtt as _, panic_probe as _};

const DEMO_ID: DeviceId = DeviceId::new([0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    defmt::info!(
        "{{name}} {} on STM32F401 — id {:?}",
        version(),
        DEMO_ID.as_bytes(),
    );

    // PC13 is the onboard LED on the blackpill v2 (active-low: low = on).
    let mut led = Output::new(p.PC13, Level::High, Speed::Low);
    loop {
        led.set_low();  // on
        Timer::after(Duration::from_millis(blink::HEARTBEAT_ON_MS)).await;
        led.set_high(); // off
        Timer::after(Duration::from_millis(blink::HEARTBEAT_OFF_MS)).await;
    }
}

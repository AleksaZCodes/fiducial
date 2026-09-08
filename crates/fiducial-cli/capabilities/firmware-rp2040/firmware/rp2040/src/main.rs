//! RP2040 firmware for {{name}}.
//! Blinks GPIO 25 (Pico onboard LED) and logs the fiducial-core version via defmt/RTT.
//!
//! Flash: `cargo run --release` (from firmware/rp2040/)
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::{Duration, Timer};
use firmware_shared::{blink, version, DeviceId};
use {defmt_rtt as _, panic_probe as _};

const DEMO_ID: DeviceId = DeviceId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    defmt::info!(
        "{{name}} {} on RP2040 — id {:?}",
        version(),
        DEMO_ID.as_bytes(),
    );

    let mut led = Output::new(p.PIN_25, Level::Low);
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(blink::HEARTBEAT_ON_MS)).await;
        led.set_low();
        Timer::after(Duration::from_millis(blink::HEARTBEAT_OFF_MS)).await;
    }
}

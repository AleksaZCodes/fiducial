//! ESP32-C6 firmware for {{name}}.
//! Blinks GPIO 15 and logs the fiducial-core version over defmt.
//!
//! Flash: `cargo run --release` (from firmware/esp32c6/, needs `espflash`)
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use firmware_shared::{blink, version, DeviceId};
use {esp_backtrace as _, esp_println as _};

// The ESP-IDF second-stage bootloader reads this descriptor.
esp_bootloader_esp_idf::esp_app_desc!();

const DEMO_ID: DeviceId = DeviceId::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    let p = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_rtos::start(timg0.timer0, p.FROM_CPU_INTR0);

    defmt::info!(
        "{{name}} {} on ESP32-C6 — id {:?}",
        version(),
        DEMO_ID.as_bytes(),
    );

    let mut led = Output::new(p.GPIO15, Level::Low, OutputConfig::default());
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(blink::HEARTBEAT_ON_MS)).await;
        led.set_low();
        Timer::after(Duration::from_millis(blink::HEARTBEAT_OFF_MS)).await;
    }
}

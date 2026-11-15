//! ESP32-C6 firmware on ESP-IDF for {{name}}.
//! Blinks GPIO 15 and logs the fiducial-core version through the `log` facade.
//!
//! Flash: `cargo run --release` (from firmware/esp32c6-idf/, needs `espflash`)
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;

/// The heartbeat every Fiducial firmware blinks: 50 ms on, 950 ms off.
const HEARTBEAT_ON_MS: u32 = 50;
const HEARTBEAT_OFF_MS: u32 = 950;

fn main() -> Result<(), esp_idf_svc::sys::EspError> {
    // Required by esp-idf-sys: links the runtime patches in.
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!(
        "{{name}} {} on ESP32-C6 (ESP-IDF)",
        fiducial_core::version()
    );

    let peripherals = Peripherals::take()?;
    let mut led = PinDriver::output(peripherals.pins.gpio15)?;
    loop {
        led.set_high()?;
        FreeRtos::delay_ms(HEARTBEAT_ON_MS);
        led.set_low()?;
        FreeRtos::delay_ms(HEARTBEAT_OFF_MS);
    }
}

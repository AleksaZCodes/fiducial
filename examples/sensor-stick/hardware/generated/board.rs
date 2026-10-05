//! Derived by fid-hardware from hardware/product.toml — do not edit.
//!
//! The I/O pins of `mcu`, by the net each is on. Include it from the
//! firmware with `#[path = "…/hardware/generated/board.rs"] mod board;`
//! and take a pin with `board::sensor_sda!(p)`.
#![allow(unused_macros, unused_imports, dead_code)]

/// `LED_DIN` — GPIO16.
pub const LED_DIN: &str = "GPIO16";
macro_rules! led_din {
    ($p:expr) => {
        $p.PIN_16
    };
}
pub(crate) use led_din;

/// `SENSOR_SCL` — GPIO5.
pub const SENSOR_SCL: &str = "GPIO5";
macro_rules! sensor_scl {
    ($p:expr) => {
        $p.PIN_5
    };
}
pub(crate) use sensor_scl;

/// `SENSOR_SDA` — GPIO4.
pub const SENSOR_SDA: &str = "GPIO4";
macro_rules! sensor_sda {
    ($p:expr) => {
        $p.PIN_4
    };
}
pub(crate) use sensor_sda;

/// `sensor` (AHT20): its I²C address.
pub const SENSOR_I2C_ADDRESS: u8 = 0x38;

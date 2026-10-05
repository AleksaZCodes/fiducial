//! The sensor stick's firmware.
//!
//! Once a second: read the AHT20, send the reading over USB serial as one
//! fiducial-protocol frame, and colour the LED by the humidity.
//!
//! Every pin comes from `hardware/generated/board.rs`, which `fid derive`
//! writes from `hardware/product.toml`. Move a net to another pin there and
//! this file follows with no edit; rename a net and it stops compiling where
//! the old name is used. A pin moved to one the peripheral cannot use (SDA
//! off I2C0's pins) fails here too, as a type error.
#![no_std]
#![no_main]

#[path = "../../../hardware/generated/board.rs"]
mod board;

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{self, I2c};
use embassy_rp::dma;
use embassy_rp::peripherals::{DMA_CH0, I2C0, PIO0, USB};
use embassy_rp::pio::{self, Pio};
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program};
use embassy_rp::usb::{self, Driver};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config};
use firmware_shared::{aht20, encode, encoded_len, reading};
use smart_leds::RGB8;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

/// Dry is blue, comfortable green, damp red.
fn colour(r: aht20::Reading) -> RGB8 {
    match r.centi_percent_rh {
        0..=3_499 => RGB8::new(0, 0, 24),
        3_500..=6_000 => RGB8::new(0, 24, 0),
        _ => RGB8::new(24, 0, 0),
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // USB serial: the stick enumerates as a CDC-ACM device.
    let driver = Driver::new(p.USB, Irqs);
    let mut config = Config::new(0x2e8a, 0x000a);
    config.manufacturer = Some("fiducial example");
    config.product = Some("sensor-stick");
    config.max_power = 100;
    static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static STATE: StaticCell<State> = StaticCell::new();
    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESC.init([0; 256]),
        BOS_DESC.init([0; 256]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );
    let mut serial = CdcAcmClass::new(&mut builder, STATE.init(State::new()), 64);
    let mut usb = builder.build();

    // The sensor, on the pins the hardware declares.
    let mut i2c = I2c::new_async(
        p.I2C0,
        board::sensor_scl!(p),
        board::sensor_sda!(p),
        Irqs,
        i2c::Config::default(),
    );

    // The LED, driven by a PIO state machine.
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let program = PioWs2812Program::new(&mut common);
    let mut led: PioWs2812<'_, PIO0, 0, 1, Grb> =
        PioWs2812::new(&mut common, sm0, p.DMA_CH0, Irqs, board::led_din!(p), &program);

    let work = async {
        Timer::after_millis(40).await;
        let _ = i2c.write_async(board::SENSOR_I2C_ADDRESS, aht20::INIT).await;
        let mut frame = [0u8; encoded_len(reading::LEN)];
        loop {
            Timer::after_millis(1000 - aht20::MEASURE_MS).await;
            if i2c.write_async(board::SENSOR_I2C_ADDRESS, aht20::MEASURE).await.is_err() {
                continue;
            }
            Timer::after_millis(aht20::MEASURE_MS).await;
            let mut raw = [0u8; 7];
            if i2c.read_async(board::SENSOR_I2C_ADDRESS, &mut raw).await.is_err() {
                continue;
            }
            let Some(r) = aht20::convert(&raw) else { continue };
            led.write(&[colour(r)]).await;
            if let Ok(n) = encode(&reading::payload(r), &mut frame) {
                // Nobody listening is not an error: the reading is dropped.
                let _ = serial.write_packet(&frame[..n]).await;
            }
        }
    };
    join(usb.run(), work).await;
}

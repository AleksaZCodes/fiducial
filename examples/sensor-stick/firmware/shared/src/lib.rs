//! Target-agnostic firmware for the sensor stick: the sensor's arithmetic and
//! the reading's wire format. No HAL here, so it is tested on the host
//! (`cargo test -p firmware-shared`).
#![no_std]

pub use fiducial_core::{version, DeviceId, VERSION};
pub use fiducial_protocol::{encode, encoded_len, FrameDecoder};

/// The AHT20 temperature and humidity sensor (Aosong), over I²C.
pub mod aht20 {
    /// Its fixed I²C address.
    pub const ADDRESS: u8 = 0x38;
    /// Calibrate, once after power-up (datasheet §5.4: wait 40 ms first).
    pub const INIT: [u8; 3] = [0xBE, 0x08, 0x00];
    /// Start a measurement; the result is ready 80 ms later.
    pub const MEASURE: [u8; 3] = [0xAC, 0x33, 0x00];
    /// Measurement time, ms.
    pub const MEASURE_MS: u64 = 80;

    /// One reading, in hundredths: 2153 is 21.53 °C, 4810 is 48.10 %RH.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Reading {
        pub centi_celsius: i16,
        pub centi_percent_rh: u16,
    }

    /// CRC-8, polynomial 0x31, initial 0xFF (datasheet §5.4.4).
    pub fn crc8(data: &[u8]) -> u8 {
        let mut crc = 0xFFu8;
        for &b in data {
            crc ^= b;
            for _ in 0..8 {
                crc = if crc & 0x80 != 0 { (crc << 1) ^ 0x31 } else { crc << 1 };
            }
        }
        crc
    }

    /// The seven bytes the sensor answers with, as a reading. None while it
    /// is still busy, or when the CRC says the bytes were damaged.
    pub fn convert(raw: &[u8; 7]) -> Option<Reading> {
        if raw[0] & 0x80 != 0 || crc8(&raw[..6]) != raw[6] {
            return None;
        }
        let rh = ((raw[1] as u32) << 12) | ((raw[2] as u32) << 4) | ((raw[3] as u32) >> 4);
        let t = (((raw[3] & 0x0F) as u32) << 16) | ((raw[4] as u32) << 8) | raw[5] as u32;
        // RH = S / 2^20 × 100 %; T = S / 2^20 × 200 − 50 °C — in hundredths.
        let centi_percent_rh = ((rh as u64 * 10_000) >> 20) as u16;
        let centi_celsius = (((t as u64 * 20_000) >> 20) as i32 - 5_000) as i16;
        Some(Reading { centi_celsius, centi_percent_rh })
    }
}

/// A reading on the wire: the payload of one fiducial-protocol frame.
/// `[1, celsius×100 as i16 LE, %RH×100 as u16 LE]` — five bytes, read by
/// the web page with the same layout (`web/src/reading.ts`).
pub mod reading {
    use super::aht20::Reading;

    /// The payload's first byte: what kind of message this is.
    pub const KIND: u8 = 1;
    pub const LEN: usize = 5;

    pub fn payload(r: Reading) -> [u8; LEN] {
        let t = r.centi_celsius.to_le_bytes();
        let h = r.centi_percent_rh.to_le_bytes();
        [KIND, t[0], t[1], h[0], h[1]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The standard check vector for this CRC-8 (poly 0x31, init 0xFF): 0xBE 0xEF → 0x92.
    #[test]
    fn crc_matches_the_reference_vector() {
        assert_eq!(aht20::crc8(&[0xBE, 0xEF]), 0x92);
    }

    fn frame(rh: u32, t: u32) -> [u8; 7] {
        let mut b = [
            0x1C,
            (rh >> 12) as u8,
            (rh >> 4) as u8,
            (((rh & 0xF) << 4) as u8) | ((t >> 16) as u8 & 0x0F),
            (t >> 8) as u8,
            t as u8,
            0,
        ];
        b[6] = aht20::crc8(&b[..6]);
        b
    }

    #[test]
    fn half_scale_is_fifty_percent_and_fifty_degrees() {
        let r = aht20::convert(&frame(1 << 19, 1 << 19)).unwrap();
        assert_eq!(r.centi_percent_rh, 5_000);
        assert_eq!(r.centi_celsius, 5_000);
    }

    #[test]
    fn zero_is_minus_fifty_degrees_and_dry() {
        let r = aht20::convert(&frame(0, 0)).unwrap();
        assert_eq!(r, aht20::Reading { centi_celsius: -5_000, centi_percent_rh: 0 });
    }

    #[test]
    fn a_busy_or_damaged_answer_is_not_a_reading() {
        let mut busy = frame(1 << 19, 1 << 19);
        busy[0] |= 0x80;
        busy[6] = aht20::crc8(&busy[..6]);
        assert_eq!(aht20::convert(&busy), None);
        let mut damaged = frame(1 << 19, 1 << 19);
        damaged[2] ^= 1;
        assert_eq!(aht20::convert(&damaged), None);
    }

    #[test]
    fn a_reading_round_trips_through_a_frame() {
        let r = aht20::Reading { centi_celsius: -1234, centi_percent_rh: 4810 };
        let payload = reading::payload(r);
        let mut wire = [0u8; 64];
        let n = encode(&payload, &mut wire).unwrap();
        let mut dec: FrameDecoder<64> = FrameDecoder::new();
        let got = wire[..n].iter().find_map(|b| dec.feed(*b).map(|p| p.to_vec()));
        assert_eq!(got.as_deref(), Some(&payload[..]));
        assert_eq!(i16::from_le_bytes([payload[1], payload[2]]), -1234);
    }
}

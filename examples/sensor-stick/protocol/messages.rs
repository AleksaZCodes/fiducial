//! Derived by fid-protocol from protocol.toml — do not edit.
//!
//! One module per message: `KIND`, `LEN`, the struct, and `encode` /
//! `decode` for its payload (the kind byte, then each field
//! little-endian). The web page's `protocol/messages.ts` is derived
//! from the same table, so the two cannot disagree.
#![allow(dead_code)]

/// The USB IDs the device enumerates with; the page filters on the same.
pub const USB_VENDOR_ID: u16 = 0x2e8a;
pub const USB_PRODUCT_ID: u16 = 0x000a;

/// One temperature and humidity reading, sent once a second.
pub mod reading {
    pub const KIND: u8 = 1;
    pub const LEN: usize = 5;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Reading {
        /// Temperature, °C × 100
        pub centi_celsius: i16,
        /// Relative humidity, %RH × 100
        pub centi_percent_rh: u16,
    }

    impl Reading {
        pub fn encode(&self) -> [u8; LEN] {
            let mut b = [0u8; LEN];
            b[0] = KIND;
            b[1..3].copy_from_slice(&self.centi_celsius.to_le_bytes());
            b[3..5].copy_from_slice(&self.centi_percent_rh.to_le_bytes());
            b
        }

        pub fn decode(b: &[u8]) -> Option<Self> {
            if b.len() != LEN || b[0] != KIND {
                return None;
            }
            Some(Self {
                centi_celsius: i16::from_le_bytes([b[1], b[2]]),
                centi_percent_rh: u16::from_le_bytes([b[3], b[4]]),
            })
        }
    }

    /// Comfortable indoor humidity — `centi_percent_rh` from 35 to 60 %RH, inclusive, in counts.
    pub const COMFORTABLE_MIN: u16 = 3500;
    pub const COMFORTABLE_MAX: u16 = 6000;
}

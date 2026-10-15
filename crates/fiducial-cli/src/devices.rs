//! Device profiles: what a part's datasheet says, written once in the
//! platform and checked against every product that uses the part.
//!
//! A product declares what it believes about a part — its pins' limit, its
//! I²C address — and nothing else knows better. So a belief the derive
//! refuses can be "corrected" until it agrees: in the paper's round 4 an
//! RP2040's `io_max_v` was raised to 5.5 V to let a 5 V pull-up through
//! (R17), a buffer's active-low enable was tied high (R16), and the sensor
//! was sent an older part's command bytes (R15). The datasheet is the
//! authority for all three; a profile is the datasheet, read once.
//!
//! A part matches the profile whose `mpn` its own MPN starts with
//! (`SN74AHCT1G125DBVR` is an `SN74AHCT1G125`). A part with no profile is
//! checked only against what the product declares, as before.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

const PROFILES: &[(&str, &str)] = &[
    ("aht20.toml", include_str!("../devices/aht20.toml")),
    ("rp2040.toml", include_str!("../devices/rp2040.toml")),
    (
        "sn74ahct1g125.toml",
        include_str!("../devices/sn74ahct1g125.toml"),
    ),
    ("ws2812b.toml", include_str!("../devices/ws2812b.toml")),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub mpn: String,
    /// The datasheet and section the facts come from, for the message.
    pub source: String,
    #[serde(default)]
    pub i2c_address: Option<u8>,
    /// The supply pin and its operating range, volts.
    #[serde(default)]
    pub supply: Option<Supply>,
    /// No pin above the supply pin's rail plus this, volts.
    #[serde(default)]
    pub io_max_over_supply: Option<f64>,
    /// Byte sequences the part is sent: `init`, `measure`.
    #[serde(default)]
    pub commands: BTreeMap<String, Vec<u8>>,
    /// By pad number: the datasheet name and, for an enable, its sense.
    #[serde(default)]
    pub pins: BTreeMap<String, PinFact>,
    /// Waits the part needs, milliseconds: `measure_ms`.
    #[serde(default)]
    pub timing: BTreeMap<String, u64>,
    /// The part is a light: it has to be seen through the case.
    #[serde(default)]
    pub light: bool,
    /// The crystal frequency the part is built around.
    #[serde(default)]
    pub crystal: Option<Crystal>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crystal {
    /// The pin the crystal drives.
    pub pin: String,
    pub hz: u64,
    /// What breaks at any other frequency, for the message.
    pub because: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    pub pin: String,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinFact {
    pub name: String,
    /// `low` or `high`: the level that turns it on.
    #[serde(default)]
    pub active: Option<String>,
    /// `enable`: the part does nothing while this pin is inactive.
    #[serde(default)]
    pub role: Option<String>,
}

fn all() -> Result<Vec<Profile>> {
    PROFILES
        .iter()
        .map(|(f, s)| toml::from_str(s).with_context(|| format!("device profile {f}")))
        .collect()
}

/// The profile for a part's MPN: the longest profile `mpn` it starts with.
pub fn for_mpn(mpn: &str) -> Result<Option<Profile>> {
    let up = mpn.to_ascii_uppercase();
    Ok(all()?
        .into_iter()
        .filter(|p| up.starts_with(&p.mpn.to_ascii_uppercase()))
        .max_by_key(|p| p.mpn.len()))
}

impl Profile {
    /// The net a product connects to this profile pin — by pad number or by
    /// the datasheet name.
    pub fn net<'a>(&self, pin: &str, pins: &'a BTreeMap<String, String>) -> Option<&'a String> {
        pins.get(pin).or_else(|| {
            let name = self.pins.get(pin).map(|f| f.name.as_str()).unwrap_or(pin);
            pins.get(name)
        })
    }
}

/// A net's level when it is a supply: a declared rail's voltage, or 0 for a
/// ground. None for a signal.
pub fn level(net: &str, rails: &BTreeMap<String, f64>) -> Option<f64> {
    if let Some(v) = rails.get(net) {
        return Some(*v);
    }
    let n = net.to_ascii_uppercase();
    (n.starts_with("GND") || n.ends_with("GND") || n == "VSS" || n == "0V").then_some(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_profile_parses_and_a_part_number_finds_its_family() {
        assert_eq!(all().unwrap().len(), PROFILES.len());
        let p = for_mpn("SN74AHCT1G125DBVR").unwrap().unwrap();
        assert_eq!(p.mpn, "SN74AHCT1G125");
        assert_eq!(p.pins["1"].active.as_deref(), Some("low"));
        assert!(for_mpn("NE555").unwrap().is_none());
        assert_eq!(
            for_mpn("AHT20").unwrap().unwrap().commands["init"],
            vec![0xBE, 0x08, 0x00]
        );
    }
}

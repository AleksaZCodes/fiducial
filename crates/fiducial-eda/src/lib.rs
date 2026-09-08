//! EDA pipeline types — `no_std`.
//!
//! Defines the `BoardInterface` schema written to `board/board.interface.json`
//! by the EDA pipeline. The JSON is tracked by `fid derive` and its hash
//! recorded in `fiducial.lock`; `fid derive --check` detects staleness.
//!
//! # Schema (board.interface.json)
//!
//! ```json
//! {
//!   "schema_version": "1.0",
//!   "board": { "name": "my-board", "revision": "A" },
//!   "connectors": [
//!     { "id": "J1", "name": "USB-C", "type": "usb-c", "pins": [
//!       { "number": 1, "name": "VBUS", "net": "PWR_5V", "direction": "power_in" }
//!     ]}
//!   ],
//!   "net_classes": [{ "name": "power", "nets": ["PWR_5V", "GND"] }]
//! }
//! ```
//!
//! # Validation (requires `std` feature)
//!
//! ```rust,ignore
//! use fiducial_eda::validate;
//! let bi = validate(json_str)?;
//! ```

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

#[cfg(test)]
extern crate std;

use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Direction of a pin signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinDirection {
    /// Driven externally into the board.
    Input,
    /// Driven by the board to external loads.
    Output,
    /// Can be driven either way (e.g. SWD, I²C).
    Bidirectional,
    /// Power enters through this pin.
    PowerIn,
    /// Power exits through this pin.
    PowerOut,
}

/// A single connector pin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    /// Physical pin number.
    pub number: u32,
    /// Signal name on the pin (e.g. "VBUS", "D+").
    pub name: String,
    /// Net this pin belongs to.
    pub net: String,
    /// Signal direction.
    pub direction: PinDirection,
}

/// A connector or port on the board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connector {
    /// Reference designator (e.g. "J1").
    pub id: String,
    /// Human-readable name (e.g. "USB-C Power").
    pub name: String,
    /// Connector family type (e.g. "usb-c", "swd", "qwiic").
    #[serde(rename = "type")]
    pub kind: String,
    /// All pins on this connector.
    pub pins: Vec<Pin>,
}

/// A named net class grouping related signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetClass {
    /// Net class name (e.g. "power", "usb", "debug").
    pub name: String,
    /// All net names that belong to this class.
    pub nets: Vec<String>,
}

/// Board identity block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    /// Machine-readable board identifier.
    pub name: String,
    /// PCB revision (e.g. "A", "B", "1.2").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Human-readable board description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Root of `board/board.interface.json`.
///
/// Declares every connector, pin, and net class on the board — the machine-
/// readable interface between the EDA pipeline and the rest of the platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardInterface {
    /// Schema version — increment when the format makes a breaking change.
    pub schema_version: String,
    /// Board identity.
    pub board: Board,
    /// All external connectors.
    pub connectors: Vec<Connector>,
    /// Net class groupings.
    pub net_classes: Vec<NetClass>,
}

// ── Validation (std feature) ──────────────────────────────────────────────────

/// Error returned by [`validate`].
#[cfg(feature = "std")]
#[derive(Debug)]
pub enum ValidationError {
    /// JSON is malformed or missing required fields.
    Parse(serde_json::Error),
    /// Schema version is not supported by this version of `fiducial-eda`.
    UnsupportedVersion(String),
}

#[cfg(feature = "std")]
impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "parse error: {e}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported schema_version: {v:?}"),
        }
    }
}

/// Parse and validate a `board.interface.json` string.
///
/// Returns `Ok(BoardInterface)` if the JSON is well-formed and the
/// `schema_version` is supported. Returns `Err(ValidationError)` otherwise.
#[cfg(feature = "std")]
pub fn validate(json: &str) -> Result<BoardInterface, ValidationError> {
    let bi: BoardInterface = serde_json::from_str(json).map_err(ValidationError::Parse)?;
    if bi.schema_version != "1.0" {
        return Err(ValidationError::UnsupportedVersion(bi.schema_version));
    }
    Ok(bi)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;

    const SEED_JSON: &str =
        include_str!("../../fiducial-cli/capabilities/eda/board/board.interface.json");

    #[test]
    fn parse_seed_json() {
        let bi: BoardInterface = serde_json::from_str(SEED_JSON).expect("seed JSON must parse");
        assert_eq!(bi.schema_version, "1.0");
        assert!(!bi.board.name.is_empty());
    }

    #[test]
    fn seed_has_at_least_one_connector() {
        let bi: BoardInterface = serde_json::from_str(SEED_JSON).unwrap();
        assert!(!bi.connectors.is_empty());
    }

    #[test]
    fn seed_connectors_have_pins() {
        let bi: BoardInterface = serde_json::from_str(SEED_JSON).unwrap();
        for connector in &bi.connectors {
            assert!(
                !connector.pins.is_empty(),
                "connector {} has no pins",
                connector.id
            );
        }
    }

    #[test]
    fn seed_net_classes_non_empty() {
        let bi: BoardInterface = serde_json::from_str(SEED_JSON).unwrap();
        assert!(!bi.net_classes.is_empty());
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let bi: BoardInterface = serde_json::from_str(SEED_JSON).unwrap();
        let json = serde_json::to_string(&bi).unwrap();
        let bi2: BoardInterface = serde_json::from_str(&json).unwrap();
        assert_eq!(bi.board.name, bi2.board.name);
        assert_eq!(bi.connectors.len(), bi2.connectors.len());
    }

    #[test]
    fn pin_direction_serde_roundtrip() {
        let dir = PinDirection::PowerIn;
        let json = serde_json::to_string(&dir).unwrap();
        assert_eq!(json, r#""power_in""#);
        let dir2: PinDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(dir, dir2);
    }

    #[test]
    fn optional_fields_omitted_when_none() {
        let board = Board {
            name: "test".to_string(),
            revision: None,
            description: None,
        };
        let json = serde_json::to_string(&board).unwrap();
        assert!(!json.contains("revision"));
        assert!(!json.contains("description"));
    }

    #[test]
    fn validate_seed_json() {
        let result = validate(SEED_JSON);
        assert!(
            result.is_ok(),
            "seed JSON must validate: {:?}",
            result.err()
        );
    }

    #[test]
    fn validate_rejects_wrong_version() {
        let bad =
            r#"{"schema_version":"2.0","board":{"name":"x"},"connectors":[],"net_classes":[]}"#;
        let result = validate(bad);
        assert!(matches!(
            result,
            Err(ValidationError::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn validate_rejects_malformed_json() {
        let result = validate("{not json}");
        assert!(matches!(result, Err(ValidationError::Parse(_))));
    }
}

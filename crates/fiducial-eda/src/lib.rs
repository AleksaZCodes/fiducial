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
//!   "outline": {
//!     "width_mm": 100.0, "height_mm": 60.0, "tolerance": "fdm",
//!     "enclosure": { "headroom_mm": 10.0, "standoff_height_mm": 3.0 }
//!   },
//!   "connectors": [
//!     { "id": "J1", "name": "USB-C", "type": "usb-c", "pins": [
//!       { "number": 1, "name": "VBUS", "net": "PWR_5V", "direction": "power_in" }
//!     ],
//!     "mount": { "side": "south", "offset_mm": 20.0 }}
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

#[cfg(any(test, feature = "std"))]
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
    /// Where this connector meets the case wall, when it needs an opening.
    ///
    /// Optional: a header meant to be reached with the lid off has no mount,
    /// and gets no hole. Omitting it is the safe default — an unnecessary
    /// opening is a leak.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount: Option<Mount>,
}

/// Where a connector meets the case wall.
///
/// `side` and `offset_mm` are in **board coordinates**: the board sits in the
/// first quadrant, and `offset_mm` is measured along the named edge from the
/// board's origin corner. Positioning by the board rather than by the wall
/// means the declaration does not have to know how thick the seal made the
/// wall.
///
/// `width_mm` and `height_mm` default to the connector family's body envelope,
/// looked up from `type`. Declare them only for a part that differs from its
/// family — the process clearance is added on top either way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    /// Board edge the connector faces: `north`, `south`, `east`, or `west`.
    pub side: String,
    /// Centre of the connector along that edge, from the board's origin corner.
    pub offset_mm: f32,
    /// Body width across the edge. Defaults to the family envelope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f32>,
    /// Body height. Defaults to the family envelope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f32>,
    /// Opening floor above the board's top surface. Negative for a connector
    /// that hangs below it, such as a mid-mount receptacle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z_offset_mm: Option<f32>,
}

impl Mount {
    /// Body envelope for this mount: the declaration if given, otherwise the
    /// family default for `kind`.
    ///
    /// Returns `None` when neither is available — the caller must then reject
    /// the declaration rather than guess a size for an unknown family.
    pub fn envelope(&self, kind: &str) -> Option<(f32, f32)> {
        let fallback = fiducial_geometry::connector_opening(kind);
        let w = self.width_mm.or_else(|| fallback.map(|o| o.width_mm))?;
        let h = self.height_mm.or_else(|| fallback.map(|o| o.height_mm))?;
        Some((w, h))
    }
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

/// Physical board outline — the declaration the mesh pipeline derives from.
///
/// Optional so that boards which do not need an enclosure omit it entirely;
/// when present, `fid derive` generates the case parts from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outline {
    /// Board width in millimetres.
    pub width_mm: f32,
    /// Board height in millimetres.
    pub height_mm: f32,
    /// PCB thickness in millimetres (standard FR4 = 1.6).
    #[serde(default = "default_thickness")]
    pub thickness_mm: f32,
    /// Manufacturing process for the generated enclosure: `fdm`, `resin`, or `cnc`.
    #[serde(default = "default_tolerance")]
    pub tolerance: String,
    /// Case overrides for values the process cannot imply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enclosure: Option<EnclosureOptions>,
}

/// Overrides for the generated case.
///
/// Everything here defaults from the tolerance profile or from a documented
/// constant. These are the values a process cannot know: how tall the tallest
/// component is, and what gasket stock the product uses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnclosureOptions {
    /// Vertical space above the board, in millimetres. Raise for tall parts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headroom_mm: Option<f32>,
    /// Lid plate thickness in millimetres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lid_thickness_mm: Option<f32>,
    /// Gasket cross-section width in millimetres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket_width_mm: Option<f32>,
    /// Gasket cross-section height in millimetres, uncompressed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket_height_mm: Option<f32>,
    /// Fraction of gasket height squeezed when closed. Must be in (0, 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gasket_compression: Option<f32>,
    /// Height of the posts the board rests on, in millimetres.
    ///
    /// Omit — or set to nothing — and the board sits on the cavity floor.
    /// Declaring a height raises the board onto four corner posts and grows
    /// the case by the same amount, because headroom is measured above the
    /// board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standoff_height_mm: Option<f32>,
    /// Footprint of each standoff post, square, in millimetres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standoff_size_mm: Option<f32>,
}

fn default_thickness() -> f32 {
    1.6
}

fn default_tolerance() -> String {
    alloc::string::ToString::to_string("fdm")
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
    /// Physical outline, when the board drives enclosure generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<Outline>,
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
    /// An `outline` block was present but describes an unbuildable board.
    InvalidOutline(String),
    /// A connector's `mount` block cannot be turned into an opening.
    InvalidMount(String),
}

#[cfg(feature = "std")]
impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "parse error: {e}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported schema_version: {v:?}"),
            Self::InvalidOutline(m) => write!(f, "invalid outline: {m}"),
            Self::InvalidMount(m) => write!(f, "invalid connector mount: {m}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            _ => None,
        }
    }
}

/// Parse and validate a `board.interface.json` string.
///
/// Returns `Ok(BoardInterface)` if the JSON is well-formed and the
/// `schema_version` is supported. Returns `Err(ValidationError)` otherwise.
#[cfg(feature = "std")]
pub fn validate(json: &str) -> Result<BoardInterface, ValidationError> {
    use alloc::string::ToString;

    let bi: BoardInterface = serde_json::from_str(json).map_err(ValidationError::Parse)?;
    if bi.schema_version != "1.0" {
        return Err(ValidationError::UnsupportedVersion(bi.schema_version));
    }
    if let Some(o) = &bi.outline {
        // A non-positive dimension produces a degenerate mesh that slicers
        // accept and then print as nothing, so reject it at the declaration.
        if !(o.width_mm > 0.0 && o.height_mm > 0.0 && o.thickness_mm > 0.0) {
            return Err(ValidationError::InvalidOutline(
                "width_mm, height_mm and thickness_mm must all be > 0".to_string(),
            ));
        }
        if !matches!(o.tolerance.as_str(), "fdm" | "resin" | "cnc") {
            return Err(ValidationError::InvalidOutline(alloc::format!(
                "unknown tolerance {:?} (expected fdm, resin or cnc)",
                o.tolerance
            )));
        }
        if let Some(e) = &o.enclosure {
            for (name, value) in [
                ("headroom_mm", e.headroom_mm),
                ("lid_thickness_mm", e.lid_thickness_mm),
                ("gasket_width_mm", e.gasket_width_mm),
                ("gasket_height_mm", e.gasket_height_mm),
                ("standoff_height_mm", e.standoff_height_mm),
                ("standoff_size_mm", e.standoff_size_mm),
            ] {
                // NaN is checked explicitly: it compares false against every
                // bound, so a plain `v <= 0.0` would let it through and produce
                // a mesh full of NaN coordinates.
                if let Some(v) = value {
                    if v.is_nan() || v <= 0.0 {
                        return Err(ValidationError::InvalidOutline(alloc::format!(
                            "enclosure.{name} must be > 0, got {v}"
                        )));
                    }
                }
            }
            // A compression of 0 never squeezes the gasket and 1 crushes it
            // flat; both produce a case that does not seal.
            if let Some(c) = e.gasket_compression {
                if c.is_nan() || c <= 0.0 || c >= 1.0 {
                    return Err(ValidationError::InvalidOutline(alloc::format!(
                        "enclosure.gasket_compression must be between 0 and 1 (exclusive), got {c}"
                    )));
                }
            }
        }
    }

    for c in &bi.connectors {
        let Some(m) = &c.mount else { continue };
        // An outline is what a mount is measured against; a mount without one
        // has nothing to cut and would be silently dropped.
        if bi.outline.is_none() {
            return Err(ValidationError::InvalidMount(alloc::format!(
                "connector {} declares a mount but the board declares no outline",
                c.id
            )));
        }
        if fiducial_geometry::Side::from_name(&m.side).is_none() {
            return Err(ValidationError::InvalidMount(alloc::format!(
                "connector {}: unknown side {:?} (expected north, south, east or west)",
                c.id,
                m.side
            )));
        }
        if m.offset_mm.is_nan() || m.offset_mm < 0.0 {
            return Err(ValidationError::InvalidMount(alloc::format!(
                "connector {}: offset_mm must be >= 0, got {}",
                c.id,
                m.offset_mm
            )));
        }
        for (name, value) in [("width_mm", m.width_mm), ("height_mm", m.height_mm)] {
            if let Some(v) = value {
                if v.is_nan() || v <= 0.0 {
                    return Err(ValidationError::InvalidMount(alloc::format!(
                        "connector {}: mount.{name} must be > 0, got {v}",
                        c.id
                    )));
                }
            }
        }
        if let Some(z) = m.z_offset_mm {
            if z.is_nan() {
                return Err(ValidationError::InvalidMount(alloc::format!(
                    "connector {}: mount.z_offset_mm is not a number",
                    c.id
                )));
            }
        }
        // A family with no entry in the opening table cannot imply a size, so
        // the declaration has to supply one rather than have one guessed.
        if m.envelope(&c.kind).is_none() {
            return Err(ValidationError::InvalidMount(alloc::format!(
                "connector {}: no body envelope known for type {:?} — declare \
                 mount.width_mm and mount.height_mm",
                c.id,
                c.kind
            )));
        }
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

    // ── Outline ──────────────────────────────────────────────────────────────

    #[test]
    fn seed_declares_an_outline() {
        let bi = validate(SEED_JSON).unwrap();
        let o = bi.outline.expect("seed must declare an outline");
        assert!(o.width_mm > 0.0 && o.height_mm > 0.0);
        assert_eq!(o.tolerance, "fdm");
    }

    #[test]
    fn outline_is_optional() {
        let json =
            r#"{"schema_version":"1.0","board":{"name":"x"},"connectors":[],"net_classes":[]}"#;
        assert!(validate(json).unwrap().outline.is_none());
    }

    #[test]
    fn outline_defaults_thickness_and_tolerance() {
        let json = r#"{"schema_version":"1.0","board":{"name":"x"},
            "outline":{"width_mm":10.0,"height_mm":5.0},
            "connectors":[],"net_classes":[]}"#;
        let o = validate(json).unwrap().outline.unwrap();
        assert_eq!(o.thickness_mm, 1.6);
        assert_eq!(o.tolerance, "fdm");
    }

    #[test]
    fn validate_rejects_non_positive_dimensions() {
        let json = r#"{"schema_version":"1.0","board":{"name":"x"},
            "outline":{"width_mm":0.0,"height_mm":5.0},
            "connectors":[],"net_classes":[]}"#;
        assert!(matches!(
            validate(json),
            Err(ValidationError::InvalidOutline(_))
        ));
    }

    #[test]
    fn validate_rejects_unknown_tolerance() {
        let json = r#"{"schema_version":"1.0","board":{"name":"x"},
            "outline":{"width_mm":10.0,"height_mm":5.0,"tolerance":"sintering"},
            "connectors":[],"net_classes":[]}"#;
        assert!(matches!(
            validate(json),
            Err(ValidationError::InvalidOutline(_))
        ));
    }

    // ── Connector mounts ─────────────────────────────────────────────────────

    fn with_mount(mount: &str) -> std::string::String {
        std::format!(
            r#"{{"schema_version":"1.0","board":{{"name":"x"}},
               "outline":{{"width_mm":100.0,"height_mm":60.0}},
               "connectors":[{{"id":"J1","name":"USB","type":"usb-c",
                 "pins":[{{"number":1,"name":"VBUS","net":"V","direction":"power_in"}}],
                 "mount":{mount}}}],
               "net_classes":[]}}"#
        )
    }

    #[test]
    fn seed_declares_a_mount_for_its_usb_port() {
        let bi = validate(SEED_JSON).unwrap();
        let j1 = bi.connectors.iter().find(|c| c.id == "J1").unwrap();
        let m = j1.mount.as_ref().expect("J1 must be mounted");
        assert_eq!(m.side, "south");
        // Size is not declared — it comes from the connector family.
        assert!(m.width_mm.is_none());
        assert_eq!(m.envelope(&j1.kind), Some((8.94, 3.26)));
    }

    #[test]
    fn the_seed_debug_header_is_deliberately_unmounted() {
        // A 2x3 shrouded header is 8.5 mm tall; an opening that tall reaches
        // the gasket groove on any sanely sized case. It is reached with the
        // lid off instead, so it gets no hole.
        let bi = validate(SEED_JSON).unwrap();
        let j2 = bi.connectors.iter().find(|c| c.id == "J2").unwrap();
        assert!(j2.mount.is_none());
    }

    #[test]
    fn mount_is_optional() {
        let bi = validate(SEED_JSON).unwrap();
        assert!(bi.connectors.iter().any(|c| c.mount.is_none()));
        assert!(bi.connectors.iter().any(|c| c.mount.is_some()));
    }

    #[test]
    fn mount_size_falls_back_to_the_connector_family() {
        let bi = validate(&with_mount(r#"{"side":"south","offset_mm":20.0}"#)).unwrap();
        let c = &bi.connectors[0];
        assert_eq!(
            c.mount.as_ref().unwrap().envelope(&c.kind),
            Some((8.94, 3.26))
        );
    }

    #[test]
    fn a_declared_mount_size_overrides_the_family() {
        let json =
            with_mount(r#"{"side":"south","offset_mm":20.0,"width_mm":12.0,"height_mm":4.0}"#);
        let bi = validate(&json).unwrap();
        let c = &bi.connectors[0];
        assert_eq!(
            c.mount.as_ref().unwrap().envelope(&c.kind),
            Some((12.0, 4.0))
        );
    }

    #[test]
    fn validate_rejects_an_unknown_side() {
        let json = with_mount(r#"{"side":"up","offset_mm":20.0}"#);
        assert!(matches!(
            validate(&json),
            Err(ValidationError::InvalidMount(_))
        ));
    }

    #[test]
    fn validate_rejects_a_negative_offset() {
        let json = with_mount(r#"{"side":"south","offset_mm":-5.0}"#);
        assert!(matches!(
            validate(&json),
            Err(ValidationError::InvalidMount(_))
        ));
    }

    #[test]
    fn validate_rejects_a_non_positive_mount_size() {
        let json = with_mount(r#"{"side":"south","offset_mm":20.0,"width_mm":0.0}"#);
        assert!(matches!(
            validate(&json),
            Err(ValidationError::InvalidMount(_))
        ));
    }

    #[test]
    fn validate_rejects_a_mount_with_no_outline_to_measure_against() {
        let json = r#"{"schema_version":"1.0","board":{"name":"x"},
            "connectors":[{"id":"J1","name":"USB","type":"usb-c",
              "pins":[{"number":1,"name":"V","net":"V","direction":"power_in"}],
              "mount":{"side":"south","offset_mm":20.0}}],
            "net_classes":[]}"#;
        assert!(matches!(
            validate(json),
            Err(ValidationError::InvalidMount(_))
        ));
    }

    #[test]
    fn validate_rejects_an_unsizable_connector_family() {
        // An unknown family cannot imply an envelope, and guessing one is how a
        // case ships with a hole the connector does not fit through.
        let json = r#"{"schema_version":"1.0","board":{"name":"x"},
            "outline":{"width_mm":100.0,"height_mm":60.0},
            "connectors":[{"id":"J7","name":"Mystery","type":"db25",
              "pins":[{"number":1,"name":"V","net":"V","direction":"power_in"}],
              "mount":{"side":"south","offset_mm":20.0}}],
            "net_classes":[]}"#;
        let err = validate(json).unwrap_err();
        let msg = std::format!("{err}");
        assert!(msg.contains("db25"), "message must name the type: {msg}");
        assert!(
            msg.contains("width_mm"),
            "message must say what to add: {msg}"
        );
    }

    #[test]
    fn mounts_survive_a_serde_roundtrip() {
        let bi = validate(SEED_JSON).unwrap();
        let json = serde_json::to_string(&bi).unwrap();
        let again = validate(&json).unwrap();
        let before = bi.connectors.iter().filter(|c| c.mount.is_some()).count();
        let after = again
            .connectors
            .iter()
            .filter(|c| c.mount.is_some())
            .count();
        assert_eq!(before, after);
        assert!(before > 0);
    }

    // ── Standoffs ────────────────────────────────────────────────────────────

    #[test]
    fn seed_declares_standoffs() {
        let e = validate(SEED_JSON)
            .unwrap()
            .outline
            .unwrap()
            .enclosure
            .unwrap();
        assert_eq!(e.standoff_height_mm, Some(3.0));
        assert_eq!(e.standoff_size_mm, Some(5.0));
    }

    #[test]
    fn standoffs_are_optional() {
        let json = r#"{"schema_version":"1.0","board":{"name":"x"},
            "outline":{"width_mm":10.0,"height_mm":5.0},
            "connectors":[],"net_classes":[]}"#;
        assert!(validate(json).unwrap().outline.unwrap().enclosure.is_none());
    }

    #[test]
    fn validate_rejects_a_non_positive_standoff() {
        let json = r#"{"schema_version":"1.0","board":{"name":"x"},
            "outline":{"width_mm":10.0,"height_mm":5.0,
              "enclosure":{"standoff_height_mm":0.0}},
            "connectors":[],"net_classes":[]}"#;
        assert!(matches!(
            validate(json),
            Err(ValidationError::InvalidOutline(_))
        ));
    }

    #[test]
    fn outline_roundtrips_through_serde() {
        let bi = validate(SEED_JSON).unwrap();
        let json = serde_json::to_string(&bi).unwrap();
        let again = validate(&json).unwrap().outline.unwrap();
        assert_eq!(again.width_mm, bi.outline.unwrap().width_mm);
    }
}

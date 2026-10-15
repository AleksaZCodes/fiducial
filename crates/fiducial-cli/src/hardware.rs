//! `fid-hardware` — a physical product, declared once.
//!
//! A product with a case is **parts** placed in **regions** of an **outline**,
//! held by **mounts**. `hardware/product.toml` declares those four things and
//! what the product may cost; this module solves where everything goes and
//! writes the answer as text a person, an agent and a CAD kernel can all read:
//!
//! | Output | Is |
//! |---|---|
//! | `layout.json` | every solved position, size and hole, and *why* — the contract `cad.py` builds from |
//! | `bom.csv` | declared parts plus the ones the design implies (screws, gaskets) |
//! | `*.kicad_pcb` | the board: outline, holes, real footprints placed, every pad on its net |
//! | `*.kicad_sch` | the schematic: each part's KiCad symbol, its pins labelled with their nets |
//! | `assembly.md` | the order it goes together in, derived from the mounts |
//!
//! Solid geometry is deliberately **not** made here. Offsetting a non-convex
//! outline, cutting a gasket groove round a boss, checking that a lid closes —
//! that is a B-rep kernel's job, and MISSION.md's first anti-goal is not
//! writing one. The kernel reads `layout.json`, and every decision it needs has
//! already been made in this file, where `fid derive --check` can gate it.
//!
//! Spec: `docs/specs/2026-09-30-a-physical-product-is-parts-in-regions.md`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use fiducial_geometry::fit::{self, Rect, P};
use serde::Deserialize;
use serde_json::{json, Value};

/// Default declaration path, relative to the product root.
pub const DECLARATION: &str = "hardware/product.toml";

// ── The declaration ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub product: Meta,
    pub outline: Shape,
    #[serde(default)]
    pub regions: BTreeMap<String, Shape>,
    pub case: Case,
    pub board: Board,
    pub cost: Cost,
    #[serde(default, rename = "part")]
    pub parts: Vec<Part>,
    #[serde(default, rename = "wire")]
    pub wires: Vec<Wire>,
    /// DC checks of the resistive set points (`crate::circuit`): nets driven,
    /// nets measured, the window each must land in, solved from the
    /// declared resistors and their nets on every derive.
    #[serde(default, rename = "check")]
    pub checks: Vec<crate::circuit::Check>,
    /// The microcontroller the firmware runs on. Its pins' nets become
    /// `hardware/generated/board.rs`: one macro per net, naming the pin.
    #[serde(default)]
    pub firmware: Option<Firmware>,
    /// The voltage of each power rail: `[rails] VBUS = 5.0, "3V3" = 3.3`.
    /// GND is 0 V. With it, every net a resistor pulls toward a rail is
    /// solved, and a pin pulled above its part's `io_max_v` fails derive.
    #[serde(default)]
    pub rails: BTreeMap<String, f64>,
}

/// `[firmware] mcu = "<part id>"` — the board part whose pins the firmware
/// drives. Its symbol's pin names say which peripheral each net is on
/// (`GPIO4` on an RP2040, `PA5` on an STM32).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Firmware {
    pub mcu: String,
}

/// A wire between two parts. Routed, drawn, and its length — path plus
/// slack — goes into the BOM, so a harness is cut to a derived length rather
/// than a guess.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub id: String,
    /// Part ids at each end.
    pub from: String,
    pub to: String,
    #[serde(default = "d_cores")]
    pub cores: u32,
    #[serde(default = "d_awg")]
    pub awg: u32,
    /// Extra length beyond the routed path. A wire from a part on the lid gets
    /// at least `case.service_loop_mm` on top, so the lid opens without
    /// unsoldering anything.
    #[serde(default = "d_slack")]
    pub slack_mm: f64,
    /// `wire` (cut to its derived length) or `coax` (a part's own pigtail).
    #[serde(default)]
    pub kind: Option<String>,
    /// A cable that comes with a part — an antenna's pigtail — has its length
    /// already: the route must fit it, and nothing is cut.
    #[serde(default)]
    pub fixed_mm: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// A shape inside an SVG: the file, an optional `data-layer`, the index of the
/// `<path>`/`<polygon>` within it, and the subpath.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub svg: String,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub path: usize,
    #[serde(default)]
    pub subpath: usize,
    /// Outline only: a fixed width in mm. Absent, the width is solved from
    /// the parts placed in regions.
    #[serde(default)]
    pub width_mm: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    #[serde(default = "d_process")]
    pub process: String,
    /// The printed case's colour, `#rrggbb` — the filament, or `#rrggbbaa`
    /// for a translucent one. The renders and the viewer draw it; the lid a
    /// shade lighter. Unset: a neutral grey.
    #[serde(default)]
    pub colour: Option<String>,
    pub wall_mm: f64,
    pub floor_mm: f64,
    pub lid_mm: f64,
    #[serde(default = "d_clearance")]
    pub clearance_mm: f64,
    /// The plate width is rounded to this, in whichever direction keeps every
    /// fit true.
    #[serde(default = "d_round")]
    pub round_mm: f64,
    /// Search step for what is still placed by search: parts on the lid,
    /// holes along an edge, wires. The floor is placed by the solver, to a
    /// tenth of a millimetre, whatever this is. Every result passes the exact
    /// fit test; this only sets how finely positions are tried.
    #[serde(default = "d_step")]
    pub grid_mm: f64,
    #[serde(default)]
    pub seal: Seal,
    #[serde(default)]
    pub fasteners: Fasteners,
    /// Wire a lid-mounted part needs beyond its route, so the lid can be
    /// lifted off and set beside the base while still connected.
    #[serde(default = "d_service_loop")]
    pub service_loop_mm: f64,
    /// Shapes cut into or raised from the top of the lid.
    #[serde(default, rename = "mark")]
    pub marks: Vec<Mark>,
    /// A lug to hang the product by.
    #[serde(default)]
    pub hang: Option<Hang>,
}

/// A shape on the lid, taken from a region of the SVG and set against a lid
/// part. `piece = "roof"` is the part of the region above its full-width span —
/// a house's roof — seated on the top edge of the part `on`: a panel that is
/// the house's walls gets its roof back, drawn into the lid.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    pub id: String,
    pub region: String,
    #[serde(default = "d_piece")]
    pub piece: String,
    pub on: String,
    /// `deboss` (cut in), `emboss` (raised) or `etch` (its outline, cut in).
    #[serde(default = "d_mark_kind")]
    pub kind: String,
    #[serde(default = "d_mark_depth")]
    pub depth_mm: f64,
    /// `etch`: the width of the line.
    #[serde(default = "d_mark_line")]
    pub line_mm: f64,
}

/// How the case hangs, from the outline's highest point.
///
/// `kind = "holes"`: the tip is solid for `solid_mm` — outside the seal, so
/// nothing reaches the cavity — with a `hole_mm` hole through it front to
/// back (a nail, a screw on a wall) and a `cross_mm` hole side to side (a
/// cable tie round a pole, a branch). `kind = "lug"`: a tab out of the top.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Hang {
    #[serde(default = "d_hang_kind")]
    pub kind: String,
    /// `top`: the outline's highest point.
    #[serde(default = "d_hang_at")]
    pub at: String,
    #[serde(default = "d_hang_hole")]
    pub hole_mm: f64,
    #[serde(default = "d_hang_solid")]
    pub solid_mm: f64,
    #[serde(default = "d_hang_cross")]
    pub cross_mm: f64,
    #[serde(default = "d_hang_width")]
    pub width_mm: f64,
    /// How far it stands out beyond the outline.
    #[serde(default = "d_hang_reach")]
    pub reach_mm: f64,
    #[serde(default = "d_hang_thick")]
    pub thickness_mm: f64,
}

/// How far parts at fixed positions (`at_mm`) hang past the board's left,
/// right, bottom and top edges.
fn overhang(parts: &[&Part], size: (f64, f64)) -> (f64, f64, f64, f64) {
    let mut o = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    for q in parts {
        let Some(a) = q.at_mm else { continue };
        let turned = q
            .rotate_deg
            .is_some_and(|r| (r.rem_euclid(180.0) - 90.0).abs() < 1e-6);
        let (w, h) = if turned {
            (q.body_mm[1], q.body_mm[0])
        } else {
            (q.body_mm[0], q.body_mm[1])
        };
        o.0 = o.0.max(w / 2.0 - a[0]);
        o.1 = o.1.max(a[0] + w / 2.0 - size.0);
        o.2 = o.2.max(h / 2.0 - a[1]);
        o.3 = o.3.max(a[1] + h / 2.0 - size.1);
    }
    o
}

fn d_window_overlap() -> f64 {
    3.0
}

fn d_pegs() -> String {
    "bl-tr".into()
}

fn d_board_mount() -> String {
    "screws".into()
}

/// `"panel"` or `["panel", "vent"]`.
fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}

fn d_piece() -> String {
    "roof".into()
}
fn d_mark_kind() -> String {
    "deboss".into()
}
fn d_mark_depth() -> f64 {
    0.8
}
fn d_mark_line() -> f64 {
    1.0
}
fn d_hang_kind() -> String {
    "holes".into()
}
fn d_hang_solid() -> f64 {
    24.0
}
fn d_hang_cross() -> f64 {
    4.0
}
fn d_hang_at() -> String {
    "top".into()
}
fn d_hang_hole() -> f64 {
    5.0
}
fn d_hang_width() -> f64 {
    12.0
}
fn d_hang_reach() -> f64 {
    10.0
}
fn d_hang_thick() -> f64 {
    4.0
}

/// Every field defaults (to a 2 mm groove, 2.5 mm deep): `kind = "none"` needs
/// nothing else, and a gasket declares only what differs from the default.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Seal {
    /// `gasket` or `none`.
    pub kind: String,
    pub groove_mm: f64,
    pub depth_mm: f64,
    /// Material either side of the groove.
    pub lip_mm: f64,
    /// Fraction the gasket is squeezed when the lid is screwed down.
    pub compression: f64,
    /// How far the gasket, uncompressed, stands proud of the rim. A gasket
    /// that sits inside its groove seals nothing until something reaches down
    /// to it; this is what the lid squeezes.
    #[serde(default = "d_proud")]
    pub proud_mm: f64,
    /// How far the lid's tongue reaches into the groove. Absent, it is derived:
    /// exactly what brings the gasket to its declared compression.
    #[serde(default)]
    pub tongue_mm: Option<f64>,
    /// Thickness of the flat gasket between a window part and the lid.
    #[serde(default = "d_window_gasket")]
    pub window_gasket_mm: f64,
    /// How far a window part overlaps its window on every side, for its own
    /// gasket. Only a product with a window needs it.
    #[serde(default = "d_window_overlap")]
    pub window_overlap_mm: f64,
    /// Surface parts: thickness of the adhesive foam gasket (3M VHB or
    /// similar) that bonds and seals the part to the top of the lid.
    #[serde(default = "d_adhesive_t")]
    pub adhesive_mm: f64,
    /// Surface parts: width of that gasket ring, inward from the part's edge.
    #[serde(default = "d_adhesive_w")]
    pub adhesive_width_mm: f64,
}

impl Default for Seal {
    fn default() -> Self {
        Seal {
            kind: "gasket".into(),
            groove_mm: 2.0,
            depth_mm: 2.5,
            lip_mm: 1.2,
            compression: 0.25,
            proud_mm: d_proud(),
            tongue_mm: None,
            window_gasket_mm: d_window_gasket(),
            window_overlap_mm: 3.0,
            adhesive_mm: d_adhesive_t(),
            adhesive_width_mm: d_adhesive_w(),
        }
    }
}

/// Every field defaults (M3, 80 mm apart): a press-fit lid has no screws
/// to size, and a screwed one declares only what differs.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Fasteners {
    /// How the lid is held on:
    ///
    /// | `closure` | Is |
    /// |---|---|
    /// | `screws-top` | screws down through the lid into bosses in the base |
    /// | `screws-back` | screws up from underneath, through the base, into the lid — nothing on the face |
    /// | `press-fit` | no screws: a skirt under the lid is an interference fit inside the base |
    #[serde(default = "d_closure")]
    pub closure: String,
    /// Press-fit only: how much bigger the skirt is than the opening it enters.
    #[serde(default = "d_interference")]
    pub interference_mm: f64,
    /// Press-fit only: how far the skirt reaches down into the base.
    #[serde(default = "d_skirt")]
    pub skirt_mm: f64,
    /// Metric screw size for the lid: `M2.5`, `M3`, `M4`.
    pub size: String,
    /// Screws are spread round the outline no further apart than this.
    pub max_spacing_mm: f64,
    #[serde(default)]
    pub count: Option<usize>,
    /// Corners sharper than this (interior angle, degrees) can hold a screw.
    #[serde(default = "d_corner")]
    pub max_corner_deg: f64,
}

impl Default for Fasteners {
    fn default() -> Self {
        Fasteners {
            closure: d_closure(),
            interference_mm: d_interference(),
            skirt_mm: d_skirt(),
            size: "M3".into(),
            max_spacing_mm: 80.0,
            count: None,
            max_corner_deg: d_corner(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    /// Fixed size. Absent, the board is sized from its parts' footprints.
    #[serde(default)]
    pub size_mm: Option<[f64; 2]>,
    /// The ceiling a derived size may not exceed — a fab price tier, usually.
    #[serde(default = "d_board_max")]
    pub max_mm: [f64; 2],
    #[serde(default = "d_board_t")]
    pub thickness_mm: f64,
    #[serde(default = "d_layers")]
    pub layers: u32,
    /// Footprint area as a fraction of board area, for a derived size. 0.4 is
    /// comfortable for hand routing; 0.6 is dense.
    #[serde(default = "d_fill")]
    pub fill: f64,
    #[serde(default = "d_standoff")]
    pub standoff_mm: f64,
    /// Screw size for the board's own mounting holes.
    #[serde(default = "d_board_screw")]
    pub screw: String,
    #[serde(default = "d_hole_inset")]
    pub hole_inset_mm: f64,
    /// What the board must reach under: lid parts by id (the panel's lead
    /// hole, a vent) — one, or a list — or an outline side.
    #[serde(default, deserialize_with = "one_or_many")]
    pub near: Vec<String>,
    /// Board zones, each a band along one board edge (`top`, `bottom`,
    /// `left`, `right`) or `centre` for what is left: `{ rf = "top", power =
    /// "bottom" }`. Parts name a zone; the band is `zone_depth` of the board.
    #[serde(default)]
    pub zones: BTreeMap<String, String>,
    #[serde(default = "d_zone_depth")]
    pub zone_depth: f64,
    /// How the board is held: `screws` (four, into standoffs) or `snap` (two
    /// pegs through diagonal corners locate it, two hooks on its side edges
    /// hold it down — it presses in, nothing to screw).
    #[serde(default = "d_board_mount")]
    pub mount: String,
    /// Nets that carry current and get wider tracks when `hardware/route.py`
    /// routes the board: a net name, or a prefix ending in `*`. GND always.
    #[serde(default)]
    pub power_nets: Vec<String>,
    /// A clear strip round the board's edge where no part sits: room for
    /// tracks to run round the perimeter. The board grows by twice it.
    #[serde(default)]
    pub edge_mm: f64,
    /// How hard the placement solver searches (hardware/place.py), in its
    /// deterministic time units — roughly five seconds each. More finds
    /// shorter connections; the answer is cached, so it is paid only when
    /// the placement model changes.
    #[serde(default = "d_placement_effort")]
    pub placement_effort: f64,
    /// Which diagonal a snap board's two locating pegs take: `bl-tr`
    /// (bottom-left and top-right, the default) or `tl-br` — whichever leaves
    /// the corner a big part needs free.
    #[serde(default = "d_pegs")]
    pub pegs: String,
    /// Signal track width and copper clearance for routing (`route.py`).
    /// Unset: 0.2 and 0.15. A 1.27 mm-pitch test-pad array escapes only at
    /// 0.15 / 0.127, which JLCPCB's standard two-layer process holds.
    #[serde(default)]
    pub track_mm: Option<f64>,
    #[serde(default)]
    pub clearance_mm: Option<f64>,
    /// Mounting holes at fixed positions, `[x, y]` mm from the board's
    /// bottom-left corner — a bought board's (a Raspberry Pi's) from its
    /// drawing. Unset: four, inset `hole_inset_mm` from the corners.
    #[serde(default)]
    pub holes_mm: Option<Vec<[f64; 2]>>,
    /// Who routes the board: `auto` (Freerouting, in `hardware/build.sh`) or
    /// `hand` — a person routes it in KiCad, starting from the derived
    /// board, and keeps it as `hardware/board-routed.kicad_pcb`. `fid derive`
    /// then fails while that board's parts, turns, sides, pad nets or outline
    /// differ from the declaration's; the build checks it with KiCad's DRC
    /// instead of routing.
    #[serde(default = "d_routing")]
    pub routing: String,
    /// How far each corner of the board may be cut to follow the case, in mm
    /// (0, the default: a rectangle). Where the case narrows — a chamfer, a
    /// taper — the board stays as long as it needs and loses its corners
    /// instead: its outline is the rectangle clipped to the cavity, written
    /// to Edge.Cuts, and nothing on it is placed in what is cut.
    #[serde(default)]
    pub cut_mm: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    pub currency: String,
    pub ceiling: f64,
    #[serde(default = "d_qty")]
    pub quantity: u32,
    /// Fail derive when any part is unpriced, not only when the priced total
    /// exceeds the ceiling.
    #[serde(default)]
    pub strict: bool,
    /// Prices for parts the design implies rather than declares, by BOM id.
    #[serde(default)]
    pub prices: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub id: String,
    pub name: String,
    /// A region name, `board`, `cavity`, or `wall`.
    pub place: String,
    /// How it is held — see [`MOUNTS`].
    pub mount: String,
    /// Envelope `[x, y, z]` in mm. For a cylinder, `x` is the length along
    /// its axis and `y` the diameter. A part with a `footprint` omits it and
    /// declares only `height_mm`: its x and y are the footprint's courtyard.
    #[serde(default)]
    pub body_mm: [f64; 3],
    #[serde(default)]
    pub height_mm: Option<f64>,
    #[serde(default = "d_shape")]
    pub shape: String,
    /// Wall parts: which way the wall faces — `bottom`, `top`, `left`, `right`.
    #[serde(default)]
    pub side: Option<String>,
    /// Region parts: how the region is scaled to the part.
    ///
    /// | `fit` | The region grows until… | Default for |
    /// |---|---|---|
    /// | `width` | its width equals the part's | `surface` |
    /// | `height` | its height equals the part's | |
    /// | `cover` | it meets the part, less the window overlap — the part hides it | `window` |
    /// | `inside` | the part fits inside it | `pocket` |
    #[serde(default)]
    pub fit: Option<String>,
    /// Surface parts: sunk into a pocket in the lid so its face is level with
    /// the lid's — it drops in, sits flat, and nothing stands proud to catch.
    #[serde(default)]
    pub flush: bool,
    /// Surface parts: diameter of the hole through the lid its leads pass
    /// through, inside the adhesive ring.
    #[serde(default = "d_pass_through")]
    pub pass_through_mm: f64,
    /// Where the part should go: another part's id (a board part near the
    /// panel's lead hole), or a side of the outline — `bottom`, `top`,
    /// `left`, `right`. A preference, not a constraint: the nearest position
    /// that fits wins.
    #[serde(default)]
    pub near: Option<String>,
    /// Board parts: the board zone it belongs in (see `board.zones`).
    #[serde(default)]
    pub zone: Option<String>,
    /// Board parts: which of the part's own sides — `top`, `bottom`, `left`,
    /// `right`, as drawn at 0° — must face the outer edge of its zone. The
    /// solver rotates it to make that true: an RF pin toward the board edge.
    #[serde(default)]
    pub faces: Option<String>,
    /// Board parts: a copper-free strip this deep beyond the facing side, to
    /// the board edge — an antenna keep-out, written into KiCad as a rule area.
    #[serde(default)]
    pub keepout_mm: f64,
    /// Board parts: parts this one must stay `away_mm` from — a gas and
    /// temperature sensor away from a charger's heat.
    #[serde(default)]
    pub away_from: Vec<String>,
    #[serde(default)]
    pub away_mm: f64,
    /// `pads` mount: one pad per net, in order, `pad_mm` each, `pitch_mm` apart.
    #[serde(default)]
    pub nets: Vec<String>,
    /// Socketed parts: the leads out of one end, in order across it, `pitch_mm`
    /// apart and `lead_mm` long — `["+", "-"]` for a radial capacitor lying
    /// on its side. They come out of the end facing the board, and a wire
    /// names one as `part#n.+`.
    #[serde(default)]
    pub leads: Vec<String>,
    #[serde(default)]
    pub lead_mm: f64,
    #[serde(default)]
    pub pad_mm: Option<[f64; 2]>,
    #[serde(default)]
    pub pitch_mm: Option<f64>,
    /// `pads` mount: a socketed part whose leads these pads take — one pad per
    /// lead, in line with it, on the board edge facing it. The leads run
    /// straight to them and bend down through; no wire.
    #[serde(default)]
    pub follows: Option<String>,
    /// `vent` mount: the board part this vent's air is for. A tube printed
    /// with the lid runs from round the membrane down to the board, sealed
    /// on it by a gasket ring round that part — outside air reaches the
    /// part, not the cavity. The part sits under the vent; nothing else may
    /// sit under the ring.
    #[serde(default)]
    pub seals_to: Option<String>,
    /// `vent` mount: where the membrane is bonded — `inside` (the default,
    /// under the lid, out of the weather) or `outside` (on the lid's face,
    /// as adhesive vents are made to be; in a debossed mark it sits below
    /// the surface). Outside, a chimney need only be as wide as the hole.
    #[serde(default)]
    pub membrane: Option<String>,
    /// `pads` mount: plated through-holes of this drill, instead of SMD pads.
    #[serde(default)]
    pub drill_mm: Option<f64>,
    /// The KiCad component, by library id. With a `footprint`, the part's
    /// size is its courtyard and its pins are the footprint's pads — read
    /// from the vendored file (`hardware/parts.py sync`), never typed. The
    /// board file gets the real footprint; the model gets its STEP.
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub footprint: Option<String>,
    /// Board parts: what each pin connects to — `{ VIN_DC = "SOLAR+", GND =
    /// "GND" }`. A key is a symbol pin name (every pin of that name: a
    /// module's three GND pins) or a pad number. The nets go onto the pads of
    /// `board.kicad_pcb` and into `board.kicad_sch`; a net with one end fails.
    #[serde(default)]
    pub pins: BTreeMap<String, String>,
    /// The component value — `4.7µF 10V X5R`, `22µH` — for the BOM, the
    /// schematic and the board's fab layer.
    #[serde(default)]
    pub value: Option<String>,
    /// The highest voltage this part's pins may see, from its datasheet's
    /// absolute maximum ratings (an RP2040's I/O: IOVDD + 0.5 V). Checked
    /// against `[rails]` on every derive.
    #[serde(default)]
    pub io_max_v: Option<f64>,
    /// The part's 7-bit I²C address, from its datasheet: `0x38`. Derived
    /// into `board.rs` as `<PART>_I2C_ADDRESS`, so the firmware never types
    /// it (the paper's study, case H10: a mistyped address in firmware code
    /// disagreed with nothing, because nothing else knew it).
    #[serde(default)]
    pub i2c_address: Option<u8>,
    /// Command bytes from the part's device profile (`crate::devices`),
    /// derived into `board.rs` as `<PART>_<COMMAND>`. Not declared.
    #[serde(skip)]
    pub commands: BTreeMap<String, Vec<u8>>,
    /// Waits from the part's device profile, ms, derived into `board.rs` as
    /// `<PART>_<NAME>`. Not declared.
    #[serde(skip)]
    pub timing: BTreeMap<String, u64>,
    /// LCSC's description of the part ordered, from `parts.lock`. Not declared.
    #[serde(skip)]
    pub catalog: Option<String>,
    /// Board parts: an explicit turn, degrees counter-clockwise — for a part
    /// whose orientation is a design choice rather than a zone edge's.
    #[serde(default)]
    pub rotate_deg: Option<f64>,
    /// Board parts: a fixed position, `[x, y]` mm from the board's
    /// bottom-left corner to the part's centre — taken from a bought board's
    /// mechanical drawing, where the ports are not a choice.
    #[serde(default)]
    pub at_mm: Option<[f64; 2]>,
    /// Board parts that `faces` an edge: the case wall in front of it gets an
    /// opening, so a plug reaches it from outside — a USB-C socket, a card slot.
    #[serde(default)]
    pub through_wall: bool,
    /// The opening's width and height: what goes in (a plug's overmould), not
    /// the socket. Unset: the part's own face plus 0.5 mm all round.
    #[serde(default)]
    pub opening_mm: Option<[f64; 2]>,
    /// How far behind the case's outer face the part's mouth may sit and a
    /// plug still seat: 3 mm through a window the socket's size, 10 mm through
    /// one sized for the plug's body (`opening_mm`), unless declared.
    #[serde(default)]
    pub recess_mm: Option<f64>,
    /// Any part: its own solid model (a product file, `hardware/vendor/…step`)
    /// drawn in place of the box or cylinder its envelope would be. It must
    /// fit the declared `body_mm`; `cad.py` fails the build if it does not.
    #[serde(default)]
    pub model: Option<String>,
    /// The turn, degrees about x, y and z, that stands `model` the way the
    /// part sits in the case.
    #[serde(default)]
    pub model_rotate_deg: Option<[f64; 3]>,
    /// Board parts: named points on the part a wire can end at that are not
    /// pads — a module's own IPEX socket. `[x, y, z]` in the footprint's own
    /// coordinates (KiCad: y down), z above the board.
    #[serde(default)]
    pub ports: BTreeMap<String, [f64; 3]>,
    /// Board parts: the pad (by number) that must face the zone's edge — an
    /// antenna pin. Resolved to `faces` from where the footprint puts it.
    #[serde(default)]
    pub faces_pin: Option<String>,
    /// Where the dimensions came from: `datasheet`, `distributor`, `search`
    /// or `assumed`. A decided part whose body is not from a datasheet or a
    /// distributor is listed as unverified — sizes remembered rather than
    /// read are how a 14 × 20 mm module gets drawn 16 × 16.
    #[serde(default)]
    pub dims_from: Option<String>,
    /// The page the part and its dimensions were read from.
    #[serde(default)]
    pub source_url: Option<String>,
    /// The manufacturer's datasheet (a PDF URL). `hardware/parts.py sync`
    /// fetches it into `hardware/lib/datasheets/`, with a text copy an agent
    /// can search, and indexes it in `hardware/lib/PARTS.md`.
    #[serde(default)]
    pub datasheet: Option<String>,
    #[serde(default = "d_one")]
    pub qty: u32,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub mpn: Option<String>,
    #[serde(default)]
    pub lcsc: Option<String>,
    /// What the part must be, instead of an `lcsc` number looked up by hand:
    /// `hardware/parts.py resolve` answers it from the JLCPCB catalogue into
    /// `hardware/parts.lock`, and derive takes the LCSC number from there.
    #[serde(default)]
    pub pick: Option<Pick>,
    #[serde(default)]
    pub second_source: Option<String>,
    #[serde(default)]
    pub unit_cost: Option<f64>,
    /// `decided` or `assumed`.
    #[serde(default = "d_status")]
    pub status: String,
    #[serde(default)]
    pub settle_by: Option<String>,
    /// Rendering hint: `pcb`, `panel`, `metal`, `cell`, `plastic`, `tpu`.
    #[serde(default)]
    pub look: Option<String>,
}

/// A part declared by requirement, resolved like a dependency. Opaque here:
/// its fields, and what they match, are `hardware/parts.py`'s, declared once
/// there. Derive asks only whether the lock answered this exact pick — a
/// changed pick is an unanswered one.
pub type Pick = toml::Table;

/// `hardware/parts.lock`, as derive reads it: only what reaches the BOM.
#[derive(Debug, Deserialize, Default)]
struct PartsLock {
    #[serde(default)]
    part: Vec<LockedPart>,
}

#[derive(Debug, Deserialize)]
struct LockedPart {
    id: String,
    /// None for a pinned part `resolve` only verified.
    #[serde(default)]
    pick: Option<Pick>,
    lcsc: String,
    #[serde(default)]
    mpn: Option<String>,
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    unit_price: Option<f64>,
    #[serde(default)]
    currency: Option<String>,
    /// LCSC's own description of the part: "1uF ±10% 50V Ceramic …".
    #[serde(default)]
    description: Option<String>,
}

/// The first resistance, capacitance or inductance in a text, in base units:
/// "5.1k" → 5100, "27R" → 27, "1uF 50V X5R" → 1e-6, "5.1kΩ ±1% 100mW" → 5100.
/// `unit_required` is for LCSC descriptions, which also carry voltages and
/// tolerances; a declared value may leave its unit implied ("5.1k").
pub fn component_quantity(text: &str, unit_required: bool) -> Option<f64> {
    for tok in text.split(|c: char| c.is_whitespace() || c == ',' || c == '/') {
        let num_end = tok
            .char_indices()
            .find(|(_, c)| !(c.is_ascii_digit() || *c == '.'))
            .map_or(tok.len(), |(i, _)| i);
        let Ok(n) = tok[..num_end].parse::<f64>() else {
            continue;
        };
        let mut rest = tok[num_end..].chars().peekable();
        let scale = match rest.peek() {
            Some('p') => 1e-12,
            Some('n') => 1e-9,
            Some('u') | Some('µ') | Some('μ') => 1e-6,
            Some('m') => 1e-3,
            Some('k') | Some('K') => 1e3,
            Some('M') => 1e6,
            _ => 1.0,
        };
        if scale != 1.0 {
            rest.next();
        }
        let unit: String = rest.collect();
        let known = matches!(unit.as_str(), "F" | "H" | "Ω" | "R" | "ohm" | "Ohm");
        if known || (!unit_required && unit.is_empty()) {
            return Some(n * scale);
        }
    }
    None
}

/// A pinned part whose declared value is not the part being ordered: the
/// schematic and BOM say one thing, the reel another (the paper's held-out
/// test, case X14). Only compared when both sides state a value.
/// The first frequency in a text, in hertz: "Crystal 12MHz ±10ppm" → 12e6,
/// "32.768kHz" → 32768.
pub fn frequency_hz(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    for (unit, scale) in [("mhz", 1e6), ("khz", 1e3), ("hz", 1.0)] {
        if let Some(at) = lower.find(unit) {
            let num: String = lower[..at]
                .trim_end()
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if let Ok(n) = num.parse::<f64>() {
                return Some(n * scale);
            }
        }
    }
    None
}

fn value_disagrees(q: &Part, hit: &LockedPart) -> Option<String> {
    let declared = component_quantity(q.value.as_deref()?, false)?;
    let desc = hit.description.as_deref()?;
    let ordered = component_quantity(desc, true)?;
    let off = (declared - ordered).abs() / ordered.abs().max(f64::MIN_POSITIVE);
    (off > 0.005).then(|| {
        format!(
            "part `{}`: value \"{}\" is not the part ordered — LCSC {} is \"{desc}\". \
             Change the value or the part, so the schematic, the BOM and the reel agree",
            q.id,
            q.value.as_deref().unwrap_or_default(),
            hit.lcsc
        )
    })
}

pub const PARTS_LOCK: &str = "hardware/parts.lock";

/// Every `pick`, answered from the lock: its LCSC number, MPN and maker, and
/// its price when the lock's currency is the cost's. An unanswered pick
/// stops derive — a BOM line without a part number is not orderable.
fn resolve_picks(root: &Path, p: &mut Product) -> Result<()> {
    let lock: PartsLock = match std::fs::read_to_string(root.join(PARTS_LOCK)) {
        Ok(raw) => toml::from_str(&raw).map_err(|e| anyhow!("{PARTS_LOCK}: {e}"))?,
        Err(_) => PartsLock::default(),
    };
    // A pinned part's price, as `resolve` read it from LCSC at the cost's
    // quantity: what the ceiling is asserted against.
    let mut disagree = Vec::new();
    // Once a product has a lock, a pinned part outside it is one nobody
    // checked against LCSC: swapped in by hand, its description — and so
    // its value and frequency — unknown (the paper's round 5, case M18).
    let resolved = lock.part.iter().any(|e| e.pick.is_none());
    for q in p.parts.iter_mut().filter(|q| q.pick.is_none()) {
        let Some(lcsc) = &q.lcsc else { continue };
        let hit = lock
            .part
            .iter()
            .find(|e| e.id == q.id && &e.lcsc == lcsc && e.pick.is_none());
        if hit.is_none() && resolved {
            disagree.push(format!(
                "part `{}`: LCSC {lcsc} is not in {PARTS_LOCK} — a part nobody checked against \
                 LCSC. Run `python3 hardware/parts.py resolve` so its catalog entry is checked \
                 against what the board needs",
                q.id
            ));
        }
        if let Some(hit) = hit {
            q.catalog = hit.description.clone();
            disagree.extend(value_disagrees(q, hit));
            if q.unit_cost.is_none() && hit.currency.as_deref() == Some(p.cost.currency.as_str()) {
                q.unit_cost = hit.unit_price;
            }
        }
    }
    if !disagree.is_empty() {
        bail!("{}", disagree.join("\n"));
    }
    if p.parts.iter().all(|q| q.pick.is_none()) {
        return Ok(());
    }
    for q in &mut p.parts {
        let Some(pick) = &q.pick else { continue };
        if q.lcsc.is_some() {
            bail!(
                "part `{}`: declares both `lcsc` and `pick` — a part is pinned or picked, not both",
                q.id
            );
        }
        let Some(hit) = lock
            .part
            .iter()
            .find(|e| e.id == q.id && e.pick.as_ref() == Some(pick))
        else {
            bail!(
                "part `{}`: its pick is not answered in {PARTS_LOCK} — run `python3 hardware/parts.py resolve`",
                q.id
            );
        };
        q.lcsc = Some(hit.lcsc.clone());
        if q.mpn.is_none() {
            q.mpn = hit.mpn.clone();
        }
        if q.manufacturer.is_none() {
            q.manufacturer = hit.manufacturer.clone();
        }
        if q.unit_cost.is_none() && hit.currency.as_deref() == Some(p.cost.currency.as_str()) {
            q.unit_cost = hit.unit_price;
        }
    }
    Ok(())
}

fn d_routing() -> String {
    "auto".into()
}

/// A hand-routed board, kept by the product (`board.routing = "hand"`).
pub const HAND_ROUTED: &str = "hardware/board-routed.kicad_pcb";

fn d_process() -> String {
    "fdm".into()
}
fn d_clearance() -> f64 {
    0.4
}
fn d_round() -> f64 {
    0.5
}
fn d_step() -> f64 {
    1.0
}
fn d_proud() -> f64 {
    0.5
}
fn d_window_gasket() -> f64 {
    1.5
}
fn d_corner() -> f64 {
    150.0
}
fn d_closure() -> String {
    "screws-top".into()
}
fn d_interference() -> f64 {
    0.15
}
fn d_skirt() -> f64 {
    5.0
}
fn d_adhesive_t() -> f64 {
    1.1
}
fn d_adhesive_w() -> f64 {
    5.0
}
fn d_service_loop() -> f64 {
    60.0
}
fn d_pass_through() -> f64 {
    6.0
}
fn d_cores() -> u32 {
    2
}
fn d_awg() -> u32 {
    24
}
/// The symbol a `pads` part stands for in the schematic: a single-row
/// connector with one pin per net, KiCad's own (`parts.py sync` vendors it).
pub fn pads_symbol(n: usize) -> String {
    format!("Connector_Generic:Conn_01x{n:02}")
}

/// How far a placed rectangle misses its target point. With `margin` 0, centre
/// to point; otherwise how far the point lies outside the rectangle shrunk by
/// `margin` (0 once it is that far inside).
/// Where a `size` rectangle's centre may be, `wall` inside `outline`: a set of
/// rectangles of centres, each wholly feasible, for the solver to choose among.
///
/// Rows every half millimetre; along each row the feasible runs are found on
/// a millimetre's scan and their ends bisected to a hundredth. Rows whose runs
/// differ by no more than a tenth at either end become one rectangle (their
/// intersection), so a straight wall is one rectangle and a chamfer a few.
/// Every rectangle is inside the true region, give or take a tenth: the
/// solver's answer is checked against the exact fit afterwards.
fn feasible_centres(outline: &[P], size: P, wall: f64) -> Vec<(P, P)> {
    feasible_centres_by(outline, size, wall, |x, y| {
        fit::rect_fits(
            outline,
            &Rect {
                centre: (x, y),
                size,
            },
            // A rectangle exactly at the wall distance fits.
            wall - 1e-6,
        )
    })
}

/// A rectangle of `size` with each corner cut `cut` along both edges: the
/// least of a board whose corners may be cut to follow the case.
fn chamfered(c: P, size: P, cut: f64) -> Vec<P> {
    let (hw, hh) = (size.0 / 2.0, size.1 / 2.0);
    let k = cut.min(hw).min(hh);
    vec![
        (c.0 - hw + k, c.1 - hh),
        (c.0 + hw - k, c.1 - hh),
        (c.0 + hw, c.1 - hh + k),
        (c.0 + hw, c.1 + hh - k),
        (c.0 + hw - k, c.1 + hh),
        (c.0 - hw + k, c.1 + hh),
        (c.0 - hw, c.1 + hh - k),
        (c.0 - hw, c.1 - hh + k),
    ]
}

/// [`feasible_centres`] with any test of whether the item fits at a centre.
fn feasible_centres_by(
    outline: &[P],
    size: P,
    wall: f64,
    fits: impl Fn(f64, f64) -> bool,
) -> Vec<(P, P)> {
    const ROW: f64 = 0.5;
    const SCAN: f64 = 1.0;
    const TOL: f64 = 0.1;
    let (lo, hi) = fit::bounds(outline);
    let (x0, x1) = (lo.0 + wall + size.0 / 2.0, hi.0 - wall - size.0 / 2.0);
    let (y0, y1) = (lo.1 + wall + size.1 / 2.0, hi.1 - wall - size.1 / 2.0);
    if x0 > x1 || y0 > y1 {
        return Vec::new();
    }
    // The boundary between an infeasible `out` and a feasible `inn`.
    let edge = |y: f64, mut out: f64, mut inn: f64| {
        for _ in 0..12 {
            let mid = (out + inn) / 2.0;
            if fits(mid, y) {
                inn = mid;
            } else {
                out = mid;
            }
        }
        inn
    };
    let cols = ((x1 - x0) / SCAN).floor() as usize;
    let xs: Vec<f64> = (0..=cols)
        .map(|k| x0 + k as f64 * SCAN)
        .chain(((x1 - x0) % SCAN > 1e-9).then_some(x1))
        .collect();
    let runs_at = |y: f64| -> Vec<(f64, f64)> {
        let ok: Vec<bool> = xs.iter().map(|&x| fits(x, y)).collect();
        let mut runs = Vec::new();
        let mut k = 0;
        while k < xs.len() {
            if !ok[k] {
                k += 1;
                continue;
            }
            let start = k;
            while k + 1 < xs.len() && ok[k + 1] {
                k += 1;
            }
            let a = if start == 0 {
                xs[0]
            } else {
                edge(y, xs[start - 1], xs[start])
            };
            let b = if k + 1 == xs.len() {
                xs[k]
            } else {
                edge(y, xs[k + 1], xs[k])
            };
            runs.push((a, b));
            k += 1;
        }
        runs
    };
    let same = |p: &[(f64, f64)], q: &[(f64, f64)]| {
        p.len() == q.len()
            && p.iter()
                .zip(q)
                .all(|(u, v)| (u.0 - v.0).abs() <= TOL && (u.1 - v.1).abs() <= TOL)
    };
    // Coarse rows; between two that differ (a wall's slope, a region's
    // start or end), fine rows a tenth apart, so the edge is found to that.
    let rows = ((y1 - y0) / ROW).floor() as usize;
    let coarse: Vec<f64> = (0..=rows)
        .map(|k| y0 + k as f64 * ROW)
        .chain(((y1 - y0) % ROW > 1e-9).then_some(y1))
        .collect();
    let mut runs_by_row: Vec<(f64, Vec<(f64, f64)>)> = Vec::new();
    for (k, &y) in coarse.iter().enumerate() {
        let runs = runs_at(y);
        if let Some((py, prev)) = runs_by_row.last().cloned() {
            if !same(&prev, &runs) {
                let fine = ((y - py) / 0.1).round() as usize;
                for f in 1..fine {
                    let fy = py + f as f64 * (y - py) / fine as f64;
                    runs_by_row.push((fy, runs_at(fy)));
                }
            }
        }
        let _ = k;
        runs_by_row.push((y, runs));
    }
    // Rows into rectangles: extend while the run moves less than TOL at
    // either end; otherwise close, and start the next from the band between.
    // Each open rectangle: its span (the intersection so far), its rows'
    // extent, the span it started with, and the last row's run.
    struct Open {
        a: f64,
        b: f64,
        y0: f64,
        y1: f64,
        a0: f64,
        b0: f64,
        last: (f64, f64),
    }
    let mut done: Vec<(P, P)> = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    for (y, runs) in &runs_by_row {
        let mut next = Vec::new();
        for &(a, b) in runs {
            match open.iter().position(|o| a <= o.last.1 && b >= o.last.0) {
                Some(i) => {
                    let o = open.remove(i);
                    let (na, nb) = (o.a.max(a), o.b.min(b));
                    if na - o.a0 <= TOL && o.b0 - nb <= TOL {
                        next.push(Open {
                            a: na,
                            b: nb,
                            y1: *y,
                            last: (a, b),
                            ..o
                        });
                    } else {
                        done.push(((o.a, o.y0), (o.b, o.y1)));
                        // The band between the two rows fits their common span.
                        let (sa, sb) = (o.last.0.max(a), o.last.1.min(b));
                        next.push(Open {
                            a: sa,
                            b: sb,
                            y0: o.y1,
                            y1: *y,
                            a0: sa,
                            b0: sb,
                            last: (a, b),
                        });
                    }
                }
                None => next.push(Open {
                    a,
                    b,
                    y0: *y,
                    y1: *y,
                    a0: a,
                    b0: b,
                    last: (a, b),
                }),
            }
        }
        for o in open.drain(..) {
            done.push(((o.a, o.y0), (o.b, o.y1)));
        }
        open = next;
    }
    for o in open {
        done.push(((o.a, o.y0), (o.b, o.y1)));
    }
    done.retain(|(l, h)| h.0 >= l.0 && h.1 >= l.1);
    done
}

fn miss(r: &Rect, p: P, margin: f64) -> f64 {
    if margin <= 0.0 {
        return (r.centre.0 - p.0).hypot(r.centre.1 - p.1);
    }
    let hw = (r.size.0 / 2.0 - margin).max(0.0);
    let hh = (r.size.1 / 2.0 - margin).max(0.0);
    let dx = ((p.0 - r.centre.0).abs() - hw).max(0.0);
    let dy = ((p.1 - r.centre.1).abs() - hh).max(0.0);
    dx.hypot(dy)
}

/// How deep each edge zone must be for what is in it: each part's depth across
/// the band (its turn follows `faces`) plus its keep-out and a millimetre, with
/// the reason, keyed by side.
/// How far a snap hook's lip reaches in over the board's edge.
const HOOK_LIP: f64 = 2.5;

fn zone_needs(b: &Board, parts: &[&Part]) -> BTreeMap<String, (f64, String)> {
    let a = |s: &str| -> f64 {
        match s {
            "right" => 0.0,
            "top" => 90.0,
            "left" => 180.0,
            _ => 270.0,
        }
    };
    let mut out: BTreeMap<String, (f64, String)> = BTreeMap::new();
    for q in parts {
        let Some(zone) = q.zone.as_ref() else {
            continue;
        };
        let side = b.zones.get(zone).map(String::as_str).unwrap_or("centre");
        if side == "centre" {
            continue;
        }
        let turned = q.faces.as_deref().is_some_and(|f| {
            let rot = (a(side) - a(f)).rem_euclid(360.0);
            rot == 90.0 || rot == 270.0
        }) || q
            .rotate_deg
            .is_some_and(|r| (r.rem_euclid(180.0) - 90.0).abs() < 1e-6);
        let (w, h) = if turned {
            (q.body_mm[1], q.body_mm[0])
        } else {
            (q.body_mm[0], q.body_mm[1])
        };
        // A snap board's hooks reach over its left and right edges: a band
        // there is deeper by the lip, or its part fits only above or below it.
        let lip = if b.mount == "snap" && matches!(side, "left" | "right") {
            HOOK_LIP
        } else {
            0.0
        };
        let depth = if matches!(side, "top" | "bottom") {
            h
        } else {
            w
        } + q.keepout_mm
            + 1.0
            + b.edge_mm
            + lip;
        if out.get(side).is_none_or(|n| depth > n.0) {
            out.insert(
                side.into(),
                (
                    depth,
                    format!("`{}` needs {} mm of zone `{zone}`", q.id, r3(depth)),
                ),
            );
        }
    }
    out
}

fn d_zone_depth() -> f64 {
    0.4
}
fn d_slack() -> f64 {
    15.0
}
fn d_board_max() -> [f64; 2] {
    [100.0, 100.0]
}
fn d_board_t() -> f64 {
    1.6
}
fn d_layers() -> u32 {
    2
}
/// A vent's chimney: a printed tube from round its membrane down to the
/// board, sealed there by a soft ring round the part it is for.
const CHIMNEY_WALL: f64 = 1.2;
/// The ring: wider than the wall so the wall lands on it wherever the
/// board sits within tolerance, standing this tall before it is squeezed.
const CHIMNEY_GASKET_W: f64 = 1.6;
/// Room in the bore round the sealed part for the vias its signals drop
/// through: the ring lands on unbroken top copper, so they leave underneath.
const CHIMNEY_VIA_ROOM: f64 = 1.2;
const CHIMNEY_GASKET_H: f64 = 1.5;
const CHIMNEY_SQUEEZE: f64 = 0.3;
/// How far the part may sit from straight under its vent: the tube leans
/// to meet it — about 25° over a typical 6 mm drop, which prints unsupported.
const CHIMNEY_LEAN: f64 = 3.0;

/// The chimney's bore radius and its ring's outer radius: the bore clears
/// the membrane inside it and the part beneath it.
fn chimney_radii(vent: &Part, part: &Part) -> (f64, f64) {
    let membrane = if vent.membrane.as_deref() == Some("outside") {
        vent.pass_through_mm / 2.0 + 0.3
    } else {
        vent.body_mm[0].max(vent.body_mm[1]) / 2.0 + 0.3
    };
    let under = part.body_mm[0].hypot(part.body_mm[1]) / 2.0 + CHIMNEY_VIA_ROOM;
    let bore = membrane.max(under);
    (bore, bore + CHIMNEY_WALL / 2.0 + CHIMNEY_GASKET_W / 2.0)
}

fn d_placement_effort() -> f64 {
    10.0
}
fn d_fill() -> f64 {
    0.4
}
fn d_standoff() -> f64 {
    4.0
}
fn d_board_screw() -> String {
    "M2.5".into()
}
fn d_hole_inset() -> f64 {
    3.5
}
fn d_qty() -> u32 {
    10
}
fn d_shape() -> String {
    "box".into()
}
fn d_one() -> u32 {
    1
}
fn d_status() -> String {
    "decided".into()
}

/// Every mount the kernel knows how to build, and where it may be used.
///
/// | Mount | Place | Case feature |
/// |---|---|---|
/// | `window` | a region, `fit = "cover"` | window cut in the lid, gasket round it, locator ribs under the lid |
/// | `pocket` | a region, `fit = "inside"` | pocket in the top of the lid |
/// | `surface` | a region, `fit = "width"` | on top of the lid, bonded and sealed by an adhesive foam ring; its leads pass through a hole inside the ring |
/// | `socket` | `cavity` (beside the board) or `under-board` | four corner locators rising from the floor |
/// | `standoffs` | the board itself | posts from the floor, screws through the board |
/// | `smd`, `tht` | `board` | nothing — it rides on the board, and its height sets the cavity depth |
/// | `bulkhead` | `wall` | a hole through the wall and a flat inside for the nut |
/// | `vent` | a `case.mark` or `lid` | a hole through the lid, a membrane bonded over it inside |
/// | `adhesive` | `lid` or a mark | bonded to the lid's underside — a flex antenna |
pub const MOUNTS: &[(&str, &[&str])] = &[
    ("window", &["region"]),
    ("pocket", &["region"]),
    ("surface", &["region"]),
    ("socket", &["cavity", "under-board"]),
    ("smd", &["board"]),
    ("tht", &["board"]),
    ("pads", &["board"]),
    ("bulkhead", &["wall"]),
    ("vent", &["mark", "lid"]),
    ("adhesive", &["lid", "mark"]),
];

/// ISO metric screws, as a plastic case uses them: `(size, clearance hole,
/// pilot for self-tapping into plastic, head diameter, head height)`.
const SCREWS: &[(&str, f64, f64, f64, f64)] = &[
    ("M2", 2.4, 1.6, 3.8, 2.0),
    ("M2.5", 2.9, 2.1, 4.5, 2.5),
    ("M3", 3.4, 2.5, 5.5, 3.0),
    ("M4", 4.5, 3.3, 7.0, 4.0),
];

const SCREW_LENGTHS: &[f64] = &[
    4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 25.0, 30.0, 35.0, 40.0,
];

fn screw(size: &str) -> Result<(f64, f64, f64, f64)> {
    SCREWS
        .iter()
        .find(|s| s.0 == size)
        .map(|s| (s.1, s.2, s.3, s.4))
        .ok_or_else(|| {
            anyhow!(
                "unknown screw size `{size}` (known: {})",
                SCREWS.iter().map(|s| s.0).collect::<Vec<_>>().join(", ")
            )
        })
}

fn nominal(size: &str) -> f64 {
    size.trim_start_matches('M').parse().unwrap_or(3.0)
}

// ── SVG ──────────────────────────────────────────────────────────────────────

type Affine = [f64; 6];
const IDENTITY: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn compose(m: &Affine, n: &Affine) -> Affine {
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}

fn numbers(s: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, out: &mut Vec<f64>| {
        if !cur.is_empty() {
            if let Ok(v) = cur.parse() {
                out.push(v);
            }
            cur.clear();
        }
    };
    for c in s.chars() {
        match c {
            '-' | '+' if !cur.ends_with('e') && !cur.ends_with('E') => {
                flush(&mut cur, &mut out);
                cur.push(c);
            }
            '.' if cur.contains('.') => {
                flush(&mut cur, &mut out);
                cur.push(c);
            }
            '0'..='9' | '.' | 'e' | 'E' | '-' | '+' => cur.push(c),
            _ => flush(&mut cur, &mut out),
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn parse_transform(t: &str) -> Result<Affine> {
    let mut m = IDENTITY;
    for chunk in t.split(')') {
        let Some((name, args)) = chunk.split_once('(') else {
            continue;
        };
        let v = numbers(args);
        let n: Affine = match name.trim().trim_start_matches(',').trim() {
            "translate" => [1.0, 0.0, 0.0, 1.0, v[0], *v.get(1).unwrap_or(&0.0)],
            "scale" => [v[0], 0.0, 0.0, *v.get(1).unwrap_or(&v[0]), 0.0, 0.0],
            "matrix" if v.len() == 6 => [v[0], v[1], v[2], v[3], v[4], v[5]],
            other => bail!("SVG transform `{other}()` is not supported (translate, scale, matrix)"),
        };
        m = compose(&m, &n);
    }
    Ok(m)
}

fn attrs(tag: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let b = tag.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if let Some(eq) = tag[i..].find('=') {
            let key_end = i + eq;
            let key = tag[i..key_end]
                .rsplit(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("")
                .to_string();
            let rest = &tag[key_end + 1..];
            let q = rest.chars().next();
            if let Some(q @ ('"' | '\'')) = q {
                if let Some(end) = rest[1..].find(q) {
                    out.insert(key, rest[1..1 + end].to_string());
                    i = key_end + 1 + end + 2;
                    continue;
                }
            }
            i = key_end + 1;
        } else {
            break;
        }
    }
    out
}

struct SvgItem {
    layer: Option<String>,
    subpaths: Vec<Vec<P>>,
}

/// Straight segments only: a curve or arc fails and names the command, rather
/// than being flattened into a shape nobody drew.
fn path_subpaths(d: &str) -> Result<Vec<Vec<P>>> {
    let mut subs: Vec<Vec<P>> = Vec::new();
    let mut cur: Vec<P> = Vec::new();
    let (mut x, mut y) = (0.0, 0.0);
    let mut cmd = 'M';
    let chars: Vec<char> = d.chars().collect();
    let mut i = 0;
    let mut nums: Vec<f64> = Vec::new();
    let flush = |cmd: char,
                 nums: &mut Vec<f64>,
                 cur: &mut Vec<P>,
                 subs: &mut Vec<Vec<P>>,
                 x: &mut f64,
                 y: &mut f64|
     -> Result<()> {
        let rel = cmd.is_ascii_lowercase();
        let up = cmd.to_ascii_uppercase();
        let per = match up {
            'M' | 'L' => 2,
            'H' | 'V' => 1,
            'Z' => 0,
            other => bail!(
                "SVG path command `{other}` is not supported — outlines must be straight \
                 segments. Convert curves to a polyline in the drawing tool."
            ),
        };
        if per == 0 {
            if !cur.is_empty() {
                subs.push(std::mem::take(cur));
            }
            return Ok(());
        }
        if nums.len() % per != 0 {
            bail!("SVG path command `{cmd}` has {} numbers", nums.len());
        }
        for (k, c) in nums.chunks(per).enumerate() {
            match up {
                'M' | 'L' => {
                    let (nx, ny) = if rel {
                        (*x + c[0], *y + c[1])
                    } else {
                        (c[0], c[1])
                    };
                    if up == 'M' && k == 0 && !cur.is_empty() {
                        subs.push(std::mem::take(cur));
                    }
                    *x = nx;
                    *y = ny;
                }
                'H' => *x = if rel { *x + c[0] } else { c[0] },
                'V' => *y = if rel { *y + c[0] } else { c[0] },
                _ => unreachable!(),
            }
            cur.push((*x, *y));
        }
        nums.clear();
        Ok(())
    };
    let mut started = false;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            if started {
                flush(cmd, &mut nums, &mut cur, &mut subs, &mut x, &mut y)?;
                if cmd.eq_ignore_ascii_case(&'z') {
                    if let Some(first) = subs.last().and_then(|s| s.first()) {
                        x = first.0;
                        y = first.1;
                    }
                }
            }
            cmd = c;
            started = true;
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len()
            && !(chars[i].is_ascii_alphabetic() && chars[i] != 'e' && chars[i] != 'E')
        {
            i += 1;
        }
        let s: String = chars[start..i].iter().collect();
        nums.extend(numbers(&s));
    }
    if started {
        flush(cmd, &mut nums, &mut cur, &mut subs, &mut x, &mut y)?;
    }
    if !cur.is_empty() {
        subs.push(cur);
    }
    Ok(subs)
}

fn svg_items(src: &str) -> Result<(Vec<SvgItem>, f64, f64)> {
    let mut items = Vec::new();
    let mut stack: Vec<(Affine, Option<String>)> = vec![(IDENTITY, None)];
    let (mut vb_y, mut vb_h) = (0.0, 0.0);
    let mut rest = src;
    while let Some(lt) = rest.find('<') {
        rest = &rest[lt..];
        if rest.starts_with("<!--") {
            let end = rest
                .find("-->")
                .ok_or_else(|| anyhow!("unterminated SVG comment"))?;
            rest = &rest[end + 3..];
            continue;
        }
        let end = rest
            .find('>')
            .ok_or_else(|| anyhow!("unterminated SVG tag"))?;
        let tag = &rest[1..end];
        rest = &rest[end + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            if name.trim() == "g" && stack.len() > 1 {
                stack.pop();
            }
            continue;
        }
        let self_closing = tag.ends_with('/');
        let name = tag
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        let a = attrs(tag);
        let (parent_m, parent_layer) = stack.last().cloned().unwrap();
        let m = match a.get("transform") {
            Some(t) => compose(&parent_m, &parse_transform(t)?),
            None => parent_m,
        };
        let layer = a.get("data-layer").cloned().or(parent_layer);
        match name {
            "svg" => {
                let vb = numbers(
                    a.get("viewBox")
                        .ok_or_else(|| anyhow!("SVG has no viewBox"))?,
                );
                if vb.len() != 4 {
                    bail!("SVG viewBox must have four numbers");
                }
                vb_y = vb[1];
                vb_h = vb[3];
            }
            "g" if !self_closing => stack.push((m, layer)),
            "path" | "polygon" => {
                let subs = if name == "path" {
                    path_subpaths(a.get("d").ok_or_else(|| anyhow!("<path> without d"))?)?
                } else {
                    let v = numbers(
                        a.get("points")
                            .ok_or_else(|| anyhow!("<polygon> without points"))?,
                    );
                    vec![v.chunks(2).map(|c| (c[0], c[1])).collect()]
                };
                let subpaths = subs
                    .into_iter()
                    .map(|s| {
                        s.into_iter()
                            .map(|(x, y)| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]))
                            .collect()
                    })
                    .collect();
                items.push(SvgItem { layer, subpaths });
            }
            _ => {}
        }
    }
    Ok((items, vb_y, vb_h))
}

/// One addressed shape as a counter-clockwise polygon, y up, in SVG units.
fn load_shape(root: &Path, s: &Shape, what: &str) -> Result<Vec<P>> {
    let src = std::fs::read_to_string(root.join(&s.svg))
        .with_context(|| format!("{what}: reading {}", s.svg))?;
    let (items, vb_y, vb_h) =
        svg_items(&src).with_context(|| format!("{what}: parsing {}", s.svg))?;
    let pool: Vec<&SvgItem> = items
        .iter()
        .filter(|i| s.layer.is_none() || i.layer == s.layer)
        .collect();
    let scope = match &s.layer {
        Some(l) => format!("layer `{l}` of {}", s.svg),
        None => s.svg.clone(),
    };
    let item = pool.get(s.path).ok_or_else(|| {
        anyhow!(
            "{what}: {scope} has {} shape(s); path = {} does not exist",
            pool.len(),
            s.path
        )
    })?;
    let mut poly = item
        .subpaths
        .get(s.subpath)
        .ok_or_else(|| {
            anyhow!(
                "{what}: path {} of {scope} has {} subpath(s); subpath = {} does not exist",
                s.path,
                item.subpaths.len(),
                s.subpath
            )
        })?
        .clone();
    if poly.len() > 1 {
        let (a, b) = (poly[0], poly[poly.len() - 1]);
        if (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9 {
            poly.pop();
        }
    }
    if poly.len() < 3 {
        bail!("{what}: the addressed subpath has fewer than three points");
    }
    let mut poly: Vec<P> = poly
        .into_iter()
        .map(|(x, y)| (x, vb_y + vb_h - y))
        .collect();
    if fit::signed_area2(&poly) < 0.0 {
        poly.reverse();
    }
    Ok(poly)
}

// ── The solve ────────────────────────────────────────────────────────────────

fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0 + 0.0
}

fn pts(p: &[P]) -> Value {
    Value::Array(p.iter().map(|&(x, y)| json!([r3(x), r3(y)])).collect())
}

fn rect_json(r: &Rect) -> Value {
    json!({ "centre_mm": [r3(r.centre.0), r3(r.centre.1)], "size_mm": [r3(r.size.0), r3(r.size.1)] })
}

fn dist_rect_point(r: &Rect, p: P) -> f64 {
    let dx = ((p.0 - r.centre.0).abs() - r.size.0 / 2.0).max(0.0);
    let dy = ((p.1 - r.centre.1).abs() - r.size.1 / 2.0).max(0.0);
    dx.hypot(dy)
}

/// The solved product. `layout` is `layout.json`; the rest feed the other outputs.
pub struct Solved {
    pub layout: Value,
    pub bom: Vec<BomLine>,
    pub assembly: Vec<String>,
    pub board: Option<(Rect, Vec<P>, f64, f64)>,
    pub cost_violation: Option<String>,
    /// The placement model and the solver's answer to it, as committed.
    pub placement: Option<(String, String)>,
    /// The same for the case floor: sockets and the board in the cavity.
    pub floor: Option<(String, String)>,
}

pub struct BomLine {
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub mpn: String,
    pub second_source: String,
    pub lcsc: String,
    pub qty: u32,
    pub unit_cost: Option<f64>,
    pub source: &'static str,
    pub status: String,
    pub settle_by: String,
    pub footprint: String,
    pub dims_from: String,
    pub source_url: String,
    pub datasheet: String,
}

/// A light inside a closed case is seen through a translucent case or a
/// window, or not at all (the paper's round 5, case M19: the status LED,
/// the stick's only output without a computer, behind matte black).
///
/// A part is a light when its device profile says so or its symbol is an
/// LED; `through_wall` gives it an opening. A case is translucent when its
/// colour says so — `#rrggbbaa`, alpha under `f0` — which is also how the
/// renders draw it, so the picture and the check read the same declaration.
fn hidden_lights(p: &Product) -> Result<()> {
    let alpha = p
        .case
        .colour
        .as_deref()
        .and_then(|c| c.strip_prefix('#'))
        .filter(|h| h.len() == 8)
        .and_then(|h| u8::from_str_radix(&h[6..], 16).ok())
        .unwrap_or(0xFF);
    if alpha < 0xF0 {
        return Ok(());
    }
    let mut hidden = Vec::new();
    for q in p
        .parts
        .iter()
        .filter(|q| q.place == "board" && !q.through_wall)
    {
        let profiled = q
            .mpn
            .as_deref()
            .and_then(|m| crate::devices::for_mpn(m).ok().flatten())
            .is_some_and(|d| d.light);
        let led_symbol = q.symbol.as_deref().is_some_and(|s| {
            let name = s.rsplit(':').next().unwrap_or(s).to_ascii_uppercase();
            name == "LED" || name.starts_with("LED_")
        });
        if profiled || led_symbol {
            hidden.push(format!("`{}`", q.id));
        }
    }
    if !hidden.is_empty() {
        bail!(
            "{} {} inside the case, and the case is opaque (case.colour = {:?}) with no \
             window over {it} — no one will see {it}.\n  Make the filament translucent \
             (`#rrggbbaa`, e.g. \"#1d4ed8b3\"), or give the light an opening (`through_wall = true`).",
            hidden.join(", "),
            if hidden.len() == 1 { "is a light" } else { "are lights" },
            p.case.colour.as_deref().unwrap_or("unset: neutral grey"),
            it = if hidden.len() == 1 { "it" } else { "them" },
        );
    }
    Ok(())
}

pub fn solve(root: &Path, p: &Product) -> Result<Solved> {
    let case = &p.case;
    if let Some(c) = &case.colour {
        let hex = c.strip_prefix('#').unwrap_or("");
        if !matches!(hex.len(), 6 | 8) || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            bail!(
                "case.colour = \"{c}\" — a colour is `#rrggbb`, e.g. \"#2563eb\", or \
                 `#rrggbbaa` for a translucent filament, e.g. \"#2563ebb3\""
            );
        }
    }
    hidden_lights(p)?;
    let clr = case.clearance_mm;
    let seal = &case.seal;
    let mut why: Vec<String> = Vec::new();

    for (i, a) in p.parts.iter().enumerate() {
        if let Some(m) = &a.model {
            if !root.join(m).is_file() {
                bail!(
                    "part `{}`: model = \"{m}\" is not a file in this repository",
                    a.id
                );
            }
        }
        if p.parts[..i].iter().any(|b| b.id == a.id) {
            bail!("two parts share the id `{}`", a.id);
        }
    }

    // 1. Shapes, in SVG units.
    let outline_u = load_shape(root, &p.outline, "outline")?;
    let mut regions_u: BTreeMap<&str, Vec<P>> = BTreeMap::new();
    for (name, s) in &p.regions {
        regions_u.insert(name, load_shape(root, s, &format!("region `{name}`"))?);
    }
    let mut shapes = json!({ "outline": fingerprint(&p.outline, &outline_u) });
    for (name, s) in &p.regions {
        shapes[format!("region:{name}")] = fingerprint(s, &regions_u[name.as_str()]);
    }
    let (olo, ohi) = fit::bounds(&outline_u);
    let ow = ohi.0 - olo.0;

    for part in &p.parts {
        let known = MOUNTS.iter().find(|m| m.0 == part.mount).ok_or_else(|| {
            anyhow!(
                "part `{}`: mount `{}` is not one of {}",
                part.id,
                part.mount,
                MOUNTS.iter().map(|m| m.0).collect::<Vec<_>>().join(", ")
            )
        })?;
        let place_kind = if regions_u.contains_key(part.place.as_str()) {
            "region"
        } else if p.case.marks.iter().any(|m| m.id == part.place) {
            "mark"
        } else {
            part.place.as_str()
        };
        if !known.1.contains(&place_kind) {
            bail!(
                "part `{}`: mount `{}` belongs in {}, not place = `{}`",
                part.id,
                part.mount,
                known.1.join(" or "),
                part.place
            );
        }
        if let Some(t) = &part.seals_to {
            if part.mount != "vent" {
                bail!(
                    "part `{}`: seals_to is for a `vent` — the tube runs from its membrane",
                    part.id
                );
            }
            if !p.parts.iter().any(|o| &o.id == t && o.place == "board") {
                bail!(
                    "part `{}`: seals_to = `{t}`, which is not a board part",
                    part.id
                );
            }
        }
        match (part.membrane.as_deref(), part.mount.as_str()) {
            (None, _) | (Some("inside" | "outside"), "vent") => {}
            (Some(m), "vent") => bail!(
                "part `{}`: membrane = `{m}` — `inside` or `outside`",
                part.id
            ),
            (Some(_), _) => bail!("part `{}`: membrane is for a `vent`", part.id),
        }
        if part.body_mm.iter().any(|&v| v.is_nan() || v <= 0.0) {
            bail!(
                "part `{}`: every body_mm dimension must be positive",
                part.id
            );
        }
    }

    // 2. Scale. `window` parts cap it (the window must stay covered), `pocket`
    //    parts floor it (the pocket must hold the part).
    let mut k_max = f64::INFINITY;
    let mut k_min = 0.0_f64;
    let mut k_exact: Option<f64> = None;
    let mut set_by: Vec<Value> = Vec::new();
    for part in p
        .parts
        .iter()
        .filter(|q| regions_u.contains_key(q.place.as_str()))
    {
        let reg = &regions_u[part.place.as_str()];
        let (lo, hi) = fit::bounds(reg);
        let (rw, rh) = (hi.0 - lo.0, hi.1 - lo.1);
        match part_fit(part)? {
            axis @ ("width" | "height") => {
                let k = if axis == "width" {
                    part.body_mm[0] / rw
                } else {
                    part.body_mm[1] / rh
                };
                why.push(format!(
                    "region `{}` is scaled so its {axis} equals `{}`'s {} mm ({} mm/unit)",
                    part.place,
                    part.id,
                    if axis == "width" {
                        part.body_mm[0]
                    } else {
                        part.body_mm[1]
                    },
                    r3(k)
                ));
                set_by.push(json!({ "part": part.id, "region": part.place, "rule": axis, "mm_per_unit": r3(k) }));
                k_exact = Some(match k_exact {
                    Some(prev) if (prev - k).abs() > 1e-9 => bail!(
                        "two parts each fix the scale exactly, and disagree: {} vs {} mm/unit",
                        r3(prev),
                        r3(k)
                    ),
                    _ => k,
                });
            }
            "cover" => {
                let o = seal.window_overlap_mm;
                let (kw, kh) = (
                    (part.body_mm[0] - 2.0 * o) / rw,
                    (part.body_mm[1] - 2.0 * o) / rh,
                );
                if kw <= 0.0 || kh <= 0.0 {
                    bail!(
                        "part `{}` is smaller than twice seal.window_overlap_mm",
                        part.id
                    );
                }
                let (k, axis) = if kw <= kh {
                    (kw, "width")
                } else {
                    (kh, "height")
                };
                why.push(format!(
                    "`{}` covers region `{}` as its window: the region grows until its {axis} meets the part less {o} mm of gasket overlap each side ({} mm/unit)",
                    part.id, part.place, r3(k)
                ));
                set_by.push(json!({ "part": part.id, "region": part.place, "rule": "cover", "binding": axis, "mm_per_unit": r3(k) }));
                k_max = k_max.min(k);
            }
            "inside" => {
                let fits = |k: f64| {
                    let reg_mm: Vec<P> = reg.iter().map(|&(x, y)| (x * k, y * k)).collect();
                    let c = fit::centroid(&reg_mm);
                    fit::place_rect(
                        &reg_mm,
                        (part.body_mm[0], part.body_mm[1]),
                        clr,
                        &[],
                        0.0,
                        c,
                        case.grid_mm,
                    )
                    .is_some()
                };
                let (mut lo_k, mut hi_k) = (0.0, 1.0);
                while !fits(hi_k) {
                    hi_k *= 2.0;
                    if hi_k > 1e6 {
                        bail!(
                            "part `{}` fits region `{}` at no scale",
                            part.id,
                            part.place
                        );
                    }
                }
                for _ in 0..60 {
                    let mid = (lo_k + hi_k) / 2.0;
                    if fits(mid) {
                        hi_k = mid
                    } else {
                        lo_k = mid
                    }
                }
                why.push(format!(
                    "`{}` must fit inside region `{}`: at least {} mm/unit",
                    part.id,
                    part.place,
                    r3(hi_k)
                ));
                set_by.push(json!({ "part": part.id, "region": part.place, "rule": "inside", "mm_per_unit": r3(hi_k) }));
                k_min = k_min.max(hi_k);
            }
            other => unreachable!("part_fit returned {other}"),
        }
    }
    let width = match p.outline.width_mm {
        Some(w) => {
            why.push(format!("outline width declared: {w} mm"));
            w
        }
        // An exact rule is not rounded: rounding is what it exists to prevent.
        None if k_exact.is_some() => k_exact.unwrap() * ow,
        None if k_max.is_finite() => (k_max * ow / case.round_mm).floor() * case.round_mm,
        None if k_min > 0.0 => (k_min * ow / case.round_mm).ceil() * case.round_mm,
        None => bail!("no outline.width_mm and no part in a region, so nothing sets the size"),
    };
    let k = width / ow;
    if k > k_max + 1e-9 || k < k_min - 1e-9 {
        bail!(
            "the parts disagree about the size: {} mm/unit is chosen, but pockets need at least {} and windows at most {}",
            r3(k),
            r3(k_min),
            r3(k_max)
        );
    }
    let to_mm = |poly: &[P]| -> Vec<P> {
        poly.iter()
            .map(|&(x, y)| ((x - olo.0) * k, (y - olo.1) * k))
            .collect()
    };
    let outline = to_mm(&outline_u);
    let (_, ohi_mm) = fit::bounds(&outline);
    why.push(format!("outline is {} × {} mm", r3(ohi_mm.0), r3(ohi_mm.1)));

    // 3. The wall, the seal, and the screws that close it.
    let (s_clear, s_pilot, s_head, s_head_h) = screw(&case.fasteners.size)?;
    let seal_on = seal.kind == "gasket";
    if !matches!(seal.kind.as_str(), "gasket" | "none") {
        bail!("case.seal.kind must be `gasket` or `none`");
    }
    let seal_band = if seal_on {
        2.0 * seal.lip_mm + seal.groove_mm
    } else {
        0.0
    };
    // The gasket is taller than its groove, so it stands proud of the rim and
    // the closing lid squeezes it. Compressed, it is (1 - c) of its free
    // height; the tongue is whatever reaches down to exactly that.
    if seal_on && (seal.compression <= 0.0 || seal.compression >= 0.6 || seal.compression.is_nan())
    {
        bail!("case.seal.compression = {} — a gasket is squeezed by more than 0 and less than 0.6 of its height", seal.compression);
    }
    let (gasket_h, tongue) = match seal.tongue_mm {
        None => {
            let h = seal.depth_mm + seal.proud_mm;
            (h, seal.depth_mm - h * (1.0 - seal.compression))
        }
        Some(t) => ((seal.depth_mm - t) / (1.0 - seal.compression), t),
    };
    if seal_on && gasket_h < seal.depth_mm - 1e-9 {
        bail!(
            "the gasket would be {} mm tall in a {} mm groove — below the rim, it seals nothing until something reaches \
             down to it. It must be at least as tall as its groove: shorten tongue_mm, or drop it and let proud_mm set it.",
            r3(gasket_h),
            seal.depth_mm
        );
    }
    if seal_on && tongue < -1e-9 {
        bail!(
            "a gasket standing {} mm proud, compressed {}, would need the lid to stop {} mm above the rim — it cannot. \
             Lower proud_mm or raise compression.",
            seal.proud_mm,
            seal.compression,
            r3(-tongue)
        );
    }
    let tongue = tongue.max(0.0);
    if seal_on {
        why.push(format!(
            "lid gasket {} mm tall in a {} mm groove, standing {} mm proud; {} it {}% to {} mm",
            r3(gasket_h),
            seal.depth_mm,
            r3(gasket_h - seal.depth_mm),
            if tongue > 1e-9 {
                format!("the lid and a {} mm tongue squeeze", r3(tongue))
            } else {
                "the lid squeezes".to_string()
            },
            r3(seal.compression * 100.0),
            r3(gasket_h * (1.0 - seal.compression))
        ));
    }
    if seal_on && case.wall_mm < seal_band - 1e-9 {
        bail!(
            "wall_mm = {} cannot hold the seal: it needs lip + groove + lip = {} mm",
            case.wall_mm,
            seal_band
        );
    }
    // Screw axis sits this far in from the outer skin; the boss round it is big
    // enough that the gasket groove passes between the screw and the cavity.
    // Far enough in that the head's counterbore keeps 1 mm of skin: nearer
    // than that and it breaks out of the side of the lid, which no fit test
    // sees because it is a cut, not a body.
    let screw_inset = (s_clear / 2.0 + seal.lip_mm.max(1.2)).max(s_head / 2.0 + 0.4 + 1.0);
    let boss_r = s_pilot / 2.0 + seal_band.max(1.2) + 0.2;
    // Hung by holes through its tip: the tip is solid from `plug_y` up, outside
    // the seal, and no screw goes there.
    let ymax_outline = outline.iter().map(|v| v.1).fold(f64::MIN, f64::max);
    let plug_y = case
        .hang
        .as_ref()
        .filter(|h| h.kind == "holes")
        .map(|h| ymax_outline - h.solid_mm);
    let candidates: Vec<P> = fit::offset_vertices(&outline, screw_inset)
        .into_iter()
        .zip(fit::interior_angles(&outline))
        .filter_map(|(v, a)| v.filter(|_| a <= case.fasteners.max_corner_deg))
        .filter(|v| plug_y.is_none_or(|py| v.1 < py - boss_r - 1.0))
        .collect();
    let per = fit::perimeter(&outline);
    let closure = case.fasteners.closure.as_str();
    if !matches!(closure, "screws-top" | "screws-back" | "press-fit") {
        bail!("case.fasteners.closure = `{closure}` must be screws-top, screws-back or press-fit");
    }
    let screwed = closure != "press-fit";
    let want = if screwed {
        case.fasteners
            .count
            .unwrap_or_else(|| ((per / case.fasteners.max_spacing_mm).ceil() as usize).max(3))
    } else {
        0
    };
    if screwed && want < 3 {
        bail!("case.fasteners.count = {want}: three screws is the fewest that hold a lid flat");
    }
    if screwed && candidates.len() < want.min(3) {
        bail!(
            "only {} corner(s) of the outline can hold a screw; a lid needs at least 3",
            candidates.len()
        );
    }
    let chosen = fit::spread(&candidates, want);
    let screws: Vec<P> = chosen.iter().map(|&i| candidates[i]).collect();
    if screwed {
        why.push(format!(
            "{} × {} lid screws {} at corners, spread as far apart as the outline allows{}",
            screws.len(),
            case.fasteners.size,
            if closure == "screws-back" {
                "driven up from underneath"
            } else {
                "driven down through the lid"
            },
            if screws.len() < want {
                " — fewer than wanted: not enough usable corners"
            } else {
                ""
            }
        ));
    } else {
        why.push(format!(
            "press-fit lid: a {} mm skirt, {} mm oversize, grips the inside of the base. No screws — \
             retention is friction alone{}; prove it on a print",
            case.fasteners.skirt_mm,
            case.fasteners.interference_mm,
            if seal_on { ", so the gasket's compression must be held by the fit" } else { "" }
        ));
    }
    let mut boss_keepouts: Vec<Rect> = screws
        .iter()
        .map(|&c| Rect {
            centre: c,
            size: (2.0 * boss_r, 2.0 * boss_r),
        })
        .collect();
    if let Some(py) = plug_y {
        // The solid tip is not cavity: nothing is placed in it.
        let (lo, hi) = fit::bounds(&outline);
        boss_keepouts.push(Rect {
            centre: ((lo.0 + hi.0) / 2.0, (py + hi.1) / 2.0 + 0.5),
            size: (hi.0 - lo.0 + 2.0, hi.1 - py + 1.0),
        });
    }
    let inner = case.wall_mm + clr;

    // 4. Region parts: centred on their region, and they must clear the walls.
    let mut region_parts: Vec<Value> = Vec::new();
    let mut window_stack: f64 = 0.0;
    for part in p
        .parts
        .iter()
        .filter(|q| regions_u.contains_key(q.place.as_str()))
    {
        let reg = to_mm(&regions_u[part.place.as_str()]);
        let (lo, hi) = fit::bounds(&reg);
        let centre = ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0);
        let body = Rect {
            centre,
            size: (part.body_mm[0], part.body_mm[1]),
        };
        if part.mount == "window" {
            let m = fit::rect_margin(&outline, &body);
            if m < inner - 1e-9 {
                bail!(
                    "part `{}` ({} × {} mm) centred behind its window reaches {} mm from the outline; it needs {} (wall + clearance). \
                     Use a narrower part, or a region further from the edge.",
                    part.id, part.body_mm[0], part.body_mm[1], r3(m), r3(inner)
                );
            }
            for (i, s) in screws.iter().enumerate() {
                if dist_rect_point(&body, *s) < boss_r + clr {
                    bail!(
                        "part `{}` behind its window collides with lid screw boss {i}",
                        part.id
                    );
                }
            }
            window_stack = window_stack
                .max(part.body_mm[2] + if seal_on { seal.window_gasket_mm } else { 0.0 });
        }
        if part.mount == "surface" && part.flush {
            let depth = part.body_mm[2] + seal.adhesive_mm;
            if depth + 1.5 > case.lid_mm {
                bail!(
                    "part `{}`: sunk flush it needs a {} mm pocket, which leaves under 1.5 mm of a {} mm lid — a thicker lid",
                    part.id,
                    r3(depth),
                    case.lid_mm
                );
            }
            why.push(format!(
                "`{}` sits in a {} mm pocket, its face level with the lid's",
                part.id,
                r3(depth)
            ));
        }
        if part.mount == "surface" {
            // On top of the lid: it must not overhang the edge, must not sit on
            // a screw head, and its leads must go down into the cavity, not
            // into the wall.
            let m = fit::rect_margin(&outline, &body);
            if m < 1.0 - 1e-9 {
                bail!(
                    "part `{}` ({} × {} mm) on the lid comes {} mm from its edge; it needs 1 mm. \
                     A smaller part, or a region further from the edge.",
                    part.id,
                    part.body_mm[0],
                    part.body_mm[1],
                    r3(m)
                );
            }
            if closure == "screws-top" {
                for (i, s) in screws.iter().enumerate() {
                    if dist_rect_point(&body, *s) < s_head / 2.0 + 1.0 {
                        bail!("part `{}` on the lid covers the head of lid screw {i}; use closure = \"screws-back\"", part.id);
                    }
                }
            }
            let inner_ring = part.body_mm[0].min(part.body_mm[1]) - 2.0 * seal.adhesive_width_mm;
            if part.pass_through_mm > inner_ring - 2.0 {
                bail!(
                    "part `{}`: a {} mm lead hole does not fit inside its {} mm-wide adhesive ring",
                    part.id,
                    part.pass_through_mm,
                    seal.adhesive_width_mm
                );
            }
            if fit::dist_to_boundary(&outline, centre) < inner + part.pass_through_mm / 2.0 {
                bail!(
                    "part `{}`: its lead hole would open into the wall, not the cavity",
                    part.id
                );
            }
        }
        region_parts.push(json!({
            "part": part.id,
            "mount": part.mount,
            "region": part.place,
            "region_mm": pts(&reg),
            "body": rect_json(&body),
            "thickness_mm": part.body_mm[2],
            "overlap_mm": if part.mount == "window" { seal.window_overlap_mm } else { 0.0 },
            "adhesive_mm": if part.mount == "surface" { seal.adhesive_mm } else { 0.0 },
            "adhesive_width_mm": if part.mount == "surface" { seal.adhesive_width_mm } else { 0.0 },
            "pass_through_mm": if part.mount == "surface" { part.pass_through_mm } else { 0.0 },
            "flush": part.flush,
        }));
    }

    // 4b. Marks on the lid: a piece of a region, set against a lid part.
    let mut marks_json: Vec<Value> = Vec::new();
    let mut mark_polys: BTreeMap<String, Vec<P>> = BTreeMap::new();
    for m in &case.marks {
        let reg =
            to_mm(regions_u.get(m.region.as_str()).ok_or_else(|| {
                anyhow!("mark `{}`: region `{}` is not declared", m.id, m.region)
            })?);
        if m.piece != "roof" {
            bail!(
                "mark `{}`: piece `{}` is not known — `roof` is the part of the region above its full-width span",
                m.id,
                m.piece
            );
        }
        if !matches!(m.kind.as_str(), "deboss" | "emboss" | "etch") {
            bail!(
                "mark `{}`: kind `{}` must be deboss, emboss or etch",
                m.id,
                m.kind
            );
        }
        if m.depth_mm <= 0.0 || m.depth_mm > case.lid_mm / 2.0 {
            bail!(
                "mark `{}`: depth {} mm must be above 0 and at most half the {} mm lid",
                m.id,
                m.depth_mm,
                case.lid_mm
            );
        }
        let (lo, hi) = fit::bounds(&reg);
        let eps = 1e-6;
        // The shoulder: the highest point at which the region is still full width.
        let shoulder = reg
            .iter()
            .filter(|v| (v.0 - lo.0).abs() < eps || (v.0 - hi.0).abs() < eps)
            .map(|v| v.1)
            .fold(f64::MIN, f64::max);
        let roof: Vec<P> = reg
            .iter()
            .filter(|v| v.1 >= shoulder - eps)
            .copied()
            .collect();
        if roof.len() < 3 {
            bail!(
                "mark `{}`: region `{}` has nothing above its full-width span",
                m.id,
                m.region
            );
        }
        let on = region_parts
            .iter()
            .find(|r| r["part"] == m.on.as_str())
            .ok_or_else(|| anyhow!("mark `{}`: on = `{}` is not a part on the lid", m.id, m.on))?;
        let (oc, os) = (&on["body"]["centre_mm"], &on["body"]["size_mm"]);
        let (cx, cy, h) = (
            oc[0].as_f64().unwrap(),
            oc[1].as_f64().unwrap(),
            os[1].as_f64().unwrap(),
        );
        let (dx, dy) = (cx - (lo.0 + hi.0) / 2.0, cy + h / 2.0 - shoulder);
        let poly: Vec<P> = roof.iter().map(|v| (v.0 + dx, v.1 + dy)).collect();
        for v in &poly {
            if !fit::contains(&outline, *v) || fit::dist_to_boundary(&outline, *v) < 1.0 {
                bail!(
                    "mark `{}` reaches the lid's edge at ({}, {}); it needs 1 mm of lid round it",
                    m.id,
                    r3(v.0),
                    r3(v.1)
                );
            }
        }
        why.push(format!(
            "mark `{}`: region `{}`'s roof, seated on `{}`'s top edge — {} {} mm into the lid",
            m.id, m.region, m.on, m.kind, m.depth_mm
        ));
        marks_json.push(json!({
            "id": m.id, "kind": m.kind, "depth_mm": m.depth_mm, "line_mm": m.line_mm,
            "polygon_mm": pts(&poly),
        }));
        mark_polys.insert(m.id.clone(), poly);
    }
    // The outline's highest point: where an antenna reaches up, where it hangs.
    let apex = {
        let ymax = outline.iter().map(|v| v.1).fold(f64::MIN, f64::max);
        let top: Vec<&P> = outline.iter().filter(|v| v.1 > ymax - 1e-6).collect();
        (
            top.iter().map(|v| v.0).sum::<f64>() / top.len() as f64,
            ymax,
        )
    };

    // 4c. Parts on the lid's underside: a vent in a mark, an antenna up in
    // the apex. Each goes nearest its `near`, clear of the lid's other parts
    // and the screw bosses.
    let lid_parts: Vec<&Part> = p
        .parts
        .iter()
        .filter(|q| matches!(q.mount.as_str(), "vent" | "adhesive"))
        .collect();
    let mut lid_taken: Vec<Rect> = boss_keepouts.clone();
    for rp in &region_parts {
        let (c, s) = (&rp["body"]["centre_mm"], &rp["body"]["size_mm"]);
        lid_taken.push(Rect {
            centre: (c[0].as_f64().unwrap(), c[1].as_f64().unwrap()),
            size: (s[0].as_f64().unwrap() + 1.0, s[1].as_f64().unwrap() + 1.0),
        });
    }
    for part in lid_parts {
        let (area, clearance) = match mark_polys.get(&part.place) {
            Some(poly) => (poly.clone(), 1.0),
            // On the underside: inside the cavity, clear of the wall.
            None => (outline.clone(), inner),
        };
        let target = match part.near.as_deref() {
            Some("apex") => apex,
            Some(id) => match region_parts.iter().find(|r| r["part"] == id) {
                Some(r) => (
                    r["body"]["centre_mm"][0].as_f64().unwrap(),
                    r["body"]["centre_mm"][1].as_f64().unwrap(),
                ),
                None => bail!(
                    "part `{}`: near = `{id}` — a lid part sits near `apex` or another lid part",
                    part.id
                ),
            },
            None => fit::centroid(&area),
        };
        let vent = part.mount == "vent";
        // A vent sealed to a board part stands over its chimney's ring, and
        // that ring must land on the board, inside the cavity, clear of the
        // screw bosses: the vent's own body goes in its area (a mark, or the
        // underside), at the nearest spot whose ring fits below. Sizing the
        // vent by its ring instead held a vent in a small mark far from where
        // the ring had room.
        let sealed_ring = part
            .seals_to
            .as_deref()
            .and_then(|t| p.parts.iter().find(|q| q.id == t))
            .map(|q| 2.0 * (chimney_radii(part, q).1 + p.board.edge_mm + case.clearance_mm));
        let ring_fits = |c: P| {
            sealed_ring.is_none_or(|d| {
                let r = Rect {
                    centre: c,
                    size: (d, d),
                };
                fit::rect_fits(&outline, &r, inner)
                    && boss_keepouts.iter().all(|k| !k.overlaps(&r, 1.0))
            })
        };
        let sizes = if vent || part.body_mm[0] == part.body_mm[1] {
            vec![(part.body_mm[0], part.body_mm[1])]
        } else {
            vec![
                (part.body_mm[0], part.body_mm[1]),
                (part.body_mm[1], part.body_mm[0]),
            ]
        };
        let best = sizes
            .iter()
            .filter_map(|sz| {
                fit::place_rect_where(&area, *sz, clearance, &lid_taken, 1.0, target, case.grid_mm / 2.0, |r| ring_fits(r.centre))
            })
            .min_by(|a, b| {
                let d = |r: &Rect| (r.centre.0 - target.0).hypot(r.centre.1 - target.1);
                d(a).partial_cmp(&d(b)).unwrap()
            })
            .ok_or_else(|| {
                anyhow!(
                    "part `{}` ({} × {} mm) does not fit {} clear of the lid's other parts and screw bosses",
                    part.id,
                    part.body_mm[0],
                    part.body_mm[1],
                    if mark_polys.contains_key(&part.place) { format!("inside mark `{}`", part.place) } else { "on the lid's underside".into() }
                )
            })?;
        lid_taken.push(Rect {
            centre: best.centre,
            size: (best.size.0 + 1.0, best.size.1 + 1.0),
        });
        why.push(format!(
            "`{}` on the lid's underside at ({}, {}){}",
            part.id,
            r3(best.centre.0),
            r3(best.centre.1),
            if vent {
                format!(", over a {} mm hole through the lid", part.pass_through_mm)
            } else {
                String::new()
            }
        ));
        region_parts.push(json!({
            "part": part.id,
            "mount": part.mount,
            "region": part.place,
            "body": rect_json(&best),
            "thickness_mm": part.body_mm[2],
            "pass_through_mm": if vent { part.pass_through_mm } else { 0.0 },
            "membrane": if vent { part.membrane.clone().unwrap_or_else(|| "inside".into()) } else { String::new() },
            "underside": true,
        }));
    }
    let hang_json = match &case.hang {
        None => Value::Null,
        Some(hg) => {
            if hg.at != "top" {
                bail!(
                    "case.hang.at = `{}`: only `top` (the outline's highest point) is known",
                    hg.at
                );
            }
            if hg.kind == "holes" {
                let py = plug_y.unwrap();
                // Front to back, a third of the way down the solid tip; side to
                // side below it. Each needs 2 mm of plastic round it.
                // Centred in the tip's width at that height: an outline's tip is
                // rarely symmetric about its highest point.
                let span_at = |y: f64| {
                    let (mut l, mut r) = (apex.0, apex.0);
                    while fit::contains(&outline, (l - 0.1, y)) {
                        l -= 0.1;
                    }
                    while fit::contains(&outline, (r + 0.1, y)) {
                        r += 0.1;
                    }
                    (l, r)
                };
                let zy = apex.1 - hg.solid_mm * 0.4;
                let (zl, zr) = span_at(zy);
                let z_at = ((zl + zr) / 2.0, zy);
                if !fit::contains(&outline, z_at)
                    || fit::dist_to_boundary(&outline, z_at) < hg.hole_mm / 2.0 + 2.0
                {
                    bail!(
                        "case.hang: a {} mm hole at the tip leaves under 2 mm of plastic — a smaller hole, or a longer solid_mm",
                        hg.hole_mm
                    );
                }
                let x_y = apex.1 - hg.solid_mm * 0.75;
                if (z_at.1 - x_y) < (hg.hole_mm + hg.cross_mm) / 2.0 + 2.0 {
                    bail!("case.hang: the two holes run into each other — a longer solid_mm");
                }
                // The side-to-side hole runs across the tip at that height.
                let (mut xl, mut xr) = (apex.0, apex.0);
                while fit::contains(&outline, (xl - 0.1, x_y)) {
                    xl -= 0.1;
                }
                while fit::contains(&outline, (xr + 0.1, x_y)) {
                    xr += 0.1;
                }
                if x_y - hg.cross_mm / 2.0 - 2.0 < py {
                    bail!("case.hang: the side-to-side hole reaches below the solid tip — a longer solid_mm");
                }
                why.push(format!(
                    "the tip is solid for {} mm, outside the seal: a {} mm hole through it front to back, a {} mm hole across it — hang it on a nail, a screw or a cable tie",
                    hg.solid_mm, hg.hole_mm, hg.cross_mm
                ));
                json!({
                    "kind": "holes",
                    "plug_y_mm": r3(py),
                    "through": { "at_mm": [r3(z_at.0), r3(z_at.1)], "d_mm": hg.hole_mm },
                    "across": { "y_mm": r3(x_y), "from_x_mm": r3(xl - 1.0), "to_x_mm": r3(xr + 1.0), "d_mm": hg.cross_mm },
                })
            } else {
                if hg.hole_mm + 3.0 > hg.width_mm || hg.hole_mm + 3.0 > hg.reach_mm {
                    bail!(
                    "case.hang: a {} mm hole needs 1.5 mm of material round it — width and reach at least {} mm",
                    hg.hole_mm,
                    hg.hole_mm + 3.0
                );
                }
                why.push(format!(
                "a {} mm-thick lug stands {} mm out of the top of the outline, outside the seal, with a {} mm hole to hang it by",
                hg.thickness_mm, hg.reach_mm, hg.hole_mm
            ));
                json!({
                    "kind": "lug",
                    "at_mm": [r3(apex.0), r3(apex.1)],
                    "hole_mm": hg.hole_mm, "width_mm": hg.width_mm, "reach_mm": hg.reach_mm, "thickness_mm": hg.thickness_mm,
                    "hole_at_mm": [r3(apex.0), r3(apex.1 + hg.reach_mm - hg.width_mm / 2.0)],
                })
            }
        }
    };

    // 5. Everything that stands on the floor — sockets and the board — packed
    //    together. Greedy placement at the centroid fails exactly when it
    //    matters (one socket in the middle leaves no room either side of it),
    //    so each item tries the centroid, then spread anchors, and a later
    //    item that fits nowhere sends the search back to move an earlier one.
    let locator = 2.0; // locator rib thickness round a socket
    let mut sockets: Vec<&Part> = p.parts.iter().filter(|q| q.place == "cavity").collect();
    sockets.sort_by(|a, b| {
        (b.body_mm[0] * b.body_mm[1])
            .partial_cmp(&(a.body_mm[0] * a.body_mm[1]))
            .unwrap()
            .then(a.id.cmp(&b.id))
    });
    let board_parts: Vec<&Part> = p.parts.iter().filter(|q| q.place == "board").collect();
    let b = &p.board;
    let (b_clear, _, _, _) = screw(&b.screw)?;
    let has_board = !board_parts.is_empty() || b.size_mm.is_some();
    // One socket per unit: `qty = 2` supercaps are two sockets. Its leads
    // stick out of one end, so the socket's length holds them too.
    let socket_size = |q: &Part| {
        (
            q.body_mm[0] + q.lead_mm + 2.0 * (clr + locator),
            q.body_mm[1] + 2.0 * (clr + locator),
        )
    };
    // The units of one socketed part go together: `qty = 2` supercaps lie
    // side by side, parallel, as one block — the shortest link between them,
    // and one decision for the packer instead of two that can disagree.
    let block_size = |q: &Part| {
        let (w, h) = socket_size(q);
        (w, h * q.qty as f64 + clr * (q.qty.saturating_sub(1)) as f64)
    };
    // `near`: a side of the outline, or a region part's centre.
    let (olo2, ohi2) = fit::bounds(&outline);
    let near_point = |s: &str| -> Result<P> {
        let mid = ((olo2.0 + ohi2.0) / 2.0, (olo2.1 + ohi2.1) / 2.0);
        Ok(match s {
            "bottom" => (mid.0, olo2.1),
            "apex" => apex,
            "top" => (mid.0, ohi2.1),
            "left" => (olo2.0, mid.1),
            "right" => (ohi2.0, mid.1),
            id => {
                let rp = region_parts.iter().find(|r| r["part"] == id).ok_or_else(|| {
                    anyhow!("near = `{id}` names neither a side (bottom, top, left, right) nor a part on the lid")
                })?;
                let c = &rp["body"]["centre_mm"];
                (c[0].as_f64().unwrap(), c[1].as_f64().unwrap())
            }
        })
    };
    let mut items: Vec<(String, P, f64, Option<P>)> = Vec::new();
    // How each item's distance from its `near` target is counted: 0 means
    // centre to target; a margin m means the target only has to lie at least
    // m inside the item — the board need only reach under the panel's wire
    // hole with room for the pads that sit there, not be centred on it.
    let mut cover: Vec<f64> = Vec::new();
    // Further targets an item must also reach under (the board: panel and vent).
    let mut also: Vec<Vec<(P, f64)>> = Vec::new();
    // Where the board's floor rectangle (its centre) may sit so that each
    // vent sealed to a board part has its chimney's ring wholly on the
    // board: a requirement, not a preference — the chimney stands on it.
    let mut board_must: Vec<(String, P, P)> = Vec::new();
    for q in &sockets {
        items.push((
            if q.qty > 1 {
                format!("`{}` ×{} socket", q.id, q.qty)
            } else {
                format!("`{}` socket", q.id)
            },
            block_size(q),
            case.wall_mm,
            q.near.as_deref().map(near_point).transpose()?,
        ));
        cover.push(0.0);
        also.push(Vec::new());
    }
    let mut board_size = (0.0, 0.0);
    if has_board {
        let size = match b.size_mm {
            Some(s) => {
                why.push(format!("board size declared: {} × {} mm", s[0], s[1]));
                (s[0], s[1])
            }
            None => {
                // A part sealed under a vent's chimney takes its whole ring.
                let sealed = |q: &Part| {
                    p.parts
                        .iter()
                        .find(|v| v.seals_to.as_deref() == Some(q.id.as_str()))
                        .map(|v| 2.0 * chimney_radii(v, q).1 + 1.0)
                };
                let area: f64 = board_parts
                    .iter()
                    .map(|q| match sealed(q) {
                        Some(d) => d * d,
                        None => q.body_mm[0] * q.body_mm[1] * q.qty as f64,
                    })
                    .sum();
                let widest = board_parts
                    .iter()
                    .map(|q| sealed(q).unwrap_or(q.body_mm[0].max(q.body_mm[1])))
                    .fold(0.0, f64::max);
                let margin = 2.0 * (b.hole_inset_mm + b_clear + b.edge_mm);
                let by_area = ((area / b.fill).sqrt().max(widest + 2.0) + margin).ceil();
                // Square unless a side would pass its ceiling: then that side
                // is the ceiling and the other carries the area — a stick's
                // board is long and narrow, not a square that does not fit.
                let other = |cap: f64| {
                    ((area / b.fill / (cap - margin).max(1.0)).max(widest + 2.0) + margin).ceil()
                };
                let (start_w, start_h) = if by_area > b.max_mm[0] {
                    (b.max_mm[0], other(b.max_mm[0]))
                } else if by_area > b.max_mm[1] {
                    (other(b.max_mm[1]), b.max_mm[1])
                } else {
                    (by_area, by_area)
                };
                // A part in an edge zone needs the zone deep enough for its
                // oriented depth plus its keep-out. Zones grow to their
                // contents (the fraction is only a floor), so the board grows
                // only along the axis the bands actually fill.
                let need = zone_needs(b, &board_parts);
                let taken = |s: &str| b.zones.values().any(|v| v == s);
                // Opposite bands, each at least its fraction of the board, and
                // a sliver of centre left between them when a zone lives there.
                let dim = |a: &str, c: &str, start: f64| {
                    let band = |s: &str, side: f64| {
                        if taken(s) {
                            need.get(s)
                                .map(|n| n.0)
                                .unwrap_or(0.0)
                                .max(side * b.zone_depth)
                        } else {
                            0.0
                        }
                    };
                    let centre = if taken("centre") { 4.0 } else { 0.0 };
                    let mut side = start;
                    for _ in 0..8 {
                        side = side.max((band(a, side) + band(c, side) + centre).ceil());
                    }
                    side
                };
                let (w, h) = (dim("left", "right", start_w), dim("top", "bottom", start_h));
                let mut note = String::new();
                for (axis, v, start, sides) in [
                    ("wide", w, start_w, ["left", "right"]),
                    ("tall", h, start_h, ["top", "bottom"]),
                ] {
                    if v > start {
                        let reasons: Vec<String> = sides
                            .iter()
                            .filter_map(|s| need.get(*s).map(|n| n.1.clone()))
                            .collect();
                        note.push_str(&format!(
                            "; {v} mm {axis} because {}",
                            reasons.join(" and ")
                        ));
                    }
                }
                let shape = if start_w == start_h {
                    format!("{by_area} mm square")
                } else {
                    format!(
                        "{start_w} × {start_h} mm: {by_area} mm square would pass board.max_mm, so one side is held at its ceiling and the other carries the area"
                    )
                };
                why.push(format!(
                    "board sized from its parts: {} mm² of footprint at fill {} plus {} mm of mounting margin → {shape}{note}",
                    r3(area),
                    b.fill,
                    r3(margin),
                ));
                (w, h)
            }
        };
        if size.0 > b.max_mm[0] + 1e-9 || size.1 > b.max_mm[1] + 1e-9 {
            bail!(
                "the board is {} × {} mm, over board.max_mm = {:?}",
                size.0,
                size.1,
                b.max_mm
            );
        }
        board_size = size;
        // Each target must lie far enough inside the board for the parts
        // placed near it: the panel's wire pads, the sensor under a vent.
        let margin = |t: &str| {
            let under = board_parts
                .iter()
                .filter(|q| q.near.as_deref() == Some(t))
                .map(|q| q.body_mm[0].max(q.body_mm[1]) / 2.0)
                .fold(0.0, f64::max);
            // A vent sealed to a board part: its whole ring on the board.
            // (Its lean is not counted here: under it is where the board
            // should reach; the floor's hard limit, below, allows for it.)
            let ring = p
                .parts
                .iter()
                .filter(|v| v.id == t)
                .filter_map(|v| {
                    let q = p
                        .parts
                        .iter()
                        .find(|o| Some(o.id.as_str()) == v.seals_to.as_deref())?;
                    Some(chimney_radii(v, q).1 + b.edge_mm + 1.0)
                })
                .fold(0.0, f64::max);
            (b.hole_inset_mm + b_clear + under).max(ring)
        };
        // The board reaches under what it is asked to, and under every vent
        // sealed to one of its parts: the chimney stands on it.
        let mut reach: Vec<&str> = b.near.iter().map(String::as_str).collect();
        for v in p.parts.iter().filter(|v| v.seals_to.is_some()) {
            if !reach.contains(&v.id.as_str()) {
                reach.push(v.id.as_str());
            }
        }
        let targets: Vec<(P, f64)> = reach
            .iter()
            .map(|t| Ok((near_point(t)?, margin(t))))
            .collect::<Result<_>>()?;
        // Parts at fixed positions may hang past the board's edge (a bought
        // board's sockets do): the board takes that room on the floor too, so
        // a socket placed against a wall is not pushed into it.
        let (ov_l, ov_r, ov_b, ov_t) = overhang(&board_parts, size);
        if ov_l + ov_r + ov_b + ov_t > 0.0 {
            why.push(format!(
                "board takes {} × {} mm of floor: parts hang {} / {} / {} / {} mm past its left / right / bottom / top edges",
                r3(size.0 + ov_l + ov_r),
                r3(size.1 + ov_b + ov_t),
                r3(ov_l),
                r3(ov_r),
                r3(ov_b),
                r3(ov_t)
            ));
        }
        for v in p.parts.iter().filter(|v| v.seals_to.is_some()) {
            // The ring may sit up to the chimney's lean from under the vent
            // (along an axis: the flat of the solver's octagon round it).
            let (t, m) = (near_point(&v.id)?, margin(&v.id) - CHIMNEY_LEAN / 1.0824);
            // The floor rectangle's centre is the board's, less the overhang.
            let d = ((ov_l - ov_r) / 2.0, (ov_b - ov_t) / 2.0);
            let (rx, ry) = (size.0 / 2.0 - m, size.1 / 2.0 - m);
            if rx < 0.0 || ry < 0.0 {
                bail!(
                    "vent `{}` seals to the board, but its chimney's ring ({} mm in from the edge) is wider than the {} × {} mm board",
                    v.id, r3(m), r3(size.0), r3(size.1)
                );
            }
            board_must.push((
                v.id.clone(),
                (t.0 - d.0 - rx, t.1 - d.1 - ry),
                (t.0 - d.0 + rx, t.1 - d.1 + ry),
            ));
        }
        items.push((
            "board".into(),
            (size.0 + ov_l + ov_r, size.1 + ov_b + ov_t),
            inner,
            targets.first().map(|t| t.0),
        ));
        cover.push(targets.first().map_or(0.0, |t| t.1));
        also.push(targets.iter().skip(1).copied().collect());
    }
    // The floor, by the solver (spec 2026-10-01, P2): each item's centre in
    // the rectangles where it fits the cavity (per turn), every item and
    // screw boss `clr` apart, and each as close to what it is `near` as the
    // rest allows. Continuous to a tenth of a millimetre; no search order,
    // no step that holds a part back.
    let (blo, bhi) = fit::bounds(&outline);
    let middle = fit::centroid(&outline);
    let mut model_items: Vec<Value> = Vec::new();
    let mut near_costs: Vec<Value> = Vec::new();
    let ids: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(k, (name, ..))| {
            if name == "board" {
                "board".to_string()
            } else {
                format!("s{k}")
            }
        })
        .collect();
    for (k, (name, size, wall, pref)) in items.iter().enumerate() {
        let turns: Vec<(f64, P)> = if name.ends_with("socket") && size.0 != size.1 {
            vec![(0.0, *size), (90.0, (size.1, size.0))]
        } else {
            vec![(0.0, *size)]
        };
        let rotations: Vec<Value> = turns
            .iter()
            .map(|(deg, sz)| {
                // A board that may lose its corners need only fit as the
                // rectangle with them cut.
                let cut = if name == "board" {
                    b.cut_mm.max(0.0)
                } else {
                    0.0
                };
                let regions = if cut > 0.0 {
                    feasible_centres_by(&outline, *sz, *wall, |x, y| {
                        fit::poly_fits(&outline, &chamfered((x, y), *sz, cut), *wall - 1e-6)
                    })
                } else {
                    feasible_centres(&outline, *sz, *wall)
                };
                // The board's centre also within reach of each sealed vent.
                let regions: Vec<(P, P)> = if name == "board" {
                    regions
                        .into_iter()
                        .filter_map(|(l, h)| {
                            board_must.iter().try_fold((l, h), |(l, h), (_, ml, mh)| {
                                let (l, h) = (
                                    (l.0.max(ml.0), l.1.max(ml.1)),
                                    (h.0.min(mh.0), h.1.min(mh.1)),
                                );
                                (l.0 <= h.0 + 1e-9 && l.1 <= h.1 + 1e-9).then_some((l, h))
                            })
                        })
                        .collect()
                } else {
                    regions
                };
                let centres: Vec<Value> = regions
                    .iter()
                    .map(|(l, h)| json!([[l.0, l.1], [h.0, h.1]]))
                    .collect();
                json!({ "deg": deg, "size_mm": [sz.0, sz.1], "centres_mm": centres })
            })
            .collect();
        if rotations
            .iter()
            .all(|r| r["centres_mm"].as_array().is_none_or(|a| a.is_empty()))
        {
            if name == "board" && !board_must.is_empty() {
                bail!(
                    "these do not fit on the floor: the board ({} × {} mm) fits nowhere inside the case, {} mm from its walls, with the chimney's ring of {} wholly on it. Move the vent inward, or enlarge the outline.",
                    r3(size.0),
                    r3(size.1),
                    r3(*wall),
                    board_must.iter().map(|(v, ..)| format!("`{v}`")).collect::<Vec<_>>().join(" and ")
                );
            }
            bail!(
                "these do not fit on the floor: {name} ({} × {} mm) fits nowhere inside the case, {} mm from its walls, either way round. Shrink it, or enlarge the outline.",
                r3(size.0),
                r3(size.1),
                r3(*wall)
            );
        }
        model_items.push(json!({
            "id": ids[k],
            "rotations": rotations,
            "inside_from": format!("{name} inside the case, {} mm from its walls", r3(*wall)),
        }));
        // What it should be near: its centre, or — when it only has to reach
        // under a target — anywhere it covers the target with the margin.
        let slack = |m: f64| {
            if m > 0.0 {
                json!([(size.0 / 2.0 - m).max(0.0), (size.1 / 2.0 - m).max(0.0)])
            } else {
                json!([0.0, 0.0])
            }
        };
        match pref {
            Some(p) => {
                // The first target outweighs the rest a little: between two
                // targets every position costs the same, and the declared
                // order says which one wins.
                near_costs.push(json!({ "item": ids[k], "to": { "at_mm": [p.0, p.1] }, "weight": 12, "slack_mm": slack(cover[k]) }));
                if cover[k] > 0.0 {
                    // Other things equal, centred on it across.
                    near_costs.push(json!({ "item": ids[k], "to": { "at_mm": [p.0, p.1] }, "weight": 1, "axes": "x" }));
                }
            }
            // Nothing asked: tidy, toward the middle.
            None => near_costs.push(
                json!({ "item": ids[k], "to": { "at_mm": [middle.0, middle.1] }, "weight": 1 }),
            ),
        }
        for (t, m) in &also[k] {
            near_costs.push(json!({ "item": ids[k], "to": { "at_mm": [t.0, t.1] }, "weight": 10, "slack_mm": slack(*m) }));
        }
    }
    let floor_model = json!({
        "grid_mm": 0.1,
        "gap_mm": clr,
        "board": { "lo_mm": [blo.0, blo.1], "hi_mm": [bhi.0, bhi.1] },
        "effort": 4.0,
        "weights": { "rotation": 1 },
        "items": model_items,
        "obstacles": boss_keepouts.iter().map(|r| json!({
            "lo_mm": [r.centre.0 - r.size.0 / 2.0, r.centre.1 - r.size.1 / 2.0],
            "hi_mm": [r.centre.0 + r.size.0 / 2.0, r.centre.1 + r.size.1 / 2.0],
            "from": "a screw boss",
        })).collect::<Vec<_>>(),
        "near": near_costs,
    });
    let floor_solved = crate::place::solve_at(
        root,
        &floor_model,
        crate::place::FLOOR_ANSWER,
        "floor items (sockets and board)",
    )
    .map_err(|e| {
        anyhow!(
            "{e}\nthey are: {}",
            items
                .iter()
                .map(|(n, s, _, _)| format!("{n} ({} × {} mm)", r3(s.0), r3(s.1)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let mut spots: Vec<Rect> = Vec::new();
    for (k, (name, size, wall, _)) in items.iter().enumerate() {
        let (c, deg) = floor_solved
            .items
            .get(&ids[k])
            .copied()
            .ok_or_else(|| anyhow!("the floor solver did not place {name}"))?;
        let sz = if (deg - 90.0).abs() < 1e-6 {
            (size.1, size.0)
        } else {
            *size
        };
        let r = Rect {
            centre: c,
            size: sz,
        };
        // The rectangles of centres are a hair inside the true region; this
        // is the exact test, so an approximation that missed is a bug, loud.
        let cut = if name == "board" {
            b.cut_mm.max(0.0)
        } else {
            0.0
        };
        let fits = if cut > 0.0 {
            fit::poly_fits(&outline, &chamfered(r.centre, r.size, cut), *wall - 0.02)
        } else {
            fit::rect_fits(&outline, &r, *wall - 0.02)
        };
        if !fits {
            bail!("the floor solver put {name} at ({}, {}), which does not fit the cavity — a fid bug, please report it", r3(c.0), r3(c.1));
        }
        spots.push(r);
    }
    let floor = Some((floor_solved.model, floor_solved.answer));
    let placed: Vec<Rect> = boss_keepouts.iter().chain(&spots).copied().collect();
    for ((((name, _, _, pref), r), m), more) in items.iter().zip(&spots).zip(&cover).zip(&also) {
        if let Some(p) = pref {
            let d = miss(r, *p, *m) + more.iter().map(|(t, mm)| miss(r, *t, *mm)).sum::<f64>();
            if d > 1.0 {
                why.push(format!("{name} sits {} mm from its `near` target: the closest spot that leaves room for the rest", r3(d)));
            }
        }
    }
    let mut socket_json: Vec<Value> = Vec::new();
    let mut stack_h = 0.0_f64;
    for (part, r) in sockets.iter().zip(&spots) {
        stack_h = stack_h.max(part.body_mm[2]);
        // Did the packer turn the block? Its placed size says. Unturned, the
        // units lie along x and stack in y; turned, the other way.
        let turned = (r.size.0 - block_size(part).0).abs() > 1e-6;
        let (uw, uh) = socket_size(part);
        let body = if turned {
            (part.body_mm[1], part.body_mm[0])
        } else {
            (part.body_mm[0], part.body_mm[1])
        };
        why.push(format!(
            "`{}` ×{} side by side on the floor at ({}, {}), lying along {}{}",
            part.id,
            part.qty,
            r3(r.centre.0),
            r3(r.centre.1),
            if turned { "y" } else { "x" },
            part.near
                .as_deref()
                .map(|nr| format!(", placed toward {nr}"))
                .unwrap_or_default()
        ));
        for n in 0..part.qty {
            let k = n as f64;
            let centre = if turned {
                (
                    r.centre.0 - r.size.0 / 2.0 + uh / 2.0 + k * (uh + clr),
                    r.centre.1,
                )
            } else {
                (
                    r.centre.0,
                    r.centre.1 - r.size.1 / 2.0 + uh / 2.0 + k * (uh + clr),
                )
            };
            let _ = uw;
            // Leads come out of the end facing the board; the can sits back
            // from that end by their length.
            let towards = if has_board {
                spots[sockets.len()].centre
            } else {
                centre
            };
            let sign = if turned {
                if towards.1 >= centre.1 {
                    1.0
                } else {
                    -1.0
                }
            } else if towards.0 >= centre.0 {
                1.0
            } else {
                -1.0
            };
            let (ax, ay) = if turned { (0.0, sign) } else { (sign, 0.0) };
            let can = (
                centre.0 - ax * part.lead_mm / 2.0,
                centre.1 - ay * part.lead_mm / 2.0,
            );
            let half = part.body_mm[0] / 2.0;
            let z_axis = case.floor_mm + part.body_mm[2] / 2.0;
            let pitch = part.pitch_mm.unwrap_or(2.5);
            let leads: Vec<Value> = part
                .leads
                .iter()
                .enumerate()
                .map(|(i, pin)| {
                    // Across the end, in increasing x (or y), in the declared order.
                    let u = (i as f64 - (part.leads.len() as f64 - 1.0) / 2.0) * pitch;
                    let (px, py) = if turned { (u, 0.0) } else { (0.0, u) };
                    let base = (can.0 + ax * half + px, can.1 + ay * half + py);
                    json!({
                        "pin": pin,
                        "from_mm": [r3(base.0), r3(base.1), r3(z_axis)],
                        "at_mm": [r3(base.0 + ax * part.lead_mm), r3(base.1 + ay * part.lead_mm), r3(z_axis)],
                    })
                })
                .collect();
            socket_json.push(json!({
                "part": part.id,
                "n": n,
                "level": "floor",
                "under_board": false,
                "axis": if turned { "y" } else { "x" },
                "body": rect_json(&Rect { centre: can, size: body }),
                "height_mm": part.body_mm[2],
                "shape": part.shape,
                "locator_mm": locator,
                "leads": leads,
            }));
        }
    }
    let mut board_json = Value::Null;
    let mut board_out = None;
    let mut placement: Option<(String, String)> = None;
    if has_board {
        let size = board_size;
        // The floor spot includes any overhang; the board is the rest of it.
        let (ov_l, ov_r, ov_b, ov_t) = overhang(&board_parts, size);
        let r = Rect {
            centre: (
                spots[sockets.len()].centre.0 + (ov_l - ov_r) / 2.0,
                spots[sockets.len()].centre.1 + (ov_b - ov_t) / 2.0,
            ),
            size,
        };
        let (x0, y0) = (r.centre.0 - size.0 / 2.0, r.centre.1 - size.1 / 2.0);
        if b.cut_mm < 0.0 {
            bail!(
                "board.cut_mm = {}: 0 (a rectangle) or how far each corner may be cut",
                b.cut_mm
            );
        }
        // Cut to the case: the rectangle clipped to the cavity. None when
        // the clip leaves it whole.
        let cut_outline: Option<Vec<P>> = (b.cut_mm > 0.0)
            .then(|| {
                fit::clip_to_rect(
                    &fit::inset(&outline, inner),
                    (x0, y0),
                    (x0 + size.0, y0 + size.1),
                )
            })
            .filter(|poly| fit::signed_area2(poly).abs() / 2.0 < size.0 * size.1 - 1e-6);
        let i = b.hole_inset_mm + if cut_outline.is_some() { b.cut_mm } else { 0.0 };
        let declared_holes = b.holes_mm.is_some();
        let mut holes = match &b.holes_mm {
            Some(hs) => {
                for h in hs {
                    if h[0] <= 0.0 || h[1] <= 0.0 || h[0] >= size.0 || h[1] >= size.1 {
                        bail!(
                            "board.holes_mm: ({}, {}) is not on the {} × {} mm board — positions are from its bottom-left corner",
                            h[0], h[1], size.0, size.1
                        );
                    }
                }
                hs.iter()
                    .map(|h| (x0 + h[0], y0 + h[1]))
                    .collect::<Vec<P>>()
            }
            None => vec![
                (x0 + i, y0 + i),
                (x0 + size.0 - i, y0 + i),
                (x0 + size.0 - i, y0 + size.1 - i),
                (x0 + i, y0 + size.1 - i),
            ],
        };
        for (what, v) in [("track_mm", b.track_mm), ("clearance_mm", b.clearance_mm)] {
            if v.is_some_and(|v| v < 0.1) {
                bail!(
                    "board.{what} = {}: under 0.1 mm no standard two-layer process holds it",
                    v.unwrap()
                );
            }
        }
        if !matches!(b.routing.as_str(), "auto" | "hand") {
            bail!(
                "board.routing = `{}`: `auto` (Freerouting) or `hand` (routed in KiCad, kept as {HAND_ROUTED})",
                b.routing
            );
        }
        let snap = match b.mount.as_str() {
            "screws" => false,
            "snap" => true,
            other => bail!("board.mount = `{other}`: `screws` (four, into standoffs) or `snap` (two pegs, two hooks)"),
        };
        if snap {
            // Two pegs through diagonal corners locate it; two hooks on the
            // side edges hold it down. Nothing to screw.
            // Declared holes are all pegs; derived ones, one diagonal pair.
            if !declared_holes {
                holes = match b.pegs.as_str() {
                    "bl-tr" => vec![holes[0], holes[2]],
                    "tl-br" => vec![holes[3], holes[1]],
                    other => bail!(
                        "board.pegs = `{other}`: `bl-tr` or `tl-br`, the diagonal the two pegs take"
                    ),
                };
            }
        }
        // Where each part sits on the board, for the render and nothing else:
        // KiCad owns the real placement. Recorded so the picture is derived,
        // not invented by the renderer.
        let board_poly = vec![
            (x0, y0),
            (x0 + size.0, y0),
            (x0 + size.0, y0 + size.1),
            (x0, y0 + size.1),
        ];
        let _ = &board_poly;
        let mut on_board: Vec<Rect> = holes
            .iter()
            .map(|&h| Rect {
                centre: h,
                size: (b_clear + 2.0, b_clear + 2.0),
            })
            .collect();
        // What each obstacle is, for a conflict to name it by.
        let mut on_board_from: Vec<String> = holes
            .iter()
            .map(|h| {
                format!(
                    "the board's hole at ({}, {}) and its clearance (board.holes_mm, board.mount)",
                    r3(h.0 - x0),
                    r3(h.1 - y0)
                )
            })
            .collect();
        // A cut corner: nothing on the board may sit in what was cut away.
        if let Some(poly) = &cut_outline {
            let mut cuts = 0;
            for (cx, cy) in [
                (x0, y0),
                (x0 + size.0, y0),
                (x0 + size.0, y0 + size.1),
                (x0, y0 + size.1),
            ] {
                if fit::contains(
                    poly,
                    (
                        cx + (r.centre.0 - cx).signum() * 1e-3,
                        cy + (r.centre.1 - cy).signum() * 1e-3,
                    ),
                ) {
                    continue;
                }
                let reach = b.cut_mm + 1e-6;
                let near: Vec<&P> = poly
                    .iter()
                    .filter(|v| (v.0 - cx).abs() <= reach && (v.1 - cy).abs() <= reach)
                    .collect();
                let (mut lx, mut hx, mut ly, mut hy) = (cx, cx, cy, cy);
                for v in near {
                    lx = lx.min(v.0);
                    hx = hx.max(v.0);
                    ly = ly.min(v.1);
                    hy = hy.max(v.1);
                }
                if hx - lx > 1e-6 && hy - ly > 1e-6 {
                    on_board.push(Rect {
                        centre: ((lx + hx) / 2.0, (ly + hy) / 2.0),
                        size: (hx - lx, hy - ly),
                    });
                    on_board_from.push(format!(
                        "the board's corner at ({}, {}), cut to follow the case (board.cut_mm)",
                        r3(cx - x0),
                        r3(cy - y0)
                    ));
                    cuts += 1;
                }
            }
            why.push(format!(
                "board cut to the case at {cuts} corner(s), each by at most {} mm (board.cut_mm): its outline is the rectangle clipped to the cavity",
                b.cut_mm
            ));
        }
        // A hook's lip reaches over the board edge: nothing may sit under it.
        let hook_w = 6.0;
        let hooks: Vec<(P, &str)> = if snap {
            vec![
                ((x0, y0 + size.1 / 2.0), "left"),
                ((x0 + size.0, y0 + size.1 / 2.0), "right"),
            ]
        } else {
            Vec::new()
        };
        for (p, side) in &hooks {
            let dx = if *side == "left" {
                HOOK_LIP / 2.0
            } else {
                -HOOK_LIP / 2.0
            };
            on_board.push(Rect {
                centre: (p.0 + dx, p.1),
                size: (HOOK_LIP, hook_w + 2.0),
            });
            on_board_from
                .push("a snap hook's lip over the board edge (board.mount = \"snap\")".to_string());
        }
        if snap {
            why.push(
                "board held by two pegs through diagonal corners and two snap hooks on its side edges — no screws, it presses in"
                    .to_string(),
            );
        }
        let n_hole_keepouts = on_board.len();

        // Zones: a band along one board edge, or `centre` for what is left.
        let need = zone_needs(b, &board_parts);
        let band = |s: &str, dim: f64| {
            need.get(s)
                .map(|n| n.0)
                .unwrap_or(0.0)
                .max(dim * b.zone_depth)
        };
        let (d_left, d_right) = (band("left", size.0), band("right", size.0));
        let (d_bottom, d_top) = (band("bottom", size.1), band("top", size.1));
        let taken = |side: &str| b.zones.values().any(|s| s == side);
        let zone_rect = |side: &str| -> Result<(Rect, Option<&'static str>)> {
            let (cx, cy) = r.centre;
            Ok(match side {
                "top" => (
                    Rect {
                        centre: (cx, y0 + size.1 - d_top / 2.0),
                        size: (size.0, d_top),
                    },
                    Some("top"),
                ),
                "bottom" => (
                    Rect {
                        centre: (cx, y0 + d_bottom / 2.0),
                        size: (size.0, d_bottom),
                    },
                    Some("bottom"),
                ),
                "left" => (
                    Rect {
                        centre: (x0 + d_left / 2.0, cy),
                        size: (d_left, size.1),
                    },
                    Some("left"),
                ),
                "right" => (
                    Rect {
                        centre: (x0 + size.0 - d_right / 2.0, cy),
                        size: (d_right, size.1),
                    },
                    Some("right"),
                ),
                "centre" => {
                    let l = if taken("left") { d_left } else { 0.0 };
                    let rr = if taken("right") { d_right } else { 0.0 };
                    let bt = if taken("bottom") { d_bottom } else { 0.0 };
                    let tp = if taken("top") { d_top } else { 0.0 };
                    let w = size.0 - l - rr;
                    let h = size.1 - bt - tp;
                    if w <= 0.0 || h <= 0.0 {
                        bail!("board.zones leave no centre; lower board.zone_depth");
                    }
                    (
                        Rect {
                            centre: (x0 + l + w / 2.0, y0 + bt + h / 2.0),
                            size: (w, h),
                        },
                        None,
                    )
                }
                other => bail!(
                    "a board zone's side must be top, bottom, left, right or centre, not `{other}`"
                ),
            })
        };
        let side_angle = |s: &str| -> Result<f64> {
            Ok(match s {
                "right" => 0.0,
                "top" => 90.0,
                "left" => 180.0,
                "bottom" => 270.0,
                other => bail!("faces = `{other}` must be top, bottom, left or right"),
            })
        };

        // Placement is solved, not searched (spec 2026-10-01). Each part's
        // rules become data in a placement model — the region it stays in,
        // the turns it may take, a fixed position, a flush edge, a distance
        // kept, a cost to keep low — and hardware/place.py hands the model to
        // the CP-SAT constraint solver. No rule has placement code of its own.
        let symbols = if board_parts.iter().any(|q| q.symbol.is_some()) {
            crate::kicad::read_symbols(root)?
        } else {
            Vec::new()
        };
        // Designators in a stable order: the pads that take a socket's
        // leads, then the biggest parts, then by name.
        let mut sorted: Vec<&Part> = board_parts.to_vec();
        sorted.sort_by(|a, c| {
            c.follows
                .is_some()
                .cmp(&a.follows.is_some())
                .then(
                    (c.body_mm[0] * c.body_mm[1])
                        .partial_cmp(&(a.body_mm[0] * a.body_mm[1]))
                        .unwrap(),
                )
                .then(a.id.cmp(&c.id))
        });
        let mut illustrative: Vec<Value> = Vec::new();
        let mut refs: BTreeMap<&str, u32> = BTreeMap::new();
        let mut keepouts: Vec<Value> = Vec::new();
        // Where each placed part's pads are, and its pin names → pad numbers.
        let mut pads_by_id: BTreeMap<String, Vec<(String, P)>> = BTreeMap::new();
        // Each footprint pad's drill, in `pads_by_id`'s order: what cad.py drills.
        let mut drills_by_id: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        let mut pin_names: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        // Every net and who is on it, for the one-ended check below.
        let mut members: BTreeMap<String, Vec<(String, bool)>> = BTreeMap::new();

        /// One board part as the placement model sees it.
        struct Plan<'a> {
            q: &'a Part,
            sym: Option<&'a crate::kicad::Symbol>,
            edge: Option<&'static str>,
            ko: f64,
            follow: Option<(Rect, Vec<(String, P)>)>,
            /// The turns it may take, the preferred one first.
            rots: Vec<f64>,
            /// Its pads at each of those turns: name, offset from its centre, net.
            pads: Vec<Vec<(String, P, Option<String>)>>,
        }
        let item_id = |q: &Part, n: u32| {
            if q.qty > 1 {
                format!("{}#{}", q.id, n + 1)
            } else {
                q.id.clone()
            }
        };
        let mm = |p: P| json!([r3(p.0), r3(p.1)]);
        let lohi = |c: P, s: P| {
            json!({
                "lo_mm": mm((c.0 - s.0 / 2.0, c.1 - s.1 / 2.0)),
                "hi_mm": mm((c.0 + s.0 / 2.0, c.1 + s.1 / 2.0)),
            })
        };
        // Costs, per unit of distance: a declared `near` outweighs short
        // connections, which outweigh the pull to the middle of the region;
        // a turn other than the preferred one only breaks ties.
        const W_NEAR: i64 = 20;
        const W_NET: i64 = 10;
        const W_MIDDLE: i64 = 1;
        const W_TURN: i64 = 1;
        let mut plans: Vec<Plan> = Vec::new();
        let mut items: Vec<Value> = Vec::new();
        let mut near: Vec<Value> = Vec::new();
        let mut apart: Vec<Value> = Vec::new();
        for q in sorted.iter().copied() {
            let sym = match &q.symbol {
                Some(sid) => Some(symbols.iter().find(|s| &s.id == sid).ok_or_else(|| {
                    anyhow!("part `{}`: symbol {sid} is not vendored in {} — run `python3 hardware/parts.py sync`", q.id, crate::kicad::SYMBOLS)
                })?),
                // Wire pads are a connector: KiCad's generic one, when vendored.
                None if q.mount == "pads" => {
                    let implied = pads_symbol(q.nets.len());
                    symbols.iter().find(|s| s.id == implied)
                }
                None => None,
            };
            if let Some(s) = sym {
                let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for p in &s.pins {
                    m.entry(p.name.clone()).or_default().push(p.number.clone());
                }
                pin_names.insert(q.id.clone(), m);
            }
            // The region it may occupy: its zone, or the whole board.
            let (region, edge) = match &q.zone {
                Some(z) => {
                    let side = b.zones.get(z).ok_or_else(|| {
                        anyhow!(
                            "part `{}`: zone `{z}` is not declared in board.zones ({})",
                            q.id,
                            b.zones.keys().cloned().collect::<Vec<_>>().join(", ")
                        )
                    })?;
                    zone_rect(side)?
                }
                None => (
                    Rect {
                        centre: r.centre,
                        size,
                    },
                    None,
                ),
            };
            // Its turn: as declared, or its `faces` side toward its zone's edge.
            let mut rot = match (q.rotate_deg, &q.faces, edge) {
                (Some(_), Some(_), _) => bail!("part `{}`: declare rotate_deg or faces, not both", q.id),
                (Some(r), None, _) => r.rem_euclid(360.0),
                (None, Some(f), Some(e)) => (side_angle(e)? - side_angle(f)?).rem_euclid(360.0),
                (None, Some(_), None) => bail!(
                    "part `{}`: `faces` needs a zone on a board edge (not centre, not none) for it to face",
                    q.id
                ),
                (None, None, _) => 0.0,
            };
            let mut wire_turned = false;
            // Wire pads nobody turned: the row lies square to the side its
            // wire comes from, so the cores land straight, side by side.
            if q.mount == "pads"
                && q.rotate_deg.is_none()
                && q.faces.is_none()
                && q.follows.is_none()
            {
                let other = p.wires.iter().find_map(|w| {
                    let id = |e: &str| e.split(['#', '.']).next().unwrap_or(e).to_string();
                    if id(&w.to) == q.id {
                        Some(id(&w.from))
                    } else if id(&w.from) == q.id {
                        Some(id(&w.to))
                    } else {
                        None
                    }
                });
                let at = other.and_then(|o| {
                    region_parts
                        .iter()
                        .chain(socket_json.iter())
                        .find(|x| x["part"] == o.as_str())
                        .map(|x| {
                            (
                                x["body"]["centre_mm"][0].as_f64().unwrap_or(0.0),
                                x["body"]["centre_mm"][1].as_f64().unwrap_or(0.0),
                            )
                        })
                });
                if let Some(o) = at {
                    let (dx, dy) = (o.0 - region.centre.0, o.1 - region.centre.1);
                    rot = if dx.abs() > dy.abs() { 90.0 } else { 0.0 };
                    wire_turned = true;
                }
            }
            // A keep-out strip between the facing side and the edge: the part
            // is placed inside the zone less that strip.
            let ko = if q.keepout_mm > 0.0 && edge.is_some() {
                q.keepout_mm
            } else {
                0.0
            };
            let (rc, rs) = (region.centre, region.size);
            let (area_c, area_s) = match edge {
                Some("top") => ((rc.0, rc.1 - ko / 2.0), (rs.0, rs.1 - ko)),
                Some("bottom") => ((rc.0, rc.1 + ko / 2.0), (rs.0, rs.1 - ko)),
                Some("left") => ((rc.0 + ko / 2.0, rc.1), (rs.0 - ko, rs.1)),
                Some("right") => ((rc.0 - ko / 2.0, rc.1), (rs.0 - ko, rs.1)),
                _ => (rc, rs),
            };
            // Nothing in the perimeter strip: that is where tracks go round.
            let (area_c, area_s) = if b.edge_mm > 0.0 {
                let e = b.edge_mm;
                let lo = (
                    (area_c.0 - area_s.0 / 2.0).max(x0 + e),
                    (area_c.1 - area_s.1 / 2.0).max(y0 + e),
                );
                let hi = (
                    (area_c.0 + area_s.0 / 2.0).min(x0 + size.0 - e),
                    (area_c.1 + area_s.1 / 2.0).min(y0 + size.1 - e),
                );
                (
                    ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0),
                    ((hi.0 - lo.0).max(0.0), (hi.1 - lo.1).max(0.0)),
                )
            } else {
                (area_c, area_s)
            };
            // Pads that take a socket's leads: one per lead, in line with it,
            // just inside the board edge that faces the socket.
            let follow: Option<(Rect, Vec<(String, P)>)> = match &q.follows {
                None => None,
                Some(sid) => {
                    let leads: Vec<(f64, f64)> = socket_json
                        .iter()
                        .filter(|s| s["part"] == sid.as_str())
                        .flat_map(|s| s["leads"].as_array().cloned().unwrap_or_default())
                        .map(|l| {
                            (
                                l["at_mm"][0].as_f64().unwrap(),
                                l["at_mm"][1].as_f64().unwrap(),
                            )
                        })
                        .collect();
                    if leads.is_empty() {
                        bail!(
                            "part `{}`: follows = `{sid}`, which is not a socketed part with leads",
                            q.id
                        );
                    }
                    if leads.len() != q.nets.len() {
                        bail!(
                            "part `{}`: {} nets for the {} leads of `{sid}` — one net per lead, in order",
                            q.id,
                            q.nets.len(),
                            leads.len()
                        );
                    }
                    let pad = q.pad_mm.unwrap_or([2.0, 2.0]);
                    let inset = pad[0].max(pad[1]) / 2.0 + 0.8;
                    let (mx, my) = (
                        leads.iter().map(|l| l.0).sum::<f64>() / leads.len() as f64,
                        leads.iter().map(|l| l.1).sum::<f64>() / leads.len() as f64,
                    );
                    let (dx, dy) = ((mx - r.centre.0) / size.0, (my - r.centre.1) / size.1);
                    let pads: Vec<(String, P)> = leads
                        .iter()
                        .enumerate()
                        .map(|(i, l)| {
                            let at = if dx.abs() >= dy.abs() {
                                (
                                    if dx > 0.0 {
                                        x0 + size.0 - inset
                                    } else {
                                        x0 + inset
                                    },
                                    l.1,
                                )
                            } else {
                                (
                                    l.0,
                                    if dy > 0.0 {
                                        y0 + size.1 - inset
                                    } else {
                                        y0 + inset
                                    },
                                )
                            };
                            ((i + 1).to_string(), (r3(at.0), r3(at.1)))
                        })
                        .collect();
                    for (_, p) in &pads {
                        if p.0 < x0 + inset - 1e-9
                            || p.0 > x0 + size.0 - inset + 1e-9
                            || p.1 < y0 + inset - 1e-9
                            || p.1 > y0 + size.1 - inset + 1e-9
                        {
                            bail!(
                                "part `{}`: a lead of `{sid}` at ({}, {}) lands off the board — the board does not reach across the leads",
                                q.id,
                                r3(p.0),
                                r3(p.1)
                            );
                        }
                    }
                    let (lo, hi) = pads.iter().fold(
                        ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)),
                        |(lo, hi), (_, p)| {
                            (
                                (lo.0.min(p.0), lo.1.min(p.1)),
                                (hi.0.max(p.0), hi.1.max(p.1)),
                            )
                        },
                    );
                    let rect = Rect {
                        centre: ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0),
                        size: (hi.0 - lo.0 + pad[0] + 1.0, hi.1 - lo.1 + pad[1] + 1.0),
                    };
                    Some((rect, pads))
                }
            };
            let fp = match &q.footprint {
                Some(fid) => Some(crate::kicad::read_footprint(root, fid)?),
                None => None,
            };
            // Free to turn: nothing declared fixes its turn, and it has pads
            // whose connections can choose one.
            let free = follow.is_none()
                && q.rotate_deg.is_none()
                && q.faces.is_none()
                && q.at_mm.is_none()
                && !wire_turned
                && (fp.is_some() || q.mount == "pads");
            let rots: Vec<f64> = if free {
                (0..4)
                    .map(|k| (rot + 90.0 * k as f64).rem_euclid(360.0))
                    .collect()
            } else {
                vec![rot]
            };
            // Which net each pad is on.
            let mut pad_net: BTreeMap<String, String> = BTreeMap::new();
            if q.mount == "pads" {
                for (k, net) in q.nets.iter().enumerate() {
                    pad_net.insert((k + 1).to_string(), net.clone());
                }
            }
            for (key, net) in &q.pins {
                let nums = match sym {
                    Some(s) => s.resolve(key),
                    None => vec![key.clone()],
                };
                for n in nums {
                    pad_net.insert(n, net.clone());
                }
            }
            let pads: Vec<Vec<(String, P, Option<String>)>> = rots
                .iter()
                .map(|&r| {
                    let (s, c) = r.to_radians().sin_cos();
                    let turn = |p: P| (c * p.0 - s * p.1, s * p.0 + c * p.1);
                    let raw: Vec<(String, P)> = if let Some((rect, at)) = &follow {
                        at.iter()
                            .map(|(n, p)| (n.clone(), (p.0 - rect.centre.0, p.1 - rect.centre.1)))
                            .collect()
                    } else if let Some(fp) = &fp {
                        // KiCad's y points down.
                        let (cx, cy) = fp.centre();
                        fp.pads
                            .iter()
                            .map(|pd| (pd.number.clone(), turn((pd.at.0 - cx, -(pd.at.1 - cy)))))
                            .collect()
                    } else if q.mount == "pads" {
                        let pitch = q.pitch_mm.unwrap_or(3.0);
                        let span = pitch * (q.nets.len().max(1) - 1) as f64;
                        (0..q.nets.len())
                            .map(|k| {
                                (
                                    (k + 1).to_string(),
                                    turn((-span / 2.0 + pitch * k as f64, 0.0)),
                                )
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    // A pad number used twice (a thermal pad in pieces) is
                    // one name per piece.
                    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
                    raw.into_iter()
                        .map(|(n, p)| {
                            let k = *seen.entry(n.clone()).and_modify(|k| *k += 1).or_insert(0);
                            let name = if k == 0 {
                                n.clone()
                            } else {
                                format!("{n}#{k}")
                            };
                            (name, p, pad_net.get(&n).cloned())
                        })
                        .collect()
                })
                .collect();
            let size_at = |r: f64| {
                if r == 90.0 || r == 270.0 {
                    (q.body_mm[1], q.body_mm[0])
                } else {
                    (q.body_mm[0], q.body_mm[1])
                }
            };
            let region_why = match &q.zone {
                Some(z) => format!("{}: zone = \"{z}\"", q.id),
                None => format!(
                    "{}: on the board, {} mm in from its edge (board.size_mm, board.edge_mm)",
                    q.id, b.edge_mm
                ),
            };
            for n in 0..q.qty {
                let iid = item_id(q, n);
                let mut it = json!({
                    "id": iid,
                    "pad_names": pads[0].iter().map(|p| p.0.clone()).collect::<Vec<_>>(),
                    "rotations": rots.iter().zip(&pads).map(|(&r, ps)| json!({
                        "deg": r,
                        "size_mm": mm(size_at(r)),
                        "pads_mm": ps.iter().map(|p| mm(p.1)).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                });
                if let Some((rect, _)) = &follow {
                    it["rotations"][0]["size_mm"] = mm(rect.size);
                    it["fixed"] = json!([{
                        "x_mm": r3(rect.centre.0),
                        "y_mm": r3(rect.centre.1),
                        "from": format!("{}: follows = \"{}\" (a pad in line with each lead)", q.id, q.follows.as_deref().unwrap_or("")),
                    }]);
                } else if let Some(a) = q.at_mm {
                    it["fixed"] = json!([{
                        "x_mm": r3(x0 + a[0]),
                        "y_mm": r3(y0 + a[1]),
                        "from": format!("{}: at_mm = [{}, {}]", q.id, a[0], a[1]),
                    }]);
                } else {
                    // Half a millimetre inside its area, as copper wants.
                    let mut reg = lohi(area_c, (area_s.0 - 1.0, area_s.1 - 1.0));
                    reg["from"] = json!(region_why);
                    it["region"] = reg;
                    if let (Some(f), Some(e)) = (&q.faces, edge) {
                        it["flush"] =
                            json!({ "side": e, "from": format!("{}: faces = \"{f}\"", q.id) });
                        // A socket reached through the wall: its face goes to
                        // the board edge and past it, as far as its own pads
                        // and holes allow, so its mouth meets the case's face.
                        if q.through_wall {
                            let over = fp.as_ref().map_or(0.0, |fp| overhang_allowed(fp, f));
                            let (k, sign, edge_at) = match e {
                                "top" => (1, 1.0, y0 + size.1),
                                "bottom" => (1, -1.0, y0),
                                "left" => (0, -1.0, x0),
                                _ => (0, 1.0, x0 + size.0),
                            };
                            let key = if sign > 0.0 { "hi_mm" } else { "lo_mm" };
                            it["region"][key][k] = json!(r3(edge_at + sign * over));
                            it["region"]["from"] = json!(format!(
                                "{region_why}, its face {} mm past the board edge (through_wall: as far as its pads allow)",
                                r3(over)
                            ));
                        }
                        // Its keep-out runs from the body to the board edge.
                        if ko > 0.0 {
                            let reach = |inner: f64, outer: f64| r3((outer - inner).abs());
                            it["keepout_mm"] = match e {
                                "top" => json!([
                                    0.0,
                                    0.0,
                                    0.0,
                                    reach(area_c.1 + area_s.1 / 2.0 - 0.5, y0 + size.1)
                                ]),
                                "bottom" => json!([
                                    0.0,
                                    0.0,
                                    reach(area_c.1 - area_s.1 / 2.0 + 0.5, y0),
                                    0.0
                                ]),
                                "left" => json!([
                                    reach(area_c.0 - area_s.0 / 2.0 + 0.5, x0),
                                    0.0,
                                    0.0,
                                    0.0
                                ]),
                                _ => json!([
                                    0.0,
                                    reach(area_c.0 + area_s.0 / 2.0 - 0.5, x0 + size.0),
                                    0.0,
                                    0.0
                                ]),
                            };
                        }
                    }
                    // Under a vent's chimney: as near beneath it as it can
                    // get — the tube may lean, a little — with its whole
                    // ring on the board and clear of everything else.
                    if let Some(v) = p
                        .parts
                        .iter()
                        .find(|v| v.seals_to.as_deref() == Some(q.id.as_str()))
                    {
                        let at = region_parts.iter().find(|r| r["part"] == v.id.as_str()).ok_or_else(|| {
                            anyhow!("part `{}`: seals_to `{}`, but the vent has no place on the lid", v.id, q.id)
                        })?;
                        let c = &at["body"]["centre_mm"];
                        let (_, ring) = chimney_radii(v, q);
                        let k = r3(ring - q.body_mm[0].min(q.body_mm[1]) / 2.0);
                        it["keepout_mm"] = json!([k, k, k, k]);
                        it["within"] = json!([{
                            "at_mm": [c[0], c[1]],
                            "mm": CHIMNEY_LEAN,
                            "from": format!("{}: seals_to = \"{}\" (under the vent's tube, leaning at most {CHIMNEY_LEAN} mm)", v.id, q.id),
                        }]);
                        near.push(json!({ "item": iid, "to": { "at_mm": [c[0], c[1]] }, "weight": W_NEAR }));
                        // The ring sits on board: the region, less the ring.
                        let m = k + b.edge_mm;
                        let lo = (
                            (area_c.0 - area_s.0 / 2.0 + 0.5).max(x0 + m),
                            (area_c.1 - area_s.1 / 2.0 + 0.5).max(y0 + m),
                        );
                        let hi = (
                            (area_c.0 + area_s.0 / 2.0 - 0.5).min(x0 + size.0 - m),
                            (area_c.1 + area_s.1 / 2.0 - 0.5).min(y0 + size.1 - m),
                        );
                        it["region"] = json!({
                            "lo_mm": mm(lo),
                            "hi_mm": mm(hi),
                            "from": format!("{}: its chimney's ring, wholly on the board ({}, board.edge_mm)", q.id, region_why),
                        });
                    }
                    // Nothing asked for: the middle of its region, or of its edge.
                    if q.near.is_none() {
                        let mid = match (q.faces.is_some(), edge) {
                            (true, Some("top")) => (area_c.0, area_c.1 + area_s.1 / 2.0),
                            (true, Some("bottom")) => (area_c.0, area_c.1 - area_s.1 / 2.0),
                            (true, Some("left")) => (area_c.0 - area_s.0 / 2.0, area_c.1),
                            (true, Some(_)) => (area_c.0 + area_s.0 / 2.0, area_c.1),
                            _ => area_c,
                        };
                        near.push(
                            json!({ "item": iid, "to": { "at_mm": mm(mid) }, "weight": W_MIDDLE }),
                        );
                    }
                }
                for a in &q.away_from {
                    let o = board_parts.iter().find(|o| &o.id == a).ok_or_else(|| {
                        anyhow!(
                            "part `{}`: away_from names `{a}`, which is not a board part",
                            q.id
                        )
                    })?;
                    for m in 0..o.qty {
                        apart.push(json!({
                            "a": iid,
                            "b": item_id(o, m),
                            "mm": q.away_mm,
                            "from": format!("{}: away_from = \"{a}\", away_mm = {}", q.id, q.away_mm),
                        }));
                    }
                }
                items.push(it);
            }
            plans.push(Plan {
                q,
                sym,
                edge,
                ko,
                follow,
                rots,
                pads,
            });
        }
        // `near`: a side of the case, a point, another board part, or one of
        // its pins (`charger.VSTOR`) — resolved now every part has its pads.
        for pl in &plans {
            let q = pl.q;
            let Some(nr) = &q.near else { continue };
            let to =
                match nr.split_once('.') {
                    Some((pid, pin)) if plans.iter().any(|o| o.q.id == pid) => {
                        let o = plans.iter().find(|o| o.q.id == pid).unwrap();
                        if o.pads[0].is_empty() {
                            json!({ "item": item_id(o.q, 0) })
                        } else {
                            let nums = o
                                .sym
                                .map(|s| s.resolve(pin))
                                .filter(|v| !v.is_empty())
                                .unwrap_or_else(|| vec![pin.to_string()]);
                            let pad = o.pads[0].iter().find(|p| nums.contains(&p.0)).ok_or_else(
                                || {
                                    anyhow!(
                                        "part `{}`: near = `{nr}`, but `{pid}` has no pin `{pin}`",
                                        q.id
                                    )
                                },
                            )?;
                            json!({ "item": item_id(o.q, 0), "pad": pad.0 })
                        }
                    }
                    _ => match plans.iter().find(|o| &o.q.id == nr) {
                        Some(o) => json!({ "item": item_id(o.q, 0) }),
                        // A socketed part: where its leads end, so the pads they
                        // are wired to sit right in front of them.
                        None if socket_json.iter().any(|s| s["part"] == nr.as_str()) => {
                            let pts: Vec<P> = socket_json
                                .iter()
                                .filter(|s| s["part"] == nr.as_str())
                                .flat_map(|s| s["leads"].as_array().cloned().unwrap_or_default())
                                .map(|l| {
                                    (
                                        l["at_mm"][0].as_f64().unwrap(),
                                        l["at_mm"][1].as_f64().unwrap(),
                                    )
                                })
                                .collect();
                            let at = if pts.is_empty() {
                                let s = socket_json
                                    .iter()
                                    .find(|s| s["part"] == nr.as_str())
                                    .unwrap();
                                (
                                    s["body"]["centre_mm"][0].as_f64().unwrap(),
                                    s["body"]["centre_mm"][1].as_f64().unwrap(),
                                )
                            } else {
                                let k = pts.len() as f64;
                                (
                                    pts.iter().map(|t| t.0).sum::<f64>() / k,
                                    pts.iter().map(|t| t.1).sum::<f64>() / k,
                                )
                            };
                            json!({ "at_mm": mm(at) })
                        }
                        None => json!({ "at_mm": mm(near_point(nr)?) }),
                    },
                };
            for n in 0..q.qty {
                near.push(json!({ "item": item_id(q, n), "to": to, "weight": W_NEAR }));
            }
        }
        // Each signal net's pads, kept close. GND is a pour, not a line.
        let mut nets: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for pl in &plans {
            for n in 0..pl.q.qty {
                for (name, _, net) in &pl.pads[0] {
                    if let Some(net) = net.as_ref().filter(|n| *n != "GND") {
                        nets.entry(net.clone())
                            .or_default()
                            .push(json!({ "item": item_id(pl.q, n), "pad": name }));
                    }
                }
            }
        }
        let model = json!({
            "grid_mm": 0.1,
            "gap_mm": 1.0,
            "effort": b.placement_effort,
            "weights": { "rotation": W_TURN },
            "board": { "lo_mm": mm((x0, y0)), "hi_mm": mm((x0 + size.0, y0 + size.1)) },
            "obstacles": on_board.iter().zip(&on_board_from).map(|(o, from)| {
                let mut v = lohi(o.centre, o.size);
                v["from"] = json!(from);
                v
            }).collect::<Vec<_>>(),
            "items": items,
            "apart": apart,
            "near": near,
            "nets": nets
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .map(|(net, pads)| json!({ "net": net, "weight": W_NET, "pads": pads }))
                .collect::<Vec<_>>(),
        });
        let solution = crate::place::solve(root, &model)?;
        why.push(format!(
            "{} board parts placed by the constraint solver — every rule a constraint, connections kept short (hardware/generated/placement.json)",
            plans.len()
        ));
        placement = Some((solution.model.clone(), solution.answer.clone()));
        for pl in &plans {
            let q = pl.q;
            let sym = pl.sym;
            let edge = pl.edge;
            let ko = pl.ko;
            let follow = &pl.follow;
            let mut rot = pl.rots[0];
            let mut prefix_n = 0;
            for n in 0..q.qty {
                let iid = item_id(q, n);
                let (centre, r) = *solution.items.get(&iid).ok_or_else(|| {
                    anyhow!("hardware/generated/placement.json has no place for `{iid}`")
                })?;
                rot = r;
                let at = match follow {
                    Some((rect, _)) => *rect,
                    None => Rect {
                        centre,
                        size: if rot == 90.0 || rot == 270.0 {
                            (q.body_mm[1], q.body_mm[0])
                        } else {
                            (q.body_mm[0], q.body_mm[1])
                        },
                    },
                };
                let prefix = match sym {
                    Some(s) => s.prefix.as_str(),
                    None if q.mount == "pads" => "J",
                    None => "U",
                };
                let c = refs.entry(prefix).or_insert(0);
                *c += 1;
                prefix_n = *c;
                // The keep-out itself: the facing side's width, from the part to the edge.
                let mut ko_json = Value::Null;
                if ko > 0.0 {
                    let (cx, cy) = at.centre;
                    let (w, h) = at.size;
                    let (kc, ks) = match edge {
                        Some("top") => {
                            let y1 = y0 + size.1;
                            let yb = cy + h / 2.0;
                            ((cx, (yb + y1) / 2.0), (w, y1 - yb))
                        }
                        Some("bottom") => {
                            let yb = cy - h / 2.0;
                            ((cx, (y0 + yb) / 2.0), (w, yb - y0))
                        }
                        Some("left") => {
                            let xb = cx - w / 2.0;
                            (((x0 + xb) / 2.0, cy), (xb - x0, h))
                        }
                        _ => {
                            let x1 = x0 + size.0;
                            let xb = cx + w / 2.0;
                            (((xb + x1) / 2.0, cy), (x1 - xb, h))
                        }
                    };
                    let kr = Rect {
                        centre: kc,
                        size: ks,
                    };
                    on_board.push(kr);
                    on_board_from.push(format!("`{}`'s keep-out", q.id));
                    ko_json = rect_json(&kr);
                    keepouts.push(json!({ "part": q.id, "body": rect_json(&kr) }));
                }
                // A real footprint: where its origin lands, so the courtyard
                // sits where the solver put the body — footprints are not
                // always centred on their origin (a THT part's is pin 1).
                let mut fp_json = Value::Null;
                if let Some(fid) = &q.footprint {
                    let fp = crate::kicad::read_footprint(root, fid)?;
                    let (cx, cy) = fp.centre();
                    let (lx, ly) = (cx, -cy); // KiCad's y points down
                    let (s, c) = rot.to_radians().sin_cos();
                    let origin = (
                        at.centre.0 - (c * lx - s * ly),
                        at.centre.1 - (s * lx + c * ly),
                    );
                    let model = fp.model.as_ref().and_then(|(stem, off, r)| {
                        let file = format!("hardware/lib/3d/{stem}.step");
                        root.join(&file)
                            .exists()
                            .then(|| json!({ "file": file, "offset_mm": off, "rotate_deg": r }))
                    });
                    // Every pin where it actually is on the board: what an
                    // agent reasons about when it asks which way a part faces.
                    let pads: Vec<Value> = fp
                        .pads
                        .iter()
                        .map(|pd| {
                            let (px, py) = (pd.at.0, -pd.at.1);
                            json!({
                                "pin": pd.number,
                                "at_mm": [r3(origin.0 + c * px - s * py), r3(origin.1 + s * px + c * py)],
                                // Its extent on the board, turned with the part.
                                "size_mm": if (rot.rem_euclid(180.0) - 90.0).abs() < 1e-6 { [pd.size.1, pd.size.0] } else { [pd.size.0, pd.size.1] },
                                // A through-hole pad's drill, turned the same way:
                                // cad.py drills it, so a lead or peg is not read
                                // as the part colliding with its own board.
                                "drill_mm": pd.drill.map(|d| if (rot.rem_euclid(180.0) - 90.0).abs() < 1e-6 { [d.1, d.0] } else { [d.0, d.1] }),
                            })
                        })
                        .collect();
                    drills_by_id.insert(
                        q.id.clone(),
                        pads.iter().map(|p| p["drill_mm"].clone()).collect(),
                    );
                    pads_by_id.insert(
                        q.id.clone(),
                        pads.iter()
                            .map(|p| {
                                (
                                    p["pin"].as_str().unwrap_or("").to_string(),
                                    (
                                        p["at_mm"][0].as_f64().unwrap(),
                                        p["at_mm"][1].as_f64().unwrap(),
                                    ),
                                )
                            })
                            .collect(),
                    );
                    // The package body as the footprint's fab layer draws it:
                    // what a model is built from when the library has none.
                    let fab = fp.fab.map(|((ax, ay), (bx, by))| {
                        let (fx, fy) = ((ax + bx) / 2.0, -(ay + by) / 2.0);
                        let (fw, fh) = (bx - ax, by - ay);
                        let turned = (rot.rem_euclid(180.0) - 90.0).abs() < 1e-6;
                        json!({
                            "centre_mm": [r3(origin.0 + c * fx - s * fy), r3(origin.1 + s * fx + c * fy)],
                            "size_mm": if turned { [r3(fh), r3(fw)] } else { [r3(fw), r3(fh)] },
                        })
                    });
                    let ports: BTreeMap<String, Value> = q
                        .ports
                        .iter()
                        .map(|(name, p)| {
                            let (px, py) = (p[0], -p[1]);
                            (
                                name.clone(),
                                json!([
                                    r3(origin.0 + c * px - s * py),
                                    r3(origin.1 + s * px + c * py),
                                    p[2]
                                ]),
                            )
                        })
                        .collect();
                    fp_json = json!({
                        "id": fid,
                        "symbol": q.symbol,
                        "origin_mm": [r3(origin.0), r3(origin.1)],
                        "pads": pads,
                        "ports": ports,
                        "model": model,
                        "fab": fab,
                    });
                }
                // What each pad connects to.
                let mut pin_nets: BTreeMap<String, String> = BTreeMap::new();
                if q.mount == "pads" {
                    let pitch = q.pitch_mm.unwrap_or(3.0);
                    let (s, c) = rot.to_radians().sin_cos();
                    let span = pitch * (q.nets.len().max(1) - 1) as f64;
                    let mut at_pads = Vec::new();
                    for (k, net) in q.nets.iter().enumerate() {
                        pin_nets.insert((k + 1).to_string(), net.clone());
                        let u = -span / 2.0 + pitch * k as f64;
                        at_pads.push((
                            (k + 1).to_string(),
                            (r3(at.centre.0 + c * u), r3(at.centre.1 + s * u)),
                        ));
                    }
                    if let Some((_, pads)) = &follow {
                        at_pads = pads.clone();
                    }
                    pads_by_id.insert(q.id.clone(), at_pads);
                }
                for (key, net) in &q.pins {
                    let fp_pads: Vec<String> = pads_by_id
                        .get(&q.id)
                        .map(|v| v.iter().map(|(n, _)| n.clone()).collect())
                        .unwrap_or_default();
                    let nums = match sym {
                        Some(s) => s.resolve(key),
                        None if fp_pads.contains(key) => vec![key.clone()],
                        None => Vec::new(),
                    };
                    if nums.is_empty() {
                        bail!(
                            "part `{}`: pin `{key}` is not a pin of {} (pins: {})",
                            q.id,
                            q.symbol
                                .as_deref()
                                .or(q.footprint.as_deref())
                                .unwrap_or("this part"),
                            sym.map(|s| s
                                .pins
                                .iter()
                                .map(|p| format!("{} {}", p.number, p.name))
                                .collect::<Vec<_>>()
                                .join(", "))
                                .unwrap_or_else(|| fp_pads.join(", "))
                        );
                    }
                    for n in nums {
                        if q.footprint.is_some() && !fp_pads.contains(&n) {
                            bail!(
                                "part `{}`: pin `{key}` is pad {n} in {}, but {} has no pad {n}",
                                q.id,
                                q.symbol.as_deref().unwrap_or("its symbol"),
                                q.footprint.as_deref().unwrap()
                            );
                        }
                        if let Some(prev) = pin_nets.insert(n.clone(), net.clone()) {
                            if &prev != net {
                                bail!("part `{}`: pad {n} is on both `{prev}` and `{net}`", q.id);
                            }
                        }
                    }
                }
                for (pad, net) in &pin_nets {
                    members
                        .entry(net.clone())
                        .or_default()
                        .push((format!("{}.{pad}", q.id), q.mount == "pads"));
                }
                illustrative.push(json!({
                    "part": q.id,
                    "n": n,
                    "footprint": fp_json,
                    "ref": format!("{prefix}{prefix_n}"),
                    "mount": q.mount,
                    "mpn": q.mpn,
                    "body": rect_json(&at),
                    "size_unrotated_mm": [q.body_mm[0], q.body_mm[1]],
                    "rotation_deg": rot,
                    "zone": q.zone,
                    "faces": q.faces,
                    "keepout": ko_json,
                    "height_mm": q.body_mm[2],
                    "nets": q.nets,
                    "pin_nets": pin_nets,
                    // The symbol's name for each connected pad: what the
                    // firmware calls the pin (`GPIO4`), for interface.json.
                    "pin_names": sym.map(|s| pin_nets.keys().filter_map(|pad| {
                        s.pins.iter().find(|sp| &sp.number == pad).map(|sp| (pad.clone(), sp.name.clone()))
                    }).collect::<BTreeMap<_, _>>()),
                    "pads_at": pads_by_id.get(&q.id).map(|v| v.iter().enumerate().map(|(i, (n, p))| {
                        let mut pad = json!({"pin": n, "at_mm": [p.0, p.1]});
                        if let Some(d) = drills_by_id.get(&q.id).and_then(|d| d.get(i)).filter(|d| !d.is_null()) {
                            pad["drill_mm"] = d.clone();
                        }
                        pad
                    }).collect::<Vec<_>>()),
                    "value": q.value,
                    "follows": q.follows,
                    "drill_mm": q.drill_mm,
                    "near": q.near,
                    "symbol": sym.map(|s| s.id.clone()),
                    "pad_mm": q.pad_mm,
                    "pitch_mm": q.pitch_mm,
                }));
            }
            let _ = prefix_n;
            if q.faces.is_some() {
                why.push(format!(
                    "`{}` turned {rot}° so its {} side faces the board's {} edge{}",
                    q.id,
                    q.faces.as_deref().unwrap(),
                    edge.unwrap(),
                    if ko > 0.0 {
                        format!(", with a {ko} mm copper keep-out to it")
                    } else {
                        String::new()
                    }
                ));
            }
            if !q.away_from.is_empty() {
                why.push(format!(
                    "`{}` kept {} mm from {}",
                    q.id,
                    q.away_mm,
                    q.away_from.join(", ")
                ));
            }
        }
        let _ = n_hole_keepouts;
        // A net that reaches one pin connects nothing. Wire pads are the
        // exception: their other end is a wire, off the board.
        for (net, m) in &members {
            // Two pins of one part (a module's GND pins) still join nothing.
            let parts: std::collections::BTreeSet<&str> = m
                .iter()
                .map(|(id, _)| id.split('.').next().unwrap_or(id))
                .collect();
            if parts.len() == 1 && !m[0].1 {
                bail!(
                    "net `{net}` reaches only `{}` — a net needs two ends; name it on the other part's pin too",
                    m.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>().join("`, `")
                );
            }
        }
        // Leads that land in pads: each runs straight to its hole and bends
        // down through the board.
        for q in board_parts.iter().filter(|q| q.follows.is_some()) {
            let sid = q.follows.as_deref().unwrap();
            let holes = &pads_by_id[&q.id];
            let mut k = 0;
            let mut longest = 0.0_f64;
            for s in socket_json.iter_mut().filter(|s| s["part"] == sid) {
                if let Some(leads) = s["leads"].as_array_mut() {
                    for l in leads {
                        let (hx, hy) = holes[k].1;
                        let f = l["from_mm"].clone();
                        let run = (hx - f[0].as_f64().unwrap()).hypot(hy - f[1].as_f64().unwrap());
                        longest = longest.max(run);
                        l["hole_mm"] = json!([hx, hy]);
                        l["at_mm"] = json!([hx, hy, f[2]]);
                        k += 1;
                    }
                }
            }
            why.push(format!(
                "`{sid}`'s leads run straight into `{}`'s plated holes on the board edge (longest {} mm before the bend) — no wires",
                q.id,
                r3(longest)
            ));
        }
        if !members.is_empty() {
            why.push(format!(
                "{} nets connect {} pins on the board",
                members.len(),
                members.values().map(Vec::len).sum::<usize>()
            ));
        }
        // Parts under the board: on the floor, clear of the standoffs, the
        // screw bosses and the sockets beside the board. The standoffs grow
        // until the board clears the tallest of them.
        let standoff_d = screw(&b.screw)?.1 + 4.4;
        // Everything placed so far except the board itself: under it is the point.
        let mut keep: Vec<Rect> = placed.iter().filter(|o| **o != r).copied().collect();
        keep.extend(holes.iter().map(|&h| Rect {
            centre: h,
            size: (standoff_d, standoff_d),
        }));
        let mut under: Vec<(&Part, u32)> = p
            .parts
            .iter()
            .filter(|q| q.place == "under-board")
            .flat_map(|q| (0..q.qty).map(move |n| (q, n)))
            .collect();
        under.sort_by(|a, c| {
            (c.0.body_mm[0] * c.0.body_mm[1])
                .partial_cmp(&(a.0.body_mm[0] * a.0.body_mm[1]))
                .unwrap()
                .then(a.0.id.cmp(&c.0.id))
                .then(a.1.cmp(&c.1))
        });
        let mut under_h = 0.0_f64;
        for (q, n) in &under {
            // Either way round: a can lying along x may not fit between the
            // standoffs where the same can along y does. Under the board wins;
            // beside it is accepted, and said.
            let s0 = socket_size(q);
            let mut best: Option<(Rect, bool)> = None;
            for (size, turned) in [(s0, false), ((s0.1, s0.0), true)] {
                if let Some(at) =
                    fit::place_rect(&outline, size, inner, &keep, clr, r.centre, case.grid_mm)
                {
                    let under_it = at.overlaps(&r, -1.0);
                    if best.is_none() || (under_it && !best.unwrap().0.overlaps(&r, -1.0)) {
                        best = Some((at, turned));
                    }
                }
            }
            let (at, turned) = best
                .ok_or_else(|| {
                    anyhow!(
                        "part `{}` #{n} ({} × {} mm with locators) fits nowhere on the floor under and around the board, \
                         clear of its standoffs",
                        q.id,
                        r3(size.0),
                        r3(size.1)
                    )
                })?;
            keep.push(at);
            under_h = under_h.max(q.body_mm[2]);
            let body = if turned {
                (q.body_mm[1], q.body_mm[0])
            } else {
                (q.body_mm[0], q.body_mm[1])
            };
            let under_it = at.overlaps(&r, -1.0);
            let ov = |a: f64, aw: f64, b: f64, bw: f64| {
                ((a + aw / 2.0).min(b + bw / 2.0) - (a - aw / 2.0).max(b - bw / 2.0)).max(0.0)
            };
            let covered = ov(at.centre.0, body.0, r.centre.0, r.size.0)
                * ov(at.centre.1, body.1, r.centre.1, r.size.1)
                / (body.0 * body.1);
            let whereabouts = if covered > 0.99 {
                "under the board".to_string()
            } else if under_it {
                format!(
                    "{}% under the board — no room for all of it",
                    (covered * 100.0).round()
                )
            } else {
                "beside the board — there is no room for it under".to_string()
            };
            why.push(format!(
                "`{}` #{n} on the floor {} at ({}, {}), lying along {}",
                q.id,
                whereabouts,
                r3(at.centre.0),
                r3(at.centre.1),
                if turned { "y" } else { "x" }
            ));
            socket_json.push(json!({
                "part": q.id,
                "n": n,
                "level": "under-board",
                "under_board": under_it,
                "axis": if turned { "y" } else { "x" },
                "body": rect_json(&Rect { centre: at.centre, size: body }),
                "height_mm": q.body_mm[2],
                "shape": q.shape,
                "locator_mm": locator,
            }));
        }
        let standoff = b.standoff_mm.max(under_h + clr);
        if standoff > b.standoff_mm + 1e-9 {
            why.push(format!(
                "standoffs raised from {} to {} mm so the board clears what is under it",
                b.standoff_mm,
                r3(standoff)
            ));
        }
        let tallest = board_parts.iter().map(|q| q.body_mm[2]).fold(0.0, f64::max);
        stack_h = stack_h.max(standoff + b.thickness_mm + tallest);
        why.push(format!(
            "board at ({}, {}) on {} mm standoffs",
            r3(r.centre.0),
            r3(r.centre.1),
            r3(standoff)
        ));
        board_json = json!({
            "body": rect_json(&r),
            "outline_mm": cut_outline.as_deref().map(pts),
            "thickness_mm": b.thickness_mm,
            "layers": b.layers,
            "power_nets": b.power_nets,
            "track_mm": b.track_mm.unwrap_or(0.2),
            "clearance_mm": b.clearance_mm.unwrap_or(0.15),
            "z_bottom_mm": r3(case.floor_mm + standoff),
            "standoff_mm": r3(standoff),
            "screw": b.screw,
            "holes_mm": pts(&holes),
            "mount": b.mount,
            "hooks": hooks.iter().map(|(p, side)| json!({ "at_mm": [r3(p.0), r3(p.1)], "side": side, "width_mm": hook_w })).collect::<Vec<_>>(),
            "hole_mm": s_clear_for(&b.screw),
            "pilot_mm": screw(&b.screw)?.1,
            "parts": board_parts.iter().map(|q| json!({ "part": q.id, "size_mm": q.body_mm, "shape": q.shape, "qty": q.qty })).collect::<Vec<_>>(),
            "illustrative_placement": illustrative,
            "zones": b.zones.iter().map(|(n, s)| Ok((n.clone(), json!({ "side": s, "body": rect_json(&zone_rect(s)?.0) })))).collect::<Result<serde_json::Map<_, _>>>()?,
            "keepouts": keepouts,
            "layers": b.layers,
            "tallest_part": board_parts.iter().max_by(|a, c| a.body_mm[2].partial_cmp(&c.body_mm[2]).unwrap()).map(|q| q.id.clone()),
        });
        board_out = Some((r, holes, b.thickness_mm, b_clear));
    } else if p.parts.iter().any(|q| q.place == "under-board") {
        bail!("a part is placed `under-board`, but this product declares no board");
    }

    // 6. Heights.
    let mut base_h =
        case.floor_mm + stack_h + clr + window_stack + if window_stack > 0.0 { clr } else { 0.0 };
    why.push(format!(
        "base is {} mm tall: floor {} + tallest stack {} + clearance{}",
        r3(base_h),
        case.floor_mm,
        r3(stack_h),
        if window_stack > 0.0 {
            format!(" + {} under the lid for the window part", r3(window_stack))
        } else {
            String::new()
        }
    ));
    // An opening through the wall must stay below the lid seal's groove: the
    // base grows until it does — the only fix, so it is derived, not asked for.
    if has_board && seal_on {
        let z_board =
            case.floor_mm + board_json["standoff_mm"].as_f64().unwrap_or(0.0) + b.thickness_mm;
        for q in p
            .parts
            .iter()
            .filter(|q| q.through_wall && q.place == "board")
        {
            let oh = q.opening_mm.map_or(q.body_mm[2] + 1.0, |o| o[1]);
            let need = z_board + q.body_mm[2] / 2.0 + oh / 2.0 + seal.depth_mm + 1.0;
            if need > base_h + 1e-9 {
                why.push(format!(
                    "base raised {} mm to {} mm so `{}`'s opening clears the lid seal",
                    r3(need - base_h),
                    r3(need),
                    q.id
                ));
                base_h = need;
            }
        }
    }
    // A press-fit lid's skirt hangs `skirt_mm` into the base, just inside the
    // wall — where the board's edge and its snap hooks are. The base grows
    // until the skirt's lower edge clears them: the only fix, so derived.
    if has_board && closure == "press-fit" {
        let z_top =
            case.floor_mm + board_json["standoff_mm"].as_f64().unwrap_or(0.0) + b.thickness_mm;
        let hooks = if b.mount == "snap" { 1.4 } else { 0.0 };
        let need = z_top + hooks + 0.5 + case.fasteners.skirt_mm;
        if need > base_h + 1e-9 {
            why.push(format!(
                "base raised {} mm to {} mm so the lid's {} mm skirt clears the board{}",
                r3(need - base_h),
                r3(need),
                case.fasteners.skirt_mm,
                if hooks > 0.0 {
                    " and its snap hooks"
                } else {
                    ""
                }
            ));
            base_h = need;
        }
    }
    // Screw length, and how much plastic its thread bites: at least 1.5 × d.
    let d_nom = nominal(&case.fasteners.size);
    let need = 1.5 * d_nom;
    let counterbore = s_head_h + 0.3;
    let screw_len = match closure {
        "screws-top" => {
            let l = SCREW_LENGTHS
                .iter()
                .copied()
                .find(|&l| l >= case.lid_mm + 2.0 * d_nom)
                .unwrap_or(30.0);
            if l - case.lid_mm > base_h - case.floor_mm {
                bail!(
                    "a {l} mm lid screw would reach the floor of a {} mm base",
                    r3(base_h)
                );
            }
            l
        }
        "screws-back" => {
            // Up from a counterbore under the base, through the base's corner
            // column, into a blind pilot in the lid that stops 1 mm short of
            // its top face.
            let through = base_h - (s_head_h + 0.3);
            let bite = case.lid_mm - 1.0;
            if bite < need - 1e-9 {
                bail!(
                    "screws from the back thread into the lid, which leaves {} mm of thread in a {} mm lid; {} needs {}. \
                     Raise case.lid_mm to {}, or use a smaller screw.",
                    r3(bite),
                    case.lid_mm,
                    case.fasteners.size,
                    r3(need),
                    r3(need + 1.0)
                );
            }
            // The shortest standard screw that bites enough. If it would bite
            // further than the lid allows, the base grows by the difference so
            // the standard length lands — what a designer does rather than
            // ordering a custom screw. (Sinking the head deeper does the
            // opposite: it pushes the screw further into the lid.)
            let l = SCREW_LENGTHS
                .iter()
                .copied()
                .find(|&l| l >= through + need)
                .ok_or_else(|| {
                    anyhow!("a {} mm base needs a screw longer than 40 mm", r3(base_h))
                })?;
            let raise = (l - through - bite).max(0.0);
            if raise > 0.0 {
                base_h += raise;
                why.push(format!(
                    "base raised {} mm to {} mm so a standard {l} mm screw lands with {} mm of thread in the lid",
                    r3(raise),
                    r3(base_h),
                    r3(bite)
                ));
            }
            l
        }
        _ => 0.0,
    };

    // 7. Wall parts: a hole in the longest edge facing the named side.
    let mut wall_json: Vec<Value> = Vec::new();
    let band = (case.floor_mm + 1.0, base_h - window_stack - clr - 1.0);
    for part in p.parts.iter().filter(|q| q.place == "wall") {
        let dir = match part.side.as_deref() {
            Some("bottom") => (0.0, -1.0),
            Some("top") => (0.0, 1.0),
            Some("left") => (-1.0, 0.0),
            Some("right") => (1.0, 0.0),
            other => bail!(
                "part `{}`: side = {other:?} must be bottom, top, left or right",
                part.id
            ),
        };
        let dia = part.body_mm[1] + 2.0 * clr;
        let n = outline.len();
        let mut best: Option<(f64, usize, P)> = None;
        for i in 0..n {
            let (a, c) = (outline[i], outline[(i + 1) % n]);
            let len = (c.0 - a.0).hypot(c.1 - a.1);
            let nrm = ((c.1 - a.1) / len, -(c.0 - a.0) / len);
            if nrm.0 * dir.0 + nrm.1 * dir.1 >= 0.7 && best.is_none_or(|b| len > b.0) {
                best = Some((len, i, nrm));
            }
        }
        let (len, i, nrm) = best.ok_or_else(|| {
            anyhow!(
                "part `{}`: no edge of the outline faces {:?}",
                part.id,
                part.side
            )
        })?;
        let (a, c) = (outline[i], outline[(i + 1) % n]);
        let tang = ((c.0 - a.0) / len, (c.1 - a.1) / len);
        // Midpoint first, then step outward along the edge until clear of every boss.
        let margin = dia / 2.0 + case.wall_mm;
        let mut at = None;
        let mut s = 0.0;
        while s <= len / 2.0 {
            for t in [len / 2.0 + s, len / 2.0 - s] {
                if t < margin || t > len - margin {
                    continue;
                }
                let pt = (a.0 + tang.0 * t, a.1 + tang.1 * t);
                if screws
                    .iter()
                    .all(|sc| (sc.0 - pt.0).hypot(sc.1 - pt.1) >= boss_r + dia / 2.0 + 1.0)
                {
                    at = Some(pt);
                    break;
                }
            }
            if at.is_some() {
                break;
            }
            s += case.grid_mm;
        }
        let at = at.ok_or_else(|| {
            anyhow!(
                "part `{}`: the {} edge has no room for a {} mm hole clear of the screw bosses",
                part.id,
                part.side.as_deref().unwrap_or(""),
                r3(dia)
            )
        })?;
        let zc = (band.0 + band.1) / 2.0;
        if band.1 - band.0 < dia {
            bail!(
                "part `{}`: a {} mm hole needs that much wall height; between floor and the parts under the lid there is {} mm",
                part.id,
                r3(dia),
                r3(band.1 - band.0)
            );
        }
        // What protrudes inside must not land on the board or a socket.
        let reach = part.body_mm[0];
        let inside = (
            at.0 - nrm.0 * (case.wall_mm + reach / 2.0),
            at.1 - nrm.1 * (case.wall_mm + reach / 2.0),
        );
        let foot = Rect {
            centre: inside,
            size: if nrm.0.abs() > nrm.1.abs() {
                (reach, dia + 4.0)
            } else {
                (dia + 4.0, reach)
            },
        };
        for (idx, o) in placed.iter().enumerate().skip(boss_keepouts.len()) {
            if foot.overlaps(o, 0.0) {
                bail!(
                    "part `{}`: its {} mm inside the wall lands on placed item {} — move it to another side",
                    part.id,
                    reach,
                    idx - boss_keepouts.len()
                );
            }
        }
        why.push(format!(
            "`{}` through the {} wall at ({}, {}), {} mm up",
            part.id,
            part.side.as_deref().unwrap_or(""),
            r3(at.0),
            r3(at.1),
            r3(zc)
        ));
        wall_json.push(json!({
            "part": part.id,
            "side": part.side,
            "centre_mm": [r3(at.0), r3(at.1), r3(zc)],
            "normal": [r3(nrm.0), r3(nrm.1)],
            "diameter_mm": r3(dia),
            "inside_mm": part.body_mm[0],
            "outside_mm": part.body_mm[2],
        }));
    }

    // 7a. Openings: a board part that faces an edge `through_wall` gets a
    //     window through the case wall in front of its mouth, sized for what
    //     plugs in, at its height. Checked here, before any solid is built:
    //     a plug must reach it, it must not cut the seal or a screw boss.
    let mut openings: Vec<Value> = Vec::new();
    for q in p.parts.iter().filter(|q| q.through_wall) {
        if q.place != "board" {
            bail!("part `{}`: through_wall is for board parts — a wall part already goes through the wall", q.id);
        }
        // Which way it faces: its zone's edge when it `faces` it, else `side`.
        let side = match (&q.faces, &q.zone, q.side.as_deref()) {
            (Some(_), Some(z), _) => b.zones.get(z).cloned().unwrap_or_default(),
            (_, _, Some(sd)) => sd.to_string(),
            _ => bail!(
                "part `{}`: through_wall needs the side it opens toward — `faces` in an edge zone, or `side = \"bottom\"`",
                q.id
            ),
        };
        let dir: P = match side.as_str() {
            "bottom" => (0.0, -1.0),
            "top" => (0.0, 1.0),
            "left" => (-1.0, 0.0),
            "right" => (1.0, 0.0),
            other => bail!(
                "part `{}`: it opens toward `{other}`; a wall is bottom, top, left or right",
                q.id
            ),
        };
        let Some(placed) = board_json["illustrative_placement"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["part"] == q.id.as_str()))
        else {
            continue;
        };
        let c = (
            placed["body"]["centre_mm"][0].as_f64().unwrap_or(0.0),
            placed["body"]["centre_mm"][1].as_f64().unwrap_or(0.0),
        );
        let sz = (
            placed["body"]["size_mm"][0].as_f64().unwrap_or(0.0),
            placed["body"]["size_mm"][1].as_f64().unwrap_or(0.0),
        );
        // The mouth: the middle of the face toward the wall — the part's own
        // face as its footprint draws the body (fab layer), not the
        // courtyard's margin in front of it: that is the metal a plug meets.
        // And its width across that face, for the window.
        let (inset, fab_w) = match (&q.footprint, &q.faces) {
            (Some(fid), Some(f)) => crate::kicad::read_footprint(root, fid)
                .ok()
                .and_then(|fp| {
                    let ((cx0, cy0), (cx1, cy1)) = fp.courtyard;
                    let ((fx0, fy0), (fx1, fy1)) = fp.fab?;
                    Some(match f.as_str() {
                        "bottom" => (cy1 - fy1, fx1 - fx0),
                        "top" => (fy0 - cy0, fx1 - fx0),
                        "left" => (fx0 - cx0, fy1 - fy0),
                        _ => (cx1 - fx1, fy1 - fy0),
                    })
                })
                .map_or((0.0, None), |(i, w)| (i.max(0.0), Some(w))),
            _ => (0.0, None),
        };
        let mouth = (
            c.0 + dir.0 * (sz.0 / 2.0 - inset),
            c.1 + dir.1 * (sz.1 / 2.0 - inset),
        );
        let face_w = fab_w.unwrap_or(if dir.0 != 0.0 { sz.1 } else { sz.0 });
        let (ow, oh) = match q.opening_mm {
            Some(o) => (o[0], o[1]),
            None => (face_w + 1.0, q.body_mm[2] + 1.0),
        };
        let z_board = board_json["z_bottom_mm"].as_f64().unwrap_or(case.floor_mm) + b.thickness_mm;
        let zc = z_board + q.body_mm[2] / 2.0;
        // Ray from the mouth to the outline: how far to the outside.
        let n = outline.len();
        let mut reach: Option<f64> = None;
        for i in 0..n {
            let (a, e) = (outline[i], outline[(i + 1) % n]);
            let (sx, sy) = (e.0 - a.0, e.1 - a.1);
            let den = dir.0 * sy - dir.1 * sx;
            if den.abs() < 1e-12 {
                continue;
            }
            let (wx, wy) = (a.0 - mouth.0, a.1 - mouth.1);
            let t = (wx * sy - wy * sx) / den;
            let u = (wx * dir.1 - wy * dir.0) / den;
            if t >= -case.wall_mm && (0.0..=1.0).contains(&u) && reach.is_none_or(|r| t < r) {
                reach = Some(t);
            }
        }
        let reach = reach
            .ok_or_else(|| anyhow!("part `{}`: no wall of the outline lies {side} of it", q.id))?;
        // A window the size of the socket's face (the default) wants the
        // socket flush with the case's face: within 1 mm, so a plug's body
        // meets the case and its shell seats fully. A window the plug's body
        // fits (`opening_mm`) lets the plug into the wall instead.
        let most = q
            .recess_mm
            .unwrap_or(if q.opening_mm.is_some() { 10.0 } else { 1.0 });
        if reach > most + 1e-9 {
            bail!(
                "part `{}`: its mouth is {} mm behind the case's outer face, and a plug seats at most {most} mm deep — \
                 add \"{side}\" to board.near (first) so the board reaches that wall, size opening_mm for the plug's \
                 body so the plug goes into the wall, or declare recess_mm",
                q.id,
                r3(reach)
            );
        }
        // Its mouth inside the wall, flush with the face: the window is then a
        // notch open to the base's top edge, so the board drops in with the
        // socket already in it, and the lid closes it. A gasket cannot cross
        // an open notch, so a sealed case keeps the socket behind the wall.
        let notch = reach < case.wall_mm - 1e-9 && !seal_on;
        if reach < case.wall_mm - 1e-9 && seal_on {
            bail!(
                "part `{}`: its mouth is inside the {} mm wall ({} mm from the outer face) — the board could not drop in past it, \
                 and a sealed case cannot have a notch for it; move the board {} mm back from that wall",
                q.id,
                case.wall_mm,
                r3(reach),
                r3(case.wall_mm - reach)
            );
        }
        if seal_on && zc + oh / 2.0 > base_h - seal.depth_mm - 0.5 {
            bail!(
                "part `{}`: its {} mm-tall opening reaches the lid seal's groove ({} mm up) — lower board.standoff_mm or raise the case",
                q.id,
                r3(oh),
                r3(base_h - seal.depth_mm)
            );
        }
        // Clear of every screw boss along the way out.
        let out_pt = (mouth.0 + dir.0 * reach, mouth.1 + dir.1 * reach);
        for sc in &screws {
            let (vx, vy) = (out_pt.0 - mouth.0, out_pt.1 - mouth.1);
            let l2 = (vx * vx + vy * vy).max(1e-12);
            let t = (((sc.0 - mouth.0) * vx + (sc.1 - mouth.1) * vy) / l2).clamp(0.0, 1.0);
            let d = (sc.0 - mouth.0 - t * vx).hypot(sc.1 - mouth.1 - t * vy);
            if d < boss_r + ow / 2.0 {
                bail!(
                    "part `{}`: its opening would cut the screw boss at ({}, {})",
                    q.id,
                    r3(sc.0),
                    r3(sc.1)
                );
            }
        }
        why.push(format!(
            "`{}` opens through the {side} wall: a {} × {} mm {}, its mouth {} mm from the outer face — not sealed: an IP-rated socket or a plug keeps water out",
            q.id,
            r3(ow),
            r3(oh),
            if notch {
                "notch open to the base's top (the board drops in with the socket in it; the lid closes it)"
            } else {
                "window"
            },
            r3(reach)
        ));
        openings.push(json!({
            "part": q.id,
            "side": side,
            "mouth_mm": [r3(mouth.0), r3(mouth.1), r3(zc)],
            "normal": [dir.0, dir.1],
            "size_mm": [r3(ow), r3(oh)],
            "recess_mm": r3(reach),
            // From just inside the mouth to past the outer face.
            "length_mm": r3(reach.max(0.0) + 2.0),
            "sealed": false,
            // Open to the base's top edge, base_h up; cad.py cuts the lid's
            // skirt to match.
            "notch": notch,
            "top_mm": r3(base_h),
        }));
    }

    // 7b. Wires. Each end is a point on its part; the route lifts to a
    //     corridor just under the lid, crosses, and drops. An end under the
    //     board first steps out from under it. It is a route, not a harness
    //     design: its job is a length you can cut to, and a picture.
    let mut wire_json: Vec<Value> = Vec::new();
    let mut wire_bom: Vec<(String, String)> = Vec::new();
    let board_top = board_json["z_bottom_mm"]
        .as_f64()
        .map(|z| z + b.thickness_mm);
    // Chimneys: a vent's tube down to the board, a ring sealing it round its
    // part. The tube stops short of the board by the ring, squeezed.
    let mut chimney_json: Vec<Value> = Vec::new();
    for v in p.parts.iter().filter(|v| v.seals_to.is_some()) {
        let t = v.seals_to.as_deref().unwrap();
        let q = p.parts.iter().find(|o| o.id == t).unwrap();
        let (Some(top), Some(at)) = (
            board_top,
            region_parts.iter().find(|r| r["part"] == v.id.as_str()),
        ) else {
            continue;
        };
        let (bore, ring) = chimney_radii(v, q);
        let squeezed = CHIMNEY_GASKET_H * (1.0 - CHIMNEY_SQUEEZE);
        // Its foot is round the part, wherever the solver put it.
        let foot = board_json["illustrative_placement"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|bp| bp["part"] == t)
            .map(|bp| bp["body"]["centre_mm"].clone())
            .unwrap_or_else(|| at["body"]["centre_mm"].clone());
        if top + squeezed >= base_h - 0.5 {
            bail!(
                "part `{}`: no room between the board and the lid for a chimney to `{t}`",
                v.id
            );
        }
        why.push(format!(
            "`{}` seals onto the board round `{t}`: a {} mm bore tube printed with the lid, on a {} mm silicone ring squeezed {}% — outside air reaches `{t}`, not the cavity",
            v.id,
            r3(2.0 * bore),
            CHIMNEY_GASKET_H,
            (CHIMNEY_SQUEEZE * 100.0).round()
        ));
        chimney_json.push(json!({
            "vent": v.id,
            "part": t,
            "centre_mm": foot,
            "top_mm": at["body"]["centre_mm"],
            "bore_mm": r3(2.0 * bore),
            "wall_mm": CHIMNEY_WALL,
            "ring_mm": r3(2.0 * ring),
            "board_top_mm": r3(top),
            "gasket": {
                "inner_mm": r3(2.0 * (bore + CHIMNEY_WALL / 2.0 - CHIMNEY_GASKET_W / 2.0)),
                "outer_mm": r3(2.0 * ring),
                "height_mm": CHIMNEY_GASKET_H,
                "squeezed_mm": r3(squeezed),
            },
            "tube_bottom_mm": r3(top + squeezed),
        }));
    }
    let board_rect = board_out.as_ref().map(|(r, ..)| *r);
    let tallest_on_board = board_parts.iter().map(|q| q.body_mm[2]).fold(0.0, f64::max);
    let mut z_route = base_h - window_stack - clr - 2.0;
    if let Some(t) = board_top {
        z_route = z_route.max(t + tallest_on_board + 1.0);
    }
    // Over the tallest part, but never into the lid: a wire is 1.2 mm thick
    // (cad.py), and where the corridor cannot clear every part it routes
    // round the tall ones instead.
    z_route = z_route.min(base_h - clr - 0.6 - 0.2);
    let lid_mounted = |end: &str| {
        let id = end.split(['#', '.']).next().unwrap_or(end);
        p.parts.iter().any(|q| {
            q.id == id
                && matches!(
                    q.mount.as_str(),
                    "surface" | "window" | "pocket" | "vent" | "adhesive"
                )
        })
    };
    // `part` or `part#n` — the n-th unit of a part declared with `qty` — and
    // optionally `.pin`: a lead of a socketed part, a pad of a board part.
    let anchor = |end: &str| -> Result<(f64, f64, f64, bool)> {
        let (end, pin) = match end.split_once('.') {
            Some((e, p)) => (e, Some(p)),
            None => (end, None),
        };
        let (id, unit) = match end.split_once('#') {
            Some((i, n)) => (
                i,
                n.parse::<u64>()
                    .map_err(|_| anyhow!("wire end `{end}`: `#{n}` is not a unit number"))?,
            ),
            None => (end, 0),
        };
        let q = p.parts.iter().find(|q| q.id == id).ok_or_else(|| {
            anyhow!(
                "a wire names part `{id}`, which is not declared (parts: {})",
                p.parts
                    .iter()
                    .map(|q| q.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
        let c = |v: &Value| (v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
        if let Some(rp) = region_parts.iter().find(|r| r["part"] == id) {
            let (x, y) = c(&rp["body"]["centre_mm"]);
            let z = if q.mount == "surface" {
                base_h + case.lid_mm
            } else {
                base_h - window_stack
            };
            return Ok((x, y, z, false));
        }
        if let Some(s) = socket_json
            .iter()
            .find(|s| s["part"] == id && s["n"].as_u64() == Some(unit))
        {
            if let Some(pin) = pin {
                let l = s["leads"]
                    .as_array()
                    .and_then(|ls| ls.iter().find(|l| l["pin"] == pin))
                    .ok_or_else(|| {
                        anyhow!(
                            "wire end `{end}.{pin}`: `{id}` has no lead `{pin}` (leads: {})",
                            q.leads.join(", ")
                        )
                    })?;
                let t = &l["at_mm"];
                return Ok((
                    t[0].as_f64().unwrap(),
                    t[1].as_f64().unwrap(),
                    t[2].as_f64().unwrap(),
                    false,
                ));
            }
            let (x, y) = c(&s["body"]["centre_mm"]);
            let (w, h) = (q.body_mm[0], s["height_mm"].as_f64().unwrap());
            // Only an end actually under the board has to step out from under it.
            let under = s["under_board"] == true;
            return Ok(if q.shape == "cylinder" && s["axis"] == "y" {
                (x, y + w / 2.0 + 1.0, case.floor_mm + h / 2.0, under)
            } else if q.shape == "cylinder" {
                (x + w / 2.0 + 1.0, y, case.floor_mm + h / 2.0, under)
            } else {
                (x, y, case.floor_mm + h, under)
            });
        }
        if let Some(bp) = board_json["illustrative_placement"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["part"] == id))
        {
            if let Some(port) = pin.and_then(|n| bp["footprint"]["ports"].get(n)) {
                // A named point on the part — a module's own IPEX socket.
                return Ok((
                    port[0].as_f64().unwrap(),
                    port[1].as_f64().unwrap(),
                    board_top.unwrap() + port[2].as_f64().unwrap(),
                    false,
                ));
            }
            if let Some(pin) = pin {
                // A wire pad, or a footprint's pad: on the copper.
                let pads = bp["pads_at"]
                    .as_array()
                    .cloned()
                    .or_else(|| bp["footprint"]["pads"].as_array().cloned())
                    .unwrap_or_default();
                let pd = pads
                    .iter()
                    .find(|x| x["pin"] == pin)
                    .ok_or_else(|| anyhow!("wire end `{end}.{pin}`: `{id}` has no pad `{pin}`"))?;
                return Ok((
                    pd["at_mm"][0].as_f64().unwrap(),
                    pd["at_mm"][1].as_f64().unwrap(),
                    board_top.unwrap() + 0.035,
                    false,
                ));
            }
            let (x, y) = c(&bp["body"]["centre_mm"]);
            return Ok((
                x,
                y,
                board_top.unwrap() + bp["height_mm"].as_f64().unwrap(),
                false,
            ));
        }
        bail!("a wire names part `{id}`, which has no position a wire can reach (a wall part?)")
    };
    // Where an end under the board comes out from under it: the nearest edge, 3 mm beyond.
    let exit = |x: f64, y: f64| -> (f64, f64) {
        let Some(r) = board_rect else { return (x, y) };
        let (hw, hh) = (r.size.0 / 2.0 + 3.0, r.size.1 / 2.0 + 3.0);
        let (dx, dy) = (x - r.centre.0, y - r.centre.1);
        let to = [
            (hw - dx, (r.centre.0 + hw, y)),
            (hw + dx, (r.centre.0 - hw, y)),
            (hh - dy, (x, r.centre.1 + hh)),
            (hh + dy, (x, r.centre.1 - hh)),
        ];
        to.iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
            .1
    };
    // What a wire must go round at the height it runs: every screw boss (they
    // stand the full height), every socket, the board and each part on it
    // that rises that high, and the solid tip a hanging hole goes through.
    // The parts a wire ends on are not in its way: it lands on them.
    let obstacles_at = |z: f64, ends: &[&str]| -> Vec<crate::wires::Obstacle> {
        use crate::wires::Obstacle;
        let mut o: Vec<Obstacle> = screws
            .iter()
            .map(|&c| Obstacle::Disc {
                centre: c,
                r: boss_r,
            })
            .collect();
        let rect = |v: &Value| Obstacle::Rect {
            centre: (
                v["centre_mm"][0].as_f64().unwrap_or(0.0),
                v["centre_mm"][1].as_f64().unwrap_or(0.0),
            ),
            size: (
                v["size_mm"][0].as_f64().unwrap_or(0.0),
                v["size_mm"][1].as_f64().unwrap_or(0.0),
            ),
        };
        for sk in &socket_json {
            if ends.iter().any(|e| sk["part"] == *e) {
                continue;
            }
            if case.floor_mm + sk["height_mm"].as_f64().unwrap_or(0.0) > z - 1.0 {
                o.push(rect(&sk["body"]));
            }
        }
        if let Some(t) = board_top {
            if t > z - 1.0 {
                o.push(rect(&board_json["body"]));
            }
            for bp in board_json["illustrative_placement"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if ends.iter().any(|e| bp["part"] == *e) {
                    continue;
                }
                if t + bp["height_mm"].as_f64().unwrap_or(0.0) > z - 1.0 {
                    o.push(rect(&bp["body"]));
                }
            }
        }
        // A chimney stands from the board to the lid: every wire goes round
        // it, foot to top.
        for ch in &chimney_json {
            for end in ["centre_mm", "top_mm"] {
                o.push(Obstacle::Disc {
                    centre: (
                        ch[end][0].as_f64().unwrap_or(0.0),
                        ch[end][1].as_f64().unwrap_or(0.0),
                    ),
                    r: ch["ring_mm"].as_f64().unwrap_or(0.0) / 2.0,
                });
            }
        }
        if let Some(py) = hang_json["plug_y_mm"].as_f64() {
            let (lo, hi) = fit::bounds(&outline);
            o.push(Obstacle::Rect {
                centre: ((lo.0 + hi.0) / 2.0, (py + hi.1) / 2.0 + 1.0),
                size: (hi.0 - lo.0 + 10.0, hi.1 - py + 2.0),
            });
        }
        o
    };
    for w in &p.wires {
        if wire_json.iter().any(|j| j["id"] == w.id.as_str()) {
            bail!("two wires share the id `{}`", w.id);
        }
        let (mut a, mut bnd) = (anchor(&w.from)?, anchor(&w.to)?);
        // A cylinder's free end — no lead named — is the end that faces the
        // other end of its wire: a coax lands on the near end of the antenna,
        // not across it to the far one.
        let near_end = |end: &str, mine: (f64, f64, f64, bool), other: (f64, f64, f64, bool)| {
            if end.contains('.') {
                return mine;
            }
            let id = end.split('#').next().unwrap_or(end);
            let unit: u64 = end
                .split_once('#')
                .and_then(|(_, n)| n.parse().ok())
                .unwrap_or(0);
            let Some(sk) = socket_json
                .iter()
                .find(|s| s["part"] == id && s["n"].as_u64() == Some(unit))
            else {
                return mine;
            };
            let Some(q) = p.parts.iter().find(|q| q.id == id) else {
                return mine;
            };
            if q.shape != "cylinder" {
                return mine;
            }
            let (x, y) = (
                sk["body"]["centre_mm"][0].as_f64().unwrap(),
                sk["body"]["centre_mm"][1].as_f64().unwrap(),
            );
            let half = q.body_mm[0] / 2.0 + 1.0;
            let ends = if sk["axis"] == "y" {
                [(x, y + half), (x, y - half)]
            } else {
                [(x + half, y), (x - half, y)]
            };
            let pick = if (ends[0].0 - other.0).hypot(ends[0].1 - other.1)
                <= (ends[1].0 - other.0).hypot(ends[1].1 - other.1)
            {
                ends[0]
            } else {
                ends[1]
            };
            (pick.0, pick.1, mine.2, mine.3)
        };
        let (a0, b0) = (a, bnd);
        a = near_end(&w.from, a0, b0);
        bnd = near_end(&w.to, b0, a);
        // The points each core lands on at the `to` end: its own pad, when the
        // end is a pads part with one pad per core.
        let to_pads: Option<(Vec<P>, Vec<String>)> = (!w.to.contains('.') && w.cores > 1)
            .then(|| {
                board_json["illustrative_placement"]
                    .as_array()
                    .and_then(|ar| {
                        ar.iter()
                            .find(|x| x["part"] == w.to.as_str() && x["mount"] == "pads")
                    })
                    .and_then(|bp| {
                        let pads = bp["pads_at"].as_array()?;
                        (pads.len() == w.cores as usize).then(|| {
                            (
                                pads.iter()
                                    .map(|pd| {
                                        (
                                            pd["at_mm"][0].as_f64().unwrap(),
                                            pd["at_mm"][1].as_f64().unwrap(),
                                        )
                                    })
                                    .collect(),
                                pads.iter()
                                    .map(|pd| {
                                        bp["pin_nets"][pd["pin"].as_str().unwrap_or("")]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string()
                                    })
                                    .collect(),
                            )
                        })
                    })
            })
            .flatten();
        let lid = lid_mounted(&w.from) || lid_mounted(&w.to);
        let low = !lid && !a.3 && !bnd.3;
        // Low wires run just over their higher end; the rest in the corridor
        // under the lid.
        let z_w = if low { a.2.max(bnd.2) + 1.0 } else { z_route };
        let (sx, sy) = if a.3 { exit(a.0, a.1) } else { (a.0, a.1) };
        let (gx, gy) = if bnd.3 {
            exit(bnd.0, bnd.1)
        } else {
            (bnd.0, bnd.1)
        };
        // The approach to a row of pads: square to the row, from the side the
        // wire comes from, the last few millimetres straight.
        let approach = 6.0;
        let (goal, row): (P, Option<(P, P)>) = match &to_pads {
            Some((pts, _)) => {
                let (u0, u1) = (pts[0], pts[pts.len() - 1]);
                let ul = (u1.0 - u0.0).hypot(u1.1 - u0.1).max(1e-9);
                let u = ((u1.0 - u0.0) / ul, (u1.1 - u0.1) / ul);
                let mut n = (-u.1, u.0);
                if (sx - gx) * n.0 + (sy - gy) * n.1 < 0.0 {
                    n = (-n.0, -n.1);
                }
                ((gx + n.0 * approach, gy + n.1 * approach), Some((u, n)))
            }
            None => ((gx, gy), None),
        };
        let part_of = |e: &str| e.split(['#', '.']).next().unwrap_or(e).to_string();
        let (pf, pt) = (part_of(&w.from), part_of(&w.to));
        let obstacles = obstacles_at(z_w, &[pf.as_str(), pt.as_str()]);
        let floor = crate::wires::Floor {
            cavity: &outline,
            wall_margin: case.wall_mm + 1.0,
            obstacles: &obstacles,
            margin: 1.0,
            step: case.grid_mm.max(0.5),
        };
        let centre = crate::wires::route(&floor, (sx, sy), goal).ok_or_else(|| {
            anyhow!(
                "wire `{}` ({} → {}): no way through at {} mm up, clear of the screw bosses and the parts that stand that tall — \
                 move the ends, or give the parts between them room",
                w.id,
                w.from,
                w.to,
                r3(z_w)
            )
        })?;
        let mut centre = centre;
        if row.is_some() {
            centre.push((gx, gy));
        }
        // A part's own cable (a coax pigtail) is one cable, whatever `cores` says.
        let ncores = if w.fixed_mm.is_some() || w.kind.as_deref() == Some("coax") {
            1
        } else {
            w.cores.max(1) as usize
        };
        let spacing = 1.3;
        let core_paths = crate::wires::cores(&centre, ncores, spacing);
        // Each core: up from its start, along its offset of the centreline,
        // across to its own pad, and down onto it.
        let mut cores_json: Vec<Value> = Vec::new();
        let mut core_nets: Vec<String> = vec![String::new(); ncores];
        let mut assign: Vec<usize> = (0..ncores).collect();
        if let (Some((pts, nets)), Some((_, n))) = (&to_pads, row) {
            // Leftmost core (the largest offset) to the leftmost pad, looking
            // along the way in: they arrive side by side and never cross.
            let order = crate::wires::left_to_right(pts, (-n.0, -n.1));
            for k in 0..ncores {
                assign[k] = order[ncores - 1 - k];
                core_nets[k] = nets[assign[k]].clone();
            }
        }
        let mut longest = 0.0_f64;
        for (k, cp) in core_paths.iter().enumerate() {
            let mut q: Vec<[f64; 3]> = Vec::new();
            let first = cp[0];
            q.push([
                if a.3 { a.0 } else { first.0 },
                if a.3 { a.1 } else { first.1 },
                a.2,
            ]);
            if a.3 {
                q.push([first.0, first.1, a.2]);
            }
            q.push([first.0, first.1, z_w]);
            let body = &cp[1..cp.len()];
            match (&to_pads, row) {
                (Some((pts, _)), Some((_, n))) => {
                    // Along the centreline to the landing, across to the point
                    // straight back from its own pad, in square to the row, down.
                    for pt in &body[..body.len() - 1] {
                        q.push([pt.0, pt.1, z_w]);
                    }
                    let pad = pts[assign[k]];
                    q.push([pad.0 + n.0 * approach, pad.1 + n.1 * approach, z_w]);
                    q.push([pad.0, pad.1, z_w]);
                    q.push([pad.0, pad.1, bnd.2]);
                }
                _ => {
                    for pt in body {
                        q.push([pt.0, pt.1, z_w]);
                    }
                    let last = *cp.last().unwrap();
                    if bnd.3 {
                        q.push([last.0, last.1, bnd.2]);
                        q.push([bnd.0, bnd.1, bnd.2]);
                    } else {
                        q.push([last.0, last.1, bnd.2]);
                    }
                }
            }
            q.dedup_by(|p2, p1| (p2[0] - p1[0]).hypot(p2[1] - p1[1]).hypot(p2[2] - p1[2]) < 1e-6);
            let len: f64 = q
                .windows(2)
                .map(|s| {
                    (s[1][0] - s[0][0])
                        .hypot(s[1][1] - s[0][1])
                        .hypot(s[1][2] - s[0][2])
                })
                .sum();
            longest = longest.max(len);
            cores_json.push(json!({
                "net": core_nets[k],
                "path_mm": q.iter().map(|v| json!([r3(v[0]), r3(v[1]), r3(v[2])])).collect::<Vec<_>>(),
            }));
        }
        let mut path: Vec<[f64; 3]> = vec![[a.0, a.1, a.2], [sx, sy, a.2], [sx, sy, z_w]];
        for pt in &centre {
            path.push([pt.0, pt.1, z_w]);
        }
        path.push([gx, gy, bnd.2]);
        path.push([bnd.0, bnd.1, bnd.2]);
        path.dedup_by(|p2, p1| (p2[0] - p1[0]).hypot(p2[1] - p1[1]).hypot(p2[2] - p1[2]) < 1e-6);
        // The longest core is the length to cut.
        let routed = longest;
        if let Some(fixed) = w.fixed_mm {
            // A part's own cable: nothing to cut, but the route has to fit it.
            if routed > fixed {
                bail!(
                    "{} `{}` ({} → {}) needs {} mm but the cable is {} mm — move them closer, or choose a longer pigtail",
                    w.kind.as_deref().unwrap_or("wire"),
                    w.id,
                    w.from,
                    w.to,
                    r3(routed),
                    fixed
                );
            }
            why.push(format!(
                "{} `{}` ({} → {}): {} mm routed of its {} mm{}",
                w.kind.as_deref().unwrap_or("wire"),
                w.id,
                w.from,
                w.to,
                r3(routed),
                fixed,
                if lid {
                    format!(
                        " — {} mm spare, so the lid lifts that far while connected",
                        r3(fixed - routed)
                    )
                } else {
                    String::new()
                }
            ));
            wire_json.push(json!({
                "id": w.id, "from": w.from, "to": w.to, "cores": 1, "awg": w.awg, "kind": w.kind,
                "path_mm": path.iter().map(|q| json!([r3(q[0]), r3(q[1]), r3(q[2])])).collect::<Vec<_>>(),
                "routed_mm": r3(routed), "length_mm": fixed, "lid_mounted": lid, "cores_mm": cores_json,
            }));
            continue;
        }
        let length = ((routed + w.slack_mm + if lid { case.service_loop_mm } else { 0.0 }) / 10.0)
            .ceil()
            * 10.0;
        why.push(format!(
            "wire `{}` ({} → {}): {} mm routed + {} slack{} → cut to {} mm",
            w.id,
            w.from,
            w.to,
            r3(routed),
            w.slack_mm,
            if lid {
                format!(
                    " + {} service loop, so the lid opens connected",
                    case.service_loop_mm
                )
            } else {
                String::new()
            },
            length
        ));
        wire_json.push(json!({
            "id": w.id, "from": w.from, "to": w.to, "cores": w.cores, "awg": w.awg,
            "path_mm": path.iter().map(|q| json!([r3(q[0]), r3(q[1]), r3(q[2])])).collect::<Vec<_>>(),
            "routed_mm": r3(routed), "length_mm": length, "lid_mounted": lid, "cores_mm": cores_json,
        }));
        wire_bom.push((
            format!("wire-{}", w.id),
            format!(
                "{}-core {} AWG wire, {} mm ({} → {})",
                w.cores, w.awg, length, w.from, w.to
            ),
        ));
    }

    // 8. BOM: declared parts, then what the design implies.
    let mut bom: Vec<BomLine> = p
        .parts
        .iter()
        .map(|q| BomLine {
            id: q.id.clone(),
            name: q.name.clone(),
            manufacturer: q.manufacturer.clone().unwrap_or_default(),
            mpn: q.mpn.clone().unwrap_or_default(),
            second_source: q.second_source.clone().unwrap_or_default(),
            lcsc: q.lcsc.clone().unwrap_or_default(),
            qty: q.qty,
            unit_cost: q.unit_cost,
            source: "declared",
            status: q.status.clone(),
            settle_by: q.settle_by.clone().unwrap_or_default(),
            footprint: q.footprint.clone().unwrap_or_default(),
            dims_from: q.dims_from.clone().unwrap_or_else(|| "assumed".into()),
            source_url: q.source_url.clone().unwrap_or_default(),
            datasheet: q.datasheet.clone().unwrap_or_default(),
        })
        .collect();
    let mut implied = |id: String, name: String, qty: u32| {
        let unit_cost = p.cost.prices.get(&id).copied();
        bom.push(BomLine {
            id,
            name,
            manufacturer: String::new(),
            mpn: String::new(),
            second_source: String::new(),
            lcsc: String::new(),
            qty,
            unit_cost,
            source: "derived",
            status: "derived".into(),
            settle_by: String::new(),
            footprint: String::new(),
            dims_from: "derived".into(),
            source_url: String::new(),
            datasheet: String::new(),
        });
    };
    if screwed {
        implied(
            format!("screw-{}x{}", case.fasteners.size, screw_len),
            format!(
                "{}×{} mm socket-head screw, lid, {} (self-tapping into plastic)",
                case.fasteners.size,
                screw_len,
                if closure == "screws-back" {
                    "from underneath"
                } else {
                    "from the top"
                }
            ),
            screws.len() as u32,
        );
    }
    for (id, name) in &wire_bom {
        implied(id.clone(), name.clone(), 1);
    }
    for ch in &chimney_json {
        let g = &ch["gasket"];
        implied(
            format!("chimney-ring-{}", ch["vent"].as_str().unwrap_or("")),
            format!(
                "Chimney seal ring, silicone (or TPU foam), {} mm ID × {} mm OD × {} mm — seals `{}`'s tube onto the board round `{}`",
                g["inner_mm"], g["outer_mm"], g["height_mm"], ch["vent"].as_str().unwrap_or(""), ch["part"].as_str().unwrap_or("")
            ),
            1,
        );
    }
    for rp in region_parts.iter().filter(|r| r["mount"] == "surface") {
        let id = rp["part"].as_str().unwrap();
        let s = &rp["body"]["size_mm"];
        implied(
            format!("adhesive-{id}"),
            format!(
                "Adhesive foam gasket (3M VHB or equal), {} mm thick, ring {} × {} mm, {} mm wide — bonds and seals `{id}`",
                seal.adhesive_mm, s[0], s[1], seal.adhesive_width_mm
            ),
            1,
        );
    }
    if let Some((_, h, t, _)) = board_out.as_ref().filter(|_| b.mount == "screws") {
        let l = SCREW_LENGTHS
            .iter()
            .copied()
            .find(|&l| l >= t + 2.0 * nominal(&b.screw))
            .unwrap_or(10.0);
        implied(
            format!("screw-{}x{}", b.screw, l),
            format!("{}×{l} mm screw, board to standoff", b.screw),
            h.len() as u32,
        );
    }
    if seal_on {
        implied(
            "gasket-lid".into(),
            "Lid gasket — print in TPU 95A".into(),
            1,
        );
        for rp in &region_parts {
            if rp["mount"] == "window" {
                implied(
                    format!("gasket-{}", rp["part"].as_str().unwrap()),
                    format!(
                        "Window gasket for {} — print in TPU 95A",
                        rp["part"].as_str().unwrap()
                    ),
                    1,
                );
            }
        }
    }
    implied(
        "case-base".into(),
        format!("Case base — print in ASA or PETG ({})", case.process),
        1,
    );
    implied(
        "case-lid".into(),
        format!("Case lid — print in ASA or PETG ({})", case.process),
        1,
    );

    let priced: f64 = bom
        .iter()
        .filter_map(|l| l.unit_cost.map(|c| c * l.qty as f64))
        .sum();
    let unpriced: Vec<String> = bom
        .iter()
        .filter(|l| l.unit_cost.is_none())
        .map(|l| l.id.clone())
        .collect();
    // The set points: the declared resistors, by their values and the nets
    // their two pins are on (a `qty` of them in parallel), against each
    // `[[check]]`. A miss fails the derive like any other contradiction.
    let mut rs: Vec<crate::circuit::Resistor> = Vec::new();
    for q in &p.parts {
        let Some(ohms) = q.value.as_deref().and_then(crate::circuit::ohms) else {
            continue;
        };
        let nets: Vec<&String> = q.pins.values().collect();
        if nets.len() != 2 {
            continue;
        }
        for _ in 0..q.qty.max(1) {
            rs.push(crate::circuit::Resistor {
                part: q.id.clone(),
                a: nets[0].clone(),
                b: nets[1].clone(),
                ohms,
            });
        }
    }
    if !p.checks.is_empty() {
        let (lines, failed) = crate::circuit::run(&rs, &p.checks)?;
        if !failed.is_empty() {
            bail!(
                "set points out of their windows:\n  {}",
                failed.join("\n  ")
            );
        }
        why.extend(lines);
    }
    // Every pin against its part's limit, with the rails at their voltages:
    // an I²C pull-up tied to 5 V instead of 3.3 V would hold a 3.3 V
    // microcontroller's pin at 5 V (the paper's held-out test, case X12).
    if !p.rails.is_empty() {
        let over = crate::circuit::pins_over_limit(
            &rs,
            &p.rails,
            p.parts
                .iter()
                .filter_map(|q| q.io_max_v.map(|max| (q.id.as_str(), max, &q.pins))),
        )?;
        if !over.is_empty() {
            bail!("pins above their parts' limits:\n  {}", over.join("\n  "));
        }
        why.push(format!(
            "pin voltages: every pin of a part with `io_max_v` is within it, with {}",
            p.rails
                .iter()
                .map(|(n, v)| format!("{n} at {v} V"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let over = priced > p.cost.ceiling + 1e-9;
    let cost_violation = if over {
        Some(format!(
            "priced parts already total {} {}, over the {} ceiling",
            r3(priced),
            p.cost.currency,
            p.cost.ceiling
        ))
    } else if p.cost.strict && !unpriced.is_empty() {
        Some(format!(
            "cost.strict: {} line(s) unpriced — {}",
            unpriced.len(),
            unpriced.join(", ")
        ))
    } else {
        None
    };
    let verdict = if over {
        "over ceiling".to_string()
    } else if unpriced.is_empty() {
        "within ceiling".to_string()
    } else {
        format!(
            "{} line(s) unpriced — the ceiling cannot be asserted yet",
            unpriced.len()
        )
    };

    // 8b. Provenance. A decided part whose size was not read from a
    //     footprint, a datasheet or a distributor is named here: remembered
    //     dimensions are how a 14 × 20 mm module gets drawn 16 × 16.
    let read_from = |d: &str| matches!(d, "footprint" | "datasheet" | "distributor");
    let unverified: Vec<String> = p
        .parts
        .iter()
        .filter(|q| {
            q.status == "decided" && !read_from(q.dims_from.as_deref().unwrap_or("assumed"))
        })
        .map(|q| format!("{} ({})", q.id, q.dims_from.as_deref().unwrap_or("assumed")))
        .collect();
    if !unverified.is_empty() {
        why.push(format!(
            "decided parts whose size was not read from a footprint, datasheet or distributor: {}",
            unverified.join(", ")
        ));
    }
    let provenance: serde_json::Map<String, Value> = p
        .parts
        .iter()
        .map(|q| {
            (
                q.id.clone(),
                json!({ "dims_from": q.dims_from.as_deref().unwrap_or("assumed"), "source_url": q.source_url, "footprint": q.footprint, "symbol": q.symbol }),
            )
        })
        .collect();

    // 9. Assembly order, from the mounts and the wires.
    let mut steps: Vec<String> = Vec::new();
    let wires_touching = |pred: &dyn Fn(&str) -> bool| -> Vec<String> {
        wire_json
            .iter()
            .filter(|w| pred(w["from"].as_str().unwrap()) || pred(w["to"].as_str().unwrap()))
            .map(|w| format!("`{}`", w["id"].as_str().unwrap()))
            .collect()
    };
    let is_under = |end: &str| {
        let id = end.split('#').next().unwrap_or(end);
        socket_json
            .iter()
            .any(|s| s["part"] == id && s["level"] == "under-board")
    };
    for s in &socket_json {
        steps.push(format!(
            "Drop `{}`{} into its socket on the floor; the corner locators hold it.",
            s["part"].as_str().unwrap(),
            if s["n"].as_u64().unwrap_or(0) > 0 {
                format!(" #{}", s["n"])
            } else {
                String::new()
            }
        ));
    }
    let under_wires = wires_touching(&is_under);
    if !under_wires.is_empty() {
        steps.push(format!(
            "Solder {} before the board goes in — once it is seated, what is under it cannot be reached.",
            under_wires.join(", ")
        ));
    }
    if let Some((_, h, _, _)) = &board_out {
        steps.push(format!(
            "Seat the board on its {} standoffs and fix it with {} × {} screws.",
            h.len(),
            h.len(),
            b.screw
        ));
    }
    for rp in region_parts.iter().filter(|r| r["mount"] == "surface") {
        let id = rp["part"].as_str().unwrap();
        let ws = wires_touching(&|x: &str| x == id);
        steps.push(format!(
            "Solder {} to the pins on the back of `{id}`, thread them down through the {} mm hole in the lid, \
             lay the adhesive ring on the lid, and press `{id}` onto it.",
            if ws.is_empty() { "its leads".to_string() } else { ws.join(", ") },
            rp["pass_through_mm"]
        ));
    }
    let rest: Vec<String> = wire_json
        .iter()
        .filter(|w| !is_under(w["from"].as_str().unwrap()) && !is_under(w["to"].as_str().unwrap()))
        .map(|w| {
            format!(
                "`{}` to `{}`",
                w["id"].as_str().unwrap(),
                w["to"].as_str().unwrap()
            )
        })
        .collect();
    if !rest.is_empty() {
        steps.push(format!(
            "Connect {} on the board, leaving the service loop free.",
            rest.join(", ")
        ));
    }
    for w in &wall_json {
        steps.push(format!(
            "Fit `{}` through the {} wall and tighten its nut from inside.",
            w["part"].as_str().unwrap(),
            w["side"].as_str().unwrap_or("")
        ));
    }
    if seal_on {
        steps.push("Press the lid gasket into the groove in the base rim.".into());
    }
    for rp in region_parts.iter().filter(|r| r["mount"] == "window") {
        steps.push(format!(
            "Turn the lid over, lay the window gasket round the window, and seat `{}` between the locator ribs.",
            rp["part"].as_str().unwrap()
        ));
    }
    steps.push(match closure {
        "screws-back" => format!(
            "Close the lid, turn the node over, and drive the {} × {}×{} screws up through the base into the lid.",
            screws.len(),
            case.fasteners.size,
            screw_len
        ),
        "screws-top" => format!(
            "Close the lid and drive the {} × {}×{} screws.",
            screws.len(),
            case.fasteners.size,
            screw_len
        ),
        _ => "Press the lid down evenly until its skirt is fully seated in the base.".into(),
    });

    let layout = json!({
        "generated_by": "fid-hardware from hardware/product.toml — do not edit",
        "product": { "name": p.product.name, "version": p.product.version },
        "shapes_read": shapes,
        "scale": { "mm_per_svg_unit": r3(k), "set_by": set_by, "outline_width_mm": r3(width) },
        "why": why,
        "case": {
            "process": case.process,
            "colour": case.colour,
            "outline_mm": pts(&outline),
            "wall_mm": case.wall_mm,
            "floor_mm": case.floor_mm,
            "lid_mm": case.lid_mm,
            "clearance_mm": clr,
            "base_height_mm": r3(base_h),
            "seal": if seal_on { json!({
                "groove_mm": seal.groove_mm, "depth_mm": seal.depth_mm, "lip_mm": seal.lip_mm,
                "compression": seal.compression,
                "tongue_mm": r3(tongue),
                "gasket_height_mm": r3(gasket_h),
                "proud_mm": r3(gasket_h - seal.depth_mm),
                "gasket_compressed_mm": r3(gasket_h * (1.0 - seal.compression)),
                "window_gasket_mm": seal.window_gasket_mm,
            }) } else { Value::Null },
            "closure": closure,
            "press_fit": if screwed { Value::Null } else { json!({
                "interference_mm": case.fasteners.interference_mm,
                "skirt_mm": case.fasteners.skirt_mm,
            }) },
            "screws": {
                "size": case.fasteners.size,
                "length_mm": screw_len,
                "clearance_mm": s_clear,
                "pilot_mm": s_pilot,
                "head_mm": [s_head, s_head_h],
                "counterbore_mm": r3(counterbore),
                "boss_radius_mm": r3(boss_r),
                "at_mm": pts(&screws),
            },
            "wall_parts": wall_json,
            "openings": openings,
            "marks": marks_json,
            "hang": hang_json,
        },
        // Parts drawn from their own solid model rather than their envelope.
        "models": p.parts.iter().filter_map(|q| q.model.as_ref().map(|m| (q.id.clone(), json!({
            "file": m,
            "rotate_deg": q.model_rotate_deg.unwrap_or([0.0; 3]),
            "envelope_mm": q.body_mm,
        })))).collect::<serde_json::Map<_, _>>(),
        "looks": p.parts.iter().map(|q| (q.id.clone(), json!(q.look.clone().unwrap_or_else(|| default_look(q).into())))).collect::<serde_json::Map<_, _>>(),
        "region_parts": region_parts,
        "sockets": socket_json,
        "board": board_json,
        "wires": wire_json,
        "chimneys": chimney_json,
        "provenance": provenance,
        "unverified_dimensions": unverified,
        "cost": {
            "currency": p.cost.currency,
            "ceiling": p.cost.ceiling,
            "at_quantity": p.cost.quantity,
            "priced_total": r3(priced),
            "unpriced": unpriced,
            "verdict": verdict,
        },
    });

    Ok(Solved {
        layout,
        bom,
        assembly: steps,
        board: board_out,
        cost_violation,
        placement,
        floor,
    })
}

/// The scaling rule for a region part: declared, or the mount's default.
fn part_fit(q: &Part) -> Result<&str> {
    let f = match (&q.fit, q.mount.as_str()) {
        (Some(f), _) => f.as_str(),
        (None, "window") => "cover",
        (None, "pocket") => "inside",
        (None, "surface") => "width",
        (None, m) => bail!("part `{}`: mount `{m}` does not sit in a region", q.id),
    };
    if !matches!(f, "width" | "height" | "cover" | "inside") {
        bail!(
            "part `{}`: fit = `{f}` must be width, height, cover or inside",
            q.id
        );
    }
    Ok(f)
}

/// How the renderer should draw a part that does not say.
fn default_look(q: &Part) -> &'static str {
    match q.mount.as_str() {
        "window" | "pocket" | "surface" => "panel",
        "socket" => "cell",
        "bulkhead" => "metal",
        _ => "component",
    }
}

fn s_clear_for(size: &str) -> f64 {
    screw(size).map(|s| s.0).unwrap_or(3.4)
}

// ── Outputs ──────────────────────────────────────────────────────────────────

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn render_bom(s: &Solved) -> String {
    let mut out = String::from(
        "id,name,manufacturer,mpn,second_source,lcsc,footprint,qty,unit_cost,source,status,dims_from,source_url,datasheet,settle_by\n",
    );
    for l in &s.bom {
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_field(&l.id),
            csv_field(&l.name),
            csv_field(&l.manufacturer),
            csv_field(&l.mpn),
            csv_field(&l.second_source),
            csv_field(&l.lcsc),
            csv_field(&l.footprint),
            l.qty,
            l.unit_cost.map(|c| c.to_string()).unwrap_or_default(),
            l.source,
            l.status,
            csv_field(&l.dims_from),
            csv_field(&l.source_url),
            csv_field(&l.datasheet),
            csv_field(&l.settle_by)
        );
    }
    out
}

/// A KiCad 7 board holding everything the declaration knows: the outline, the
/// mounting holes, every board part placed and turned — a real footprint where
/// one is vendored, a courtyard where it is not yet — the solder pads with
/// their nets, and each antenna keep-out as a rule area. Routing happens in
/// KiCad, in a project that starts from this; re-deriving overwrites it.
pub fn render_kicad(s: &Solved, root: &Path) -> Result<String> {
    let (r, holes, t, hole) = s
        .board
        .as_ref()
        .ok_or_else(|| anyhow!("no board is declared, so there is no .kicad_pcb to derive"))?;
    let (w, h) = r.size;
    let (x0, y0) = (100.0, 100.0);
    let (ox, oy) = (r.centre.0 - w / 2.0, r.centre.1 - h / 2.0);
    // Solver coordinates (y up) → KiCad board coordinates (y down).
    let kx = |x: f64| r3(x0 + x - ox);
    let ky = |y: f64| r3(y0 + h - (y - oy));
    let placed: Vec<Value> = s.layout["board"]["illustrative_placement"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut nets: Vec<String> = Vec::new();
    for p in &placed {
        let named = p["nets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| n.as_str());
        let pinned = p["pin_nets"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(_, n)| n.as_str());
        for n in named.chain(pinned) {
            if !nets.iter().any(|x| x == n) {
                nets.push(n.to_string());
            }
        }
    }
    let net_of = |p: &Value| -> BTreeMap<String, (usize, String)> {
        p["pin_nets"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(pad, n)| {
                let n = n.as_str()?;
                Some((
                    pad.clone(),
                    (nets.iter().position(|x| x == n)? + 1, n.to_string()),
                ))
            })
            .collect()
    };
    let mut out = String::new();
    let _ = writeln!(
        out,
        "(kicad_pcb (version 20221018) (generator fid_hardware)"
    );
    let _ = writeln!(out, "  (general (thickness {t}))");
    out.push_str("  (paper \"A4\")\n  (layers\n    (0 \"F.Cu\" signal) (31 \"B.Cu\" signal)\n    (34 \"B.Paste\" user) (35 \"F.Paste\" user)\n    (36 \"B.SilkS\" user \"B.Silkscreen\") (37 \"F.SilkS\" user \"F.Silkscreen\")\n    (38 \"B.Mask\" user) (39 \"F.Mask\" user)\n    (44 \"Edge.Cuts\" user) (46 \"B.CrtYd\" user \"B.Courtyard\") (47 \"F.CrtYd\" user \"F.Courtyard\")\n    (48 \"B.Fab\" user) (49 \"F.Fab\" user)\n  )\n  (setup (pad_to_mask_clearance 0))\n  (net 0 \"\")\n");
    for (i, n) in nets.iter().enumerate() {
        let _ = writeln!(out, "  (net {} \"{n}\")", i + 1);
    }
    let text = |kind: &str, v: &str, y: f64, layer: &str| {
        format!("    (fp_text {kind} \"{v}\" (at 0 {y}) (layer \"{layer}\") (effects (font (size 0.8 0.8) (thickness 0.12))))\n")
    };
    for p in &placed {
        let reference = p["ref"].as_str().unwrap_or("U?");
        let rot = p["rotation_deg"].as_f64().unwrap_or(0.0);
        let c = &p["body"]["centre_mm"];
        let (cx, cy) = (c[0].as_f64().unwrap(), c[1].as_f64().unwrap());
        let us = &p["size_unrotated_mm"];
        let (uw, uh) = (us[0].as_f64().unwrap(), us[1].as_f64().unwrap());
        let at_rot = if rot != 0.0 {
            format!(" {rot}")
        } else {
            String::new()
        };
        if let Some(fid) = p["footprint"]["id"].as_str() {
            let fp = crate::kicad::read_footprint(root, fid)?;
            let o = &p["footprint"]["origin_mm"];
            let model = p["footprint"]["model"]["file"]
                .as_str()
                .map(|f| format!("../{}", f.trim_start_matches("hardware/")));
            let _ = writeln!(
                out,
                "  {}",
                crate::kicad::place_footprint(
                    &fp,
                    kx(o[0].as_f64().unwrap()),
                    ky(o[1].as_f64().unwrap()),
                    rot,
                    reference,
                    model.as_deref(),
                    &net_of(p)
                )
            );
        } else if p["mount"] == "pads" {
            // Solder pads for wires: one per net, along the part's length.
            let names: Vec<&str> = p["nets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|n| n.as_str())
                .collect();
            let pad = p["pad_mm"]
                .as_array()
                .map(|a| (a[0].as_f64().unwrap(), a[1].as_f64().unwrap()))
                .unwrap_or((2.0, 3.0));
            let pitch = p["pitch_mm"].as_f64().unwrap_or(pad.0 + 1.0);
            let _ = writeln!(
                out,
                "  (footprint \"fid:WirePads_1x{}\" (layer \"F.Cu\") (at {} {}{at_rot})",
                names.len(),
                kx(cx),
                ky(cy)
            );
            out.push_str(&text(
                "reference",
                reference,
                -(pad.1 / 2.0 + 1.2),
                "F.SilkS",
            ));
            out.push_str(&text("value", &names.join("/"), pad.1 / 2.0 + 1.2, "F.Fab"));
            let span = pitch * (names.len().max(1) - 1) as f64;
            // Pads that take a socket's leads sit where the leads are, not on
            // a pitch: their positions are in the layout, absolute.
            let absolute: Vec<(f64, f64)> = p["pads_at"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|q| {
                            (
                                q["at_mm"][0].as_f64().unwrap(),
                                q["at_mm"][1].as_f64().unwrap(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let followed = p["follows"].is_string() && absolute.len() == names.len();
            for (k, n) in names.iter().enumerate() {
                let net = nets.iter().position(|x| x == n).unwrap() + 1;
                // Footprint coordinates: x along, y down.
                let (px, py) = if followed {
                    (r3(absolute[k].0 - cx), r3(-(absolute[k].1 - cy)))
                } else {
                    (r3(-span / 2.0 + pitch * k as f64), 0.0)
                };
                match p["drill_mm"].as_f64() {
                    Some(d) => {
                        let _ = writeln!(
                            out,
                            "    (pad \"{}\" thru_hole circle (at {px} {py}{at_rot}) (size {} {}) (drill {d}) (layers \"*.Cu\" \"*.Mask\") (net {net} \"{n}\"))",
                            k + 1,
                            pad.0,
                            pad.0
                        );
                    }
                    None => {
                        let _ = writeln!(
                            out,
                            "    (pad \"{}\" smd rect (at {px} {py}{at_rot}) (size {} {}) (layers \"F.Cu\" \"F.Paste\" \"F.Mask\") (net {net} \"{n}\"))",
                            k + 1,
                            pad.0,
                            pad.1
                        );
                    }
                }
                let _ = writeln!(out, "    (fp_text user \"{n}\" (at {px} {} ) (layer \"F.SilkS\") (effects (font (size 0.6 0.6) (thickness 0.1))))", r3(py - pad.1 / 2.0 - 0.6));
            }
            let _ = writeln!(out, "    (fp_rect (start {} {}) (end {} {}) (stroke (width 0.05) (type solid)) (fill none) (layer \"F.CrtYd\"))", r3(-uw / 2.0), r3(-uh / 2.0), r3(uw / 2.0), r3(uh / 2.0));
            out.push_str("  )\n");
        } else {
            // No footprint vendored yet: its courtyard, so the space is held
            // and the part is visibly still to come.
            let _ = writeln!(
                out,
                "  (footprint \"fid:Pending_{}\" (layer \"F.Cu\") (at {} {}{at_rot})",
                p["part"].as_str().unwrap_or("part"),
                kx(cx),
                ky(cy)
            );
            out.push_str(&text("reference", reference, -(uh / 2.0 + 1.0), "F.SilkS"));
            out.push_str(&text(
                "value",
                p["mpn"]
                    .as_str()
                    .unwrap_or(p["part"].as_str().unwrap_or("")),
                0.0,
                "F.Fab",
            ));
            let _ = writeln!(out, "    (fp_rect (start {} {}) (end {} {}) (stroke (width 0.05) (type solid)) (fill none) (layer \"F.CrtYd\"))", r3(-uw / 2.0), r3(-uh / 2.0), r3(uw / 2.0), r3(uh / 2.0));
            let _ = writeln!(out, "    (fp_rect (start {} {}) (end {} {}) (stroke (width 0.1) (type solid)) (fill none) (layer \"F.Fab\"))", r3(-uw / 2.0), r3(-uh / 2.0), r3(uw / 2.0), r3(uh / 2.0));
            out.push_str("  )\n");
        }
    }
    // Antenna keep-outs: no copper, no vias, no pour — on both layers.
    for k in s.layout["board"]["keepouts"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let c = &k["body"]["centre_mm"];
        let sz = &k["body"]["size_mm"];
        let (cx, cy, kw, kh) = (
            c[0].as_f64().unwrap(),
            c[1].as_f64().unwrap(),
            sz[0].as_f64().unwrap(),
            sz[1].as_f64().unwrap(),
        );
        let (a, b, cc, d) = (
            kx(cx - kw / 2.0),
            ky(cy - kh / 2.0),
            kx(cx + kw / 2.0),
            ky(cy + kh / 2.0),
        );
        let _ = writeln!(
            out,
            "  (zone (net 0) (net_name \"\") (layers \"F&B.Cu\") (name \"keepout-{}\") (hatch edge 0.5) (connect_pads (clearance 0)) (min_thickness 0.25) (filled_areas_thickness no) (keepout (tracks not_allowed) (vias not_allowed) (pads not_allowed) (copperpour not_allowed) (footprints allowed)) (fill (thermal_gap 0.5) (thermal_bridge_width 0.5)) (polygon (pts (xy {a} {b}) (xy {cc} {b}) (xy {cc} {d}) (xy {a} {d}))))",
            k["part"].as_str().unwrap_or("rf")
        );
    }
    // Under a chimney's ring the top copper is unbroken pour — no track, no
    // via — so the ring seals on a flat land; the sealed part's signals
    // drop through vias inside the bore. Twelve convex pieces, not one
    // windowed area: Freerouting crashes on a keep-out with a hole.
    for ch in s.layout["chimneys"].as_array().into_iter().flatten() {
        let c = &ch["centre_mm"];
        let (cx, cy) = (c[0].as_f64().unwrap_or(0.0), c[1].as_f64().unwrap_or(0.0));
        let r_in = ch["bore_mm"].as_f64().unwrap_or(0.0) / 2.0;
        // The outer chords lie outside the ring's circle.
        let r_out =
            ch["ring_mm"].as_f64().unwrap_or(0.0) / 2.0 / (std::f64::consts::PI / 12.0).cos();
        for i in 0..12 {
            let (a0, a1) = (
                i as f64 * std::f64::consts::PI / 6.0,
                (i + 1) as f64 * std::f64::consts::PI / 6.0,
            );
            let pt =
                |r: f64, a: f64| format!("(xy {} {})", kx(cx + r * a.cos()), ky(cy + r * a.sin()));
            let _ = writeln!(
                out,
                "  (zone (net 0) (net_name \"\") (layers \"F.Cu\") (name \"chimney-land-{}-{i}\") (hatch edge 0.5) (connect_pads (clearance 0)) (min_thickness 0.25) (filled_areas_thickness no) (keepout (tracks not_allowed) (vias not_allowed) (pads allowed) (copperpour allowed) (footprints allowed)) (fill (thermal_gap 0.5) (thermal_bridge_width 0.5)) (polygon (pts {} {} {} {})))",
                ch["vent"].as_str().unwrap_or("vent"),
                pt(r_in, a0),
                pt(r_out, a0),
                pt(r_out, a1),
                pt(r_in, a1)
            );
        }
    }
    // The outline: the rectangle, or — cut to the case (board.cut_mm) — the
    // rectangle clipped to the cavity.
    match s.layout["board"]["outline_mm"]
        .as_array()
        .filter(|a| a.len() >= 3)
    {
        Some(poly) => {
            let xy: Vec<String> = poly
                .iter()
                .map(|v| {
                    format!(
                        "(xy {} {})",
                        kx(v[0].as_f64().unwrap_or(0.0)),
                        ky(v[1].as_f64().unwrap_or(0.0))
                    )
                })
                .collect();
            let _ = writeln!(
                out,
                "  (gr_poly (pts {}) (stroke (width 0.1) (type default)) (fill none) (layer \"Edge.Cuts\"))",
                xy.join(" ")
            );
        }
        None => {
            let _ = writeln!(
                out,
                "  (gr_rect (start {x0} {y0}) (end {} {}) (stroke (width 0.1) (type default)) (fill none) (layer \"Edge.Cuts\"))",
                r3(x0 + w),
                r3(y0 + h)
            );
        }
    }
    for (i, (hx, hy)) in holes.iter().enumerate() {
        // KiCad's y grows downward.
        let (px, py) = (r3(x0 + hx - ox), r3(y0 + h - (hy - oy)));
        let _ = writeln!(
            out,
            "  (footprint \"fid:MountingHole\" (layer \"F.Cu\") (at {px} {py})"
        );
        let _ = writeln!(out, "    (fp_text reference \"H{}\" (at 0 -3) (layer \"F.SilkS\") (effects (font (size 1 1) (thickness 0.15))))", i + 1);
        let _ = writeln!(out, "    (fp_text value \"MountingHole\" (at 0 3) (layer \"F.Fab\") (effects (font (size 1 1) (thickness 0.15))))");
        let _ = writeln!(out, "    (pad \"\" np_thru_hole circle (at 0 0) (size {hole} {hole}) (drill {hole}) (layers \"*.Cu\" \"*.Mask\"))");
        out.push_str("  )\n");
    }
    out.push_str(")\n");
    Ok(out)
}

/// The schematic: every board part's own KiCad symbol, laid out on one sheet,
/// with a net label at each connected pin and a no-connect flag on each
/// unused one. Connection by label is how a generated schematic stays
/// readable: no wire crosses another, and the net names are the declared ones.
/// Every UUID is derived from what it names, so the file is byte-stable and
/// `fid derive --check` can gate it.
pub fn render_schematic(s: &Solved, root: &Path, product: &str) -> Result<String> {
    let placed: Vec<Value> = s.layout["board"]["illustrative_placement"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    // A product with no symbols yet gets an empty sheet, not a failure.
    let symbols = if placed.iter().any(|p| p["symbol"].is_string()) {
        crate::kicad::read_symbols(root)?
    } else {
        Vec::new()
    };
    let uuid = |seed: &str| {
        let h = crate::lock::sha256_hex(seed.as_bytes());
        format!(
            "{}-{}-{}-{}-{}",
            &h[0..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..32]
        )
    };
    let root_uuid = uuid(&format!("{product}/sheet"));
    // KiCad's connection grid: everything that must meet sits on 1.27 mm.
    let snap = |v: f64| (v / 2.54).round() * 2.54;
    let mut used: Vec<&crate::kicad::Symbol> = Vec::new();
    let mut body = String::new();
    let (mut cx, mut cy, mut row_h) = (25.4, 30.48, 0.0_f64);
    let width = 380.0;
    // One row per circuit: a part and everything placed `near` it — the
    // charger with its inductors and dividers, the radio with its
    // decoupling — so the sheet reads the way the board is grouped.
    let root_of = |p: &Value| -> String {
        let mut id = p["part"].as_str().unwrap_or("").to_string();
        for _ in 0..placed.len() {
            let near = placed
                .iter()
                .find(|o| o["part"] == id.as_str())
                .and_then(|o| o["near"].as_str())
                .map(|n| n.split_once('.').map(|(a, _)| a).unwrap_or(n).to_string());
            match near {
                Some(n) if placed.iter().any(|o| o["part"] == n.as_str()) => id = n,
                _ => break,
            }
        }
        id
    };
    let mut groups: Vec<String> = Vec::new();
    for p in &placed {
        let r = root_of(p);
        if !groups.contains(&r) {
            groups.push(r);
        }
    }
    let mut ordered: Vec<&Value> = Vec::new();
    for g in &groups {
        // The circuit's own part first, then what hangs off it.
        ordered.extend(placed.iter().filter(|p| p["part"] == g.as_str()));
        ordered.extend(
            placed
                .iter()
                .filter(|p| p["part"] != g.as_str() && &root_of(p) == g),
        );
    }
    let mut group_of_prev: Option<String> = None;
    for p in ordered {
        let g = root_of(p);
        if group_of_prev.as_ref().is_some_and(|x| *x != g) && cx > 25.4 {
            cx = 25.4;
            cy += row_h + 7.62;
            row_h = 0.0;
        }
        group_of_prev = Some(g);
        let Some(sid) = p["symbol"].as_str() else {
            continue;
        };
        let sym = symbols.iter().find(|x| x.id == sid).ok_or_else(|| {
            anyhow!("symbol {sid} is not vendored — run `python3 hardware/parts.py sync`")
        })?;
        if !used.iter().any(|u| u.id == sym.id) {
            used.push(sym);
        }
        let reference = p["ref"].as_str().unwrap_or("U?");
        let value = p["value"]
            .as_str()
            .or(p["mpn"].as_str())
            .unwrap_or(p["part"].as_str().unwrap_or(""));
        let footprint = match p["footprint"]["id"].as_str() {
            Some(f) => f.to_string(),
            None if p["mount"] == "pads" => format!(
                "fid:WirePads_1x{}",
                p["nets"].as_array().map_or(0, Vec::len)
            ),
            None => String::new(),
        };
        let footprint = footprint.as_str();
        let nets = p["pin_nets"].as_object().cloned().unwrap_or_default();
        let mut units: Vec<u32> = sym.pins.iter().map(|q| q.unit.max(1)).collect();
        units.sort();
        units.dedup();
        for unit in units {
            let ((lx, ly), (hx, hy)) = sym.bounds;
            // Room for the labels either side: the longest net name.
            let label_w = nets
                .values()
                .filter_map(|n| n.as_str())
                .map(|n| n.len() as f64 * 1.3)
                .fold(0.0, f64::max)
                + 2.54;
            // A two-pin part carries its name beside it: room for that too.
            let beside = if hx - lx < 6.0 {
                reference.len().max(value.chars().count()) as f64 * 1.1 + 1.27
            } else {
                0.0
            };
            let (w, h) = (hx - lx + 2.0 * label_w + beside, hy - ly + 7.62);
            if cx + w > width {
                cx = 25.4;
                cy += row_h + 7.62;
                row_h = 0.0;
            }
            // The symbol's origin, so its bounds start at (cx, cy).
            let (ox, oy) = (snap(cx + label_w - lx), snap(cy + 3.81 + hy));
            cx += w + 5.08;
            row_h = row_h.max(h);
            let id = format!("{}/{}/{unit}", p["part"].as_str().unwrap_or(""), p["n"]);
            let _ = writeln!(body, "  (symbol (lib_id \"{}\") (at {ox} {oy} 0) (unit {unit}) (in_bom yes) (on_board yes) (dnp no)", sym.id);
            let _ = writeln!(body, "    (uuid {})", uuid(&id));
            let prop = |k: &str, v: &str, x: f64, y: f64, hide: bool| {
                format!(
                    "    (property \"{k}\" \"{}\" (at {} {} 0) (effects (font (size 1.27 1.27)){}))\n",
                    v.replace('"', "'"),
                    r3(x),
                    r3(y),
                    if hide { " hide" } else { "" }
                )
            };
            if hx - lx < 6.0 {
                // A two-pin part: its name beside it, clear of the labels
                // at its ends.
                let side = |k: &str, v: &str, y: f64| {
                    format!(
                        "    (property \"{k}\" \"{}\" (at {} {} 0) (effects (font (size 1.27 1.27)) (justify left)))\n",
                        v.replace('"', "'"),
                        r3(ox + hx + 1.27),
                        r3(y)
                    )
                };
                body.push_str(&side("Reference", reference, oy - 1.27));
                body.push_str(&side("Value", value, oy + 1.27));
            } else {
                body.push_str(&prop("Reference", reference, ox, oy - hy - 1.905, false));
                body.push_str(&prop("Value", value, ox, oy - ly + 1.905, false));
            }
            body.push_str(&prop("Footprint", footprint, ox, oy, true));
            body.push_str(&prop("Datasheet", "", ox, oy, true));
            for q in sym.pins.iter().filter(|q| q.unit == 0 || q.unit == unit) {
                let _ = writeln!(
                    body,
                    "    (pin \"{}\" (uuid {}))",
                    q.number,
                    uuid(&format!("{id}/pin/{}", q.number))
                );
            }
            let _ = writeln!(
                body,
                "    (instances (project \"{product}\" (path \"/{root_uuid}\" (reference \"{reference}\") (unit {unit}))))\n  )"
            );
            for q in sym.pins.iter().filter(|q| q.unit == 0 || q.unit == unit) {
                // Symbol y points up, the sheet's down.
                let (px, py) = (r3(ox + q.at.0), r3(oy - q.at.1));
                match nets.get(&q.number).and_then(|n| n.as_str()) {
                    Some(net) => {
                        // The label points away from the body: opposite the pin.
                        let a = (q.angle + 180.0).rem_euclid(360.0);
                        let justify = if a == 180.0 || a == 270.0 {
                            "right bottom"
                        } else {
                            "left bottom"
                        };
                        let _ = writeln!(
                            body,
                            "  (label \"{net}\" (at {px} {py} {a}) (fields_autoplaced) (effects (font (size 1.27 1.27)) (justify {justify})) (uuid {}))",
                            uuid(&format!("{id}/label/{}", q.number))
                        );
                    }
                    None => {
                        let _ = writeln!(
                            body,
                            "  (no_connect (at {px} {py}) (uuid {}))",
                            uuid(&format!("{id}/nc/{}", q.number))
                        );
                    }
                }
            }
        }
    }
    let paper = if cy + row_h < 277.0 { "A3" } else { "A2" };
    let mut out = String::new();
    let _ = writeln!(out, "(kicad_sch (version 20230121) (generator fid_hardware)\n  (uuid {root_uuid})\n  (paper \"{paper}\")");
    let _ = writeln!(out, "  (title_block (title \"{product}\") (comment 1 \"Generated by fid-hardware from hardware/product.toml — do not edit\"))");
    out.push_str("  (lib_symbols\n");
    for u in &used {
        let _ = writeln!(out, "    {}", u.block);
    }
    out.push_str("  )\n");
    out.push_str(&body);
    let _ = writeln!(out, "  (sheet_instances (path \"/\" (page \"1\")))\n)");
    Ok(out)
}

pub fn render_assembly(s: &Solved, name: &str) -> String {
    let mut out = format!(
        "# Assembling {name}\n\nGenerated by `fid-hardware` from `hardware/product.toml` — do not edit. \
         The order follows from each part's `mount`; change a mount and this changes with it.\n\n"
    );
    for (i, step) in s.assembly.iter().enumerate() {
        let _ = writeln!(out, "{}. {step}", i + 1);
    }
    out.push_str("\n## Why it is shaped this way\n\n");
    for w in s.layout["why"].as_array().into_iter().flatten() {
        let _ = writeln!(out, "- {}", w.as_str().unwrap_or(""));
    }
    out
}

/// Every file the declaration at `decl` makes this pipeline read: itself and
/// each SVG it addresses. A declaration that does not parse reports only
/// itself — `fid derive` is where the parse error belongs.
pub fn inputs(root: &Path, decl: &str) -> Vec<String> {
    let mut out = vec![decl.to_string()];
    if let Ok(p) = std::fs::read_to_string(root.join(decl))
        .map_err(anyhow::Error::from)
        .and_then(|raw| toml::from_str::<Product>(&raw).map_err(anyhow::Error::from))
    {
        out.push(p.outline.svg.clone());
        out.extend(p.regions.values().map(|r| r.svg.clone()));
        // A vendored footprint is an input like a logo: when it changes, the
        // layout it sized is stale.
        out.extend(
            p.parts
                .iter()
                .filter_map(|q| q.footprint.as_deref())
                .filter_map(|f| crate::kicad::footprint_path(f).ok()),
        );
        // So is a part's own solid model: a changed STEP is a changed part.
        out.extend(p.parts.iter().filter_map(|q| q.model.clone()));
        // And the lock that answers its picks and prices its pins: a re-pick
        // or a re-priced part changes the BOM.
        if p.parts.iter().any(|q| q.pick.is_some()) || root.join(PARTS_LOCK).exists() {
            out.push(PARTS_LOCK.to_string());
        }
        // A hand-routed board is checked against the declaration on every
        // derive, so an edit to it is an edit to an input.
        if p.board.routing == "hand" {
            out.push(HAND_ROUTED.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// What was read from an SVG, in a form a reviewer can diff: a redrawn logo
/// shows up in `layout.json` as a changed vertex count or bounds, not only as
/// a different hash in the lock.
fn fingerprint(s: &Shape, poly: &[P]) -> Value {
    let (lo, hi) = fit::bounds(poly);
    json!({
        "svg": s.svg,
        "layer": s.layer,
        "path": s.path,
        "subpath": s.subpath,
        "vertices": poly.len(),
        "bounds_svg_units": [[r3(lo.0), r3(lo.1)], [r3(hi.0), r3(hi.1)]],
        "area_svg_units": r3(fit::signed_area2(poly) / 2.0),
    })
}

/// Take each part with a `footprint` from the footprint itself: its size is
/// the courtyard, and a `faces_pin` becomes the side that pad is on. The
/// declared height stays — a footprint is flat — and provenance becomes
/// `footprint`, because the numbers are now read, not remembered.
fn resolve_components(root: &Path, p: &mut Product) -> Result<()> {
    for q in &mut p.parts {
        let Some(id) = q.footprint.clone() else {
            if q.faces_pin.is_some() {
                bail!(
                    "part `{}`: faces_pin needs a footprint to find the pin in",
                    q.id
                );
            }
            continue;
        };
        let fp =
            crate::kicad::read_footprint(root, &id).with_context(|| format!("part `{}`", q.id))?;
        let (w, h) = fp.size();
        q.body_mm[0] = w;
        q.body_mm[1] = h;
        if let Some(t) = q.height_mm {
            q.body_mm[2] = t;
        }
        if q.body_mm[2] <= 0.0 {
            bail!(
                "part `{}`: a footprint is flat — declare its height_mm",
                q.id
            );
        }
        q.dims_from = Some("footprint".into());
        if let Some(pin) = &q.faces_pin {
            let side = fp
                .side_of_pad(pin)
                .with_context(|| format!("part `{}`: faces_pin", q.id))?;
            if let Some(f) = &q.faces {
                if f != side {
                    bail!(
                        "part `{}`: faces = `{f}` but its pin `{pin}` is on its {side} side in {id} — declare one or the other",
                        q.id
                    );
                }
            }
            q.faces = Some(side.to_string());
        }
    }
    Ok(())
}

/// Every part with a device profile against its datasheet, and every I²C
/// bus against the rule that it is pulled up. What the product leaves out
/// (an `io_max_v`, an address) the profile fills in; what it declares
/// differently is refused, naming the datasheet.
fn datasheet_checks(p: &mut Product) -> Result<()> {
    let mut problems = Vec::new();
    let mut crystals = Vec::new();
    let rails = p.rails.clone();
    for q in &mut p.parts {
        let Some(mpn) = q.mpn.clone() else { continue };
        let Some(d) = crate::devices::for_mpn(&mpn)? else {
            continue;
        };
        let says = |what: String| format!("part `{}` ({mpn}): {what} — {}", q.id, d.source);
        if let Some(a) = d.i2c_address {
            match q.i2c_address {
                Some(b) if b != a => problems.push(says(format!(
                    "i2c_address = 0x{b:02X}, but the datasheet's address is 0x{a:02X}"
                ))),
                _ => q.i2c_address = Some(a),
            }
        }
        if let Some(sup) = &d.supply {
            let rail = d
                .net(&sup.pin, &q.pins)
                .and_then(|n| rails.get(n).map(|v| (n.clone(), *v)));
            if let Some((net, v)) = &rail {
                if *v < sup.min - 1e-9 || *v > sup.max + 1e-9 {
                    problems.push(says(format!(
                        "its supply pin {} is on {net} at {v} V, outside {}–{} V",
                        sup.pin, sup.min, sup.max
                    )));
                }
                if let Some(over) = d.io_max_over_supply {
                    let limit = ((v + over) * 1000.0).round() / 1000.0;
                    match q.io_max_v {
                        Some(m) if m > limit + 1e-9 => problems.push(says(format!(
                            "io_max_v = {m} V, but no pin may go above {} + {over} = {limit} V \
                             with {} on {net} — the limit is the datasheet's, not a setting",
                            v, sup.pin
                        ))),
                        Some(_) => {}
                        None => q.io_max_v = Some(limit),
                    }
                }
            }
        }
        for (pad, f) in &d.pins {
            if f.role.as_deref() != Some("enable") {
                continue;
            }
            let Some(net) = d.net(pad, &q.pins) else {
                continue;
            };
            let Some(v) = crate::devices::level(net, &rails) else {
                continue;
            };
            let low_active = f.active.as_deref() == Some("low");
            if (low_active && v > 0.0) || (!low_active && v == 0.0) {
                problems.push(says(format!(
                    "pin {pad} ({}, active {}) is tied to {net}: the part is never enabled — \
                     tie it {} or drive it",
                    f.name,
                    if low_active { "low" } else { "high" },
                    if low_active {
                        "to ground"
                    } else {
                        "to its supply"
                    }
                )));
            }
        }
        q.commands = d.commands.clone();
        q.timing = d.timing.clone();
        if let Some(x) = &d.crystal {
            if let Some(net) = d.net(&x.pin, &q.pins) {
                crystals.push((
                    q.id.clone(),
                    mpn.clone(),
                    net.clone(),
                    x.hz,
                    x.because.clone(),
                    d.source.clone(),
                ));
            }
        }
    }
    for (id, mpn, net, hz, because, source) in crystals {
        for x in p
            .parts
            .iter()
            .filter(|x| x.id != id && x.pins.values().any(|n| *n == net))
        {
            let Some(got) = x.catalog.as_deref().and_then(frequency_hz) else {
                continue;
            };
            if (got - hz as f64).abs() > hz as f64 * 1e-4 {
                problems.push(format!(
                    "part `{id}` ({mpn}): `{}` on {net} runs at {} MHz, but it needs {} MHz — \
                     {because} — {source}",
                    x.id,
                    got / 1e6,
                    hz as f64 / 1e6
                ));
            }
        }
    }
    problems.extend(unpulled_i2c(p));
    if !problems.is_empty() {
        bail!("{}", problems.join("\n"));
    }
    Ok(())
}

/// I²C lines with no pull-up. The bus is open-drain: nothing drives a line
/// high but a resistor to a supply, so a line without one never reads high
/// and the bus never works. A line is a net on a pin called SDA or SCL; a
/// pull-up is a two-pin resistor between it and a rail (`[rails]`, or the
/// board's `power_nets` other than ground). In the paper's round 4 a copied
/// pull-up kept its source's net, and SCL was left with none (R18).
fn unpulled_i2c(p: &Product) -> Vec<String> {
    let is_bus_pin = |k: &str| {
        let k = k.to_ascii_uppercase();
        let k = k.trim_start_matches("I2C_").trim_start_matches("I2C");
        ["SDA", "SCL"].iter().any(|b| {
            k.strip_prefix(b)
                .is_some_and(|r| r.chars().all(|c| c.is_ascii_digit()))
        })
    };
    let supplies: Vec<String> = if p.rails.is_empty() {
        p.board
            .power_nets
            .clone()
            .into_iter()
            .filter(|n| crate::devices::level(n, &BTreeMap::new()).is_none())
            .collect()
    } else {
        p.rails
            .iter()
            .filter(|(_, v)| **v > 0.0)
            .map(|(n, _)| n.clone())
            .collect()
    };
    if supplies.is_empty() {
        return Vec::new();
    }
    let mut lines: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for q in &p.parts {
        for (k, net) in &q.pins {
            if is_bus_pin(k) {
                lines.entry(net.as_str()).or_default().push(q.id.as_str());
            }
        }
    }
    let pulled = |net: &str| {
        p.parts.iter().any(|q| {
            q.value.as_deref().and_then(crate::circuit::ohms).is_some()
                && q.pins.len() == 2
                && q.pins.values().any(|n| n == net)
                && q.pins.values().any(|n| supplies.contains(n))
        })
    };
    lines
        .into_iter()
        .filter(|(net, _)| !pulled(net))
        .map(|(net, on)| {
            format!(
                "I²C line {net} (on {}) has no pull-up resistor to a supply — an open-drain \
                 line nothing pulls high never reads high; add one, or check a copied \
                 resistor's nets",
                on.join(", ")
            )
        })
        .collect()
}

/// Parse and solve the declaration at `root/decl`.
pub fn load(root: &Path, decl: &str) -> Result<(Product, Solved)> {
    let raw =
        std::fs::read_to_string(root.join(decl)).with_context(|| format!("reading {decl}"))?;
    // `{:#}` so the reason reaches the terminal: derive prints only the
    // outermost message, and "hardware/product.toml" alone says nothing.
    let mut product: Product = toml::from_str(&raw).map_err(|e| anyhow!("{decl}: {e}"))?;
    resolve_components(root, &mut product).map_err(|e| anyhow!("{decl}: {e:#}"))?;
    resolve_picks(root, &mut product).map_err(|e| anyhow!("{decl}: {e:#}"))?;
    datasheet_checks(&mut product).map_err(|e| anyhow!("{decl}: {e:#}"))?;
    let solved = solve(root, &product).map_err(|e| anyhow!("{decl}: {e:#}"))?;
    Ok((product, solved))
}

// ── The interface: what the other disciplines read from the hardware ─────────

/// A net's name as a Rust or TypeScript identifier: `SENSOR_SDA` stays,
/// `+3V3` becomes `NET_3V3`.
fn net_ident(net: &str) -> String {
    let mut out = String::new();
    for c in net.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("NET_{out}")
    } else {
        out
    }
}

/// The field of an Embassy `Peripherals` a pin name is: `GPIO4` → `PIN_4`
/// (RP2040), `PA5` → `PA5` (STM32). None for a pin that is not an I/O.
fn peripheral(pin: &str) -> Option<String> {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if let Some(n) = pin.strip_prefix("GPIO").filter(|n| digits(n)) {
        return Some(format!("PIN_{n}"));
    }
    let mut c = pin.chars();
    match (c.next(), c.next()) {
        (Some('P'), Some(port)) if ('A'..='K').contains(&port) && digits(c.as_str()) => {
            Some(pin.to_string())
        }
        _ => None,
    }
}

/// The directory firmware lives in, relative to the product root.
pub const FIRMWARE_DIR: &str = "firmware";

/// Places firmware names an MCU pin directly — `p.PIN_4`, `p.PA5` — instead of
/// taking it through the derived `board.rs`.
///
/// A literal pin compiles before a pin moves and after it, so the move
/// reaches every caller of `board::sensor_sda!(p)` and silently skips this
/// one (the paper's study, case H8). The escape is explicit: a line that
/// carries `fid: allow-pin` is a pin no declared net owns, on purpose.
pub fn firmware_pin_literals(root: &Path, p: &Product) -> Vec<String> {
    if p.firmware.is_none() {
        return Vec::new();
    }
    // The device profiles of the parts on this board: their commands are
    // derived into board.rs, so a byte sequence typed in firmware is a copy.
    let profiles: Vec<(String, crate::devices::Profile)> = p
        .parts
        .iter()
        .filter_map(|q| {
            let d = crate::devices::for_mpn(q.mpn.as_deref()?).ok()??;
            Some((q.id.clone(), d))
        })
        .collect();
    let addresses_declared = p.parts.iter().any(|q| q.i2c_address.is_some())
        || profiles.iter().any(|(_, d)| d.i2c_address.is_some());
    let commands: Vec<(String, String, &str)> = profiles
        .iter()
        .flat_map(|(id, d)| {
            d.commands
                .keys()
                .map(move |c| (c.to_ascii_uppercase(), id.clone(), d.mpn.as_str()))
        })
        .collect();
    let timings: Vec<(String, String, &str)> = profiles
        .iter()
        .flat_map(|(id, d)| {
            d.timing
                .keys()
                .map(move |c| (c.to_ascii_uppercase(), id.clone(), d.mpn.as_str()))
        })
        .collect();
    let mut files = Vec::new();
    collect_rs(&root.join(FIRMWARE_DIR), FIRMWARE_DIR, &mut files);
    let mut found = Vec::new();
    for rel in files {
        let Ok(src) = std::fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        for (i, line) in src.lines().enumerate() {
            if line.contains("fid: allow-pin") {
                continue;
            }
            let code = line.split("//").next().unwrap_or("");
            // An I²C address typed into a transfer — `write_async(0x38, …)`
            // — instead of taken from a part's declared `i2c_address`.
            if addresses_declared && !line.contains("fid: allow-address") {
                for call in [
                    "write(",
                    "read(",
                    "write_read(",
                    "write_async(",
                    "read_async(",
                    "write_read_async(",
                ] {
                    let mut rest = code;
                    while let Some(at) = rest.find(call) {
                        let arg = rest[at + call.len()..].trim_start();
                        if arg.starts_with("0x") || arg.starts_with("0X") {
                            let lit: String = arg
                                .chars()
                                .take_while(|c| c.is_ascii_alphanumeric())
                                .collect();
                            found.push(format!(
                                "  {rel}:{}: addresses an I²C device as {lit} directly — take it from \
                                 `board::<PART>_I2C_ADDRESS` (the part's declared `i2c_address`), or \
                                 mark the line `// fid: allow-address`",
                                i + 1
                            ));
                        }
                        // …or the bytes sent typed in: `write(addr, &[0xE1, …])`.
                        if !commands.is_empty() {
                            let args = &rest[at + call.len()..];
                            if let Some((_, second)) = args.split_once(',') {
                                let second = second.trim_start().trim_start_matches('&');
                                if second.starts_with('[')
                                    && second[1..]
                                        .trim_start()
                                        .starts_with(|c: char| c.is_ascii_digit())
                                {
                                    found.push(format!(
                                        "  {rel}:{}: sends a device bytes typed in place — take the command \
                                         from `board::<PART>_<COMMAND>` (the part's device profile), or mark \
                                         the line `// fid: allow-address`",
                                        i + 1
                                    ));
                                }
                            }
                        }
                        rest = &rest[at + call.len()..];
                    }
                }
            }
            // A part's command written out by hand: `const INIT: [u8; 3] = [0xE1, …]`
            // beside the profile's `board::SENSOR_INIT` (the paper's round 4, R15).
            let decl = code.trim_start();
            let decl = decl.strip_prefix("pub ").unwrap_or(decl);
            if let Some(rest) = decl.strip_prefix("const ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                let typed = rest
                    .split_once('=')
                    .is_some_and(|(_, v)| v.trim_start().starts_with('['));
                let number = rest
                    .split_once('=')
                    .is_some_and(|(_, v)| v.trim_start().starts_with(|c: char| c.is_ascii_digit()));
                if let Some((_, id, mpn)) = timings.iter().find(|(c, _, _)| *c == name) {
                    if number && !line.contains("fid: allow-timing") {
                        found.push(format!(
                            "  {rel}:{}: writes out {mpn}'s `{name}` wait by hand — use \
                             `board::{}_{name}`, from the part's datasheet profile, or mark the \
                             line `// fid: allow-timing`",
                            i + 1,
                            net_ident(id)
                        ));
                    }
                }
                if let Some((_, id, mpn)) = commands.iter().find(|(c, _, _)| *c == name) {
                    if typed && !line.contains("fid: allow-command") {
                        found.push(format!(
                            "  {rel}:{}: writes out {mpn}'s `{name}` command by hand — use \
                             `board::{}_{name}`, from the part's datasheet profile, or mark the \
                             line `// fid: allow-command`",
                            i + 1,
                            net_ident(id)
                        ));
                    }
                }
            }
            for tok in code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                let gpio = tok.strip_prefix("PIN_").map(|n| format!("GPIO{n}"));
                if peripheral(gpio.as_deref().unwrap_or(tok)).as_deref() == Some(tok) {
                    found.push(format!(
                        "  {rel}:{}: takes {tok} directly — take it through `board::<net>!(p)` \
                         (hardware/generated/board.rs) so a moved pin moves it too, or mark the \
                         line `// fid: allow-pin` if no declared net owns it",
                        i + 1
                    ));
                }
            }
        }
    }
    found
}

fn collect_rs(dir: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().to_string();
        let child = format!("{rel}/{name}");
        match e.file_type() {
            Ok(t) if t.is_dir() && name != "target" && !name.starts_with('.') => {
                collect_rs(&e.path(), &child, out)
            }
            Ok(t) if t.is_file() && name.ends_with(".rs") => out.push(child),
            _ => {}
        }
    }
}

/// What firmware and web need from the hardware, and nothing else: the
/// board's size, the sockets reached through the case, every net and the
/// pins on it, and — with `[firmware]` — which I/O pin of the MCU each net is
/// on. Derived from the solved layout, so a renamed net or a moved pin
/// reaches firmware and web in the same `fid derive` that moves the copper.
pub fn interface(p: &Product, solved: &Solved) -> Result<Value> {
    let l = &solved.layout;
    let placed = l["board"]["illustrative_placement"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut nets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut parts = serde_json::Map::new();
    for it in &placed {
        let Some(pin_nets) = it["pin_nets"].as_object().filter(|m| !m.is_empty()) else {
            continue;
        };
        let reference = it["ref"].as_str().unwrap_or_default().to_string();
        let mut pins = serde_json::Map::new();
        for (pad, net) in pin_nets {
            let net = net.as_str().unwrap_or_default();
            let name = it["pin_names"]
                .get(pad)
                .and_then(Value::as_str)
                .unwrap_or(pad);
            pins.insert(pad.clone(), json!({ "name": name, "net": net }));
            let member = format!("{reference}.{name}");
            let on = nets.entry(net.to_string()).or_default();
            if !on.contains(&member) {
                on.push(member);
            }
        }
        parts.insert(reference, json!({ "part": it["part"], "pins": pins }));
    }
    let firmware = match &p.firmware {
        None => Value::Null,
        Some(f) => {
            let mcu = placed
                .iter()
                .find(|it| it["part"] == f.mcu.as_str())
                .ok_or_else(|| anyhow!("[firmware] mcu = \"{}\" is not a board part", f.mcu))?;
            if mcu["pin_names"].is_null() {
                bail!("[firmware] mcu = \"{}\": the part has no symbol, so its pins have no names to drive", f.mcu);
            }
            let mut pins: BTreeMap<String, String> = BTreeMap::new();
            let mut idents: BTreeMap<String, String> = BTreeMap::new();
            for (pad, net) in mcu["pin_nets"].as_object().into_iter().flatten() {
                let (Some(net), Some(name)) = (net.as_str(), mcu["pin_names"][pad].as_str()) else {
                    continue;
                };
                if peripheral(name).is_none() {
                    continue;
                }
                if let Some(other) = pins.insert(net.to_string(), name.to_string()) {
                    if other != name {
                        bail!("[firmware] net `{net}` is on two pins of `{}` ({other}, {name}) — firmware drives a net from one", f.mcu);
                    }
                }
                if let Some(other) = idents.insert(net_ident(net), net.to_string()) {
                    if other != net {
                        bail!("[firmware] nets `{other}` and `{net}` are both `{}` in code — rename one", net_ident(net));
                    }
                }
            }
            json!({ "mcu": f.mcu, "pins": pins })
        }
    };
    let connectors: Vec<Value> = l["case"]["openings"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|o| json!({ "part": o["part"], "side": o["side"], "opening_mm": o["size_mm"] }))
        .collect();
    Ok(json!({
        "generated_by": "fid-hardware from hardware/product.toml — do not edit",
        "product": p.product.name,
        "board_mm": {
            "size": l["board"]["body"]["size_mm"],
            "thickness": l["board"]["thickness_mm"],
        },
        "connectors": connectors,
        "nets": nets,
        "parts": parts,
        "firmware": firmware,
    }))
}

/// The interface as TypeScript: a web app imports the nets as a type, so a
/// renamed net fails its typecheck where it is still used.
fn render_interface_ts(i: &Value) -> String {
    let lit = |v: &Value| serde_json::to_string(v).unwrap_or_else(|_| "null".into());
    let nets: Vec<String> = i["nets"]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let mut out = String::from(
        "// Derived by fid-hardware from hardware/product.toml — do not edit.\n\
         // A renamed net or a moved pin changes this file in the same `fid derive`.\n\n",
    );
    out += &format!("export const NETS = {} as const;\n", lit(&json!(nets)));
    out += "export type Net = (typeof NETS)[number];\n\n";
    out += "/** The MCU pin each net is on, for nets the firmware drives. */\n";
    out += &format!(
        "export const MCU_PINS = {} as const;\n\n",
        lit(i["firmware"].get("pins").unwrap_or(&json!({})))
    );
    out += "/** Sockets reached through the case: the part, the wall, the window. */\n";
    out += &format!(
        "export const CONNECTORS = {} as const;\n\n",
        lit(&i["connectors"])
    );
    out += &format!(
        "export const BOARD_MM = {} as const;\n",
        lit(&i["board_mm"])
    );
    out
}

/// The interface as Rust, for the firmware: per net on an MCU I/O pin, a
/// constant naming the pin and a macro taking it from Embassy's peripherals
/// — `board::sensor_sda!(p)` is `p.PIN_4`. Moving the net to another pin
/// changes the macro and the firmware follows with no edit; renaming the net
/// removes the old macro, so code still using it stops compiling.
fn render_board_rs(p: &Product, i: &Value) -> Result<String> {
    let Some(f) = &p.firmware else {
        bail!("board.rs needs `[firmware] mcu = \"<part id>\"` in hardware/product.toml");
    };
    let mut out = format!(
        "//! Derived by fid-hardware from hardware/product.toml — do not edit.\n\
         //!\n\
         //! The I/O pins of `{}`, by the net each is on. Include it from the\n\
         //! firmware with `#[path = \"…/hardware/generated/board.rs\"] mod board;`\n\
         //! and take a pin with `board::sensor_sda!(p)`.\n\
         #![allow(unused_macros, unused_imports, dead_code)]\n",
        f.mcu
    );
    for (net, pin) in i["firmware"]["pins"].as_object().into_iter().flatten() {
        let pin = pin.as_str().unwrap_or_default();
        let field = peripheral(pin).unwrap_or_default();
        let id = net_ident(net);
        let mac = id.to_ascii_lowercase();
        out += &format!(
            "\n/// `{net}` — {pin}.\npub const {id}: &str = \"{pin}\";\n\
             macro_rules! {mac} {{\n    ($p:expr) => {{\n        $p.{field}\n    }};\n}}\n\
             pub(crate) use {mac};\n"
        );
    }
    for q in p.parts.iter().filter(|q| q.i2c_address.is_some()) {
        let addr = q.i2c_address.unwrap_or_default();
        let id = net_ident(&q.id);
        out += &format!(
            "\n/// `{}`{}: its I²C address.\npub const {id}_I2C_ADDRESS: u8 = 0x{addr:02X};\n",
            q.id,
            q.mpn
                .as_deref()
                .map(|m| format!(" ({m})"))
                .unwrap_or_default()
        );
    }
    for q in &p.parts {
        for (name, ms) in &q.timing {
            out += &format!(
                "\n/// `{}` ({}): `{name}`, from its device profile.\n\
                 pub const {}_{}: u64 = {ms};\n",
                q.id,
                q.mpn.as_deref().unwrap_or_default(),
                net_ident(&q.id),
                name.to_ascii_uppercase()
            );
        }
        for (name, bytes) in &q.commands {
            out += &format!(
                "\n/// `{}` ({}): the `{name}` command, from its device profile.\n\
                 pub const {}_{}: [u8; {}] = [{}];\n",
                q.id,
                q.mpn.as_deref().unwrap_or_default(),
                net_ident(&q.id),
                name.to_ascii_uppercase(),
                bytes.len(),
                bytes
                    .iter()
                    .map(|b| format!("0x{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    Ok(out)
}

/// KiCad's default copper-to-board-edge clearance: the routed board's DRC
/// refuses copper closer to the edge than this.
const COPPER_TO_EDGE_MM: f64 = 0.5;

/// How far a part may hang past the board edge its `faces` side is turned
/// to: from its body's face to its nearest pad or hole on that side, less
/// KiCad's 0.5 mm copper-to-edge clearance, which the board's DRC holds. A USB socket's shell reaches out into the
/// case wall this far, so its mouth comes flush with the case's face.
/// `faces` is as drawn at 0° with y up; KiCad's footprint y points down.
fn overhang_allowed(fp: &crate::kicad::Footprint, faces: &str) -> f64 {
    let ((x0, y0), (x1, y1)) = fp.courtyard;
    let gap = fp
        .pads
        .iter()
        .map(|p| {
            let (hw, hh) = (p.size.0 / 2.0, p.size.1 / 2.0);
            match faces {
                "bottom" => y1 - (p.at.1 + hh),
                "top" => (p.at.1 - hh) - y0,
                "left" => (p.at.0 - hw) - x0,
                _ => x1 - (p.at.0 + hw),
            }
        })
        .fold(f64::INFINITY, f64::min);
    if gap.is_finite() {
        // 0.05 mm under the rule, then down to a tenth: placement rounds
        // to the micron, and DRC measures an oval pad's end exactly.
        ((gap - COPPER_TO_EDGE_MM - 0.05) * 10.0).floor().max(0.0) / 10.0
    } else {
        0.0
    }
}

/// A hand-routed board must still be the declared one: the same parts, at
/// the same places and turns, on the same sides, every pad on its net, the
/// same outline. The routing — tracks, vias, zones — is the person's; the
/// rest follows from `product.toml`, and a move made in KiCad instead of
/// there is drift like any other.
fn check_hand_routed(derived: &str, root: &Path) -> Result<()> {
    let routed = std::fs::read_to_string(root.join(HAND_ROUTED)).map_err(|_| {
        anyhow!(
            "board.routing = \"hand\" and there is no {HAND_ROUTED}: copy hardware/generated/board.kicad_pcb there, route it in KiCad, and commit it"
        )
    })?;
    let diffs = crate::kicad::board_differences(derived, &routed)?;
    if !diffs.is_empty() {
        bail!(
            "{HAND_ROUTED} no longer matches the declaration — change product.toml, not the board, then re-route what moved:\n  {}",
            diffs.join("\n  ")
        );
    }
    Ok(())
}

/// What `fid-hardware` writes for one output path, chosen by file name.
pub fn render_output(out: &str, product: &Product, solved: &Solved, root: &Path) -> Result<String> {
    let file = Path::new(out)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    Ok(match file {
        "layout.json" => serde_json::to_string_pretty(&solved.layout)? + "\n",
        "floor-model.json" | "floor.json" => match &solved.floor {
            Some((model, answer)) => if file == "floor.json" { answer.clone() } else { model.clone() },
            None => "{}\n".to_string(),
        },
        "placement-model.json" | "placement.json" => match &solved.placement {
            Some((model, answer)) => if file == "placement.json" { answer.clone() } else { model.clone() },
            None => "{}\n".to_string(),
        },
        "bom.csv" => render_bom(solved),
        "interface.json" => serde_json::to_string_pretty(&interface(product, solved)?)? + "\n",
        "interface.ts" => render_interface_ts(&interface(product, solved)?),
        "board.rs" => render_board_rs(product, &interface(product, solved)?)?,
        "assembly.md" => render_assembly(solved, &product.product.name),
        f if f.ends_with(".kicad_pcb") => {
            let derived = render_kicad(solved, root)?;
            if product.board.routing == "hand" {
                check_hand_routed(&derived, root)?;
            }
            derived
        }
        f if f.ends_with(".kicad_sch") => render_schematic(solved, root, &product.product.name)?,
        other => bail!("fid-hardware: unknown output `{other}` (layout.json, floor-model.json, floor.json, placement-model.json, placement.json, bom.csv, assembly.md, interface.json, interface.ts, board.rs, *.kicad_pcb, *.kicad_sch)"),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_component_value_reads_the_same_from_a_declaration_and_from_lcsc() {
        use super::component_quantity as q;
        assert_eq!(q("5.1k", false), Some(5100.0));
        assert_eq!(q("27R", false), Some(27.0));
        assert_eq!(
            q("5.1kΩ ±1% 100mW 0603 Thick Film Resistor", true),
            Some(5100.0)
        );
        assert_eq!(q("1uF 50V X5R", false), Some(1e-6));
        assert_eq!(
            q("1uF ±10% 50V Ceramic Capacitor X5R 0603", true),
            Some(1e-6)
        );
        assert!((q("100nF ±10% 50V", true).unwrap() - 1e-7).abs() < 1e-15);
        assert!((q("33pF 50V C0G", false).unwrap() - 33e-12).abs() < 1e-18);
        // A description with no component quantity is not compared.
        assert_eq!(q("USB-C Receptacle Connector 16 Position", true), None);
    }

    use super::*;

    #[test]
    fn the_floor_region_reaches_the_wall_to_a_tenth() {
        // The sensor stick's cavity and board: the board's lowest centre is
        // 1.9 mm from the wall plus half its height, 41.4 mm — not the next
        // half-millimetre row.
        let outline = [
            (0.0, 82.8),
            (0.0, 3.6),
            (3.6, 0.0),
            (32.4, 0.0),
            (36.0, 3.6),
            (36.0, 82.8),
            (34.8, 84.0),
            (1.2, 84.0),
        ];
        let rs = feasible_centres(&outline, (26.0, 79.0), 1.9);
        let low = rs.iter().map(|(l, _)| l.1).fold(f64::INFINITY, f64::min);
        let high = rs
            .iter()
            .map(|(_, h)| h.1)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((low - 41.4).abs() < 0.05, "{rs:?}");
        assert!(high > 42.5, "{rs:?}");
    }

    #[test]
    fn a_covered_target_costs_nothing_and_an_uncovered_one_its_gap() {
        let r = Rect {
            centre: (0.0, 0.0),
            size: (40.0, 40.0),
        };
        // Centre distance when no margin is asked for.
        assert!((miss(&r, (3.0, 4.0), 0.0) - 5.0).abs() < 1e-9);
        // 10 mm inside a 20 mm half-size: covered with room for the pads.
        assert_eq!(miss(&r, (10.0, -10.0), 8.0), 0.0);
        // 15 mm out with an 8 mm margin: 3 mm short of the shrunk edge.
        assert!((miss(&r, (15.0, 0.0), 8.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn path_parser_reads_absolute_relative_and_implicit_lineto() {
        let s = path_subpaths("M0,0 L10,0 v10 h-10 Z M2,2 l1,0 1,1 z").unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(
            s[0],
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        );
        assert_eq!(s[1], vec![(2.0, 2.0), (3.0, 2.0), (4.0, 3.0)]);
    }

    #[test]
    fn a_curve_is_refused_by_name() {
        let e = path_subpaths("M0,0 C1,1 2,2 3,3 Z")
            .unwrap_err()
            .to_string();
        assert!(e.contains("`C`"), "{e}");
    }

    #[test]
    fn numbers_split_on_signs_and_second_dots() {
        assert_eq!(numbers("1.5.5-2e-1,3"), vec![1.5, 0.5, -0.2, 3.0]);
    }

    #[test]
    fn group_transforms_and_layers_apply() {
        let svg = r#"<svg viewBox="0 0 10 10"><!-- a -- comment --><g data-layer="x" transform="translate(1,2) scale(2)"><path d="M0,0 H1 V1 Z"/></g><polygon points="0 0 1 0 1 1"/></svg>"#;
        let (items, _, h) = svg_items(svg).unwrap();
        assert_eq!(h, 10.0);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].layer.as_deref(), Some("x"));
        assert_eq!(
            items[0].subpaths[0],
            vec![(1.0, 2.0), (3.0, 2.0), (3.0, 4.0)]
        );
        assert_eq!(items[1].layer, None);
    }

    /// The capability's seed declaration plus `extra` parts and tables.
    fn declared(extra: &str) -> Product {
        let seed = include_str!("../capabilities/hardware/declarations/hardware/product.toml");
        toml::from_str(&format!("{seed}\n{extra}")).unwrap()
    }

    const BUS: &str = r#"
[rails]
VBUS  = 5.0
"3V3" = 3.3
"1V8" = 1.8

[[part]]
id = "mcu2"
name = "RP2040"
place = "board"
mount = "smd"
mpn = "RP2040"
pins = { IOVDD = "3V3", GPIO4 = "SDA", GPIO5 = "SCL" }

[[part]]
id = "sensor"
name = "AHT20"
place = "board"
mount = "smd"
mpn = "AHT20"
pins = { VDD = "3V3", SDA = "SDA", SCL = "SCL" }

[[part]]
id = "r-sda"
name = "SDA pull-up"
place = "board"
mount = "smd"
value = "4.7k"
pins = { 1 = "SDA", 2 = "3V3" }

[[part]]
id = "r-scl"
name = "SCL pull-up"
place = "board"
mount = "smd"
value = "4.7k"
pins = { 1 = "SCL", 2 = "3V3" }

[[part]]
id = "buf"
name = "Buffer"
place = "board"
mount = "smd"
mpn = "SN74AHCT1G125DBVR"
pins = { 1 = "GND", 2 = "A", 3 = "GND", 4 = "Y", 5 = "VBUS" }
"#;

    #[test]
    fn a_profile_fills_in_what_the_datasheet_says() {
        let mut p = declared(BUS);
        datasheet_checks(&mut p).unwrap();
        let get = |id: &str| p.parts.iter().find(|q| q.id == id).unwrap();
        assert_eq!(get("mcu2").io_max_v, Some(3.8));
        assert_eq!(get("sensor").i2c_address, Some(0x38));
        assert_eq!(get("sensor").commands["init"], vec![0xBE, 0x08, 0x00]);
    }

    #[test]
    fn a_belief_the_datasheet_contradicts_is_refused_naming_it() {
        let cases = [
            // R17: the limit raised to let a 5 V pull-up through.
            (
                "pins = { IOVDD = \"3V3\", GPIO4",
                "io_max_v = 5.5\npins = { IOVDD = \"3V3\", GPIO4",
                "no pin may go above 3.3 + 0.5 = 3.8 V",
            ),
            // R16: an active-low enable tied high.
            (
                "{ 1 = \"GND\", 2 = \"A\"",
                "{ 1 = \"VBUS\", 2 = \"A\"",
                "pin 1 (OE, active low) is tied to VBUS",
            ),
            // A part run off the wrong supply.
            (
                "{ VDD = \"3V3\", SDA",
                "{ VDD = \"1V8\", SDA",
                "on 1V8 at 1.8 V, outside 2–5.5 V",
            ),
            // An address the datasheet does not give.
            (
                "mpn = \"AHT20\"",
                "mpn = \"AHT20\"\ni2c_address = 0x39",
                "the datasheet's address is 0x38",
            ),
            // R18: a copied pull-up that kept its source's net.
            (
                "pins = { 1 = \"SCL\", 2 = \"3V3\" }",
                "pins = { 1 = \"SDA\", 2 = \"3V3\" }",
                "I²C line SCL (on sensor) has no pull-up",
            ),
        ];
        for (from, to, says) in cases {
            assert!(BUS.contains(from), "{from}");
            let mut p = declared(&BUS.replacen(from, to, 1));
            let e = format!("{:#}", datasheet_checks(&mut p).unwrap_err());
            assert!(e.contains(says), "{to}: {e}");
        }
    }

    #[test]
    fn a_datasheet_command_typed_into_firmware_is_named() {
        let p = declared(&format!("{BUS}\n[firmware]\nmcu = \"mcu2\"\n"));
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("firmware/shared/src");
        std::fs::create_dir_all(&src).unwrap();
        let lib = src.join("lib.rs");
        std::fs::write(
            &lib,
            "pub mod aht20 {\n    pub use crate::board::SENSOR_INIT as INIT;\n}\n",
        )
        .unwrap();
        assert!(firmware_pin_literals(dir.path(), &p).is_empty());
        // R15: an older part's init, written out by hand.
        std::fs::write(
            &lib,
            "pub mod aht20 {\n    pub const INIT: [u8; 3] = [0xE1, 0x08, 0x00];\n}\n",
        )
        .unwrap();
        let found = firmware_pin_literals(dir.path(), &p);
        assert!(
            found[0].contains("lib.rs:2: writes out AHT20's `INIT` command by hand"),
            "{found:?}"
        );
        // …or typed straight into the transfer.
        std::fs::write(
            &lib,
            "fn f(i2c: I) {\n    i2c.write(board::SENSOR_I2C_ADDRESS, &[0xE1, 0x08, 0x00]);\n}\n",
        )
        .unwrap();
        let found = firmware_pin_literals(dir.path(), &p);
        assert!(
            found[0].contains("lib.rs:2: sends a device bytes typed in place"),
            "{found:?}"
        );
    }
}

//! KiCad's own formats, read and written without KiCad.
//!
//! A part's footprint is a `.kicad_mod`: an S-expression holding its pads (its
//! pins, numbered, positioned) and its courtyard (the area it claims). Reading
//! it is what lets `fid-hardware` take a part's size and pin positions from
//! the component itself instead of from someone typing them — and writing it
//! back, placed and turned, is what puts the real component into the board
//! file rather than a drawing of one.

use anyhow::{anyhow, bail, Context, Result};
use std::fmt::Write as _;
use std::path::Path;

/// An S-expression: an atom (remembering whether it was quoted, so it can be
/// written back as it came) or a list.
#[derive(Debug, Clone, PartialEq)]
pub enum S {
    A(String, bool),
    L(Vec<S>),
}

impl S {
    pub fn head(&self) -> Option<&str> {
        match self {
            S::L(v) => match v.first() {
                Some(S::A(s, _)) => Some(s),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn items(&self) -> &[S] {
        match self {
            S::L(v) => v,
            _ => &[],
        }
    }
    pub fn atom(&self) -> Option<&str> {
        match self {
            S::A(s, _) => Some(s),
            _ => None,
        }
    }
    /// Direct children whose head is `name`.
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a S> + 'a {
        self.items().iter().filter(move |c| c.head() == Some(name))
    }
    pub fn child<'a>(&'a self, name: &'a str) -> Option<&'a S> {
        self.children(name).next()
    }
    /// The numbers following the head: `(at 1 2 90)` → [1, 2, 90].
    pub fn nums(&self) -> Vec<f64> {
        self.items()
            .iter()
            .skip(1)
            .filter_map(|c| c.atom()?.parse().ok())
            .collect()
    }
}

pub fn parse(text: &str) -> Result<S> {
    let b = text.as_bytes();
    let mut i = 0;
    let mut stack: Vec<Vec<S>> = vec![Vec::new()];
    while i < b.len() {
        match b[i] {
            b'(' => {
                stack.push(Vec::new());
                i += 1;
            }
            b')' => {
                let done = stack.pop().ok_or_else(|| anyhow!("unbalanced `)`"))?;
                stack
                    .last_mut()
                    .ok_or_else(|| anyhow!("unbalanced `)`"))?
                    .push(S::L(done));
                i += 1;
            }
            b'"' => {
                let mut s = String::new();
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' && i + 1 < b.len() {
                        s.push(b[i + 1] as char);
                        i += 2;
                        continue;
                    }
                    let ch = text[i..].chars().next().unwrap();
                    s.push(ch);
                    i += ch.len_utf8();
                }
                i += 1;
                stack.last_mut().unwrap().push(S::A(s, true));
            }
            c if c.is_ascii_whitespace() => i += 1,
            _ => {
                let start = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'(' && b[i] != b')' {
                    i += 1;
                }
                stack
                    .last_mut()
                    .unwrap()
                    .push(S::A(text[start..i].to_string(), false));
            }
        }
    }
    if stack.len() != 1 {
        bail!("unbalanced `(`");
    }
    stack
        .pop()
        .unwrap()
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("empty S-expression"))
}

pub fn write(s: &S, out: &mut String) {
    match s {
        S::A(a, q) => {
            if *q || a.is_empty() || a.contains([' ', '(', ')', '"']) {
                let _ = write!(out, "\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\""));
            } else {
                out.push_str(a);
            }
        }
        S::L(v) => {
            out.push('(');
            for (k, c) in v.iter().enumerate() {
                if k > 0 {
                    out.push(' ');
                }
                write(c, out);
            }
            out.push(')');
        }
    }
}

fn num(x: f64) -> S {
    let r = (x * 10000.0).round() / 10000.0 + 0.0;
    S::A(format!("{r}"), false)
}

/// A pad: number, centre and size, in footprint coordinates (KiCad: y down).
#[derive(Debug, Clone)]
pub struct Pad {
    pub number: String,
    pub at: (f64, f64),
    pub size: (f64, f64),
    /// A through-hole pad's drill, width × height (a round drill is square):
    /// the hole the part's lead or peg goes into.
    pub drill: Option<(f64, f64)>,
}

/// What `fid-hardware` needs from a footprint.
#[derive(Debug, Clone)]
pub struct Footprint {
    pub name: String,
    /// Courtyard bounds, footprint coordinates: (min, max).
    pub courtyard: ((f64, f64), (f64, f64)),
    pub pads: Vec<Pad>,
    /// The model's file stem, and its offset and rotation.
    pub model: Option<(String, [f64; 3], [f64; 3])>,
    /// Fab-layer bounds — the package body as drawn — when the footprint has one.
    pub fab: Option<((f64, f64), (f64, f64))>,
    pub tree: S,
}

impl Footprint {
    pub fn size(&self) -> (f64, f64) {
        let ((a, b), (c, d)) = self.courtyard;
        (c - a, d - b)
    }
    pub fn centre(&self) -> (f64, f64) {
        let ((a, b), (c, d)) = self.courtyard;
        ((a + c) / 2.0, (b + d) / 2.0)
    }
    /// Which side of the part — `top`, `bottom`, `left`, `right`, as drawn at
    /// 0° with y up — a pad sits nearest.
    pub fn side_of_pad(&self, number: &str) -> Result<&'static str> {
        let p = self
            .pads
            .iter()
            .find(|p| p.number == number)
            .ok_or_else(|| {
                anyhow!(
                    "footprint {} has no pad `{number}` (pads: {})",
                    self.name,
                    self.pads
                        .iter()
                        .map(|p| p.number.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        let (cx, cy) = self.centre();
        let (w, h) = self.size();
        // Normalised by the body's half-size, so a pad on the short side of a
        // long module is still found on that side.
        let (dx, dy) = ((p.at.0 - cx) / (w / 2.0), -(p.at.1 - cy) / (h / 2.0));
        Ok(if dx.abs() >= dy.abs() {
            if dx > 0.0 {
                "right"
            } else {
                "left"
            }
        } else if dy > 0.0 {
            "top"
        } else {
            "bottom"
        })
    }
}

/// The vendored file for a `Library:Name` footprint id.
pub fn footprint_path(id: &str) -> Result<String> {
    let name = id
        .split_once(':')
        .map(|(_, n)| n)
        .ok_or_else(|| anyhow!("footprint `{id}` is not a library id — expected Library:Name"))?;
    Ok(format!("hardware/lib/footprints/{name}.kicad_mod"))
}

pub fn read_footprint(root: &Path, id: &str) -> Result<Footprint> {
    let rel = footprint_path(id)?;
    let text = std::fs::read_to_string(root.join(&rel))
        .with_context(|| format!("footprint {id} is declared but not vendored at {rel} — run `python3 hardware/parts.py sync`"))?;
    let tree = parse(&text).with_context(|| format!("parsing {rel}"))?;
    // KiCad 5's `(module …)`: embedded in a current board it loses its pads'
    // nets, and the board cannot be routed. Refused by name, with the fix.
    if tree.head() == Some("module") {
        bail!(
            "footprint {id} ({rel}) is in KiCad 5's format — convert its library with \
             `kicad-cli fp upgrade --force <lib>.pretty` and sync again"
        );
    }
    let mut pts: Vec<(f64, f64)> = Vec::new();
    let mut fab: Vec<(f64, f64)> = Vec::new();
    for g in tree.items() {
        let layer = g
            .child("layer")
            .and_then(|l| l.items().get(1))
            .and_then(|a| a.atom());
        if layer == Some("F.Fab")
            && matches!(
                g.head(),
                Some("fp_line") | Some("fp_rect") | Some("fp_poly")
            )
        {
            for k in ["start", "end"] {
                if let Some(n) = g.child(k).map(|c| c.nums()) {
                    fab.push((n[0], n[1]));
                }
            }
            if let Some(p) = g.child("pts") {
                for xy in p.children("xy") {
                    let n = xy.nums();
                    fab.push((n[0], n[1]));
                }
            }
        }
        if layer != Some("F.CrtYd") {
            continue;
        }
        match g.head() {
            Some("fp_line") | Some("fp_rect") => {
                for k in ["start", "end"] {
                    if let Some(n) = g.child(k).map(|c| c.nums()) {
                        pts.push((n[0], n[1]));
                    }
                }
            }
            Some("fp_poly") => {
                if let Some(p) = g.child("pts") {
                    for xy in p.children("xy") {
                        let n = xy.nums();
                        pts.push((n[0], n[1]));
                    }
                }
            }
            Some("fp_circle") => {
                if let (Some(c), Some(e)) = (g.child("center"), g.child("end")) {
                    let (c, e) = (c.nums(), e.nums());
                    let r = (e[0] - c[0]).hypot(e[1] - c[1]);
                    pts.extend([(c[0] - r, c[1] - r), (c[0] + r, c[1] + r)]);
                }
            }
            _ => {}
        }
    }
    if pts.is_empty() {
        bail!("footprint {id} has no courtyard (F.CrtYd), so its size cannot be read");
    }
    let pads: Vec<Pad> = tree
        .children("pad")
        .filter_map(|p| {
            let number = p.items().get(1)?.atom()?.to_string();
            let at = p.child("at")?.nums();
            let size = p.child("size")?.nums();
            // `(drill 0.65)` or `(drill oval 0.6 1.2)`.
            let drill = p
                .child("drill")
                .map(|d| d.nums())
                .and_then(|n| match n.as_slice() {
                    [d] => Some((*d, *d)),
                    [w, h, ..] => Some((*w, *h)),
                    _ => None,
                });
            Some(Pad {
                number,
                at: (at[0], at[1]),
                size: (size[0], size[1]),
                drill,
            })
        })
        .collect();
    // The courtyard, and every pad with KiCad's 0.25 mm courtyard margin round
    // it: a courtyard that misses its own pads (libraries converted from other
    // tools have them) would let the placer put a neighbour on that copper.
    // A turned pad is taken at its larger side both ways.
    for (pad, node) in pads.iter().zip(tree.children("pad")) {
        let turned = node
            .child("at")
            .map(|a| a.nums())
            .and_then(|n| n.get(2).copied())
            .unwrap_or(0.0)
            % 180.0
            != 0.0;
        let (hw, hh) = if turned {
            let m = pad.size.0.max(pad.size.1) / 2.0;
            (m, m)
        } else {
            (pad.size.0 / 2.0, pad.size.1 / 2.0)
        };
        pts.push((pad.at.0 - hw - 0.25, pad.at.1 - hh - 0.25));
        pts.push((pad.at.0 + hw + 0.25, pad.at.1 + hh + 0.25));
    }
    let lo = pts
        .iter()
        .fold((f64::MAX, f64::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
    let hi = pts
        .iter()
        .fold((f64::MIN, f64::MIN), |a, p| (a.0.max(p.0), a.1.max(p.1)));
    let model = tree.child("model").and_then(|m| {
        let path = m.items().get(1)?.atom()?;
        let stem = Path::new(path).file_stem()?.to_str()?.to_string();
        let xyz = |k: &str| -> [f64; 3] {
            m.child(k)
                .and_then(|o| o.child("xyz"))
                .map(|x| {
                    let n = x.nums();
                    [n[0], n[1], n[2]]
                })
                .unwrap_or([0.0; 3])
        };
        Some((stem, xyz("offset"), xyz("rotate")))
    });
    let fab = (!fab.is_empty()).then(|| {
        (
            fab.iter()
                .fold((f64::MAX, f64::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1))),
            fab.iter()
                .fold((f64::MIN, f64::MIN), |a, p| (a.0.max(p.0), a.1.max(p.1))),
        )
    });
    Ok(Footprint {
        name: id.to_string(),
        courtyard: (lo, hi),
        pads,
        model,
        fab,
        tree,
    })
}

/// A symbol pin: its number (the footprint pad it lands on), its name, and
/// where it connects, in symbol coordinates (y up) with the direction the pin
/// runs from that point into the body.
#[derive(Debug, Clone)]
pub struct SymPin {
    pub number: String,
    pub name: String,
    pub at: (f64, f64),
    pub angle: f64,
    /// The unit it belongs to; 0 is common to every unit.
    pub unit: u32,
}

/// What `fid-hardware` needs from a symbol.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: String,
    /// The designator letter KiCad gives it: `U`, `R`, `C`, `L`, `J`.
    pub prefix: String,
    pub pins: Vec<SymPin>,
    /// Bounds of its body and pins, symbol coordinates: (min, max).
    pub bounds: ((f64, f64), (f64, f64)),
    /// The library block, verbatim — what a schematic embeds.
    pub block: String,
}

impl Symbol {
    /// The pad numbers a declared pin key means: a pad number as it stands,
    /// or every pin carrying that name (a module's three `GND` pins).
    pub fn resolve(&self, key: &str) -> Vec<String> {
        if self.pins.iter().any(|p| p.number == key) {
            return vec![key.to_string()];
        }
        self.pins
            .iter()
            .filter(|p| {
                p.name == key || p.name.trim_start_matches("~{").trim_end_matches('}') == key
            })
            .map(|p| p.number.clone())
            .collect()
    }
}

pub const SYMBOLS: &str = "hardware/lib/symbols.kicad_sym";

/// Every symbol in the vendored library, by id.
pub fn read_symbols(root: &Path) -> Result<Vec<Symbol>> {
    let text = std::fs::read_to_string(root.join(SYMBOLS))
        .with_context(|| format!("symbols are declared but not vendored at {SYMBOLS} — run `python3 hardware/parts.py sync`"))?;
    let tree = parse(&text).with_context(|| format!("parsing {SYMBOLS}"))?;
    let mut out = Vec::new();
    for sym in tree.children("symbol") {
        let Some(id) = sym.items().get(1).and_then(|a| a.atom()) else {
            continue;
        };
        let mut pins = Vec::new();
        let mut pts: Vec<(f64, f64)> = Vec::new();
        // Units are nested `(symbol "Name_u_s" …)` blocks: unit u, body style
        // s. Style 2 is the De Morgan alternate — the same pins drawn again.
        for unit in sym.children("symbol") {
            let tag = unit.items().get(1).and_then(|a| a.atom()).unwrap_or("");
            let mut parts = tag.rsplitn(3, '_');
            let style: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(1);
            let unit_no: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(1);
            if style > 1 {
                continue;
            }
            for g in unit.items() {
                match g.head() {
                    Some("pin") => {
                        let at = g.child("at").map(|a| a.nums()).unwrap_or_default();
                        let len = g
                            .child("length")
                            .map(|l| l.nums())
                            .and_then(|n| n.first().copied())
                            .unwrap_or(0.0);
                        let name = g
                            .child("name")
                            .and_then(|n| n.items().get(1)?.atom().map(String::from))
                            .unwrap_or_default();
                        let number = g
                            .child("number")
                            .and_then(|n| n.items().get(1)?.atom().map(String::from))
                            .unwrap_or_default();
                        if at.len() < 2 {
                            continue;
                        }
                        let angle = at.get(2).copied().unwrap_or(0.0);
                        let (dx, dy) = (
                            angle.to_radians().cos() * len,
                            angle.to_radians().sin() * len,
                        );
                        pts.extend([(at[0], at[1]), (at[0] + dx, at[1] + dy)]);
                        pins.push(SymPin {
                            number,
                            name,
                            at: (at[0], at[1]),
                            angle,
                            unit: unit_no,
                        });
                    }
                    Some("rectangle") => {
                        for k in ["start", "end"] {
                            if let Some(n) = g.child(k).map(|c| c.nums()) {
                                pts.push((n[0], n[1]));
                            }
                        }
                    }
                    Some("polyline") => {
                        if let Some(p) = g.child("pts") {
                            for xy in p.children("xy") {
                                let n = xy.nums();
                                pts.push((n[0], n[1]));
                            }
                        }
                    }
                    Some("circle") => {
                        if let (Some(c), Some(r)) = (g.child("center"), g.child("radius")) {
                            let (c, r) = (c.nums(), r.nums()[0]);
                            pts.extend([(c[0] - r, c[1] - r), (c[0] + r, c[1] + r)]);
                        }
                    }
                    _ => {}
                }
            }
        }
        if pts.is_empty() {
            pts.push((0.0, 0.0));
        }
        let lo = pts
            .iter()
            .fold((f64::MAX, f64::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
        let hi = pts
            .iter()
            .fold((f64::MIN, f64::MIN), |a, p| (a.0.max(p.0), a.1.max(p.1)));
        let prefix = sym
            .children("property")
            .find(|p| p.items().get(1).and_then(|a| a.atom()) == Some("Reference"))
            .and_then(|p| p.items().get(2)?.atom().map(String::from))
            .unwrap_or_else(|| "U".into());
        let mut block = String::new();
        write(sym, &mut block);
        out.push(Symbol {
            id: id.to_string(),
            prefix,
            pins,
            bounds: (lo, hi),
            block,
        });
    }
    Ok(out)
}

/// The footprint as it goes into a board: placed at `(x, y)` in board
/// coordinates, turned `rot` degrees, its reference set, its model pointed at
/// the vendored STEP, each pad's orientation made absolute (KiCad stores a
/// pad's angle in the board, not relative to its footprint), and each pad on
/// its net: `nets` maps a pad number to the net's number and name.
pub fn place_footprint(
    fp: &Footprint,
    x: f64,
    y: f64,
    rot: f64,
    reference: &str,
    model_rel: Option<&str>,
    nets: &std::collections::BTreeMap<String, (usize, String)>,
) -> String {
    let mut out: Vec<S> = Vec::new();
    let mut placed = false;
    for (k, c) in fp.tree.items().iter().enumerate() {
        if k == 1 {
            out.push(S::A(fp.name.clone(), true));
            continue;
        }
        match c.head() {
            Some("version") | Some("generator") | Some("generator_version") => continue,
            // The footprint's own layer — the first one — is where its
            // placement goes.
            Some("layer") if !placed => {
                placed = true;
                out.push(c.clone());
                let mut at = vec![S::A("at".into(), false), num(x), num(y)];
                if rot != 0.0 {
                    at.push(num(rot));
                }
                out.push(S::L(at));
                continue;
            }
            Some("fp_text") if c.items().get(1).and_then(|a| a.atom()) == Some("reference") => {
                let mut v = c.items().to_vec();
                v[2] = S::A(reference.to_string(), true);
                out.push(S::L(v));
                continue;
            }
            Some("pad") => {
                let number = c.items().get(1).and_then(|a| a.atom()).unwrap_or("");
                let mut v: Vec<S> = c
                    .items()
                    .iter()
                    .filter(|e| e.head() != Some("net"))
                    .map(|e| {
                        if e.head() == Some("at") {
                            let n = e.nums();
                            let a = n.get(2).copied().unwrap_or(0.0) + rot;
                            let mut at = vec![S::A("at".into(), false), num(n[0]), num(n[1])];
                            if a.rem_euclid(360.0) != 0.0 {
                                at.push(num(a.rem_euclid(360.0)));
                            }
                            S::L(at)
                        } else {
                            e.clone()
                        }
                    })
                    .collect();
                if let Some((n, name)) = nets.get(number) {
                    v.push(S::L(vec![
                        S::A("net".into(), false),
                        S::A(n.to_string(), false),
                        S::A(name.clone(), true),
                    ]));
                }
                out.push(S::L(v));
                continue;
            }
            Some("model") => {
                if let Some(rel) = model_rel {
                    let mut v = c.items().to_vec();
                    v[1] = S::A(rel.to_string(), true);
                    out.push(S::L(v));
                }
                continue;
            }
            _ => out.push(c.clone()),
        }
    }
    let mut s = String::new();
    write(&S::L(out), &mut s);
    s
}

/// One footprint on a board, as far as the declaration decides it.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub lib: String,
    pub layer: String,
    pub at: (f64, f64, f64),
    /// Pad number → net name; a pad on no net is absent.
    pub nets: std::collections::BTreeMap<String, String>,
}

/// Footprints by reference, and the outline's extent `[x0, y0, x1, y1]`.
pub type BoardContents = (std::collections::BTreeMap<String, Placed>, Option<[f64; 4]>);

/// What a board file says about what the declaration owns: each footprint by
/// reference, and the outline's extent on `Edge.Cuts`. Tracks, zones, vias
/// and silkscreen are not here — they are the routing, which is the person's.
pub fn board_contents(text: &str) -> Result<BoardContents> {
    let root = parse(text)?;
    let pcb = root
        .items()
        .iter()
        .find(|c| c.head() == Some("kicad_pcb"))
        .unwrap_or(&root);
    let mut parts = std::collections::BTreeMap::new();
    for fp in pcb.children("footprint") {
        let lib = fp
            .items()
            .get(1)
            .and_then(S::atom)
            .unwrap_or("")
            .to_string();
        let layer = fp
            .child("layer")
            .and_then(|l| l.items().get(1)?.atom())
            .unwrap_or("")
            .to_string();
        let a = fp.child("at").map(S::nums).unwrap_or_default();
        let at = (
            a.first().copied().unwrap_or(0.0),
            a.get(1).copied().unwrap_or(0.0),
            a.get(2).copied().unwrap_or(0.0).rem_euclid(360.0),
        );
        // KiCad 6/7 name it in `fp_text reference`; KiCad 8 in a property.
        let reference = fp
            .children("fp_text")
            .find(|t| t.items().get(1).and_then(S::atom) == Some("reference"))
            .and_then(|t| t.items().get(2)?.atom())
            .or_else(|| {
                fp.children("property")
                    .find(|t| t.items().get(1).and_then(S::atom) == Some("Reference"))
                    .and_then(|t| t.items().get(2)?.atom())
            })
            .ok_or_else(|| anyhow!("a footprint `{lib}` has no reference"))?
            .to_string();
        let mut nets = std::collections::BTreeMap::new();
        for pad in fp.children("pad") {
            let num = pad
                .items()
                .get(1)
                .and_then(S::atom)
                .unwrap_or("")
                .to_string();
            if let Some(net) = pad.child("net").and_then(|n| n.items().get(2)?.atom()) {
                if !net.is_empty() && !num.is_empty() {
                    nets.insert(num, net.to_string());
                }
            }
        }
        parts.insert(
            reference,
            Placed {
                lib,
                layer,
                at,
                nets,
            },
        );
    }
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for g in pcb.items() {
        let on_edge = g
            .child("layer")
            .and_then(|l| l.items().get(1)?.atom())
            .is_some_and(|l| l == "Edge.Cuts");
        if !on_edge || !g.head().is_some_and(|h| h.starts_with("gr_")) {
            continue;
        }
        for k in ["start", "end", "mid", "center"] {
            if let Some(v) = g.child(k).map(S::nums) {
                if v.len() >= 2 {
                    pts.push((v[0], v[1]));
                }
            }
        }
        if let Some(list) = g.child("pts") {
            for xy in list.children("xy") {
                let v = xy.nums();
                if v.len() >= 2 {
                    pts.push((v[0], v[1]));
                }
            }
        }
    }
    let outline = (!pts.is_empty()).then(|| {
        let f = |sel: fn(&(f64, f64)) -> f64, min: bool| {
            pts.iter().map(sel).fold(
                if min {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                },
                |a, b| if min { a.min(b) } else { a.max(b) },
            )
        };
        [
            f(|p| p.0, true),
            f(|p| p.1, true),
            f(|p| p.0, false),
            f(|p| p.1, false),
        ]
    });
    Ok((parts, outline))
}

/// Where a hand-routed board no longer matches the derived one: a part moved,
/// turned, flipped, swapped, added or missing, a pad on another net, or the
/// outline changed. Empty when it still follows from the declaration.
pub fn board_differences(derived: &str, routed: &str) -> Result<Vec<String>> {
    const MM: f64 = 0.01;
    const DEG: f64 = 0.1;
    let (want, want_edge) = board_contents(derived)?;
    let (have, have_edge) = board_contents(routed)?;
    let mut out = Vec::new();
    for (r, w) in &want {
        let Some(h) = have.get(r) else {
            out.push(format!("{r} is missing"));
            continue;
        };
        if h.lib != w.lib {
            out.push(format!(
                "{r} is `{}`, the declaration says `{}`",
                h.lib, w.lib
            ));
        }
        if h.layer != w.layer {
            out.push(format!(
                "{r} is on {}, the declaration puts it on {}",
                h.layer, w.layer
            ));
        }
        if (h.at.0 - w.at.0).abs() > MM || (h.at.1 - w.at.1).abs() > MM {
            out.push(format!(
                "{r} moved: at ({}, {}), the declaration places it at ({}, {})",
                h.at.0, h.at.1, w.at.0, w.at.1
            ));
        }
        let turn = (h.at.2 - w.at.2).rem_euclid(360.0);
        if turn.min(360.0 - turn) > DEG {
            out.push(format!(
                "{r} turned: {}°, the declaration says {}°",
                h.at.2, w.at.2
            ));
        }
        for (pad, net) in &w.nets {
            match h.nets.get(pad) {
                Some(n) if n == net => {}
                Some(n) => out.push(format!(
                    "{r} pad {pad} is on {n}, the declaration puts it on {net}"
                )),
                None => out.push(format!(
                    "{r} pad {pad} is on no net, the declaration puts it on {net}"
                )),
            }
        }
        for (pad, net) in &h.nets {
            if !w.nets.contains_key(pad) {
                out.push(format!(
                    "{r} pad {pad} is on {net}, the declaration leaves it unconnected"
                ));
            }
        }
    }
    for r in have.keys().filter(|r| !want.contains_key(*r)) {
        out.push(format!(
            "{r} is not in the declaration — declare it in product.toml"
        ));
    }
    match (want_edge, have_edge) {
        (Some(w), Some(h)) if w.iter().zip(&h).any(|(a, b)| (a - b).abs() > MM) => out.push(
            format!("the outline changed: {:?}, the declaration's is {:?}", h, w),
        ),
        (Some(_), None) => out.push("the outline (Edge.Cuts) is missing".into()),
        _ => {}
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    const KICAD7: &str = r#"(kicad_pcb (version 20221018)
  (gr_rect (start 100 100) (end 130 140) (layer "Edge.Cuts"))
  (footprint "R:R_0402" (layer "F.Cu") (at 110 120 90)
    (fp_text reference "R1" (at 0 0) (layer "F.SilkS"))
    (pad "1" smd rect (at -0.5 0) (size 0.5 0.5) (layers "F.Cu") (net 1 "VBUS"))
    (pad "2" smd rect (at 0.5 0) (size 0.5 0.5) (layers "F.Cu") (net 2 "GND")))
  (segment (start 1 1) (end 2 2) (width 0.2) (layer "F.Cu") (net 1)))"#;

    #[test]
    fn a_board_saved_by_kicad_8_reads_the_same_as_kicad_7() {
        let k8 = KICAD7
            .replace(
                "(fp_text reference \"R1\" (at 0 0) (layer \"F.SilkS\"))",
                "(property \"Reference\" \"R1\" (at 0 0 0) (layer \"F.SilkS\"))",
            )
            .replace("(at 110 120 90)", "(at 110.000001 120 450)");
        assert!(board_differences(KICAD7, &k8).unwrap().is_empty());
    }

    #[test]
    fn routing_is_the_persons_and_everything_else_is_the_declarations() {
        // Tracks added: still the declared board.
        let routed = KICAD7.replace(
            "(segment",
            "(segment (start 3 3) (end 4 4) (width 0.2) (layer \"B.Cu\") (net 2)) (segment",
        );
        assert!(board_differences(KICAD7, &routed).unwrap().is_empty());
        let netted = KICAD7.replace("(net 2 \"GND\")", "(net 1 \"VBUS\")");
        let d = board_differences(KICAD7, &netted).unwrap();
        assert_eq!(
            d,
            vec!["R1 pad 2 is on VBUS, the declaration puts it on GND".to_string()]
        );
        let flipped = KICAD7.replace("(layer \"F.Cu\") (at 110", "(layer \"B.Cu\") (at 110");
        assert!(board_differences(KICAD7, &flipped).unwrap()[0].contains("R1 is on B.Cu"));
        let turned = KICAD7.replace("(at 110 120 90)", "(at 110 120 0)");
        assert!(board_differences(KICAD7, &turned).unwrap()[0].contains("R1 turned"));
        let grown = KICAD7.replace("(end 130 140)", "(end 131 140)");
        assert!(board_differences(KICAD7, &grown).unwrap()[0].contains("outline changed"));
        let extra = KICAD7.replacen("(footprint", "(footprint \"X:Y\" (layer \"F.Cu\") (at 1 1) (fp_text reference \"TP1\" (at 0 0))) (footprint", 1);
        assert!(
            board_differences(KICAD7, &extra).unwrap()[0].contains("TP1 is not in the declaration")
        );
    }

    use super::*;

    const FP: &str = r#"(footprint Test (version 20221018) (generator x)
  (layer F.Cu)
  (fp_text reference "REF**" (at 0 -2) (layer F.SilkS))
  (fp_line (start -2 -1.5) (end 2 -1.5) (stroke (width 0.05) (type solid)) (layer F.CrtYd))
  (fp_line (start 2 -1.5) (end 2 1.5) (stroke (width 0.05) (type solid)) (layer F.CrtYd))
  (fp_line (start 2 1.5) (end -2 1.5) (stroke (width 0.05) (type solid)) (layer F.CrtYd))
  (pad 1 smd rect (at -1.5 0) (size 0.5 1) (layers F.Cu F.Mask))
  (pad ANT smd rect (at 1.5 0) (size 0.5 1) (layers F.Cu F.Mask))
  (model ${KICAD7_3DMODEL_DIR}/Lib.3dshapes/Test.wrl (offset (xyz 0 0 0)) (scale (xyz 1 1 1)) (rotate (xyz 0 0 90))))"#;

    #[test]
    fn a_round_trip_keeps_quoting() {
        let t = parse(r#"(a "b c" d (e "f"))"#).unwrap();
        let mut s = String::new();
        write(&t, &mut s);
        assert_eq!(s, r#"(a "b c" d (e "f"))"#);
    }

    #[test]
    fn a_footprint_reads_its_courtyard_pads_and_model() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hardware/lib/footprints")).unwrap();
        std::fs::write(
            dir.path().join("hardware/lib/footprints/Test.kicad_mod"),
            FP,
        )
        .unwrap();
        let fp = read_footprint(dir.path(), "Lib:Test").unwrap();
        assert_eq!(fp.size(), (4.0, 3.0));
        assert_eq!(fp.pads.len(), 2);
        assert_eq!(fp.side_of_pad("ANT").unwrap(), "right");
        assert_eq!(fp.side_of_pad("1").unwrap(), "left");
        assert_eq!(fp.model.as_ref().unwrap().0, "Test");
        assert_eq!(fp.model.as_ref().unwrap().2, [0.0, 0.0, 90.0]);
        let nets = std::collections::BTreeMap::from([("ANT".to_string(), (3, "RF".to_string()))]);
        let placed = place_footprint(
            &fp,
            110.0,
            105.0,
            90.0,
            "U3",
            Some("../lib/3d/Test.step"),
            &nets,
        );
        assert!(
            placed.starts_with("(footprint \"Lib:Test\" (layer F.Cu) (at 110 105 90)"),
            "{placed}"
        );
        assert!(
            placed.contains("\"U3\"")
                && placed.contains("(at -1.5 0 90)")
                && placed.contains("\"../lib/3d/Test.step\"")
        );
        assert!(
            placed.contains("(net 3 \"RF\")") && placed.matches("(net ").count() == 1,
            "{placed}"
        );
        assert!(!placed.contains("version"));
    }

    #[test]
    fn a_courtyard_that_misses_its_pads_is_grown_to_cover_them() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("hardware/lib/footprints");
        std::fs::create_dir_all(&lib).unwrap();
        // A courtyard 4 × 3 mm, a pad reaching 3 mm right of centre: as
        // libraries converted from other tools draw them.
        std::fs::write(
            lib.join("Short.kicad_mod"),
            FP.replace("(footprint Test", "(footprint Short").replace(
                "(pad ANT smd rect (at 1.5 0) (size 0.5 1)",
                "(pad ANT smd rect (at 2.5 0) (size 1 1)",
            ),
        )
        .unwrap();
        let fp = read_footprint(dir.path(), "Lib:Short").unwrap();
        // Right edge: the pad's 3.0 plus KiCad's 0.25 mm margin; left: the courtyard's -2.
        assert!((fp.size().0 - 5.25).abs() < 1e-9, "{:?}", fp.size());
        assert_eq!(fp.size().1, 3.0);
    }

    #[test]
    fn a_through_hole_pad_carries_its_drill_round_or_oval() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("hardware/lib/footprints");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(
            lib.join("Holes.kicad_mod"),
            FP.replace("(footprint Test", "(footprint Holes").replace(
                "(pad ANT smd rect (at 1.5 0) (size 0.5 1) (layers F.Cu F.Mask))",
                "(pad S1 thru_hole oval (at 1.5 0) (size 1 2.1) (drill oval 0.6 1.7) (layers *.Cu))\n  (pad \"\" np_thru_hole circle (at 0 0) (size 0.65 0.65) (drill 0.65) (layers *.Cu))",
            ),
        )
        .unwrap();
        let fp = read_footprint(dir.path(), "Lib:Holes").unwrap();
        let drill = |n: &str| fp.pads.iter().find(|p| p.number == n).unwrap().drill;
        assert_eq!(drill("S1"), Some((0.6, 1.7)));
        assert_eq!(drill(""), Some((0.65, 0.65)));
        assert_eq!(drill("1"), None);
    }

    #[test]
    fn a_kicad_5_footprint_is_refused_with_its_fix() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("hardware/lib/footprints");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(
            lib.join("Old.kicad_mod"),
            FP.replace("(footprint Test", "(module Old"),
        )
        .unwrap();
        let e = read_footprint(dir.path(), "Lib:Old")
            .unwrap_err()
            .to_string();
        assert!(
            e.contains("KiCad 5") && e.contains("kicad-cli fp upgrade"),
            "{e}"
        );
    }

    #[test]
    fn a_symbol_pin_resolves_by_number_or_by_every_pin_of_that_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("hardware/lib")).unwrap();
        std::fs::write(
            dir.path().join(SYMBOLS),
            r#"(kicad_symbol_lib (version 20220914)
  (symbol "Lib:M" (property "Reference" "U" (at 0 0 0))
    (symbol "M_0_1" (rectangle (start -5 5) (end 5 -5)))
    (symbol "M_1_1"
      (pin power_in line (at -7.54 2.54 0) (length 2.54) (name "GND" (effects)) (number "13" (effects)))
      (pin power_in line (at -7.54 0 0) (length 2.54) (name "GND" (effects)) (number "15" (effects)))
      (pin input line (at 7.54 0 180) (length 2.54) (name "~{EN}" (effects)) (number "5" (effects))))))"#,
        )
        .unwrap();
        let s = read_symbols(dir.path()).unwrap();
        assert_eq!(s[0].id, "Lib:M");
        assert_eq!(s[0].prefix, "U");
        assert_eq!(s[0].resolve("GND"), vec!["13", "15"]);
        assert_eq!(s[0].resolve("15"), vec!["15"]);
        assert_eq!(s[0].resolve("EN"), vec!["5"]);
        assert!(s[0].resolve("VDD").is_empty());
        assert_eq!(s[0].bounds, ((-7.54, -5.0), (7.54, 5.0)));
    }

    #[test]
    fn a_missing_footprint_says_how_to_vendor_it() {
        let dir = tempfile::tempdir().unwrap();
        let e = read_footprint(dir.path(), "Lib:Nope")
            .unwrap_err()
            .to_string();
        assert!(e.contains("parts.py sync"), "{e}");
    }
}

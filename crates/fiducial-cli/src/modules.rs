//! Hardware built from off-the-shelf modules and a harness.
//!
//! A node's hardware declaration with `[board] kind = "modules"` lists modules
//! from the platform's library (`modules/*.toml`, data: pins with their roles,
//! protocol, address, supply, price) and the wires between them. The pin rules
//! of an MCU family are roles in its library entry, never code, and a bus is
//! checked against the wires: a bus declared in `topology.toml` needs a module
//! speaking its protocol wired to the controller. Nothing is added to make a
//! check pass.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::topology::{Node, Point, PointKind, Topology};

const LIBRARY: &[&str] = &[
    include_str!("../modules/buzzer-active-3v3.toml"),
    include_str!("../modules/capacitive-soil-v1.2.toml"),
    include_str!("../modules/contact-aux.toml"),
    include_str!("../modules/ds18b20-probe.toml"),
    include_str!("../modules/esp32-c6-devkitc-1.toml"),
    include_str!("../modules/float-switch-vertical.toml"),
    include_str!("../modules/modbus-soil-probe.toml"),
    include_str!("../modules/modbus-temp-probe.toml"),
    include_str!("../modules/relay-2ch-opto.toml"),
    include_str!("../modules/relay-4ch-opto.toml"),
    include_str!("../modules/rohm-bh1750.toml"),
    include_str!("../modules/seeed-rs485-breakout-xiao.toml"),
    include_str!("../modules/seeed-xiao-esp32c6.toml"),
    include_str!("../modules/sensirion-scd41.toml"),
    include_str!("../modules/supply-5v-din-mp15s.toml"),
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleDef {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub i2c_address: Option<u32>,
    #[serde(default)]
    pub supply: Option<f64>,
    pub source: String,
    pub price: Price,
    pub pins: BTreeMap<String, PinDef>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Price {
    pub rsd: f64,
    pub vendor: String,
    #[serde(default)]
    pub url: Option<String>,
    pub checked: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinDef {
    #[serde(default)]
    pub gpio: Option<u8>,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hardware {
    pub board: Board,
    #[serde(default)]
    pub module: Vec<Module>,
    #[serde(default)]
    pub wire: Vec<Wire>,
    #[serde(default)]
    pub point: Vec<Binding>,
    #[serde(default)]
    pub cost: Option<Cost>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    pub kind: String,
    pub mcu: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub id: String,
    pub part: String,
    #[serde(default)]
    pub price_rsd: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub net: String,
    pub ends: Vec<String>,
    #[serde(default)]
    pub colour: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    #[serde(default)]
    pub bus: Option<String>,
    #[serde(default)]
    pub address: Option<u32>,
    #[serde(default)]
    pub net: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    pub ceiling_rsd: f64,
}

pub fn library() -> Result<BTreeMap<String, ModuleDef>> {
    LIBRARY
        .iter()
        .map(|s| {
            let m: ModuleDef = toml::from_str(s).context("module library entry")?;
            Ok((m.id.clone(), m))
        })
        .collect()
}

pub fn load(root: &Path, rel: &str) -> Result<Hardware> {
    let raw = std::fs::read_to_string(root.join(rel))
        .with_context(|| format!("cannot read hardware declaration `{rel}`"))?;
    let h: Hardware = toml::from_str(&raw).with_context(|| format!("`{rel}` does not parse"))?;
    if h.board.kind != "modules" {
        bail!("`{rel}`: [board] kind = \"{}\" — a topology node's hardware is built from modules (`kind = \"modules\"`)", h.board.kind);
    }
    Ok(h)
}

/// A point as this hardware binds it: the variant's own binding where it
/// declares one, else the topology's.
pub fn effective(p: &Point, h: &Hardware) -> Point {
    let mut q = p.clone();
    if let Some(b) = h.point.iter().find(|b| b.id == p.id) {
        q.bus = b.bus.clone();
        q.address = b.address;
        q.net = b.net.clone();
        q.adc = None;
        q.pin = None;
    }
    q
}

/// Every problem with one node's hardware declaration against the topology
/// and the module library.
pub fn validate(
    t: &Topology,
    node: &Node,
    rel: &str,
    h: &Hardware,
    lib: &BTreeMap<String, ModuleDef>,
) -> Vec<String> {
    let mut out = Vec::new();
    let at = |m: String| format!("{rel}: {m}");
    let modules: BTreeMap<&str, &Module> = h.module.iter().map(|m| (m.id.as_str(), m)).collect();
    for m in &h.module {
        if !lib.contains_key(&m.part) {
            out.push(at(format!(
                "module `{}`: part `{}` is not in the module library ({})",
                m.id,
                m.part,
                lib.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
    }
    let Some(mcu) = modules
        .get(h.board.mcu.as_str())
        .and_then(|m| lib.get(&m.part))
    else {
        out.push(at(format!(
            "[board] mcu = \"{}\" is not a declared module from the library",
            h.board.mcu
        )));
        return out;
    };
    if mcu.kind != "mcu" {
        out.push(at(format!(
            "[board] mcu = \"{}\" is a {}, not an mcu",
            h.board.mcu, mcu.kind
        )));
    }

    let mut mcu_pin_net: BTreeMap<String, String> = BTreeMap::new();
    for w in &h.wire {
        if w.ends.len() < 2 {
            out.push(at(format!("wire `{}` has fewer than two ends", w.net)));
        }
        for end in &w.ends {
            let Some((mid, pin)) = end.split_once('.') else {
                out.push(at(format!(
                    "wire `{}`: end `{end}` is not `<module>.<pin>`",
                    w.net
                )));
                continue;
            };
            let Some(def) = modules.get(mid).and_then(|m| lib.get(&m.part)) else {
                out.push(at(format!(
                    "wire `{}`: `{mid}` is not a declared module",
                    w.net
                )));
                continue;
            };
            if !def.pins.contains_key(pin) {
                out.push(at(format!(
                    "wire `{}`: {} has no pin `{pin}` (it has {})",
                    w.net,
                    def.name,
                    def.pins.keys().cloned().collect::<Vec<_>>().join(", ")
                )));
            }
            if mid == h.board.mcu {
                let roles = &def
                    .pins
                    .get(pin)
                    .map(|p| p.roles.clone())
                    .unwrap_or_default();
                let power = roles.iter().any(|r| r == "power" || r == "ground");
                if !power {
                    if let Some(prev) = mcu_pin_net.insert(pin.to_string(), w.net.clone()) {
                        if prev != w.net {
                            out.push(at(format!(
                                "{} pin {pin} carries both `{prev}` and `{}`",
                                mcu.name, w.net
                            )));
                        }
                    }
                }
            }
        }
    }

    let net_pin = |net: &str| -> Option<String> {
        h.wire
            .iter()
            .filter(|w| w.net == net)
            .flat_map(|w| w.ends.iter())
            .find_map(|e| {
                e.strip_prefix(&format!("{}.", h.board.mcu))
                    .map(str::to_string)
            })
    };
    let role_of = |pin: &str| {
        mcu.pins
            .get(pin)
            .map(|p| p.roles.clone())
            .unwrap_or_default()
    };

    for p in t.point.iter().filter(|p| p.node == node.id) {
        let e = effective(p, h);
        let pin = e
            .net
            .as_deref()
            .and_then(net_pin)
            .or_else(|| e.pin.clone())
            .or_else(|| e.adc.clone());
        if e.bus.is_some() {
            continue;
        }
        let Some(pin) = pin else {
            out.push(at(format!(
                "point `{}`: net `{}` reaches no {} pin",
                p.id,
                e.net.clone().unwrap_or_default(),
                mcu.name
            )));
            continue;
        };
        if !mcu.pins.contains_key(&pin) {
            out.push(at(format!(
                "point `{}`: {} has no pin `{pin}`",
                p.id, mcu.name
            )));
            continue;
        }
        let roles = role_of(&pin);
        let analog = p.kind == PointKind::Sensor;
        if analog && !roles.iter().any(|r| r == "adc") {
            out.push(at(format!(
                "point `{}` is analog, and {} pin {pin} has no ADC — {}",
                p.id, mcu.name, mcu.source
            )));
        }
        if p.kind == PointKind::Actuator {
            if let Some(r) = roles.iter().find(|r| *r == "strapping" || *r == "usb") {
                out.push(at(format!(
                    "point `{}`: an output on {} pin {pin}, a {r} pin, changes how the chip boots or \
                     breaks USB — {}",
                    p.id, mcu.name, mcu.source
                )));
            }
        }
        if p.kind == PointKind::Contact && roles.iter().any(|r| r == "boot-driven") {
            out.push(at(format!(
                "point `{}`: a contact to GND on {} pin {pin}, which the boot ROM drives at power-up, \
                 shorts that output — {}",
                p.id, mcu.name, mcu.source
            )));
        }
    }

    let used_buses: BTreeSet<String> = t
        .point
        .iter()
        .filter(|p| p.node == node.id)
        .filter_map(|p| effective(p, h).bus)
        .collect();
    for bus_id in &used_buses {
        let Some(bus) = t.bus.iter().find(|b| &b.id == bus_id) else {
            out.push(at(format!(
                "a point is bound to bus `{bus_id}`, which topology.toml does not declare"
            )));
            continue;
        };
        if !bus
            .connect
            .iter()
            .any(|e| e.split('.').next() == Some(node.id.as_str()))
        {
            out.push(at(format!(
                "bus `{bus_id}` does not connect node `{}`",
                node.id
            )));
        }
        let speaking: Vec<&Module> = h
            .module
            .iter()
            .filter(|m| m.id != h.board.mcu)
            .filter(|m| {
                lib.get(&m.part).and_then(|d| d.protocol.as_deref()) == Some(bus.kind.as_str())
            })
            .collect();
        let signal = |e: &String| {
            e.split_once('.')
                .and_then(|(mid, pin)| {
                    modules
                        .get(mid)
                        .and_then(|m| lib.get(&m.part))?
                        .pins
                        .get(pin)
                })
                .is_some_and(|p| !p.roles.iter().any(|r| r == "power" || r == "ground"))
        };
        let wired = speaking.iter().any(|m| {
            h.wire.iter().any(|w| {
                w.ends
                    .iter()
                    .any(|e| e.starts_with(&format!("{}.", h.board.mcu)) && signal(e))
                    && w.ends
                        .iter()
                        .any(|e| e.starts_with(&format!("{}.", m.id)) && signal(e))
            })
        });
        if !wired {
            out.push(at(format!(
                "bus `{bus_id}` ({}) on node `{}`: no module speaking {} is wired to `{}` — add the \
                 transceiver or sensor that carries it; nothing is added for you",
                bus.kind, node.id, bus.kind, h.board.mcu
            )));
        }
        if bus.kind == "i2c" {
            for p in t.point.iter().filter(|p| p.node == node.id) {
                let e = effective(p, h);
                if e.bus.as_deref() != Some(bus_id.as_str()) {
                    continue;
                }
                if let Some(a) = e.address {
                    if !speaking
                        .iter()
                        .any(|m| lib.get(&m.part).and_then(|d| d.i2c_address) == Some(a))
                    {
                        out.push(at(format!(
                            "point `{}` is at I²C address 0x{a:02X} on `{bus_id}`, and no module there answers at it",
                            p.id
                        )));
                    }
                }
            }
        }
    }

    for w in &h.wire {
        let rail = w.ends.iter().find_map(|e| {
            let (mid, pin) = e.split_once('.')?;
            let def = modules.get(mid).and_then(|m| lib.get(&m.part))?;
            let power = def.pins.get(pin)?.roles.iter().any(|r| r == "power");
            if !power {
                return None;
            }
            if def.kind == "mcu" {
                return match pin {
                    "3V3" => Some((3.3, e.clone())),
                    "5V" => Some((5.0, e.clone())),
                    _ => None,
                };
            }
            (def.kind == "supply" || def.kind == "breakout")
                .then_some(())
                .and(def.supply.map(|v| (v, e.clone())))
        });
        let Some((volts, from)) = rail else { continue };
        for e in &w.ends {
            let Some((mid, pin)) = e.split_once('.') else {
                continue;
            };
            let Some(def) = modules.get(mid).and_then(|m| lib.get(&m.part)) else {
                continue;
            };
            let power = def
                .pins
                .get(pin)
                .is_some_and(|p| p.roles.iter().any(|r| r == "power"));
            if !power || def.kind == "mcu" || def.kind == "supply" || *e == from {
                continue;
            }
            if let Some(want) = def.supply {
                if (want - volts).abs() > want * 0.1 {
                    out.push(at(format!(
                        "`{mid}` ({}) runs at {want} V, and its {pin} is on `{}` at {volts} V from {from}",
                        def.name, w.net
                    )));
                }
            }
        }
    }

    if let Some(c) = &h.cost {
        let total = bom_total(h, lib);
        if total > c.ceiling_rsd {
            out.push(at(format!(
                "costs {total:.0} RSD in modules, above its ceiling of {:.0} RSD",
                c.ceiling_rsd
            )));
        }
    }
    out
}

pub fn price(m: &Module, lib: &BTreeMap<String, ModuleDef>) -> f64 {
    m.price_rsd
        .or_else(|| lib.get(&m.part).map(|d| d.price.rsd))
        .unwrap_or(0.0)
}

pub fn bom_total(h: &Hardware, lib: &BTreeMap<String, ModuleDef>) -> f64 {
    h.module.iter().map(|m| price(m, lib)).sum()
}

pub fn render_bom(h: &Hardware, lib: &BTreeMap<String, ModuleDef>, rel: &str) -> String {
    let mut by: BTreeMap<&str, (u32, &Module)> = BTreeMap::new();
    for m in &h.module {
        by.entry(m.part.as_str()).or_insert((0, m)).0 += 1;
    }
    let mut s = format!("# Derived by fid-topology from {rel} — do not edit.\npart,name,qty,each_rsd,total_rsd,vendor,url,checked\n");
    for (part, (qty, m)) in &by {
        let Some(d) = lib.get(*part) else { continue };
        let each = price(m, lib);
        s.push_str(&format!(
            "{part},\"{}\",{qty},{each:.0},{:.0},\"{}\",{},{}\n",
            d.name,
            each * *qty as f64,
            d.price.vendor,
            d.price.url.clone().unwrap_or_default(),
            d.price.checked
        ));
    }
    s.push_str(&format!("total,,,,{:.0},,,\n", bom_total(h, lib)));
    s
}

pub fn render_wiring(h: &Hardware, rel: &str) -> String {
    let mut s =
        format!("# Derived by fid-topology from {rel} — do not edit.\nnet,from,to,colour\n");
    for w in &h.wire {
        let Some(first) = w.ends.first() else {
            continue;
        };
        for to in &w.ends[1..] {
            s.push_str(&format!(
                "{},{first},{to},{}\n",
                w.net,
                w.colour.clone().unwrap_or_default()
            ));
        }
    }
    s
}

pub fn render_board_rs(h: &Hardware, lib: &BTreeMap<String, ModuleDef>, rel: &str) -> String {
    let mcu = h
        .module
        .iter()
        .find(|m| m.id == h.board.mcu)
        .and_then(|m| lib.get(&m.part));
    let mut s = format!("// Derived by fid-topology from {rel} — do not edit.\n\n");
    let mut seen = BTreeSet::new();
    for w in &h.wire {
        for e in &w.ends {
            let Some(pin) = e.strip_prefix(&format!("{}.", h.board.mcu)) else {
                continue;
            };
            let Some(gpio) = mcu.and_then(|m| m.pins.get(pin)).and_then(|p| p.gpio) else {
                continue;
            };
            let name = w.net.replace(['-', ' ', '.'], "_").to_ascii_uppercase();
            if seen.insert(name.clone()) {
                s.push_str(&format!("pub const {name}: u8 = {gpio};\n"));
            }
        }
    }
    s
}

fn colour_hex(name: Option<&str>) -> &'static str {
    match name.unwrap_or("") {
        "red" => "#c0392b",
        "black" => "#222222",
        "orange" => "#d35400",
        "yellow" => "#b7950b",
        "green" => "#1e8449",
        "blue" => "#1f5fa8",
        "white" => "#8a8a8a",
        "grey" | "gray" => "#7f8c8d",
        "purple" => "#7d3c98",
        _ => "#555555",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A wiring diagram: the controller on the left with every pin in use, each
/// module on the right with its wired pins, and one line per wire end.
pub fn render_wiring_svg(h: &Hardware, lib: &BTreeMap<String, ModuleDef>, rel: &str) -> String {
    const ROW: f64 = 20.0;
    const PAD: f64 = 28.0;
    let mcu_prefix = format!("{}.", h.board.mcu);
    let mut mcu_pins: Vec<String> = Vec::new();
    let mut mod_pins: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for w in &h.wire {
        for e in &w.ends {
            let Some((m, pin)) = e.split_once('.') else {
                continue;
            };
            if m == h.board.mcu {
                if !mcu_pins.contains(&pin.to_string()) {
                    mcu_pins.push(pin.to_string());
                }
            } else {
                let v = mod_pins.entry(m.to_string()).or_default();
                if !v.contains(&pin.to_string()) {
                    v.push(pin.to_string());
                }
            }
        }
    }
    let order: Vec<&str> = h
        .module
        .iter()
        .map(|m| m.id.as_str())
        .filter(|m| *m != h.board.mcu)
        .collect();
    let mut at: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let (mx, mw) = (40.0, 170.0);
    let my = PAD + 30.0;
    for (i, pin) in mcu_pins.iter().enumerate() {
        at.insert(
            format!("{mcu_prefix}{pin}"),
            (mx + mw, my + 24.0 + i as f64 * ROW),
        );
    }
    let mcu_h = 24.0 + mcu_pins.len() as f64 * ROW;
    let (rx, rw) = (560.0, 190.0);
    let mut y = PAD + 30.0;
    let mut boxes = String::new();
    for m in &order {
        let pins = mod_pins.get(*m).cloned().unwrap_or_default();
        let name = h
            .module
            .iter()
            .find(|x| x.id == *m)
            .and_then(|x| lib.get(&x.part))
            .map(|d| d.name.clone())
            .unwrap_or_default();
        let bh = 24.0 + pins.len().max(1) as f64 * ROW;
        boxes.push_str(&format!(
            "<rect x=\"{rx}\" y=\"{y}\" width=\"{rw}\" height=\"{bh}\" fill=\"#f7f7f7\" stroke=\"#333\"/>\n\
             <text x=\"{}\" y=\"{}\" font-size=\"12\" font-weight=\"bold\">{}</text>\n<text x=\"{}\" y=\"{}\" font-size=\"10\" fill=\"#666\">{}</text>\n",
            rx + 6.0,
            y + 14.0,
            esc(m),
            rx + rw + 6.0,
            y + 14.0,
            esc(&name)
        ));
        for (i, pin) in pins.iter().enumerate() {
            let py = y + 24.0 + i as f64 * ROW;
            at.insert(format!("{m}.{pin}"), (rx, py));
            boxes.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"11\">{}</text>\n",
                rx + 6.0,
                py + 4.0,
                esc(pin)
            ));
        }
        y += bh + 12.0;
    }
    let height = (my + mcu_h).max(y) + PAD;
    let mcu_name = h
        .module
        .iter()
        .find(|m| m.id == h.board.mcu)
        .and_then(|m| lib.get(&m.part))
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\" height=\"{height}\" viewBox=\"0 0 960 {height}\" font-family=\"sans-serif\">\n\
         <!-- Derived by fid-topology from {} — do not edit. -->\n<rect width=\"960\" height=\"{height}\" fill=\"white\"/>\n\
         <rect x=\"{mx}\" y=\"{my}\" width=\"{mw}\" height=\"{mcu_h}\" fill=\"#eef3f8\" stroke=\"#333\"/>\n\
         <text x=\"{}\" y=\"{}\" font-size=\"12\" font-weight=\"bold\">{}</text>\n",
        esc(rel),
        mx + 6.0,
        my + 14.0,
        esc(&mcu_name)
    );
    for (i, pin) in mcu_pins.iter().enumerate() {
        s.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" text-anchor=\"end\">{}</text>\n",
            mx + mw - 6.0,
            my + 28.0 + i as f64 * ROW,
            esc(pin)
        ));
    }
    s.push_str(&boxes);
    for (k, w) in h.wire.iter().enumerate() {
        let colour = colour_hex(w.colour.as_deref());
        let mut pts: Vec<(f64, f64)> = w.ends.iter().filter_map(|e| at.get(e).copied()).collect();
        if let Some(i) = w.ends.iter().position(|e| e.starts_with(&mcu_prefix)) {
            if let Some(p) = at.get(&w.ends[i]).copied() {
                pts.retain(|q| *q != p);
                pts.insert(0, p);
            }
        }
        let Some(&(x0, y0)) = pts.first() else {
            continue;
        };
        let lane = 240.0 + (k as f64 * 17.0) % 280.0;
        let side = rx - 10.0 - (k % 5) as f64 * 5.0;
        for &(x1, y1) in &pts[1..] {
            let mid = if x0 == x1 { side } else { lane };
            s.push_str(&format!(
                "<polyline points=\"{x0},{y0} {mid},{y0} {mid},{y1} {x1},{y1}\" fill=\"none\" stroke=\"{colour}\" stroke-width=\"1.6\"/>\n"
            ));
        }
        s.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"9\" fill=\"{colour}\">{}</text>\n",
            x0 + 4.0,
            y0 - 3.0,
            esc(&w.net)
        ));
    }
    s.push_str("</svg>\n");
    s
}

pub fn spare_pins(h: &Hardware, lib: &BTreeMap<String, ModuleDef>) -> Vec<String> {
    let Some(mcu) = h
        .module
        .iter()
        .find(|m| m.id == h.board.mcu)
        .and_then(|m| lib.get(&m.part))
    else {
        return Vec::new();
    };
    let used: BTreeSet<&str> = h
        .wire
        .iter()
        .flat_map(|w| w.ends.iter())
        .filter_map(|e| e.strip_prefix(&format!("{}.", h.board.mcu)))
        .collect();
    mcu.pins
        .iter()
        .filter(|(n, p)| {
            p.gpio.is_some()
                && !used.contains(n.as_str())
                && !p
                    .roles
                    .iter()
                    .any(|r| ["strapping", "usb", "boot-driven", "uart0-rx"].contains(&r.as_str()))
        })
        .map(|(n, _)| n.clone())
        .collect()
}

pub fn render_variants(
    t: &Topology,
    node: &Node,
    root: &Path,
    lib: &BTreeMap<String, ModuleDef>,
) -> Result<String> {
    let mut s = format!(
        "<!-- Derived by fid-topology from topology.toml — do not edit. -->\n\n# Hardware variants for `{}`\n\n\
         | Variant | Built | Controller and sensors (RSD) | All modules (RSD) | Spare pins | Buses |\n|---|---|---|---|---|---|\n",
        node.id
    );
    for v in &node.variants {
        let h = load(root, v)?;
        let buses: BTreeSet<String> = t
            .point
            .iter()
            .filter(|p| p.node == node.id)
            .filter_map(|p| effective(p, &h).bus)
            .collect();
        let spare = spare_pins(&h, lib);
        let core: f64 = h
            .module
            .iter()
            .filter(|m| {
                lib.get(&m.part)
                    .is_some_and(|d| ["mcu", "sensor", "breakout"].contains(&d.kind.as_str()))
            })
            .map(|m| price(m, lib))
            .sum();
        s.push_str(&format!(
            "| `{v}` | {} | {core:.0} | {:.0} | {} | {} |\n",
            if node.hardware.as_deref() == Some(v.as_str()) {
                "yes"
            } else {
                ""
            },
            bom_total(&h, lib),
            spare.len(),
            if buses.is_empty() {
                "—".to_string()
            } else {
                buses.into_iter().collect::<Vec<_>>().join(", ")
            }
        ));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_parses_and_knows_the_c6_pin_rules() {
        let lib = library().unwrap();
        let c6 = &lib["esp32-c6-devkitc-1"];
        assert!(c6.pins["GPIO8"].roles.contains(&"strapping".to_string()));
        assert!(c6.pins["GPIO4"].roles.contains(&"adc".to_string()));
        let xiao = &lib["seeed-xiao-esp32c6"];
        assert!(xiao.pins["D6"].roles.contains(&"boot-driven".to_string()));
        assert_eq!(lib["sensirion-scd41"].i2c_address, Some(0x62));
    }

    const TOPO: &str = r#"
[[system]]
id = "s"
nodes = ["n", "srv"]

[[node]]
id = "n"
kind = "mcu"
target = "riscv32imac-unknown-none-elf"
runtime = "embassy-esp"
firmware = "n"

[[node]]
id = "srv"
kind = "host"
target = "aarch64-unknown-linux-gnu"
runtime = "linux-compose"
apps = ["server"]

[[bus]]
id = "i2c"
type = "i2c"
connect = ["n.i2c0"]

[[point]]
id = "co2"
node = "n"
kind = "sensor"
unit = "ppm"
range = [400, 5000]
bus = "i2c"
address = 0x62
display = true

[[point]]
id = "soil"
node = "n"
kind = "sensor"
unit = "%"
range = [0, 100]
net = "SOIL"
display = true

[[point]]
id = "stop"
node = "n"
kind = "contact"
net = "STOP"

[[point]]
id = "relay"
node = "n"
kind = "actuator"
net = "RELAY"
"#;

    const HW: &str = r#"
[board]
kind = "modules"
mcu = "mcu"

[[module]]
id = "mcu"
part = "esp32-c6-devkitc-1"

[[module]]
id = "scd41"
part = "sensirion-scd41"

[[module]]
id = "soil"
part = "capacitive-soil-v1.2"

[[wire]]
net = "3V3"
ends = ["mcu.3V3", "scd41.VDD", "soil.VCC"]

[[wire]]
net = "SDA"
ends = ["mcu.GPIO6", "scd41.SDA"]

[[wire]]
net = "SOIL"
ends = ["mcu.GPIO4", "soil.AOUT"]

[[module]]
id = "stop"
part = "contact-aux"

[[module]]
id = "relays"
part = "relay-2ch-opto"

[[wire]]
net = "STOP"
ends = ["mcu.GPIO2", "stop.C1"]

[[wire]]
net = "RELAY"
ends = ["mcu.GPIO18", "relays.IN1"]
"#;

    fn check(hw: &str) -> String {
        let t: Topology = toml::from_str(TOPO).unwrap();
        let h: Hardware = toml::from_str(hw).unwrap();
        validate(&t, &t.node[0], "hw.toml", &h, &library().unwrap()).join("\n")
    }

    #[test]
    fn a_sound_harness_passes() {
        assert_eq!(check(HW), "");
    }

    #[test]
    fn pin_roles_are_enforced_from_the_library() {
        let e = check(&HW.replace("mcu.GPIO4\", \"soil.AOUT", "mcu.GPIO10\", \"soil.AOUT"));
        assert!(
            e.contains(
                "point `soil` is analog, and ESP32-C6-DevKitC-1 (WROOM-1) pin GPIO10 has no ADC"
            ),
            "{e}"
        );
        let e = check(&HW.replace("mcu.GPIO18", "mcu.GPIO9"));
        assert!(e.contains("a strapping pin"), "{e}");
        let e = check(&HW.replace("mcu.GPIO2\", \"stop.C1", "mcu.GPIO16\", \"stop.C1"));
        assert!(e.contains("which the boot ROM drives"), "{e}");
    }

    #[test]
    fn a_bus_without_its_module_fails_and_a_wrong_rail_is_named() {
        let e = check(&HW.replace(
            "[[wire]]\nnet = \"SDA\"\nends = [\"mcu.GPIO6\", \"scd41.SDA\"]\n",
            "",
        ));
        assert!(
            e.contains("no module speaking i2c is wired to `mcu`"),
            "{e}"
        );
        let e = check(&HW.replace(
            "ends = [\"mcu.3V3\", \"scd41.VDD\", \"soil.VCC\"]",
            "ends = [\"mcu.5V\", \"scd41.VDD\", \"soil.VCC\"]",
        ));
        assert!(
            e.contains("runs at 3.3 V, and its VDD is on `3V3` at 5 V"),
            "{e}"
        );
    }
}

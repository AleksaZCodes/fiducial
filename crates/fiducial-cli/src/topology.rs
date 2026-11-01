//! The `fid-topology` pipeline: a system of several computers, declared once.
//!
//! `topology.toml` names the nodes, the buses between them, the points each
//! node measures or drives, the rules that read and drive them, and the
//! values a person edits at runtime. Firmware tables, topics, web types, the
//! database schema and the broker ACL are derived from it; see
//! `docs/specs/2026-11-01-topology-is-a-declaration.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub const DECLARATION: &str = "topology.toml";

const UNITS: [&str; 18] = [
    "ppm", "%", "°C", "lx", "V", "mV", "A", "mA", "W", "kWh", "Pa", "hPa", "mm", "m", "s", "min",
    "h", "L",
];
const MCU_TARGETS: [&str; 5] = [
    "riscv32imac-unknown-none-elf",
    "thumbv6m-none-eabi",
    "thumbv7em-none-eabihf",
    "thumbv8m.main-none-eabihf",
    "xtensa-esp32s3-none-elf",
];
const MCU_RUNTIMES: [&str; 3] = ["embassy-esp", "embassy-rp", "embassy-stm32"];
const HOST_RUNTIMES: [&str; 2] = ["linux-compose", "linux-rt"];
const BUS_TYPES: [&str; 9] = [
    "i2c", "spi", "uart", "rs485", "onewire", "ip", "can", "can-fd", "usb",
];
pub const TOPICS: [&str; 6] = [
    "stanje", "nalog", "potvrda", "pragovi", "raspored", "online",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topology {
    #[serde(default)]
    pub system: Vec<System>,
    #[serde(default)]
    pub node: Vec<Node>,
    #[serde(default)]
    pub bus: Vec<Bus>,
    #[serde(default)]
    pub point: Vec<Point>,
    #[serde(default)]
    pub rule: Vec<Rule>,
    #[serde(default)]
    pub parameter: Vec<Parameter>,
    #[serde(default)]
    pub schedule: Vec<Schedule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct System {
    pub id: String,
    pub nodes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub target: String,
    pub runtime: String,
    #[serde(default)]
    pub firmware: Option<String>,
    #[serde(default)]
    pub hardware: Option<String>,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub apps: Vec<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Mcu,
    Host,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bus {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub broker: Option<String>,
    pub connect: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub id: String,
    pub node: String,
    pub kind: PointKind,
    #[serde(default)]
    pub quantity: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    #[serde(default)]
    pub tolerance: Option<String>,
    #[serde(default)]
    pub bus: Option<String>,
    #[serde(default)]
    pub address: Option<u32>,
    #[serde(default)]
    pub adc: Option<String>,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub calibration: Option<BTreeMap<String, toml::Value>>,
    #[serde(default)]
    pub net: Option<String>,
    #[serde(default)]
    pub interlock: Option<String>,
    #[serde(default)]
    pub display: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PointKind {
    Sensor,
    Actuator,
    Contact,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub scope: String,
    pub reads: Vec<String>,
    pub drives: Vec<String>,
    #[serde(default)]
    pub doc: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub id: String,
    pub scope: String,
    pub unit: String,
    pub range: [f64; 2],
    pub default: f64,
    pub rule: String,
    #[serde(default)]
    pub limits: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    pub id: String,
    pub scope: String,
    pub granularity: String,
    pub default_when_missing: String,
}

impl Topology {
    pub fn role(&self, node: &str) -> Option<&str> {
        self.node
            .iter()
            .find(|n| n.id == node)
            .and_then(|n| n.firmware.as_deref())
    }

    pub fn roles(&self) -> Vec<String> {
        let set: BTreeSet<String> = self
            .node
            .iter()
            .filter_map(|n| n.firmware.clone())
            .collect();
        set.into_iter().collect()
    }

    pub fn first_node_of(&self, role: &str) -> Option<&Node> {
        self.node
            .iter()
            .find(|n| n.firmware.as_deref() == Some(role))
    }

    pub fn points_of_role(&self, role: &str) -> Vec<&Point> {
        let Some(node) = self.first_node_of(role) else {
            return Vec::new();
        };
        self.point.iter().filter(|p| p.node == node.id).collect()
    }

    pub fn system_of(&self, node: &str) -> Option<&str> {
        self.system
            .iter()
            .find(|s| s.nodes.iter().any(|n| n == node))
            .map(|s| s.id.as_str())
    }

    pub fn decides(&self, point: &str, role: &str) -> Vec<&str> {
        self.rule
            .iter()
            .filter(|r| r.scope == role && r.reads.iter().any(|x| x == point))
            .map(|r| r.id.as_str())
            .collect()
    }
}

pub fn load(root: &Path, decl: &str) -> Result<Topology> {
    let raw = std::fs::read_to_string(root.join(decl)).with_context(|| {
        format!("fid-topology: cannot read `{decl}` — it is the declaration (`fid add capability topology`)")
    })?;
    let t: Topology =
        toml::from_str(&raw).with_context(|| format!("fid-topology: `{decl}` does not parse"))?;
    let problems = validate(&t);
    if !problems.is_empty() {
        bail!(
            "fid-topology: `{decl}` does not hold together:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(t)
}

pub fn validate(t: &Topology) -> Vec<String> {
    let mut out = Vec::new();
    let nodes: BTreeMap<&str, &Node> = t.node.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut seen = BTreeSet::new();
    for n in &t.node {
        if !seen.insert(n.id.as_str()) {
            out.push(format!("node `{}` is declared twice", n.id));
        }
        match n.kind {
            NodeKind::Mcu => {
                if !MCU_TARGETS.contains(&n.target.as_str()) {
                    out.push(format!(
                        "node `{}`: target `{}` is not one the firmware workspaces build ({})",
                        n.id,
                        n.target,
                        MCU_TARGETS.join(", ")
                    ));
                }
                if !MCU_RUNTIMES.contains(&n.runtime.as_str()) {
                    out.push(format!(
                        "node `{}`: runtime `{}` is not an MCU runtime ({})",
                        n.id,
                        n.runtime,
                        MCU_RUNTIMES.join(", ")
                    ));
                }
                if n.firmware.is_none() {
                    out.push(format!(
                        "node `{}` is an MCU with no `firmware` member",
                        n.id
                    ));
                }
                if let Some(h) = &n.hardware {
                    if !n.variants.is_empty() && !n.variants.contains(h) {
                        out.push(format!(
                            "node `{}`: hardware `{h}` is not one of its variants",
                            n.id
                        ));
                    }
                }
            }
            NodeKind::Host => {
                if n.apps.is_empty() {
                    out.push(format!("node `{}` is a host that runs no `apps`", n.id));
                }
                if !HOST_RUNTIMES.contains(&n.runtime.as_str()) {
                    out.push(format!(
                        "node `{}`: runtime `{}` has no deploy implementation ({})",
                        n.id,
                        n.runtime,
                        HOST_RUNTIMES.join(", ")
                    ));
                }
            }
        }
        if t.system_of(&n.id).is_none() {
            out.push(format!("node `{}` is in no [[system]]", n.id));
        }
    }
    for s in &t.system {
        for n in &s.nodes {
            if !nodes.contains_key(n.as_str()) {
                out.push(format!(
                    "system `{}` lists `{n}`, which is not a declared node",
                    s.id
                ));
            }
        }
    }
    for n in t
        .node
        .iter()
        .filter(|n| t.system.iter().filter(|s| s.nodes.contains(&n.id)).count() > 1)
    {
        out.push(format!("node `{}` is in more than one [[system]]", n.id));
    }

    let buses: BTreeMap<&str, &Bus> = t.bus.iter().map(|b| (b.id.as_str(), b)).collect();
    for b in &t.bus {
        if !BUS_TYPES.contains(&b.kind.as_str()) {
            out.push(format!(
                "bus `{}`: type `{}` is not one of {}",
                b.id,
                b.kind,
                BUS_TYPES.join(", ")
            ));
        }
        let mut systems = BTreeSet::new();
        for end in &b.connect {
            let Some((node, iface)) = end.split_once('.') else {
                out.push(format!(
                    "bus `{}`: endpoint `{end}` is not `<node>.<interface>`",
                    b.id
                ));
                continue;
            };
            match nodes.get(node) {
                None => out.push(format!(
                    "bus `{}`: endpoint `{end}` names no declared node",
                    b.id
                )),
                Some(n) => {
                    if !n.interfaces.is_empty() && !n.interfaces.iter().any(|i| i == iface) {
                        out.push(format!(
                            "bus `{}`: node `{node}` has no interface `{iface}` (it has {})",
                            b.id,
                            n.interfaces.join(", ")
                        ));
                    }
                    if let Some(s) = t.system_of(node) {
                        systems.insert(s.to_string());
                    }
                }
            }
        }
        if systems.len() > 1 {
            out.push(format!(
                "bus `{}` crosses systems {} — a bus stays inside one system",
                b.id,
                systems
                    .into_iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ));
        }
        if b.kind == "ip" {
            match b.protocol.as_deref() {
                Some("mqtt") if !b.tls => out.push(format!(
                    "bus `{}`: MQTT without `tls = true` sends broker passwords in the clear",
                    b.id
                )),
                Some("mqtt") if b.broker.is_none() => {
                    out.push(format!("bus `{}`: MQTT names its `broker` node", b.id))
                }
                Some("mqtt") => {}
                Some(other) => out.push(format!(
                    "bus `{}`: protocol `{other}` over ip is not supported (mqtt)",
                    b.id
                )),
                None => out.push(format!("bus `{}`: an ip bus names its `protocol`", b.id)),
            }
        }
        if let Some(broker) = &b.broker {
            match nodes.get(broker.as_str()) {
                Some(n) if n.kind == NodeKind::Host => {}
                Some(_) => out.push(format!(
                    "bus `{}`: broker `{broker}` is not a host node",
                    b.id
                )),
                None => out.push(format!(
                    "bus `{}`: broker `{broker}` is not a declared node",
                    b.id
                )),
            }
        }
    }

    let mut ids = BTreeSet::new();
    let mut addresses: BTreeMap<(String, String, u32), String> = BTreeMap::new();
    for p in &t.point {
        if !ids.insert((p.node.as_str(), p.id.as_str())) {
            out.push(format!(
                "point `{}` is declared twice on node `{}`",
                p.id, p.node
            ));
        }
        if !nodes.contains_key(p.node.as_str()) {
            out.push(format!(
                "point `{}`: node `{}` is not declared",
                p.id, p.node
            ));
        }
        if p.kind == PointKind::Sensor {
            match &p.unit {
                None => out.push(format!(
                    "point `{}`: a sensor has a unit — a physical fact is never a bare number",
                    p.id
                )),
                Some(u) if !UNITS.contains(&u.as_str()) => out.push(format!(
                    "point `{}`: unit `{u}` is not one this platform knows ({})",
                    p.id,
                    UNITS.join(", ")
                )),
                _ => {}
            }
            match p.range {
                None => out.push(format!("point `{}`: a sensor has a range", p.id)),
                Some([lo, hi]) if lo >= hi => {
                    out.push(format!("point `{}`: range [{lo}, {hi}] is empty", p.id))
                }
                _ => {}
            }
            if let (Some(tol), Some(unit)) = (&p.tolerance, &p.unit) {
                let ok = tol
                    .split_once(' ')
                    .is_some_and(|(n, u)| n.parse::<f64>().is_ok_and(|v| v > 0.0) && u == unit);
                if !ok {
                    out.push(format!(
                        "point `{}`: tolerance `{tol}` is not `<positive number> {unit}`",
                        p.id
                    ));
                }
            }
            if p.bus.is_none() && p.adc.is_none() && p.pin.is_none() {
                out.push(format!(
                    "point `{}`: a sensor is bound to a bus, an ADC channel or a pin",
                    p.id
                ));
            }
        }
        if p.kind != PointKind::Sensor && p.net.is_none() && p.pin.is_none() {
            out.push(format!(
                "point `{}`: an {} is bound to a net or a pin",
                p.id,
                kind_name(p.kind)
            ));
        }
        if let Some(bus) = &p.bus {
            match buses.get(bus.as_str()) {
                None => out.push(format!("point `{}`: bus `{bus}` is not declared", p.id)),
                Some(b) => {
                    if !b
                        .connect
                        .iter()
                        .any(|e| e.split('.').next() == Some(p.node.as_str()))
                    {
                        out.push(format!(
                            "point `{}`: bus `{bus}` does not connect node `{}`",
                            p.id, p.node
                        ));
                    }
                    if let Some(a) = p.address {
                        let key = (p.node.clone(), bus.clone(), a);
                        let quantity = p.quantity.clone().unwrap_or_default();
                        if let Some(other) = addresses.get(&key) {
                            if !shares_device(other, &p.id, t) {
                                out.push(format!(
                                    "point `{}`: address 0x{a:02X} on bus `{bus}` is already `{other}`'s",
                                    p.id
                                ));
                            }
                        } else {
                            addresses.insert(key, p.id.clone());
                        }
                        let _ = quantity;
                    }
                }
            }
        }
        if let Some(il) = &p.interlock {
            if !t.point.iter().any(|q| q.node == p.node && &q.id == il) {
                out.push(format!(
                    "point `{}`: interlock `{il}` is not a point on `{}`",
                    p.id, p.node
                ));
            }
        }
    }

    let roles = t.roles();
    for role in &roles {
        let members: Vec<&Node> = t
            .node
            .iter()
            .filter(|n| n.firmware.as_deref() == Some(role))
            .collect();
        let shape = |n: &Node| -> Vec<String> {
            let mut v: Vec<String> = t
                .point
                .iter()
                .filter(|p| p.node == n.id)
                .map(|p| p.id.clone())
                .collect();
            v.sort();
            v
        };
        if let Some(first) = members.first() {
            for other in &members[1..] {
                if shape(first) != shape(other) {
                    out.push(format!(
                        "nodes `{}` and `{}` share firmware `{role}` but declare different points — one \
                         firmware reads one set of points",
                        first.id, other.id
                    ));
                }
            }
        }
    }

    let rules: BTreeMap<&str, &Rule> = t.rule.iter().map(|r| (r.id.as_str(), r)).collect();
    for r in &t.rule {
        if !roles.contains(&r.scope) {
            out.push(format!(
                "rule `{}`: scope `{}` is no node's firmware",
                r.id, r.scope
            ));
            continue;
        }
        let pts: Vec<&Point> = t.points_of_role(&r.scope);
        for input in &r.reads {
            match pts.iter().find(|p| &p.id == input) {
                None => out.push(format!(
                    "rule `{}` reads `{input}`, which is not a point of `{}`",
                    r.id, r.scope
                )),
                Some(p) if p.kind == PointKind::Actuator => out.push(format!(
                    "rule `{}` reads `{input}`, an actuator — a rule reads sensors and contacts",
                    r.id
                )),
                _ => {}
            }
        }
        for output in &r.drives {
            match pts.iter().find(|p| &p.id == output) {
                None => out.push(format!(
                    "rule `{}` drives `{output}`, which is not a point of `{}`",
                    r.id, r.scope
                )),
                Some(p) if p.kind != PointKind::Actuator => out.push(format!(
                    "rule `{}` drives `{output}`, which is not an actuator",
                    r.id
                )),
                _ => {}
            }
        }
    }
    for p in &t.point {
        if p.kind == PointKind::Sensor {
            let role = t.role(&p.node).unwrap_or_default();
            let read = !t.decides(&p.id, role).is_empty();
            if !read && !p.display {
                out.push(format!(
                    "point `{}` is read by no rule and not `display = true` — an orphan reading",
                    p.id
                ));
            }
            if read && p.display {
                out.push(format!(
                    "point `{}` is read by a rule and also `display = true` — a reading a rule \
                     uses is always shown; drop `display`",
                    p.id
                ));
            }
        }
    }
    for prm in &t.parameter {
        match rules.get(prm.rule.as_str()) {
            None => out.push(format!(
                "parameter `{}` names rule `{}`, which is not declared",
                prm.id, prm.rule
            )),
            Some(r) if r.scope != prm.scope => out.push(format!(
                "parameter `{}` is scoped `{}` but rule `{}` runs on `{}`",
                prm.id, prm.scope, prm.rule, r.scope
            )),
            Some(r) => {
                if let Some(l) = &prm.limits {
                    if !r.reads.contains(l) {
                        out.push(format!(
                            "parameter `{}` limits `{l}`, which rule `{}` does not read",
                            prm.id, r.id
                        ));
                    }
                }
            }
        }
        let [lo, hi] = prm.range;
        if !(lo <= prm.default && prm.default <= hi) {
            out.push(format!(
                "parameter `{}`: default {} is outside its range [{lo}, {hi}]",
                prm.id, prm.default
            ));
        }
        if !UNITS.contains(&prm.unit.as_str()) {
            out.push(format!(
                "parameter `{}`: unit `{}` is not one this platform knows",
                prm.id, prm.unit
            ));
        }
    }
    for s in &t.schedule {
        if !roles.contains(&s.scope) {
            out.push(format!(
                "schedule `{}`: scope `{}` is no node's firmware",
                s.id, s.scope
            ));
        }
        if !["occupied", "empty"].contains(&s.default_when_missing.as_str()) {
            out.push(format!(
                "schedule `{}`: default_when_missing is `occupied` or `empty`, not `{}`",
                s.id, s.default_when_missing
            ));
        }
        if granularity_min(&s.granularity).is_none() {
            out.push(format!(
                "schedule `{}`: granularity `{}` is not `<n> min` dividing an hour",
                s.id, s.granularity
            ));
        }
    }
    out
}

fn shares_device(a: &str, b: &str, t: &Topology) -> bool {
    let q = |id: &str| {
        t.point
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.quantity.clone())
    };
    q(a) != q(b)
}

fn kind_name(k: PointKind) -> &'static str {
    match k {
        PointKind::Sensor => "sensor",
        PointKind::Actuator => "actuator",
        PointKind::Contact => "contact",
    }
}

pub fn granularity_min(g: &str) -> Option<u16> {
    let n: u16 = g.trim().strip_suffix("min")?.trim().parse().ok()?;
    (n > 0 && 60 % n == 0).then_some(n)
}

fn pascal(s: &str) -> String {
    s.split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn upper(s: &str) -> String {
    s.replace('-', "_").to_ascii_uppercase()
}

fn f(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

pub fn render_points_rs(t: &Topology, role: &str, decl: &str) -> String {
    let pts = t.points_of_role(role);
    let mut s = format!(
        "// Derived by fid-topology from {decl} for firmware `{role}` — do not edit.\n\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(u16)]\npub enum Point {{\n"
    );
    for (i, p) in pts.iter().enumerate() {
        s.push_str(&format!("    {} = {},\n", pascal(&p.id), i + 1));
    }
    s.push_str("}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Binding {\n    Bus { bus: &'static str, address: u32 },\n    Adc { channel: &'static str },\n    Pin { pin: &'static str },\n    Net { net: &'static str },\n}\n\n");
    s.push_str(&format!("pub const POINTS: [Point; {}] = [", pts.len()));
    s.push_str(
        &pts.iter()
            .map(|p| format!("Point::{}", pascal(&p.id)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str(
        "];\n\nimpl Point {\n    pub const fn id(self) -> u16 {\n        self as u16\n    }\n\n",
    );
    let arm = |body: &dyn Fn(&Point) -> String, ty: &str, name: &str| {
        let mut a = format!("    pub const fn {name}(self) -> {ty} {{\n        match self {{\n");
        for p in &pts {
            a.push_str(&format!(
                "            Point::{} => {},\n",
                pascal(&p.id),
                body(p)
            ));
        }
        a.push_str("        }\n    }\n");
        a
    };
    s.push_str(&arm(&|p| format!("{:?}", p.id), "&'static str", "name"));
    s.push('\n');
    s.push_str(&arm(
        &|p| match &p.unit {
            Some(u) => format!("Some({u:?})"),
            None => "None".into(),
        },
        "Option<&'static str>",
        "unit",
    ));
    s.push('\n');
    s.push_str(&arm(
        &|p| match p.range {
            Some([lo, hi]) => format!("Some(({}, {}))", f(lo), f(hi)),
            None => "None".into(),
        },
        "Option<(f32, f32)>",
        "range",
    ));
    s.push('\n');
    s.push_str(&arm(
        &|p| {
            if let (Some(b), Some(a)) = (&p.bus, p.address) {
                format!("Binding::Bus {{ bus: {b:?}, address: 0x{a:02X} }}")
            } else if let Some(c) = &p.adc {
                format!("Binding::Adc {{ channel: {c:?} }}")
            } else if let Some(pin) = &p.pin {
                format!("Binding::Pin {{ pin: {pin:?} }}")
            } else if let Some(n) = &p.net {
                format!("Binding::Net {{ net: {n:?} }}")
            } else if let Some(b) = &p.bus {
                format!("Binding::Bus {{ bus: {b:?}, address: 0 }}")
            } else {
                "Binding::Pin { pin: \"\" }".into()
            }
        },
        "Binding",
        "binding",
    ));
    s.push('\n');
    s.push_str(&arm(
        &|p| format!("{}", p.calibration.is_some()),
        "bool",
        "calibrated",
    ));
    s.push('\n');
    s.push_str(&arm(
        &|p| match &p.interlock {
            Some(i) => format!("Some(Point::{})", pascal(i)),
            None => "None".into(),
        },
        "Option<Point>",
        "interlock",
    ));
    s.push_str("}\n");
    s
}

pub fn render_params_rs(t: &Topology, role: &str, decl: &str) -> String {
    let prms: Vec<&Parameter> = t.parameter.iter().filter(|p| p.scope == role).collect();
    let mut s = format!(
        "// Derived by fid-topology from {decl} for firmware `{role}` — do not edit.\n\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(u16)]\npub enum Parameter {{\n"
    );
    for (i, p) in prms.iter().enumerate() {
        s.push_str(&format!("    {} = {},\n", pascal(&p.id), i + 1));
    }
    s.push_str(&format!(
        "}}\n\npub const PARAMETERS: [Parameter; {}] = [",
        prms.len()
    ));
    s.push_str(
        &prms
            .iter()
            .map(|p| format!("Parameter::{}", pascal(&p.id)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str("];\n\nimpl Parameter {\n    pub const fn id(self) -> u16 {\n        self as u16\n    }\n\n    pub fn from_id(id: u16) -> Option<Parameter> {\n        PARAMETERS.iter().copied().find(|p| p.id() == id)\n    }\n\n");
    for (name, ty, body) in [
        (
            "default",
            "f32",
            &(|p: &Parameter| f(p.default)) as &dyn Fn(&Parameter) -> String,
        ),
        ("min", "f32", &|p: &Parameter| f(p.range[0])),
        ("max", "f32", &|p: &Parameter| f(p.range[1])),
        ("unit", "&'static str", &|p: &Parameter| {
            format!("{:?}", p.unit)
        }),
    ] {
        s.push_str(&format!(
            "    pub const fn {name}(self) -> {ty} {{\n        match self {{\n"
        ));
        for p in &prms {
            s.push_str(&format!(
                "            Parameter::{} => {},\n",
                pascal(&p.id),
                body(p)
            ));
        }
        s.push_str("        }\n    }\n\n");
    }
    s.push_str("    pub fn accepts(self, value: f32) -> bool {\n        self.min() <= value && value <= self.max()\n    }\n}\n");
    for sch in t.schedule.iter().filter(|x| x.scope == role) {
        s.push_str(&format!(
            "\npub const {}_GRANULARITY_MIN: u16 = {};\npub const {}_OCCUPIED_WHEN_MISSING: bool = {};\n",
            upper(&sch.id),
            granularity_min(&sch.granularity).unwrap_or(5),
            upper(&sch.id),
            sch.default_when_missing == "occupied"
        ));
    }
    s
}

pub fn render_topics_rs(t: &Topology, decl: &str) -> String {
    let mut s = format!(
        "// Derived by fid-topology from {decl} — do not edit.\n\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Topic {{\n"
    );
    for k in TOPICS {
        s.push_str(&format!("    {},\n", pascal(k)));
    }
    s.push_str("}\n\npub const TOPICS: [Topic; 6] = [");
    s.push_str(
        &TOPICS
            .iter()
            .map(|k| format!("Topic::{}", pascal(k)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str("];\n\nimpl Topic {\n    pub const fn suffix(self) -> &'static str {\n        match self {\n");
    for k in TOPICS {
        s.push_str(&format!("            Topic::{} => {k:?},\n", pascal(k)));
    }
    s.push_str("        }\n    }\n\n    pub const fn retained(self) -> bool {\n        matches!(self, Topic::Pragovi | Topic::Raspored)\n    }\n\n    pub const fn from_device(self) -> bool {\n        matches!(self, Topic::Stanje | Topic::Potvrda)\n    }\n}\n\n");
    s.push_str("pub struct NodeInfo {\n    pub id: &'static str,\n    pub system: &'static str,\n    pub firmware: Option<&'static str>,\n}\n\n");
    s.push_str(&format!(
        "pub const NODES: [NodeInfo; {}] = [\n",
        t.node.len()
    ));
    for n in &t.node {
        s.push_str(&format!(
            "    NodeInfo {{ id: {:?}, system: {:?}, firmware: {} }},\n",
            n.id,
            t.system_of(&n.id).unwrap_or_default(),
            n.firmware
                .as_ref()
                .map(|f| format!("Some({f:?})"))
                .unwrap_or_else(|| "None".into())
        ));
    }
    s.push_str("];\n\n#[cfg(feature = \"std\")]\npub fn topic(system: &str, node: &str, kind: Topic) -> std::string::String {\n    std::format!(\"{system}/{node}/{}\", kind.suffix())\n}\n");
    s
}

pub fn render_points_ts(t: &Topology, decl: &str) -> String {
    let mut s = format!("// Derived by fid-topology from {decl} — do not edit.\n\n");
    s.push_str("export type PointKind = \"sensor\" | \"actuator\" | \"contact\"\n\n");
    s.push_str("export interface PointInfo {\n  readonly id: number\n  readonly name: string\n  readonly kind: PointKind\n  readonly quantity: string | null\n  readonly unit: string | null\n  readonly range: readonly [number, number] | null\n  readonly decides: readonly string[]\n}\n\n");
    s.push_str("export interface ParameterInfo {\n  readonly id: number\n  readonly name: string\n  readonly unit: string\n  readonly min: number\n  readonly max: number\n  readonly default: number\n  readonly rule: string\n  readonly limits: string | null\n}\n\n");
    s.push_str("export const POINTS = {\n");
    for role in t.roles() {
        s.push_str(&format!("  {role:?}: [\n"));
        for (i, p) in t.points_of_role(&role).iter().enumerate() {
            let range = p
                .range
                .map(|[a, b]| format!("[{a}, {b}]"))
                .unwrap_or_else(|| "null".into());
            let dec: Vec<String> = t
                .decides(&p.id, &role)
                .iter()
                .map(|d| format!("{d:?}"))
                .collect();
            s.push_str(&format!(
                "    {{ id: {}, name: {:?}, kind: {:?}, quantity: {}, unit: {}, range: {range}, decides: [{}] }},\n",
                i + 1,
                p.id,
                kind_name(p.kind),
                p.quantity.as_ref().map(|q| format!("{q:?}")).unwrap_or_else(|| "null".into()),
                p.unit.as_ref().map(|u| format!("{u:?}")).unwrap_or_else(|| "null".into()),
                dec.join(", ")
            ));
        }
        s.push_str("  ],\n");
    }
    s.push_str("} as const satisfies Record<string, readonly PointInfo[]>\n\nexport const PARAMETERS = {\n");
    for role in t.roles() {
        s.push_str(&format!("  {role:?}: [\n"));
        for (i, p) in t.parameter.iter().filter(|p| p.scope == role).enumerate() {
            s.push_str(&format!(
                "    {{ id: {}, name: {:?}, unit: {:?}, min: {}, max: {}, default: {}, rule: {:?}, limits: {} }},\n",
                i + 1,
                p.id,
                p.unit,
                p.range[0],
                p.range[1],
                p.default,
                p.rule,
                p.limits.as_ref().map(|l| format!("{l:?}")).unwrap_or_else(|| "null".into())
            ));
        }
        s.push_str("  ],\n");
    }
    s.push_str(
        "} as const satisfies Record<string, readonly ParameterInfo[]>\n\nexport const NODES = [\n",
    );
    for n in &t.node {
        s.push_str(&format!(
            "  {{ id: {:?}, system: {:?}, firmware: {} }},\n",
            n.id,
            t.system_of(&n.id).unwrap_or_default(),
            n.firmware
                .as_ref()
                .map(|f| format!("{f:?}"))
                .unwrap_or_else(|| "null".into())
        ));
    }
    s.push_str("] as const\n\nexport const RULES = [\n");
    for r in &t.rule {
        let list = |v: &[String]| {
            v.iter()
                .map(|x| format!("{x:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        s.push_str(&format!(
            "  {{ id: {:?}, scope: {:?}, reads: [{}], drives: [{}], doc: {:?} }},\n",
            r.id,
            r.scope,
            list(&r.reads),
            list(&r.drives),
            r.doc
        ));
    }
    s.push_str("] as const\n\nexport type Role = keyof typeof POINTS\n");
    s
}

pub fn render_cards_ts(t: &Topology, decl: &str) -> String {
    let mut s = format!(
        "// Derived by fid-topology from {decl} — do not edit.\n\n\
         export interface Card {{\n  readonly point: string\n  readonly unit: string\n  readonly limits: readonly string[]\n}}\n\n\
         export const CARDS = {{\n"
    );
    for role in t.roles() {
        s.push_str(&format!("  {role:?}: [\n"));
        for p in t
            .points_of_role(&role)
            .iter()
            .filter(|p| p.kind == PointKind::Sensor)
        {
            let limits: Vec<String> = t
                .parameter
                .iter()
                .filter(|prm| prm.scope == role && prm.limits.as_deref() == Some(p.id.as_str()))
                .map(|prm| format!("{:?}", prm.id))
                .collect();
            s.push_str(&format!(
                "    {{ point: {:?}, unit: {:?}, limits: [{}] }},\n",
                p.id,
                p.unit.clone().unwrap_or_default(),
                limits.join(", ")
            ));
        }
        s.push_str("  ],\n");
    }
    s.push_str("} as const satisfies Record<string, readonly Card[]>\n");
    s
}

pub fn render_migration(t: &Topology, decl: &str, dialect: &str) -> Result<String> {
    if dialect != "sqlite" {
        bail!(
            "fid-topology: migrations for `[adapters] database = \"{dialect}\"` are not derived yet — \
             `sqlite` and `d1` are"
        );
    }
    let roles = t.roles();
    let mut s = format!(
        "-- Derived by fid-topology from {decl} — do not edit.\n\
         -- Readings, roll-ups, settings, command state, warnings and descriptions\n\
         -- for {} node(s) across {} firmware role(s).\n\n",
        t.node.len(),
        roles.len()
    );
    s.push_str(
        "CREATE TABLE IF NOT EXISTS reading (\n  node TEXT NOT NULL,\n  point INTEGER NOT NULL,\n  at INTEGER NOT NULL,\n  value REAL,\n  PRIMARY KEY (node, point, at)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS reading_hourly (\n  node TEXT NOT NULL,\n  point INTEGER NOT NULL,\n  hour INTEGER NOT NULL,\n  mean REAL,\n  min REAL,\n  max REAL,\n  samples INTEGER NOT NULL,\n  PRIMARY KEY (node, point, hour)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS parameter_value (\n  scope_node TEXT NOT NULL,\n  parameter INTEGER NOT NULL,\n  value REAL NOT NULL,\n  changed_at INTEGER NOT NULL,\n  PRIMARY KEY (scope_node, parameter)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS schedule_slot (\n  node TEXT NOT NULL,\n  schedule TEXT NOT NULL,\n  weekday INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),\n  from_min INTEGER NOT NULL,\n  to_min INTEGER NOT NULL CHECK (to_min > from_min),\n  PRIMARY KEY (node, schedule, weekday, from_min)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS device_state (\n  node TEXT NOT NULL,\n  device INTEGER NOT NULL,\n  output TEXT NOT NULL,\n  selector TEXT NOT NULL,\n  reason TEXT NOT NULL,\n  last_command INTEGER,\n  last_outcome TEXT,\n  at INTEGER NOT NULL,\n  PRIMARY KEY (node, device)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS warning (\n  id INTEGER PRIMARY KEY,\n  node TEXT NOT NULL,\n  kind TEXT NOT NULL,\n  source INTEGER,\n  raised_at INTEGER NOT NULL,\n  cleared_at INTEGER\n);\n\n\
         CREATE TABLE IF NOT EXISTS description (\n  scope TEXT NOT NULL,\n  hour INTEGER NOT NULL,\n  text TEXT NOT NULL,\n  PRIMARY KEY (scope, hour)\n) WITHOUT ROWID;\n",
    );
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHOOL: &str = r#"
[[system]]
id = "school"
nodes = ["roof", "classroom_1", "server"]

[[node]]
id = "roof"
kind = "mcu"
target = "riscv32imac-unknown-none-elf"
runtime = "embassy-esp"
firmware = "roof"
interfaces = ["wifi"]

[[node]]
id = "classroom_1"
kind = "mcu"
target = "riscv32imac-unknown-none-elf"
runtime = "embassy-esp"
firmware = "classroom"
interfaces = ["wifi", "i2c0"]

[[node]]
id = "server"
kind = "host"
target = "aarch64-unknown-linux-gnu"
runtime = "linux-compose"
apps = ["server", "web"]
interfaces = ["eth0"]

[[bus]]
id = "classroom_i2c"
type = "i2c"
connect = ["classroom_1.i2c0"]

[[bus]]
id = "lan"
type = "ip"
protocol = "mqtt"
tls = true
broker = "server"
connect = ["roof.wifi", "classroom_1.wifi", "server.eth0"]

[[point]]
id = "co2"
node = "classroom_1"
kind = "sensor"
quantity = "co2"
unit = "ppm"
range = [400, 5000]
bus = "classroom_i2c"
address = 0x62

[[point]]
id = "humidity"
node = "classroom_1"
kind = "sensor"
quantity = "relative_humidity"
unit = "%"
range = [0, 100]
bus = "classroom_i2c"
address = 0x62
display = true

[[point]]
id = "recuperator"
node = "classroom_1"
kind = "actuator"
net = "RELAY_RECUPERATOR"

[[point]]
id = "soil_1"
node = "roof"
kind = "sensor"
unit = "%"
range = [0, 100]
adc = "GPIO4"
display = true

[[rule]]
id = "recuperator"
scope = "classroom"
reads = ["co2"]
drives = ["recuperator"]

[[parameter]]
id = "co2_on"
scope = "classroom"
unit = "ppm"
range = [600, 2000]
default = 1000
rule = "recuperator"
limits = "co2"

[[schedule]]
id = "timetable"
scope = "classroom"
granularity = "5 min"
default_when_missing = "occupied"
"#;

    fn parse(s: &str) -> Topology {
        toml::from_str(s).unwrap()
    }

    #[test]
    fn the_school_holds_together() {
        assert_eq!(validate(&parse(SCHOOL)), Vec::<String>::new());
    }

    #[test]
    fn a_bus_across_systems_fails_naming_both() {
        let s = SCHOOL.replace(
            "nodes = [\"roof\", \"classroom_1\", \"server\"]",
            "nodes = [\"roof\", \"classroom_1\"]\n\n[[system]]\nid = \"office\"\nnodes = [\"server\"]",
        );
        let e = validate(&parse(&s)).join("\n");
        assert!(
            e.contains("bus `lan` crosses systems `office` and `school`"),
            "{e}"
        );
    }

    #[test]
    fn a_parameter_naming_a_missing_rule_fails() {
        let s = SCHOOL.replace("rule = \"recuperator\"", "rule = \"ventilation\"");
        let e = validate(&parse(&s)).join("\n");
        assert!(
            e.contains("parameter `co2_on` names rule `ventilation`, which is not declared"),
            "{e}"
        );
    }

    #[test]
    fn an_orphan_reading_and_an_unknown_unit_fail() {
        let s = SCHOOL.replace("adc = \"GPIO4\"\ndisplay = true", "adc = \"GPIO4\"");
        let e = validate(&parse(&s)).join("\n");
        assert!(e.contains("point `soil_1` is read by no rule"), "{e}");
        let s = SCHOOL.replace(
            "unit = \"ppm\"\nrange = [400",
            "unit = \"parts\"\nrange = [400",
        );
        assert!(validate(&parse(&s)).join("\n").contains("unit `parts`"));
    }

    #[test]
    fn a_second_classroom_must_declare_the_same_points() {
        let s = SCHOOL.replace("nodes = [\"roof\", \"classroom_1\", \"server\"]", "nodes = [\"roof\", \"classroom_1\", \"classroom_2\", \"server\"]")
            + "\n[[node]]\nid = \"classroom_2\"\nkind = \"mcu\"\ntarget = \"riscv32imac-unknown-none-elf\"\nruntime = \"embassy-esp\"\nfirmware = \"classroom\"\n";
        let e = validate(&parse(&s)).join("\n");
        assert!(
            e.contains("share firmware `classroom` but declare different points"),
            "{e}"
        );
    }

    #[test]
    fn the_derivations_carry_every_point() {
        let t = parse(SCHOOL);
        let rs = render_points_rs(&t, "classroom", DECLARATION);
        assert!(
            rs.contains("Co2 = 1,")
                && rs.contains("Binding::Bus { bus: \"classroom_i2c\", address: 0x62 }"),
            "{rs}"
        );
        let p = render_params_rs(&t, "classroom", DECLARATION);
        assert!(
            p.contains("Parameter::Co2On => 1000.0")
                && p.contains("TIMETABLE_OCCUPIED_WHEN_MISSING: bool = true"),
            "{p}"
        );
        let cards = render_cards_ts(&t, DECLARATION);
        assert!(
            cards.contains("{ point: \"co2\", unit: \"ppm\", limits: [\"co2_on\"] }"),
            "{cards}"
        );
        assert!(
            cards.contains("{ point: \"humidity\", unit: \"%\", limits: [] }"),
            "{cards}"
        );
        let topics = render_topics_rs(&t, DECLARATION);
        assert!(topics.contains(
            "NodeInfo { id: \"classroom_1\", system: \"school\", firmware: Some(\"classroom\") }"
        ));
    }
}

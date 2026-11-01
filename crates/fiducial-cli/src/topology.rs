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

/// Units every product knows. A product adds its own with `[[unit]]`.
const UNITS: [&str; 18] = [
    "ppm", "%", "°C", "lx", "V", "mV", "A", "mA", "W", "kWh", "Pa", "hPa", "mm", "m", "s", "min",
    "h", "L",
];

/// The topics a node exchanges when `topology.toml` declares none.
const DEFAULT_TOPICS: [(&str, Direction, bool); 6] = [
    ("state", Direction::Up, false),
    ("command", Direction::Down, false),
    ("ack", Direction::Up, false),
    ("parameters", Direction::Down, true),
    ("schedule", Direction::Down, true),
    ("online", Direction::Will, false),
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
    pub parameter: Vec<Parameter>,
    #[serde(default)]
    pub topic: Vec<Topic>,
    #[serde(default)]
    pub unit: Vec<Unit>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topic {
    pub id: String,
    pub direction: Direction,
    #[serde(default)]
    pub retained: bool,
    #[serde(default)]
    pub doc: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    Up,
    Down,
    Will,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub symbol: String,
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
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub id: String,
    pub scope: String,
    pub unit: String,
    pub range: [f64; 2],
    pub default: f64,
    #[serde(default)]
    pub doc: String,
}

impl Topology {
    pub fn topics(&self) -> Vec<Topic> {
        if !self.topic.is_empty() {
            return self.topic.clone();
        }
        DEFAULT_TOPICS
            .iter()
            .map(|(id, direction, retained)| Topic {
                id: id.to_string(),
                direction: *direction,
                retained: *retained,
                doc: String::new(),
            })
            .collect()
    }

    pub fn knows_unit(&self, u: &str) -> bool {
        UNITS.contains(&u) || self.unit.iter().any(|x| x.symbol == u)
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

    pub fn system_of(&self, node: &str) -> Option<&str> {
        self.system
            .iter()
            .find(|s| s.nodes.iter().any(|n| n == node))
            .map(|s| s.id.as_str())
    }

    pub fn known_units(&self) -> Vec<String> {
        UNITS
            .iter()
            .map(|u| u.to_string())
            .chain(self.unit.iter().map(|u| u.symbol.clone()))
            .collect()
    }
}

pub fn load(root: &Path, decl: &str) -> Result<Topology> {
    let raw = std::fs::read_to_string(root.join(decl)).with_context(|| {
        format!("fid-topology: cannot read `{decl}` — it is the declaration (`fid add capability topology`)")
    })?;
    let t: Topology =
        toml::from_str(&raw).with_context(|| format!("fid-topology: `{decl}` does not parse"))?;
    let mut problems = validate(&t);
    if problems.is_empty() {
        problems.extend(hardware_problems(root, &t)?);
    }
    if !problems.is_empty() {
        bail!(
            "fid-topology: `{decl}` does not hold together:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(t)
}

/// Every node's hardware variants against the module library and the
/// topology: pin roles and bus contracts (validations 3 and 5).
pub fn hardware_problems(root: &Path, t: &Topology) -> Result<Vec<String>> {
    let lib = crate::modules::library(root)?;
    let mut out = Vec::new();
    for n in &t.node {
        for rel in hardware_files(n) {
            match crate::modules::load(root, rel) {
                Ok(h) => out.extend(crate::modules::validate(t, n, rel, &h, &lib)),
                Err(e) => out.push(format!("node `{}`: {e:#}", n.id)),
            }
        }
    }
    Ok(out)
}

/// A node's hardware declarations: every variant, and the built one.
pub fn hardware_files(n: &Node) -> Vec<&String> {
    let mut files: Vec<&String> = n.variants.iter().collect();
    if let Some(h) = &n.hardware {
        if !files.contains(&h) {
            files.push(h);
        }
    }
    files
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

    for b in &t.bus {
        if b.kind.is_empty()
            || !b
                .kind
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            out.push(format!(
                "bus `{}`: type `{}` is not a lowercase name (i2c, rs485, can-fd, or the product's own)",
                b.id, b.kind
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

    let roles = t.roles();
    let mut param_ids = BTreeSet::new();
    for prm in &t.parameter {
        if !param_ids.insert((prm.scope.as_str(), prm.id.as_str())) {
            out.push(format!(
                "parameter `{}` is declared twice for `{}`",
                prm.id, prm.scope
            ));
        }
        if !roles.contains(&prm.scope) {
            out.push(format!(
                "parameter `{}`: scope `{}` is no node's firmware",
                prm.id, prm.scope
            ));
        }
        let [lo, hi] = prm.range;
        if lo >= hi {
            out.push(format!(
                "parameter `{}`: range [{lo}, {hi}] is empty",
                prm.id
            ));
        } else if !(lo <= prm.default && prm.default <= hi) {
            out.push(format!(
                "parameter `{}`: default {} is outside its range [{lo}, {hi}]",
                prm.id, prm.default
            ));
        }
        if !t.knows_unit(&prm.unit) {
            out.push(format!(
                "parameter `{}`: unit `{}` is not one this platform knows ({}) — declare it with [[unit]]",
                prm.id, prm.unit, t.known_units().join(", ")
            ));
        }
    }
    out
}

pub(crate) fn pascal(s: &str) -> String {
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

pub(crate) fn upper(s: &str) -> String {
    s.replace('-', "_").to_ascii_uppercase()
}

pub(crate) fn f(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

/// The languages firmware tables are generated in, chosen by the output's
/// extension. Any firmware — Embassy, ESP-IDF, Zephyr, Arduino, a vendor SDK,
/// MicroPython — reads one of these; `.json` is for every other language.
pub const FIRMWARE_EXTENSIONS: [&str; 3] = ["rs", "h", "json"];

/// One generated constant, in a language-neutral form.
pub(crate) enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
}

pub(crate) struct Table {
    /// `Point`, `Parameter`, `Topic`.
    pub name: &'static str,
    /// Each entry's identifier and its fields, in declaration order.
    pub entries: Vec<(String, Vec<(&'static str, Value)>)>,
    /// Standalone constants: `TIMETABLE_GRANULARITY_MIN`.
    pub constants: Vec<(String, Value)>,
}

pub(crate) fn opt_str(v: &Option<String>) -> Value {
    v.clone().map(Value::Str).unwrap_or(Value::None)
}

fn params_table(t: &Topology, role: &str) -> Table {
    let entries = t
        .parameter
        .iter()
        .filter(|p| p.scope == role)
        .enumerate()
        .map(|(i, p)| {
            (
                p.id.clone(),
                vec![
                    ("id", Value::Int(i as i64 + 1)),
                    ("unit", Value::Str(p.unit.clone())),
                    ("default", Value::Float(p.default)),
                    ("min", Value::Float(p.range[0])),
                    ("max", Value::Float(p.range[1])),
                    ("doc", Value::Str(p.doc.clone())),
                ],
            )
        })
        .collect();
    Table {
        name: "Parameter",
        entries,
        constants: Vec::new(),
    }
}

fn topics_table(t: &Topology) -> Table {
    let entries = t
        .topics()
        .iter()
        .enumerate()
        .map(|(i, k)| {
            (
                k.id.clone(),
                vec![
                    ("id", Value::Int(i as i64 + 1)),
                    ("suffix", Value::Str(k.id.clone())),
                    (
                        "direction",
                        Value::Str(
                            match k.direction {
                                Direction::Up => "up",
                                Direction::Down => "down",
                                Direction::Will => "will",
                            }
                            .into(),
                        ),
                    ),
                    ("retained", Value::Bool(k.retained)),
                    ("doc", Value::Str(k.doc.clone())),
                ],
            )
        })
        .collect();
    let constants = t
        .node
        .iter()
        .flat_map(|n| {
            let id = upper(&n.id);
            [
                (
                    format!("NODE_{id}_SYSTEM"),
                    Value::Str(t.system_of(&n.id).unwrap_or_default().into()),
                ),
                (format!("NODE_{id}_TARGET"), opt_str(&n.target)),
                (format!("NODE_{id}_RUNTIME"), opt_str(&n.runtime)),
                (format!("NODE_{id}_LANGUAGE"), opt_str(&n.language)),
            ]
        })
        .filter(|(_, v)| !matches!(v, Value::None))
        .collect();
    Table {
        name: "Topic",
        entries,
        constants,
    }
}

pub(crate) fn render(table: &Table, ext: &str, header: &str) -> Result<String> {
    Ok(match ext {
        "rs" => render_rs(table, header),
        "h" => render_h(table, header),
        "json" => render_json(table),
        other => bail!(
            "fid-topology: no firmware table in `.{other}` (supported: {})",
            FIRMWARE_EXTENSIONS.join(", ")
        ),
    })
}

fn render_rs(t: &Table, header: &str) -> String {
    let mut s = format!("// {header} — do not edit.\n\n");
    s.push_str(&format!(
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n#[repr(u16)]\npub enum {} {{\n",
        t.name
    ));
    for (i, (id, _)) in t.entries.iter().enumerate() {
        s.push_str(&format!("    {} = {},\n", pascal(id), i + 1));
    }
    s.push_str(&format!(
        "}}\n\npub const ALL: [{}; {}] = [",
        t.name,
        t.entries.len()
    ));
    s.push_str(
        &t.entries
            .iter()
            .map(|(id, _)| format!("{}::{}", t.name, pascal(id)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str(&format!(
        "];\n\nimpl {} {{\n    pub const fn id(self) -> u16 {{\n        self as u16\n    }}\n\n",
        t.name
    ));
    s.push_str(&format!(
        "    pub fn from_id(id: u16) -> Option<{0}> {{\n        ALL.iter().copied().find(|x| x.id() == id)\n    }}\n\n",
        t.name
    ));
    s.push_str("    pub const fn name(self) -> &'static str {\n        match self {\n");
    for (id, _) in &t.entries {
        s.push_str(&format!(
            "            {}::{} => {id:?},\n",
            t.name,
            pascal(id)
        ));
    }
    s.push_str("        }\n    }\n");
    let fields: Vec<&'static str> = t
        .entries
        .first()
        .map(|(_, f)| f.iter().map(|(k, _)| *k).filter(|k| *k != "id").collect())
        .unwrap_or_default();
    for field in fields {
        let vals: Vec<&Value> = t
            .entries
            .iter()
            .map(|(_, f)| {
                f.iter()
                    .find(|(k, _)| *k == field)
                    .map(|(_, v)| v)
                    .unwrap_or(&Value::None)
            })
            .collect();
        let optional = vals.iter().any(|v| matches!(v, Value::None));
        let ty = match vals.iter().find(|v| !matches!(v, Value::None)) {
            Some(Value::Int(_)) => "u32",
            Some(Value::Float(_)) => "f32",
            Some(Value::Bool(_)) => "bool",
            _ => "&'static str",
        };
        let ty = if optional {
            format!("Option<{ty}>")
        } else {
            ty.to_string()
        };
        s.push_str(&format!(
            "\n    pub const fn {field}(self) -> {ty} {{\n        match self {{\n"
        ));
        for ((id, _), v) in t.entries.iter().zip(&vals) {
            let lit = rs_lit(v);
            let lit = if optional && !matches!(v, Value::None) {
                format!("Some({lit})")
            } else {
                lit
            };
            s.push_str(&format!(
                "            {}::{} => {lit},\n",
                t.name,
                pascal(id)
            ));
        }
        s.push_str("        }\n    }\n");
    }
    s.push_str("}\n");
    for (k, v) in &t.constants {
        let ty = match v {
            Value::Int(_) => "u32",
            Value::Float(_) => "f32",
            Value::Bool(_) => "bool",
            _ => "&str",
        };
        s.push_str(&format!("\npub const {k}: {ty} = {};\n", rs_lit(v)));
    }
    s
}

fn rs_lit(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Float(x) => f(*x),
        Value::Str(x) => format!("{x:?}"),
        Value::Bool(b) => b.to_string(),
        Value::None => "None".into(),
    }
}

fn render_h(t: &Table, header: &str) -> String {
    let guard = format!("FID_{}_H", t.name.to_ascii_uppercase());
    let prefix = t.name.to_ascii_uppercase();
    let mut s = format!(
        "/* {header} — do not edit. */\n#ifndef {guard}\n#define {guard}\n\n#include <stdbool.h>\n#include <stdint.h>\n\n"
    );
    s.push_str(&format!("enum {} {{\n", t.name.to_ascii_lowercase()));
    for (i, (id, _)) in t.entries.iter().enumerate() {
        s.push_str(&format!("    {prefix}_{} = {},\n", upper(id), i + 1));
    }
    s.push_str("};\n\n");
    s.push_str(&format!("#define {prefix}_COUNT {}\n", t.entries.len()));
    for (id, fields) in &t.entries {
        for (k, v) in fields {
            if *k == "id" {
                continue;
            }
            let lit = match v {
                Value::Int(i) => format!("{i}"),
                Value::Float(x) => format!("{}f", f(*x)),
                Value::Str(x) => format!("{x:?}"),
                Value::Bool(b) => {
                    if *b {
                        "true".into()
                    } else {
                        "false".into()
                    }
                }
                Value::None => continue,
            };
            s.push_str(&format!(
                "#define {prefix}_{}_{} {lit}\n",
                upper(id),
                k.to_ascii_uppercase()
            ));
        }
    }
    for (k, v) in &t.constants {
        let lit = match v {
            Value::Int(i) => format!("{i}"),
            Value::Float(x) => format!("{}f", f(*x)),
            Value::Str(x) => format!("{x:?}"),
            Value::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Value::None => continue,
        };
        s.push_str(&format!("#define {k} {lit}\n"));
    }
    s.push_str(&format!("\n#endif /* {guard} */\n"));
    s
}

fn json_value(v: &Value) -> serde_json::Value {
    match v {
        Value::Int(i) => serde_json::json!(i),
        Value::Float(x) => serde_json::json!(x),
        Value::Str(x) => serde_json::json!(x),
        Value::Bool(b) => serde_json::json!(b),
        Value::None => serde_json::Value::Null,
    }
}

fn render_json(t: &Table) -> String {
    let entries: Vec<serde_json::Value> = t
        .entries
        .iter()
        .map(|(id, fields)| {
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!(id));
            for (k, v) in fields {
                m.insert((*k).into(), json_value(v));
            }
            serde_json::Value::Object(m)
        })
        .collect();
    let constants: serde_json::Map<String, serde_json::Value> = t
        .constants
        .iter()
        .map(|(k, v)| (k.clone(), json_value(v)))
        .collect();
    let doc = serde_json::json!({
        "_": "Derived by fid-topology from topology.toml — do not edit.",
        "entries": entries,
        "constants": constants,
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default() + "\n"
}

pub fn render_params(t: &Topology, role: &str, decl: &str, ext: &str) -> Result<String> {
    render(
        &params_table(t, role),
        ext,
        &format!("Derived by fid-topology from {decl} for firmware `{role}`"),
    )
}

pub fn render_topics(t: &Topology, decl: &str, ext: &str) -> Result<String> {
    render(
        &topics_table(t),
        ext,
        &format!("Derived by fid-topology from {decl}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SITE: &str = r#"
[[system]]
id = "site"
nodes = ["room_1", "server"]

[[node]]
id = "room_1"
kind = "mcu"
target = "riscv32imac-unknown-none-elf"
runtime = "embassy-esp"
firmware = "room"
interfaces = ["wifi", "i2c0"]

[[node]]
id = "server"
kind = "host"
target = "aarch64-unknown-linux-gnu"
runtime = "linux-compose"
apps = ["server"]
interfaces = ["eth0"]

[[bus]]
id = "room_i2c"
type = "i2c"
connect = ["room_1.i2c0"]

[[bus]]
id = "lan"
type = "ip"
protocol = "mqtt"
tls = true
broker = "server"
connect = ["room_1.wifi", "server.eth0"]

[[parameter]]
id = "co2_on"
scope = "room"
unit = "ppm"
range = [600, 2000]
default = 1000
"#;

    fn parse(s: &str) -> Topology {
        toml::from_str(s).unwrap()
    }

    #[test]
    fn the_site_holds_together() {
        assert_eq!(validate(&parse(SITE)), Vec::<String>::new());
    }

    #[test]
    fn a_bus_across_systems_fails_naming_both() {
        let s = SITE.replace(
            "nodes = [\"room_1\", \"server\"]",
            "nodes = [\"room_1\"]\n\n[[system]]\nid = \"office\"\nnodes = [\"server\"]",
        );
        let e = validate(&parse(&s)).join("\n");
        assert!(
            e.contains("bus `lan` crosses systems `office` and `site`"),
            "{e}"
        );
    }

    #[test]
    fn a_setting_outside_its_range_or_in_an_unknown_unit_fails() {
        let e = validate(&parse(&SITE.replace("default = 1000", "default = 5000"))).join("\n");
        assert!(e.contains("default 5000 is outside its range"), "{e}");
        let e = validate(&parse(&SITE.replace("unit = \"ppm\"", "unit = \"parts\""))).join("\n");
        assert!(
            e.contains("unit `parts` is not one this platform knows"),
            "{e}"
        );
    }

    #[test]
    fn mqtt_without_tls_is_refused() {
        let e = validate(&parse(&SITE.replace("tls = true\n", ""))).join("\n");
        assert!(e.contains("MQTT without `tls = true`"), "{e}");
    }

    #[test]
    fn any_firmware_reads_the_same_tables_in_its_own_language() {
        let t = parse(SITE);
        let h = render_params(&t, "room", DECLARATION, "h").unwrap();
        assert!(
            h.contains("#define PARAMETER_CO2_ON_DEFAULT 1000.0f"),
            "{h}"
        );
        let rs = render_params(&t, "room", DECLARATION, "rs").unwrap();
        assert!(rs.contains("Parameter::Co2On => 1000.0"), "{rs}");
        let j: serde_json::Value =
            serde_json::from_str(&render_params(&t, "room", DECLARATION, "json").unwrap()).unwrap();
        assert_eq!(j["entries"][0]["name"], "co2_on");
        assert!(render_params(&t, "room", DECLARATION, "py").is_err());
    }

    #[test]
    fn a_product_names_its_own_topics_runtime_and_units() {
        let s = SITE.replace(
            "runtime = \"embassy-esp\"\nfirmware = \"room\"",
            "runtime = \"zephyr\"\nlanguage = \"c\"\nfirmware = \"room\"",
        )
            + "\n[[topic]]\nid = \"telemetry\"\ndirection = \"up\"\n\n[[unit]]\nsymbol = \"dB\"\n";
        let t = parse(&s.replace("unit = \"ppm\"", "unit = \"dB\""));
        assert_eq!(validate(&t), Vec::<String>::new());
        let topics = render_topics(&t, DECLARATION, "h").unwrap();
        assert!(
            topics.contains("TOPIC_TELEMETRY = 1,") && !topics.contains("TOPIC_STATE"),
            "{topics}"
        );
        assert!(
            topics.contains("#define NODE_ROOM_1_RUNTIME \"zephyr\""),
            "{topics}"
        );
    }
}

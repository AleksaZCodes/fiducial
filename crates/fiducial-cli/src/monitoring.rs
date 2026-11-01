//! The `fid-monitoring` pipeline: what a monitoring and control product
//! measures, drives and schedules, declared once.
//!
//! `monitoring.toml` names the points (a measured or switched quantity with a
//! unit, a range and a binding), the rules that read and drive them, the
//! schedules a person edits, and which parameters draw limit lines. It reads
//! `topology.toml` for the nodes and parameters it refers to. Point tables, web
//! types and cards and the readings schema are derived from it. Products that
//! are not monitoring and control (a game controller, a flight controller) do
//! not install it: `topology` alone keeps their nodes, buses, pins and settings
//! consistent.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::topology::{self, opt_str, Node, Table, Topology, Value};

pub const DECLARATION: &str = "monitoring.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Monitoring {
    #[serde(default)]
    pub point: Vec<Point>,
    #[serde(default)]
    pub rule: Vec<Rule>,
    #[serde(default)]
    pub schedule: Vec<Schedule>,
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
    /// Parameters drawn as limit lines on this point's card.
    #[serde(default)]
    pub limits: Vec<String>,
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
    /// The topology parameters this rule reads.
    #[serde(default)]
    pub parameters: Vec<String>,
    #[serde(default)]
    pub doc: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    pub id: String,
    pub scope: String,
    pub granularity: String,
    pub default_when_missing: String,
}

impl Monitoring {
    pub fn role<'a>(&self, t: &'a Topology, node: &str) -> Option<&'a str> {
        t.node
            .iter()
            .find(|n| n.id == node)
            .and_then(|n| n.firmware.as_deref())
    }

    pub fn points_of_role(&self, t: &Topology, role: &str) -> Vec<&Point> {
        let Some(node) = t.first_node_of(role) else {
            return Vec::new();
        };
        self.point.iter().filter(|p| p.node == node.id).collect()
    }

    pub fn decides(&self, t: &Topology, point: &str, role: &str) -> Vec<&str> {
        let _ = t;
        self.rule
            .iter()
            .filter(|r| r.scope == role && r.reads.iter().any(|x| x == point))
            .map(|r| r.id.as_str())
            .collect()
    }
}

pub fn load(root: &Path, decl: &str, t: &Topology) -> Result<Monitoring> {
    let raw = std::fs::read_to_string(root.join(decl)).with_context(|| {
        format!("fid-monitoring: cannot read `{decl}` — it is the declaration (`fid add capability monitoring`)")
    })?;
    let m: Monitoring =
        toml::from_str(&raw).with_context(|| format!("fid-monitoring: `{decl}` does not parse"))?;
    let problems = validate(&m, t);
    if !problems.is_empty() {
        bail!(
            "fid-monitoring: `{decl}` does not hold together:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(m)
}

pub fn validate(m: &Monitoring, t: &Topology) -> Vec<String> {
    let mut out = Vec::new();
    let nodes: BTreeMap<&str, &Node> = t.node.iter().map(|n| (n.id.as_str(), n)).collect();
    let buses: BTreeMap<&str, &topology::Bus> = t.bus.iter().map(|b| (b.id.as_str(), b)).collect();
    let mut ids = BTreeSet::new();
    let mut addresses: BTreeMap<(String, String, u32), String> = BTreeMap::new();
    for p in &m.point {
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
                Some(u) if !t.knows_unit(u) => out.push(format!(
                    "point `{}`: unit `{u}` is not one this platform knows ({}) — declare it with [[unit]]",
                    p.id,
                    t.known_units().join(", ")
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
            if p.bus.is_none() && p.adc.is_none() && p.pin.is_none() && p.net.is_none() {
                out.push(format!(
                    "point `{}`: a sensor is bound to a bus, an ADC channel, a pin or an analog net",
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
                            if !shares_device(other, &p.id, m) {
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
            if !m.point.iter().any(|q| q.node == p.node && &q.id == il) {
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
            let mut v: Vec<String> = m
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

    let rules: BTreeMap<&str, &Rule> = m.rule.iter().map(|r| (r.id.as_str(), r)).collect();
    for r in &m.rule {
        if !roles.contains(&r.scope) {
            out.push(format!(
                "rule `{}`: scope `{}` is no node's firmware",
                r.id, r.scope
            ));
            continue;
        }
        let pts: Vec<&Point> = m.points_of_role(t, &r.scope);
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
        for prm in &r.parameters {
            match t.parameter.iter().find(|x| &x.id == prm) {
                None => out.push(format!(
                    "rule `{}` reads parameter `{prm}`, which topology.toml does not declare",
                    r.id
                )),
                Some(x) if x.scope != r.scope => out.push(format!(
                    "rule `{}` runs on `{}` but parameter `{prm}` is scoped `{}`",
                    r.id, r.scope, x.scope
                )),
                _ => {}
            }
        }
    }
    for prm in &t.parameter {
        if !m.rule.iter().any(|r| r.parameters.contains(&prm.id)) {
            out.push(format!("parameter `{}` is read by no rule", prm.id));
        }
    }
    for p in &m.point {
        if p.kind == PointKind::Sensor {
            let role = m.role(t, &p.node).unwrap_or_default();
            let read = !m.decides(t, &p.id, role).is_empty();
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
        for l in &p.limits {
            let ok = rules
                .values()
                .any(|r| r.reads.contains(&p.id) && r.parameters.contains(l));
            if !ok {
                out.push(format!(
                    "point `{}` draws limit `{l}`, which no rule that reads it uses",
                    p.id
                ));
            }
        }
    }
    for s in &m.schedule {
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

fn shares_device(a: &str, b: &str, m: &Monitoring) -> bool {
    let q = |id: &str| {
        m.point
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

pub fn schedules_table(m: &Monitoring, role: &str) -> Table {
    let entries = m
        .schedule
        .iter()
        .filter(|x| x.scope == role)
        .enumerate()
        .map(|(i, x)| {
            (
                x.id.clone(),
                vec![
                    ("id", Value::Int(i as i64 + 1)),
                    (
                        "granularity_min",
                        Value::Int(granularity_min(&x.granularity).unwrap_or(5) as i64),
                    ),
                    (
                        "occupied_when_missing",
                        Value::Bool(x.default_when_missing == "occupied"),
                    ),
                ],
            )
        })
        .collect();
    Table {
        name: "Schedule",
        entries,
        constants: Vec::new(),
    }
}

pub fn points_table(m: &Monitoring, t: &Topology, role: &str) -> Table {
    let entries = m
        .points_of_role(t, role)
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (lo, hi) = match p.range {
                Some([a, b]) => (Value::Float(a), Value::Float(b)),
                None => (Value::None, Value::None),
            };
            (
                p.id.clone(),
                vec![
                    ("id", Value::Int(i as i64 + 1)),
                    ("kind", Value::Str(kind_name(p.kind).into())),
                    ("unit", opt_str(&p.unit)),
                    ("min", lo),
                    ("max", hi),
                    ("bus", opt_str(&p.bus)),
                    (
                        "address",
                        p.address
                            .map(|a| Value::Int(a as i64))
                            .unwrap_or(Value::None),
                    ),
                    ("net", opt_str(&p.net)),
                    ("pin", opt_str(&p.pin.clone().or_else(|| p.adc.clone()))),
                    ("calibrated", Value::Bool(p.calibration.is_some())),
                    ("interlock", opt_str(&p.interlock)),
                ],
            )
        })
        .collect();
    Table {
        name: "Point",
        entries,
        constants: Vec::new(),
    }
}

pub fn render_points(
    m: &Monitoring,
    t: &Topology,
    role: &str,
    decl: &str,
    ext: &str,
) -> Result<String> {
    topology::render(
        &points_table(m, t, role),
        ext,
        &format!("Derived by fid-monitoring from {decl} for firmware `{role}`"),
    )
}

pub fn render_schedules(m: &Monitoring, role: &str, decl: &str, ext: &str) -> Result<String> {
    topology::render(
        &schedules_table(m, role),
        ext,
        &format!("Derived by fid-monitoring from {decl} for firmware `{role}`"),
    )
}

pub fn render_points_ts(m: &Monitoring, t: &Topology, decl: &str) -> String {
    let mut s = format!("// Derived by fid-monitoring from {decl} — do not edit.\n\n");
    s.push_str("export type PointKind = \"sensor\" | \"actuator\" | \"contact\"\n\n");
    s.push_str("export interface PointInfo {\n  readonly id: number\n  readonly name: string\n  readonly kind: PointKind\n  readonly quantity: string | null\n  readonly unit: string | null\n  readonly range: readonly [number, number] | null\n  readonly decides: readonly string[]\n}\n\n");
    s.push_str("export interface ParameterInfo {\n  readonly id: number\n  readonly name: string\n  readonly unit: string\n  readonly min: number\n  readonly max: number\n  readonly default: number\n}\n\n");
    s.push_str("export const POINTS = {\n");
    for role in t.roles() {
        s.push_str(&format!("  {role:?}: [\n"));
        for (i, p) in m.points_of_role(t, &role).iter().enumerate() {
            let range = p
                .range
                .map(|[a, b]| format!("[{a}, {b}]"))
                .unwrap_or_else(|| "null".into());
            let dec: Vec<String> = m
                .decides(t, &p.id, &role)
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
                "    {{ id: {}, name: {:?}, unit: {:?}, min: {}, max: {}, default: {} }},\n",
                i + 1,
                p.id,
                p.unit,
                p.range[0],
                p.range[1],
                p.default
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
    for r in &m.rule {
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

pub fn render_cards_ts(m: &Monitoring, t: &Topology, decl: &str) -> String {
    let mut s = format!(
        "// Derived by fid-monitoring from {decl} — do not edit.\n\n\
         export interface Card {{\n  readonly point: string\n  readonly unit: string\n  readonly limits: readonly string[]\n}}\n\n\
         export const CARDS = {{\n"
    );
    for role in t.roles() {
        s.push_str(&format!("  {role:?}: [\n"));
        for p in m
            .points_of_role(t, &role)
            .iter()
            .filter(|p| p.kind == PointKind::Sensor)
        {
            let limits: Vec<String> = p.limits.iter().map(|l| format!("{l:?}")).collect();
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

pub fn render_migration(m: &Monitoring, t: &Topology, decl: &str, dialect: &str) -> Result<String> {
    if dialect != "sqlite" {
        bail!(
            "fid-monitoring: migrations for `[adapters] database = \"{dialect}\"` are not derived yet — \
             `sqlite` and `d1` are"
        );
    }
    let roles = t.roles();
    let mut s = format!(
        "-- Derived by fid-monitoring from {decl} — do not edit.\n\
         -- Readings, hourly roll-ups, parameter values and schedule slots\n\
         -- for {} point(s) on {} node(s) across {} firmware role(s).\n\n",
        m.point.len(),
        t.node.len(),
        roles.len()
    );
    s.push_str(
        "CREATE TABLE IF NOT EXISTS reading (\n  node TEXT NOT NULL,\n  point INTEGER NOT NULL,\n  at INTEGER NOT NULL,\n  value REAL,\n  PRIMARY KEY (node, point, at)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS reading_hourly (\n  node TEXT NOT NULL,\n  point INTEGER NOT NULL,\n  hour INTEGER NOT NULL,\n  mean REAL,\n  min REAL,\n  max REAL,\n  samples INTEGER NOT NULL,\n  PRIMARY KEY (node, point, hour)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS parameter_value (\n  scope_node TEXT NOT NULL,\n  parameter INTEGER NOT NULL,\n  value REAL NOT NULL,\n  changed_at INTEGER NOT NULL,\n  PRIMARY KEY (scope_node, parameter)\n) WITHOUT ROWID;\n\n\
         CREATE TABLE IF NOT EXISTS schedule_slot (\n  node TEXT NOT NULL,\n  schedule TEXT NOT NULL,\n  weekday INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),\n  from_min INTEGER NOT NULL,\n  to_min INTEGER NOT NULL CHECK (to_min > from_min),\n  PRIMARY KEY (node, schedule, weekday, from_min)\n) WITHOUT ROWID;\n",

    );
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOPO: &str = r#"
[[system]]
id = "site"
nodes = ["room_1", "room_2", "server"]

[[node]]
id = "room_1"
kind = "mcu"
firmware = "room"

[[node]]
id = "room_2"
kind = "mcu"
firmware = "room"

[[node]]
id = "server"
kind = "host"
runtime = "linux-compose"
apps = ["server"]

[[bus]]
id = "i2c"
type = "i2c"
connect = ["room_1.i2c0", "room_2.i2c0"]

[[parameter]]
id = "co2_on"
scope = "room"
unit = "ppm"
range = [600, 2000]
default = 1000
"#;

    const MON: &str = r#"
[[point]]
id = "co2"
node = "room_1"
kind = "sensor"
unit = "ppm"
range = [400, 5000]
bus = "i2c"
address = 0x62
limits = ["co2_on"]

[[point]]
id = "fan"
node = "room_1"
kind = "actuator"
net = "RELAY_FAN"

[[point]]
id = "co2"
node = "room_2"
kind = "sensor"
unit = "ppm"
range = [400, 5000]
bus = "i2c"
address = 0x62
limits = ["co2_on"]

[[point]]
id = "fan"
node = "room_2"
kind = "actuator"
net = "RELAY_FAN"

[[rule]]
id = "ventilation"
scope = "room"
reads = ["co2"]
drives = ["fan"]
parameters = ["co2_on"]
"#;

    fn check(mon: &str) -> String {
        let t: Topology = toml::from_str(TOPO).unwrap();
        let m: Monitoring = toml::from_str(mon).unwrap();
        validate(&m, &t).join("\n")
    }

    #[test]
    fn a_closed_declaration_holds() {
        assert_eq!(check(MON), "");
    }

    #[test]
    fn a_rule_a_limit_and_a_unit_must_all_resolve() {
        let e = check(&MON.replace("parameters = [\"co2_on\"]", "parameters = []"));
        assert!(
            e.contains("parameter `co2_on` is read by no rule")
                && e.contains("draws limit `co2_on`"),
            "{e}"
        );
        let e = check(&MON.replace("unit = \"ppm\"\nrange = [400, 5000]\nbus = \"i2c\"\naddress = 0x62\nlimits = [\"co2_on\"]\n\n[[point]]\nid = \"fan\"\nnode = \"room_1\"", "unit = \"parts\"\nrange = [400, 5000]\nbus = \"i2c\"\naddress = 0x62\nlimits = [\"co2_on\"]\n\n[[point]]\nid = \"fan\"\nnode = \"room_1\""));
        assert!(e.contains("unit `parts`"), "{e}");
    }

    #[test]
    fn nodes_of_one_firmware_declare_the_same_points() {
        let e = check(&MON.replace("[[point]]\nid = \"fan\"\nnode = \"room_2\"\nkind = \"actuator\"\nnet = \"RELAY_FAN\"\n", ""));
        assert!(
            e.contains("share firmware `room` but declare different points"),
            "{e}"
        );
    }

    #[test]
    fn point_tables_and_cards_carry_every_point() {
        let t: Topology = toml::from_str(TOPO).unwrap();
        let m: Monitoring = toml::from_str(MON).unwrap();
        let rs = render_points(&m, &t, "room", DECLARATION, "rs").unwrap();
        assert!(
            rs.contains("Co2 = 1,") && rs.contains("Point::Co2 => Some(\"i2c\")"),
            "{rs}"
        );
        let h = render_points(&m, &t, "room", DECLARATION, "h").unwrap();
        assert!(h.contains("#define POINT_CO2_ADDRESS 98"), "{h}");
        let cards = render_cards_ts(&m, &t, DECLARATION);
        assert!(
            cards.contains("{ point: \"co2\", unit: \"ppm\", limits: [\"co2_on\"] }"),
            "{cards}"
        );
    }
}

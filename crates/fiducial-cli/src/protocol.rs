//! The `fid-protocol` pipeline: every message a device and its apps exchange,
//! declared once in `protocol.toml`, derived into a Rust codec for the
//! firmware and a TypeScript codec for the web.
//!
//! The fact this exists to hold is the byte layout of a payload. Written by
//! hand on both sides it drifts: in Outreach the radio and the app disagreed on
//! a message's structure, and in the sensor stick the reading's layout was
//! written once in Rust and again in TypeScript with only a hand-written test
//! between them (the paper's study, case H9). Declared here, both sides are
//! generated from one table and cannot disagree.
//!
//! Framing (start, length, CRC) stays `fiducial-protocol`'s job; this is the
//! payload inside a frame: one `kind` byte, then the fields, little-endian.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

/// The declaration's default path, relative to the product root.
pub const DECLARATION: &str = "protocol.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    #[serde(default, rename = "message")]
    pub messages: Vec<Message>,
    /// The USB IDs the device enumerates with and the page asks the browser
    /// for: one number on two sides, so it is declared here with the rest of
    /// the interface (the paper's round 4, case R20).
    #[serde(default)]
    pub usb: Option<Usb>,
    /// Named ranges of a field, in its unit: the thresholds firmware and page
    /// act on, declared once (case R14).
    #[serde(default, rename = "band")]
    pub bands: Vec<Band>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usb {
    pub vendor_id: u16,
    pub product_id: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    /// `snake_case`; `COMFORTABLE_MIN` / `_MAX` in Rust.
    pub name: String,
    /// `<message>.<field>`.
    pub field: String,
    /// Inclusive bounds in the field's unit — `35.0` %RH, not `3500`.
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub doc: String,
}

/// What a field name's leading SI prefix says one count is worth. A field
/// called `centi_percent_rh` with `scale = 0.1` says two different things,
/// and the page shows ten times the humidity (case R11).
const PREFIXES: &[(&str, f64)] = &[
    ("deci", 0.1),
    ("centi", 0.01),
    ("milli", 0.001),
    ("micro", 0.000_001),
    ("kilo", 1000.0),
];

/// The scale a field's name implies, when it starts with an SI prefix.
fn implied_scale(name: &str) -> Option<(&'static str, f64)> {
    let head = name.split('_').next()?;
    PREFIXES.iter().find(|(p, _)| *p == head).copied()
}

/// A band bound in counts: the unit value divided by the field's scale.
fn counts(f: &Field, v: f64) -> f64 {
    v / f.scale.unwrap_or(1.0)
}

impl File {
    fn field(&self, path: &str) -> Option<(&Message, &Field)> {
        let (m, f) = path.split_once('.')?;
        let m = self.messages.iter().find(|x| x.name == m)?;
        Some((m, m.fields.iter().find(|x| x.name == f)?))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// `snake_case`; the Rust module and the TypeScript names follow from it.
    pub name: String,
    /// The payload's first byte, unique per message.
    pub kind: u8,
    #[serde(default)]
    pub doc: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub doc: String,
    /// What one count is worth: `0.01` for a value sent in hundredths. The
    /// web side's `<message>Values` applies it, so no page divides by hand.
    #[serde(default)]
    pub scale: Option<f64>,
    /// The unit of the scaled value: `"°C"`, `"%RH"`.
    #[serde(default)]
    pub unit: Option<String>,
}

/// A field type: its size, its Rust type and the `DataView` accessor.
struct Ty {
    size: usize,
    rust: &'static str,
    view: &'static str,
}

fn ty(name: &str) -> Option<Ty> {
    let t = |size, rust, view| Some(Ty { size, rust, view });
    match name {
        "u8" => t(1, "u8", "Uint8"),
        "i8" => t(1, "i8", "Int8"),
        "u16" => t(2, "u16", "Uint16"),
        "i16" => t(2, "i16", "Int16"),
        "u32" => t(4, "u32", "Uint32"),
        "i32" => t(4, "i32", "Int32"),
        "f32" => t(4, "f32", "Float32"),
        _ => None,
    }
}

/// The values an integer field type can carry.
fn int_range(ty: &str) -> (f64, f64) {
    match ty {
        "u8" => (0.0, u8::MAX as f64),
        "i8" => (i8::MIN as f64, i8::MAX as f64),
        "u16" => (0.0, u16::MAX as f64),
        "i16" => (i16::MIN as f64, i16::MAX as f64),
        "u32" => (0.0, u32::MAX as f64),
        _ => (i32::MIN as f64, i32::MAX as f64),
    }
}

fn is_snake(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn pascal(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn camel(s: &str) -> String {
    let p = pascal(s);
    let mut c = p.chars();
    c.next()
        .map(|f| f.to_ascii_lowercase().to_string() + c.as_str())
        .unwrap_or_default()
}

/// Read and check the declaration: names, unique kinds, known types.
pub fn load(root: &Path, decl: &str) -> Result<File> {
    let raw = std::fs::read_to_string(root.join(decl)).with_context(|| {
        format!("fid-protocol: cannot read `{decl}` — it is the declaration (`fid add capability protocol`)")
    })?;
    let file: File =
        toml::from_str(&raw).with_context(|| format!("fid-protocol: `{decl}` does not parse"))?;
    if file.messages.is_empty() {
        bail!("fid-protocol: `{decl}` declares no [[message]]");
    }
    let mut problems = Vec::new();
    for (i, m) in file.messages.iter().enumerate() {
        if !is_snake(&m.name) {
            problems.push(format!("message `{}`: name must be snake_case", m.name));
        }
        if let Some(o) = file.messages[..i].iter().find(|o| o.kind == m.kind) {
            problems.push(format!(
                "message `{}`: kind {} is already `{}`'s — each message needs its own",
                m.name, m.kind, o.name
            ));
        }
        if file.messages[..i].iter().any(|o| o.name == m.name) {
            problems.push(format!("message `{}` is declared twice", m.name));
        }
        if m.fields.is_empty() {
            problems.push(format!("message `{}` has no fields", m.name));
        }
        for (j, f) in m.fields.iter().enumerate() {
            if !is_snake(&f.name) {
                problems.push(format!(
                    "message `{}`: field `{}` must be snake_case",
                    m.name, f.name
                ));
            }
            if m.fields[..j].iter().any(|o| o.name == f.name) {
                problems.push(format!(
                    "message `{}`: field `{}` is declared twice",
                    m.name, f.name
                ));
            }
            if ty(&f.ty).is_none() {
                problems.push(format!(
                    "message `{}`: field `{}` has type `{}` (one of u8, i8, u16, i16, u32, i32, f32)",
                    m.name, f.name, f.ty
                ));
            }
            if let (Some((prefix, implied)), Some(scale)) = (implied_scale(&f.name), f.scale) {
                if ((scale - implied) / implied).abs() > 1e-9 {
                    problems.push(format!(
                        "message `{}`: field `{}` has scale = {scale}, but its name says \
                         {prefix} ({implied}) — rename the field or correct the scale",
                        m.name, f.name
                    ));
                }
            }
        }
    }
    for (i, b) in file.bands.iter().enumerate() {
        if !is_snake(&b.name) {
            problems.push(format!("band `{}`: name must be snake_case", b.name));
        }
        if file.bands[..i]
            .iter()
            .any(|o| o.name == b.name && o.field.split('.').next() == b.field.split('.').next())
        {
            problems.push(format!("band `{}` is declared twice", b.name));
        }
        let Some((_, f)) = file.field(&b.field) else {
            problems.push(format!(
                "band `{}`: field = \"{}\" names no declared `<message>.<field>`",
                b.name, b.field
            ));
            continue;
        };
        if b.min > b.max {
            problems.push(format!(
                "band `{}`: min {} is above max {}",
                b.name, b.min, b.max
            ));
        }
        if f.ty != "f32" {
            for v in [b.min, b.max] {
                let c = counts(f, v);
                if (c - c.round()).abs() > 1e-6 {
                    problems.push(format!(
                        "band `{}`: {v} is {c} counts of `{}` — not a whole number",
                        b.name, b.field
                    ));
                }
                // A bound the field cannot carry is a band no reading can
                // reach — most often one written in counts, not in the unit
                // (the paper's round 5, case M12).
                let (lo, hi) = int_range(&f.ty);
                if c < lo || c > hi {
                    problems.push(format!(
                        "band `{}`: {v}{} is {c} counts, outside what a {} holds ({lo}–{hi}) — \
                         no reading can reach it. A band is written in the field's unit{}, not in counts",
                        b.name,
                        f.unit.as_deref().map(|u| format!(" {u}")).unwrap_or_default(),
                        f.ty,
                        f.unit.as_deref().map(|u| format!(" ({u})")).unwrap_or_default(),
                    ));
                }
            }
        }
    }
    if !problems.is_empty() {
        bail!(
            "fid-protocol: `{decl}` does not hold together:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(file)
}

fn len(m: &Message) -> usize {
    1 + m
        .fields
        .iter()
        .map(|f| ty(&f.ty).map_or(0, |t| t.size))
        .sum::<usize>()
}

/// The firmware's side: `no_std`, no dependencies.
pub fn render_rust(file: &File, decl: &str) -> String {
    let mut s = format!(
        "//! Derived by fid-protocol from {decl} — do not edit.\n\
         //!\n\
         //! One module per message: `KIND`, `LEN`, the struct, and `encode` /\n\
         //! `decode` for its payload (the kind byte, then each field\n\
         //! little-endian). The web page's `protocol/messages.ts` is derived\n\
         //! from the same table, so the two cannot disagree.\n\
         #![allow(dead_code)]\n"
    );
    if let Some(u) = &file.usb {
        s.push_str(&format!(
            "\n/// The USB IDs the device enumerates with; the page filters on the same.\n\
             pub const USB_VENDOR_ID: u16 = {:#06x};\npub const USB_PRODUCT_ID: u16 = {:#06x};\n",
            u.vendor_id, u.product_id
        ));
    }
    if !file.bands.is_empty() {
        s.push_str(
            "\n/// Where a value falls against a declared band. The bounds are inside —\n\
             /// the page's `<message>Band` decides the same way, in the same counts.\n\
             #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
             pub enum Band {\n    Below,\n    Inside,\n    Above,\n}\n",
        );
    }
    for m in &file.messages {
        let name = pascal(&m.name);
        let n = len(m);
        s.push_str(&format!(
            "\n/// {}\npub mod {} {{\n",
            one_line(&m.doc, &m.name),
            m.name
        ));
        s.push_str(&format!(
            "    pub const KIND: u8 = {};\n    pub const LEN: usize = {n};\n\n",
            m.kind
        ));
        s.push_str(&format!(
            "    #[derive(Debug, Clone, Copy, PartialEq)]\n    pub struct {name} {{\n"
        ));
        for f in &m.fields {
            if !f.doc.is_empty() {
                s.push_str(&format!("        /// {}\n", f.doc));
            }
            s.push_str(&format!(
                "        pub {}: {},\n",
                f.name,
                ty(&f.ty).unwrap().rust
            ));
        }
        s.push_str("    }\n\n");
        s.push_str(&format!("    impl {name} {{\n        pub fn encode(&self) -> [u8; LEN] {{\n            let mut b = [0u8; LEN];\n            b[0] = KIND;\n"));
        let mut at = 1;
        for f in &m.fields {
            let t = ty(&f.ty).unwrap();
            s.push_str(&format!(
                "            b[{at}..{}].copy_from_slice(&self.{}.to_le_bytes());\n",
                at + t.size,
                f.name
            ));
            at += t.size;
        }
        s.push_str("            b\n        }\n\n");
        s.push_str("        pub fn decode(b: &[u8]) -> Option<Self> {\n            if b.len() != LEN || b[0] != KIND {\n                return None;\n            }\n            Some(Self {\n");
        let mut at = 1;
        for f in &m.fields {
            let t = ty(&f.ty).unwrap();
            let bytes: Vec<String> = (at..at + t.size).map(|i| format!("b[{i}]")).collect();
            s.push_str(&format!(
                "                {}: {}::from_le_bytes([{}]),\n",
                f.name,
                t.rust,
                bytes.join(", ")
            ));
            at += t.size;
        }
        s.push_str("            })\n        }\n    }\n");
        for b in file
            .bands
            .iter()
            .filter(|b| b.field.split('.').next() == Some(&m.name))
        {
            let (_, f) = file.field(&b.field).unwrap();
            let t = ty(&f.ty).unwrap();
            let unit = f.unit.as_deref().unwrap_or("");
            let lit = |v: f64| {
                let c = counts(f, v);
                if f.ty == "f32" {
                    format!("{c:?}")
                } else {
                    format!("{}", c.round() as i64)
                }
            };
            s.push_str(&format!(
                "\n    /// {} — `{}` from {} to {} {unit}, inclusive, in counts.\n\
                 \x20   pub const {}_MIN: {} = {};\n    pub const {}_MAX: {} = {};\n",
                one_line(&b.doc, &b.name),
                f.name,
                b.min,
                b.max,
                b.name.to_ascii_uppercase(),
                t.rust,
                lit(b.min),
                b.name.to_ascii_uppercase(),
                t.rust,
                lit(b.max),
            ));
            let upper = b.name.to_ascii_uppercase();
            s.push_str(&format!(
                "\n    /// Where `{}` falls against `{}`, bounds inside. Test the band with\n\
                 \x20   /// this, not with the bounds: one rule, on both sides.\n\
                 \x20   pub fn {}(counts: {}) -> super::Band {{\n\
                 \x20       if counts < {upper}_MIN {{\n\
                 \x20           super::Band::Below\n\
                 \x20       }} else if counts > {upper}_MAX {{\n\
                 \x20           super::Band::Above\n\
                 \x20       }} else {{\n\
                 \x20           super::Band::Inside\n\
                 \x20       }}\n    }}\n",
                f.name, b.name, b.name, t.rust
            ));
        }
        s.push_str("}\n");
    }
    s
}

/// The web's side: plain TypeScript, no imports.
pub fn render_ts(file: &File, decl: &str) -> String {
    let mut s = format!(
        "// Derived by fid-protocol from {decl} — do not edit.\n\
         // The firmware's protocol/messages.rs is derived from the same table.\n"
    );
    if let Some(u) = &file.usb {
        s.push_str(&format!(
            "\n/** The USB IDs the device enumerates with: filter on these. */\n\
             export const USB_VENDOR_ID = {:#06x}\nexport const USB_PRODUCT_ID = {:#06x}\n",
            u.vendor_id, u.product_id
        ));
    }
    for m in &file.messages {
        let name = pascal(&m.name);
        let upper = m.name.to_ascii_uppercase();
        let n = len(m);
        s.push_str(&format!("\n/** {} */\n", one_line(&m.doc, &m.name)));
        s.push_str(&format!(
            "export const {upper}_KIND = {}\nexport const {upper}_LEN = {n}\n\n",
            m.kind
        ));
        s.push_str(&format!("export interface {name} {{\n"));
        for f in &m.fields {
            if !f.doc.is_empty() {
                s.push_str(&format!("  /** {} */\n", f.doc));
            }
            s.push_str(&format!("  {}: number\n", camel(&f.name)));
        }
        s.push_str("}\n\n");
        s.push_str(&format!(
            "/** A payload as a `{}`, or null when it is another kind of message. */\nexport function decode{name}(p: Uint8Array): {name} | null {{\n  if (p.length !== {n} || p[0] !== {}) return null\n  const v = new DataView(p.buffer, p.byteOffset, p.byteLength)\n  return {{\n",
            m.name, m.kind
        ));
        let mut at = 1;
        for f in &m.fields {
            let t = ty(&f.ty).unwrap();
            let le = if t.size > 1 { ", true" } else { "" };
            s.push_str(&format!(
                "    {}: v.get{}({at}{le}),\n",
                camel(&f.name),
                t.view
            ));
            at += t.size;
        }
        s.push_str("  }\n}\n\n");
        s.push_str(&format!(
            "export function encode{name}(m: {name}): Uint8Array {{\n  const p = new Uint8Array({n})\n  const v = new DataView(p.buffer)\n  p[0] = {}\n",
            m.kind
        ));
        let mut at = 1;
        for f in &m.fields {
            let t = ty(&f.ty).unwrap();
            let le = if t.size > 1 { ", true" } else { "" };
            s.push_str(&format!(
                "  v.set{}({at}, m.{}{le})\n",
                t.view,
                camel(&f.name)
            ));
            at += t.size;
        }
        s.push_str("  return p\n}\n");
        if m.fields
            .iter()
            .any(|f| f.scale.is_some() || f.unit.is_some())
        {
            s.push_str(&format!(
                "\n/** Each field's unit, as declared. */\nexport const {upper}_UNITS = {{\n"
            ));
            for f in &m.fields {
                s.push_str(&format!(
                    "  {}: {:?},\n",
                    camel(&f.name),
                    f.unit.clone().unwrap_or_default()
                ));
            }
            s.push_str("} as const\n\n");
            s.push_str(&format!(
                "/** A `{}` in its units: each field times its declared scale. */\nexport function {}Values(m: {name}): {name} {{\n  return {{\n",
                m.name,
                camel(&m.name)
            ));
            for f in &m.fields {
                let k = camel(&f.name);
                match f.scale {
                    Some(sc) => s.push_str(&format!("    {k}: m.{k} * {sc:?},\n")),
                    None => s.push_str(&format!("    {k}: m.{k},\n")),
                }
            }
            s.push_str("  }\n}\n");
        }
        let bands: Vec<&Band> = file
            .bands
            .iter()
            .filter(|b| b.field.split('.').next() == Some(&m.name))
            .collect();
        if !bands.is_empty() {
            s.push_str(&format!(
                "\n/** Named ranges, in each field's unit, inclusive — the firmware acts on the same. */\nexport const {upper}_BANDS = {{\n"
            ));
            for b in &bands {
                let f = b.field.split_once('.').unwrap().1;
                s.push_str(&format!(
                    "  {}: {{ field: {:?}, min: {:?}, max: {:?} }},\n",
                    camel(&b.name),
                    camel(f),
                    b.min,
                    b.max
                ));
            }
            s.push_str("} as const\n");
            s.push_str(&format!(
                "\n/** Where a value, in its unit, falls against a band — bounds inside, compared in\n * counts exactly as the firmware's `{}::<band>()` does. Test bands with this, not with min/max. */\n\
                 export function {}Band(band: keyof typeof {upper}_BANDS, value: number): Band {{\n  switch (band) {{\n",
                m.name,
                camel(&m.name)
            ));
            for b in &bands {
                let (_, f) = file.field(&b.field).unwrap();
                let (v, lo, hi) = match (f.ty.as_str(), f.scale) {
                    ("f32", _) | (_, None) => (
                        "value".to_string(),
                        format!("{:?}", b.min),
                        format!("{:?}", b.max),
                    ),
                    (_, Some(sc)) => (
                        format!("Math.round(value / {sc:?})"),
                        format!("{}", counts(f, b.min).round() as i64),
                        format!("{}", counts(f, b.max).round() as i64),
                    ),
                };
                s.push_str(&format!(
                    "    case {:?}: {{\n      const c = {v}\n      return c < {lo} ? 'below' : c > {hi} ? 'above' : 'inside'\n    }}\n",
                    camel(&b.name)
                ));
            }
            s.push_str("  }\n}\n");
        }
    }
    if !file.bands.is_empty() {
        s.push_str("\n/** Where a value falls against a declared band. */\nexport type Band = 'below' | 'inside' | 'above'\n");
    }
    s
}

/// Where hand-written code compares a scaled field with a bare number, or
/// types a USB ID, instead of using what this declaration generates.
///
/// `match r.centi_percent_rh { 35..=60 => … }` reads as percent and is
/// hundredths of a percent (the paper's round 4, case R14); a band declared
/// in the unit is converted once, here. `Config::new(0x1209, …)` in firmware
/// beside `usbVendorId: 0x2e8a` on the page is the same number twice (R20).
/// Zero means the same in every unit and is allowed. A line that means it says
/// `// fid: allow-units` (or `allow-usb`).
pub fn typed_literals(root: &Path, file: &File) -> Vec<String> {
    let scaled: Vec<String> = file
        .messages
        .iter()
        .flat_map(|m| m.fields.iter())
        .filter(|f| f.scale.is_some())
        .flat_map(|f| [f.name.clone(), camel(&f.name)])
        .filter(|n| n.contains(|c: char| c == '_' || c.is_ascii_uppercase()))
        .collect();
    let mut files = Vec::new();
    crate::i18n::collect_code(root, &mut files);
    collect_ext(&root.join("firmware"), ".rs", &mut files);
    files.sort();
    files.dedup();
    let mut out = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.starts_with("protocol/") || rel.contains("/generated/") {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut in_match: Option<(String, i32)> = None;
        for (i, line) in src.lines().enumerate() {
            let code = strip_comment(line);
            let at = format!("  {rel}:{}", i + 1);
            if file.usb.is_some() && !line.contains("fid: allow-usb") {
                let typed = [
                    "Config::new(",
                    "usbVendorId:",
                    "vendorId:",
                    "usbProductId:",
                    "productId:",
                ]
                .iter()
                .any(|k| {
                    code.match_indices(k).any(|(j, _)| {
                        code[j + k.len()..]
                            .trim_start()
                            .starts_with(|c: char| c.is_ascii_digit())
                    })
                });
                if typed {
                    out.push(format!(
                        "{at}: types a USB ID — use USB_VENDOR_ID / USB_PRODUCT_ID from the \
                         generated protocol, declared once in `[usb]`"
                    ));
                }
            }
            if !file.bands.is_empty() && !line.contains("fid: allow-band") {
                let ts_bounds =
                    code.contains("_BANDS") && (code.contains("min") || code.contains("max"));
                let rs_bounds = file.bands.iter().any(|b| {
                    let u = b.name.to_ascii_uppercase();
                    [format!("{u}_MIN"), format!("{u}_MAX")]
                        .iter()
                        .any(|k| code.contains(k.as_str()) && !code.contains("use "))
                });
                if ts_bounds || rs_bounds {
                    out.push(format!(
                        "{at}: tests a band against its bounds by hand — call the generated \
                         band function (`<message>Band(…)` on the page, `<message>::<band>(…)` in \
                         firmware), so both sides agree on whether a bound is inside"
                    ));
                }
            }
            if line.contains("fid: allow-units") {
                continue;
            }
            if let Some((field, depth)) = &mut in_match {
                if let Some((pat, _)) = code.split_once("=>") {
                    if has_nonzero_int(pat) {
                        out.push(units_message(&at, field));
                    }
                }
                *depth += brace_delta(code);
                if *depth <= 0 {
                    in_match = None;
                }
                continue;
            }
            for f in &scaled {
                let dotted = format!(".{f}");
                for (j, _) in code.match_indices(&dotted) {
                    let after = &code[j + dotted.len()..];
                    if after.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    if receiverless(&code[..j]).ends_with("match") {
                        if code.trim_end().ends_with('{') {
                            in_match = Some((f.clone(), brace_delta(code)));
                        }
                        continue;
                    }
                    let a = after.trim_start();
                    let compared_after = ["<=", ">=", "==", "!=", "<", ">"]
                        .iter()
                        .find(|op| a.starts_with(**op))
                        .is_some_and(|op| starts_nonzero_int(a[op.len()..].trim_start()));
                    if compared_after || compared_before(&code[..j]) {
                        out.push(units_message(&at, f));
                    }
                }
            }
        }
    }
    out
}

fn units_message(at: &str, field: &str) -> String {
    format!(
        "{at}: compares `{field}` with a bare number — the field is in counts of its \
         declared scale; declare the threshold as a [[band]] in protocol.toml and use \
         its generated band function"
    )
}

fn strip_comment(line: &str) -> &str {
    line.split("//").next().unwrap_or(line)
}

fn brace_delta(s: &str) -> i32 {
    s.chars().filter(|&c| c == '{').count() as i32 - s.chars().filter(|&c| c == '}').count() as i32
}

/// An integer literal other than zero: `35`, `3_500`, `0x10`.
fn starts_nonzero_int(s: &str) -> bool {
    let s = s.trim_start_matches('-');
    let lit: String = s
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    lit.starts_with(|c: char| c.is_ascii_digit())
        && lit
            .trim_start_matches("0x")
            .chars()
            .any(|c| c.is_ascii_digit() && c != '0')
}

fn has_nonzero_int(pat: &str) -> bool {
    let mut prev = ' ';
    for (i, c) in pat.char_indices() {
        if c.is_ascii_digit()
            && !(prev.is_alphanumeric() || prev == '_')
            && starts_nonzero_int(&pat[i..])
        {
            return true;
        }
        prev = c;
    }
    false
}

/// `35 < r.field`: a literal on the left of a comparison that ends here.
fn compared_before(head: &str) -> bool {
    let rest = receiverless(head);
    ["<=", ">=", "==", "!=", "<", ">"].iter().any(|op| {
        rest.strip_suffix(op).is_some_and(|lhs| {
            let lhs = lhs.trim_end();
            let start = lhs
                .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .map_or(0, |k| k + 1);
            starts_nonzero_int(&lhs[start..])
        })
    })
}

/// What comes before `r.field`, without the `r`.
fn receiverless(head: &str) -> &str {
    head.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '.')
        .trim_end()
}

fn collect_ext(dir: &Path, ext: &str, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name != "target" && !name.starts_with('.') {
                collect_ext(&p, ext, out);
            }
        } else if name.ends_with(ext) {
            out.push(p);
        }
    }
}

fn one_line(doc: &str, name: &str) -> String {
    if doc.is_empty() {
        format!("The `{name}` message.")
    } else {
        doc.lines().next().unwrap_or(doc).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECL: &str = r#"
[[message]]
name = "reading"
kind = 1
doc = "One reading."
fields = [
  { name = "centi_celsius", type = "i16" },
  { name = "centi_percent_rh", type = "u16" },
]
"#;

    fn file(s: &str) -> Result<File> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DECLARATION), s).unwrap();
        load(dir.path(), DECLARATION)
    }

    #[test]
    fn both_sides_carry_the_same_layout() {
        let f = file(DECL).unwrap();
        let rs = render_rust(&f, DECLARATION);
        let ts = render_ts(&f, DECLARATION);
        assert!(rs.contains("pub const LEN: usize = 5;"));
        assert!(rs.contains("b[1..3].copy_from_slice(&self.centi_celsius.to_le_bytes());"));
        assert!(rs.contains("centi_percent_rh: u16::from_le_bytes([b[3], b[4]]),"));
        assert!(ts.contains("export const READING_LEN = 5"));
        assert!(ts.contains("centiCelsius: v.getInt16(1, true),"));
        assert!(ts.contains("centiPercentRh: v.getUint16(3, true),"));
    }

    #[test]
    fn a_shared_kind_an_unknown_type_and_a_bad_name_are_named() {
        let e = file(
            r#"
[[message]]
name = "a"
kind = 1
fields = [{ name = "x", type = "u24" }]
[[message]]
name = "B"
kind = 1
fields = [{ name = "y", type = "u8" }]
"#,
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("type `u24`"), "{e}");
        assert!(e.contains("kind 1 is already `a`'s"), "{e}");
        assert!(e.contains("`B`: name must be snake_case"), "{e}");
    }

    const SCALED: &str = r#"
[usb]
vendor_id  = 0x2e8a
product_id = 0x000a

[[message]]
name = "reading"
kind = 1
fields = [
  { name = "centi_celsius", type = "i16", scale = 0.01, unit = "°C" },
  { name = "centi_percent_rh", type = "u16", scale = 0.01, unit = "%RH" },
]

[[band]]
name  = "comfortable"
field = "reading.centi_percent_rh"
min   = 35.0
max   = 60.0
"#;

    #[test]
    fn a_scale_that_disagrees_with_the_fields_name_is_refused() {
        let e = file(&SCALED.replace(
            "type = \"u16\", scale = 0.01",
            "type = \"u16\", scale = 0.1",
        ))
        .unwrap_err()
        .to_string();
        assert!(e.contains("its name says centi (0.01)"), "{e}");
    }

    #[test]
    fn usb_ids_and_bands_reach_both_sides_in_their_own_terms() {
        let f = file(SCALED).unwrap();
        let rs = render_rust(&f, DECLARATION);
        let ts = render_ts(&f, DECLARATION);
        assert!(
            rs.contains("pub const USB_VENDOR_ID: u16 = 0x2e8a;"),
            "{rs}"
        );
        assert!(ts.contains("export const USB_VENDOR_ID = 0x2e8a"), "{ts}");
        // Declared in %RH, generated in counts for the firmware...
        assert!(
            rs.contains("pub const COMFORTABLE_MIN: u16 = 3500;"),
            "{rs}"
        );
        assert!(
            rs.contains("pub const COMFORTABLE_MAX: u16 = 6000;"),
            "{rs}"
        );
        // ...and in %RH for the page, which already shows values in units.
        assert!(
            ts.contains("comfortable: { field: \"centiPercentRh\", min: 35.0, max: 60.0 }"),
            "{ts}"
        );
        let e = file(&SCALED.replace(
            "field = \"reading.centi_percent_rh\"",
            "field = \"reading.rh\"",
        ))
        .unwrap_err()
        .to_string();
        assert!(e.contains("names no declared"), "{e}");
        // One rule for "inside" on both sides, bounds included.
        assert!(
            rs.contains("pub fn comfortable(counts: u16) -> super::Band"),
            "{rs}"
        );
        assert!(ts.contains("export function readingBand("), "{ts}");
        assert!(
            ts.contains("return c < 3500 ? 'below' : c > 6000 ? 'above' : 'inside'"),
            "{ts}"
        );
        // A band written in counts is one no reading can reach.
        let e = file(&SCALED.replace(
            "min   = 35.0\nmax   = 60.0",
            "min   = 3500.0\nmax   = 6000.0",
        ))
        .unwrap_err()
        .to_string();
        assert!(
            e.contains("outside what a u16 holds") && e.contains("not in counts"),
            "{e}"
        );
    }

    #[test]
    fn a_band_tested_against_its_bounds_by_hand_is_named() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DECLARATION), SCALED).unwrap();
        let f = load(dir.path(), DECLARATION).unwrap();
        let web = dir.path().join("web/src");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(
            web.join("reading.ts"),
            "const ok = readingBand('comfortable', v) === 'inside'\n",
        )
        .unwrap();
        assert!(typed_literals(dir.path(), &f).is_empty());
        std::fs::write(
            web.join("reading.ts"),
            "const { min, max } = READING_BANDS.comfortable\nconst ok = v > min && v < max\n",
        )
        .unwrap();
        let found = typed_literals(dir.path(), &f);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("reading.ts:1") && found[0].contains("by hand"),
            "{found:?}"
        );
    }

    #[test]
    fn a_bare_number_against_a_scaled_field_or_a_typed_usb_id_is_named() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DECLARATION), SCALED).unwrap();
        let f = load(dir.path(), DECLARATION).unwrap();
        let fw = dir.path().join("firmware/src");
        std::fs::create_dir_all(&fw).unwrap();
        let good = "fn colour(r: Reading) -> u8 {\n    let band = reading::comfortable(r.centi_percent_rh);\n    match band {\n        Band::Inside => 1,\n        _ => 2,\n    }\n}\nlet c = Config::new(USB_VENDOR_ID, USB_PRODUCT_ID);\nif r.centi_celsius < 0 { }\n";
        std::fs::write(fw.join("main.rs"), good).unwrap();
        assert!(
            typed_literals(dir.path(), &f).is_empty(),
            "{:?}",
            typed_literals(dir.path(), &f)
        );
        let bad = "fn colour(r: Reading) -> u8 {\n    match r.centi_percent_rh {\n        0..=34 => 1,\n        35..=60 => 2,\n        _ => 3,\n    }\n}\nlet c = Config::new(0x1209, 0x0001);\nif 3_000 < r.centi_celsius { }\nlet hot = r.centi_celsius >= 3_000;\n";
        std::fs::write(fw.join("main.rs"), bad).unwrap();
        let found = typed_literals(dir.path(), &f);
        assert_eq!(found.len(), 5, "{found:#?}");
        assert!(
            found
                .iter()
                .any(|m| m.contains("main.rs:8") && m.contains("USB")),
            "{found:#?}"
        );
        std::fs::write(
            dir.path().join("page.ts"),
            "const s = await open({ filters: [{ usbVendorId: 0x2e8a }] })\nif (r.centiPercentRh > 6000) warn()\n",
        )
        .unwrap();
        assert_eq!(typed_literals(dir.path(), &f).len(), 7);
    }
}

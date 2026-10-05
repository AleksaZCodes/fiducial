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
        s.push_str("            })\n        }\n    }\n}\n");
    }
    s
}

/// The web's side: plain TypeScript, no imports.
pub fn render_ts(file: &File, decl: &str) -> String {
    let mut s = format!(
        "// Derived by fid-protocol from {decl} — do not edit.\n\
         // The firmware's protocol/messages.rs is derived from the same table.\n"
    );
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
    }
    s
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
}

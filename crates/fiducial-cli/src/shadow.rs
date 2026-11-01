//! Shadow copies: a declared value typed again into hand-written code.
//!
//! The most common way a declared product drifts is not an edited artifact but
//! a second copy of a fact in code nobody derives: a footer that types the
//! domain and the contact email (the paper's independent set, I6), a page that
//! divides by a typed `100` where the protocol declares the scale (I20). Each
//! earlier rule caught one such value in one place. This is the one scan for
//! all of them, keyed by value rather than by place:
//!
//! - a **distinctive string** declared anywhere in `fiducial.toml` or a
//!   pipeline's TOML declaration — an email, a domain or URL, a colour, a
//!   multi-word name — found in any hand-written source file;
//! - a **number** declared in a pipeline's TOML declaration, found as a literal
//!   in a hand-written file that imports what that pipeline derives. The
//!   import is what makes the number this fact rather than a coincidence: a
//!   file that reads the generated protocol and also types `100` is typing the
//!   protocol's scale.
//!
//! Tests are left out: they state expected values on purpose. A line that
//! means its literal says `fid: allow-value`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::pipeline::Pipeline;

const ALLOW: &str = "fid: allow-value";

/// Blocks whose strings are configuration of the platform, not facts about
/// the product: vendor names, rule IDs, locale tags, file paths.
const NOT_FACTS: [&str; 8] = [
    "product",
    "capabilities",
    "guard",
    "adapters",
    "spine",
    "i18n",
    "identity",
    "freshness",
];

/// Hand-written source in any language a product's firmware or apps use: web,
/// Rust, C and C++ (ESP-IDF, Zephyr, Arduino, vendor SDKs) and Python.
const CODE: [&str; 19] = [
    ".ts", ".tsx", ".js", ".jsx", ".mjs", ".svelte", ".vue", ".astro", ".rs", ".html", ".css",
    ".c", ".h", ".cpp", ".hpp", ".cc", ".ino", ".py", ".go",
];

#[derive(Debug, Clone)]
struct Fact {
    /// Where it is declared: `fiducial.toml [brand] domain`.
    origin: String,
    /// Which generated name to use instead, when one is known.
    instead: String,
}

/// Every shadow copy under `root`, as `--check` lines.
pub fn copies(root: &Path, pipelines: &[&Pipeline]) -> Vec<String> {
    let owned = derived_and_platform_files(root, pipelines);
    let strings = string_facts(root, pipelines);
    let lists = list_facts(root);
    let numbers = number_facts(root, pipelines);
    let mut files = Vec::new();
    collect(root, root, &mut files);
    let mut out = Vec::new();
    for path in files {
        let rel = rel(root, &path);
        if owned.contains(&rel) || is_test(&rel) {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let imported: Vec<&(String, BTreeMap<String, Fact>)> = numbers
            .iter()
            .filter(|(outputs, _)| imports_any(&src, outputs))
            .collect();
        for (i, line) in src.lines().enumerate() {
            if line.contains("#[cfg(test)]") {
                break;
            }
            if line.contains(ALLOW) || is_import(line) {
                continue;
            }
            let code = strip_line_comment(line, &rel);
            let at = format!("  {rel}:{}", i + 1);
            let mut said = BTreeSet::new();
            for literal in string_lists(code) {
                if let Some((_, origin)) = lists.iter().find(|(set, _)| *set == literal) {
                    if said.insert(origin.clone()) {
                        out.push(format!(
                            "{at}: types the list declared in {origin} — import it from what that \
                             block derives, so a value added there reaches this line"
                        ));
                    }
                }
            }
            for (value, fact) in &strings {
                if contains_token(code, value) && said.insert(value.clone()) {
                    out.push(format!(
                        "{at}: types \"{value}\", declared in {} — {}",
                        fact.origin, fact.instead
                    ));
                }
            }
            for (_, facts) in &imported {
                for n in numeric_literals(code) {
                    if let Some(fact) = facts.get(&key(n)) {
                        if said.insert(key(n)) {
                            out.push(format!(
                                "{at}: types {}, declared in {} — {}",
                                trim_num(n),
                                fact.origin,
                                fact.instead
                            ));
                        }
                    }
                }
            }
        }
    }
    out
}

/// Files nobody hand-writes: every artifact a pipeline derives and every file
/// the platform installed. A copy there is the derivation, not a shadow.
fn derived_and_platform_files(root: &Path, pipelines: &[&Pipeline]) -> BTreeSet<String> {
    let mut owned: BTreeSet<String> = pipelines
        .iter()
        .flat_map(|p| p.outputs.iter().cloned())
        .collect();
    if let Ok(lock) = crate::lock::Lock::load(&root.join(crate::lock::LOCK_FILE)) {
        owned.extend(lock.templates.keys().cloned());
        owned.extend(lock.artifacts.keys().cloned());
    }
    owned
}

fn string_facts(root: &Path, pipelines: &[&Pipeline]) -> BTreeMap<String, Fact> {
    let mut facts = BTreeMap::new();
    if let Some(doc) = read_toml(&root.join(crate::config::CONFIG_FILE)) {
        if let Some(t) = doc.as_table() {
            for (block, v) in t {
                if NOT_FACTS.contains(&block.as_str()) {
                    continue;
                }
                walk(
                    v,
                    &mut |path, leaf| {
                        if let Some(s) = leaf.as_str().filter(|s| distinctive(s)) {
                            let key = path.join(".");
                            facts.entry(s.to_string()).or_insert(Fact {
                                origin: format!("fiducial.toml [{block}] {key}"),
                                instead: instead_for(block),
                            });
                        }
                    },
                    &mut Vec::new(),
                );
            }
        }
    }
    for (decl, _) in declarations(root, pipelines) {
        if let Some(doc) = read_toml(&root.join(&decl)) {
            walk(
                &doc,
                &mut |path, leaf| {
                    if let Some(s) = leaf.as_str().filter(|s| distinctive(s)) {
                        facts.entry(s.to_string()).or_insert(Fact {
                            origin: format!("{decl} {}", path.join(".")),
                            instead: "read it from what that declaration derives".into(),
                        });
                    }
                },
                &mut Vec::new(),
            );
        }
    }
    facts
}

/// Every list of two or more strings declared in `fiducial.toml`, as a set,
/// with where it is declared. A hand-typed `["en", "sr"]` is the locale list
/// again, in any order (the paper's fourth set, F18).
fn list_facts(root: &Path) -> Vec<(BTreeSet<String>, String)> {
    let mut out = Vec::new();
    let Some(doc) = read_toml(&root.join(crate::config::CONFIG_FILE)) else {
        return out;
    };
    let Some(t) = doc.as_table() else {
        return out;
    };
    for (block, v) in t {
        if ["capabilities", "guard"].contains(&block.as_str()) {
            continue;
        }
        let Some(tbl) = v.as_table() else { continue };
        for (k, val) in tbl {
            let Some(arr) = val.as_array() else { continue };
            let set: BTreeSet<String> = arr
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect();
            if set.len() >= 2 && set.len() == arr.len() {
                out.push((set, format!("fiducial.toml [{block}] {k}")));
            }
        }
    }
    out
}

/// The string elements of each bracketed list on a line: `["en", "sr"]`.
fn string_lists(code: &str) -> Vec<BTreeSet<String>> {
    let mut out = Vec::new();
    let mut rest = code;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else { break };
        let inner = &after[..close];
        let items: Vec<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .collect();
        let strings: Vec<String> = items
            .iter()
            .filter_map(|x| {
                let q = x.chars().next()?;
                (matches!(q, '"' | '\'' | '`') && x.len() >= 2 && x.ends_with(q))
                    .then(|| x[1..x.len() - 1].to_string())
            })
            .collect();
        if strings.len() >= 2 && strings.len() == items.len() {
            out.push(strings.into_iter().collect());
        }
        rest = &after[close + 1..];
    }
    out
}

fn instead_for(block: &str) -> String {
    match block {
        "brand" => "read it from the generated brand module, so a rename reaches this line".into(),
        "legal" => "read it from the generated legal module".into(),
        _ => format!("read it from what `[{block}]` derives"),
    }
}

/// Per pipeline with a TOML declaration: the stems of what it derives, and the
/// numbers it declares.
fn number_facts(root: &Path, pipelines: &[&Pipeline]) -> Vec<(String, BTreeMap<String, Fact>)> {
    let mut out = Vec::new();
    for (decl, outputs) in declarations(root, pipelines) {
        let Some(doc) = read_toml(&root.join(&decl)) else {
            continue;
        };
        let mut facts = BTreeMap::new();
        walk(
            &doc,
            &mut |path, leaf| {
                let v = match leaf {
                    toml::Value::Integer(i) => *i as f64,
                    toml::Value::Float(f) => *f,
                    _ => return,
                };
                let name = path.last().cloned().unwrap_or_default();
                let mut add = |n: f64, how: &str| {
                    if meaningful(n) {
                        facts.entry(key(n)).or_insert(Fact {
                        origin: format!("{decl} {}{how}", path.join(".")),
                        instead: "use the generated value, so a change to the declaration reaches this line"
                            .into(),
                    });
                    }
                };
                add(v, "");
                if name == "scale" && v != 0.0 {
                    add(1.0 / v, " (its inverse)");
                }
            },
            &mut Vec::new(),
        );
        if !facts.is_empty() {
            out.push((outputs, facts));
        }
    }
    out
}

/// Each pipeline's TOML declaration (other than `fiducial.toml`) and the
/// stems of its outputs joined by `|`.
fn declarations(root: &Path, pipelines: &[&Pipeline]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for p in pipelines {
        let stems: Vec<String> = p
            .outputs
            .iter()
            .filter_map(|o| {
                Path::new(o)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .map(|s| s.trim_end_matches(".generated").to_string())
            .collect();
        for input in crate::commands::derive::pipeline_inputs(p, root) {
            let file = input.split('#').next().unwrap_or(&input).to_string();
            if file.ends_with(".toml")
                && file != crate::config::CONFIG_FILE
                && root.join(&file).is_file()
            {
                out.push((file, stems.join("|")));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn imports_any(src: &str, stems: &str) -> bool {
    let stems: Vec<&str> = stems.split('|').filter(|s| !s.is_empty()).collect();
    src.lines().filter(|l| is_import(l)).any(|l| {
        stems.iter().any(|s| {
            l.match_indices(s).any(|(i, _)| {
                let before = l[..i].chars().last();
                let after = l[i + s.len()..].chars().next();
                !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                    && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
            })
        })
    })
}

/// A test states expected values on purpose: it is the oracle the derivation
/// is checked against, not a second source of the fact.
fn is_test(rel: &str) -> bool {
    rel.contains(".test.")
        || rel.contains(".spec.")
        || rel.contains("_test.")
        || rel.split('/').any(|d| d == "tests" || d == "__tests__")
}

fn is_import(line: &str) -> bool {
    let t = line.trim_start();
    [
        "import ",
        "use ",
        "pub use ",
        "mod ",
        "pub mod ",
        "include!",
        "#include",
        "export * from",
        "export {",
    ]
    .iter()
    .any(|k| t.starts_with(k))
        || (t.contains("require(") && t.contains("const "))
        || (t.starts_with("} from ") || t.starts_with("from "))
}

/// A string another file would only contain by copying it.
fn distinctive(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 6 || s.starts_with("REPLACE_WITH") || s.contains('\n') {
        return false;
    }
    let email = s.contains('@') && s.rsplit('@').next().is_some_and(is_domain);
    let url = s.starts_with("https://") || s.starts_with("http://");
    let colour =
        s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit());
    let name = s.contains(' ')
        && s.split_whitespace().count() <= 6
        && s.chars().next().is_some_and(|c| c.is_uppercase())
        && !s.ends_with('.')
        && s.split_whitespace()
            .filter(|w| w.chars().next().is_some_and(char::is_uppercase))
            .count()
            >= 2;
    email || url || colour || is_domain(s) || name
}

fn is_domain(s: &str) -> bool {
    const FILES: [&str; 14] = [
        "json", "md", "svg", "toml", "ts", "rs", "png", "csv", "js", "sql", "html", "css", "txt",
        "xml",
    ];
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        && parts
            .last()
            .is_some_and(|t| t.len() >= 2 && t.chars().all(|c| c.is_ascii_lowercase()))
        && !FILES.contains(parts.last().unwrap_or(&""))
}

/// Not a number every program types for its own reasons.
fn meaningful(n: f64) -> bool {
    let a = n.abs();
    a > 2.0 && a != 10.0 && a.fract() == 0.0 || (a.fract() != 0.0 && a > 0.0 && a != 0.5)
}

fn key(n: f64) -> String {
    format!("{:.6}", n)
}

fn trim_num(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Literals not inside a string and not part of an identifier: `100`,
/// `0x2e8a`, `3_500`, `35.0`.
fn numeric_literals(code: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let b = code.as_bytes();
    let mut i = 0;
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            if c == q && (i == 0 || b[i - 1] != b'\\') {
                quote = None;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' || c == b'`' {
            quote = Some(c);
            i += 1;
            continue;
        }
        let prev_ident = i > 0
            && (b[i - 1].is_ascii_alphanumeric()
                || b[i - 1] == b'_'
                || b[i - 1] == b'.'
                || b[i - 1] == b'#');
        if c.is_ascii_digit() && !prev_ident {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.') {
                if b[i] == b'.' && !(i + 1 < b.len() && b[i + 1].is_ascii_digit()) {
                    break;
                }
                i += 1;
            }
            let lit = code[start..i].replace('_', "");
            let v = if let Some(h) = lit.strip_prefix("0x") {
                let h: String = h.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                i64::from_str_radix(&h, 16).ok().map(|v| v as f64)
            } else {
                let n: String = lit
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                n.parse::<f64>().ok()
            };
            if let Some(v) = v {
                out.push(v);
            }
            continue;
        }
        i += 1;
    }
    out
}

fn contains_token(code: &str, value: &str) -> bool {
    code.match_indices(value).any(|(i, _)| {
        let before = code[..i].chars().last();
        let after = code[i + value.len()..].chars().next();
        let edge =
            |c: Option<char>| !c.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_');
        edge(before) && edge(after)
    })
}

fn strip_line_comment<'a>(line: &'a str, rel: &str) -> &'a str {
    if rel.ends_with(".css") || rel.ends_with(".html") {
        return line;
    }
    let t = line.trim_start();
    if t.starts_with("//") || t.starts_with("*") || t.starts_with("/*") {
        return "";
    }
    if rel.ends_with(".py") {
        if t.starts_with('#') {
            return "";
        }
        return match line.find(" #") {
            Some(i) => &line[..i],
            None => line,
        };
    }
    match line.find(" // ") {
        Some(i) => &line[..i],
        None => line,
    }
}

fn walk(v: &toml::Value, f: &mut dyn FnMut(&[String], &toml::Value), path: &mut Vec<String>) {
    match v {
        toml::Value::Table(t) => {
            for (k, v) in t {
                path.push(k.clone());
                walk(v, f, path);
                path.pop();
            }
        }
        toml::Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                let label = v
                    .get("name")
                    .or_else(|| v.get("id"))
                    .and_then(|n| n.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| i.to_string());
                path.push(label);
                walk(v, f, path);
                path.pop();
            }
        }
        leaf => f(path, leaf),
    }
}

fn read_toml(path: &Path) -> Option<toml::Value> {
    toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            let skip = name.starts_with('.')
                || [
                    "node_modules",
                    "target",
                    "dist",
                    "build",
                    "generated",
                    "harvest",
                    "out",
                ]
                .contains(&name.as_str())
                || (p != root && p.join(crate::config::CONFIG_FILE).exists());
            if !skip {
                collect(root, &p, out);
            }
        } else if !name.contains(".generated.") && CODE.iter().any(|x| name.ends_with(x)) {
            out.push(p);
        }
    }
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinctive_strings_are_the_ones_only_a_copy_would_contain() {
        for s in [
            "hello@example.com",
            "example.com",
            "https://example.com",
            "#0EA5E9",
            "Example LLC",
        ] {
            assert!(distinctive(s), "{s}");
        }
        for s in [
            "necessary",
            "EU",
            "Example",
            "messages/en.json",
            "REPLACE_WITH_ID",
            "Hello there.",
        ] {
            assert!(!distinctive(s), "{s}");
        }
    }

    #[test]
    fn a_c_firmware_that_includes_the_tables_is_scanned_like_rust() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("protocol.toml"), "[usb]\nvendor_id = 0x2e8a\n").unwrap();
        std::fs::create_dir_all(root.join("firmware/main")).unwrap();
        std::fs::write(
            root.join("firmware/main/app.c"),
            "#include \"messages.h\"\nint vid = 0x2e8a; // the vendor\n",
        )
        .unwrap();
        let p = Pipeline {
            name: "protocol".into(),
            executor: "fid-protocol".into(),
            outputs: vec!["firmware/main/generated/messages.h".into()],
            ..Default::default()
        };
        let found = copies(root, &[&p]);
        assert!(
            found
                .iter()
                .any(|m| m.contains("firmware/main/app.c:2: types 11914")),
            "{found:?}"
        );
    }

    #[test]
    fn a_list_is_found_in_any_order() {
        assert_eq!(
            string_lists(r#"const LOCALES = ["sr", 'en'] as const; f([1, 2])"#),
            vec![["en".to_string(), "sr".to_string()]
                .into_iter()
                .collect::<BTreeSet<_>>()]
        );
        assert!(string_lists("x[0] + y[\"a\"]").is_empty());
    }

    #[test]
    fn literals_outside_strings_and_identifiers() {
        assert_eq!(numeric_literals("r.centiCelsius / 100"), vec![100.0]);
        assert_eq!(
            numeric_literals("Config::new(0x2e8a, 0x000a)"),
            vec![11914.0, 10.0]
        );
        assert_eq!(
            numeric_literals("let a = \"100\"; b2 = 3_500u16"),
            vec![3500.0]
        );
        assert_eq!(numeric_literals("x = 35.0"), vec![35.0]);
    }

    #[test]
    fn a_scale_is_found_as_its_inverse_in_a_file_that_imports_the_protocol() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join("protocol.toml"),
            "[[message]]\nname = \"reading\"\nkind = 1\nfields = [{ name = \"centi\", type = \"i16\", scale = 0.01 }]\n",
        )
        .unwrap();
        std::fs::write(
            root.join("fiducial.toml"),
            "[brand]\ndomain = \"example.com\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("web")).unwrap();
        std::fs::write(
            root.join("web/page.ts"),
            "import { decodeReading } from '../protocol/messages.ts'\nconst c = r.centi / 100\nconst u = 'https://example.com/x'\n",
        )
        .unwrap();
        std::fs::write(root.join("web/other.ts"), "const c = r.centi / 100\n").unwrap();
        let p = Pipeline {
            name: "protocol".into(),
            executor: "fid-protocol".into(),
            outputs: vec!["protocol/messages.rs".into(), "protocol/messages.ts".into()],
            ..Default::default()
        };
        let found = copies(root, &[&p]);
        assert!(
            found.iter().any(
                |m| m.contains("web/page.ts:2: types 100") && m.contains("scale (its inverse)")
            ),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|m| m.contains("web/page.ts:3") && m.contains("[brand] domain")),
            "{found:?}"
        );
        assert!(
            !found.iter().any(|m| m.contains("other.ts")),
            "no import, no claim: {found:?}"
        );
    }
}

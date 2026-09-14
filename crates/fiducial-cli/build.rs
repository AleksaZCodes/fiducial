//! Extract the platform's principles from `MISSION.md` at build time.
//!
//! The principles are declared in exactly one place: `MISSION.md`. A scaffolded
//! product cannot link to that file — its own `MISSION.md` states what the
//! *product* is for — so it needs the text. Copying it into the template would
//! be a second declaration, which is the thing principle 1 forbids.
//!
//! So the template carries a `{{principles}}` placeholder and this script fills
//! it from the source of truth. Editing `MISSION.md` changes what every future
//! product inherits, with no second file to remember.
//!
//! An earlier attempt kept a hand-written copy in the template and added a test
//! asserting the two agreed. That caught drift but did not prevent it — a gated
//! duplicate is still a duplicate.

use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let mission = Path::new(&manifest)
        .ancestors()
        .nth(2)
        .expect("fiducial-cli sits two levels below the workspace root")
        .join("MISSION.md");

    println!("cargo:rerun-if-changed={}", mission.display());

    let text = std::fs::read_to_string(&mission)
        .unwrap_or_else(|e| panic!("reading {}: {e}", mission.display()));

    let principles = extract_principles(&text)
        .unwrap_or_else(|| panic!("no principles section found in {}", mission.display()));

    let out = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("principles.md");
    std::fs::write(&out, principles).expect("writing principles.md");
}

/// The body between the principles heading and the next top-level heading.
///
/// Matched on the `· ` marker each principle carries rather than on the heading
/// text, so renaming the section — "Seven principles" stopped being accurate the
/// moment 1b, 1c, 5b and 5c were added — does not silently produce an empty
/// block.
fn extract_principles(text: &str) -> Option<String> {
    let mut lines = text.lines().enumerate();

    let start = lines.find_map(|(index, line)| {
        let lower = line.to_ascii_lowercase();
        (line.starts_with("## ") && lower.contains("principle")).then_some(index)
    })?;

    let body: Vec<&str> = text.lines().skip(start + 1).collect();
    let end = body
        .iter()
        .position(|line| line.starts_with("## "))
        .unwrap_or(body.len());

    let section = body[..end].join("\n");
    let section = section.trim_matches(|c: char| c == '\n' || c == '-' || c.is_whitespace());

    // A section that carries no principle markers means the format moved.
    if !section.contains(" · ") {
        return None;
    }
    Some(section.to_string())
}

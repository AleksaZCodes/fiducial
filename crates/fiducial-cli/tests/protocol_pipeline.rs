//! The `protocol` capability: a message's payload declared once in
//! `protocol.toml`, derived into a Rust codec for the firmware and a
//! TypeScript codec for the web — so the two sides cannot disagree.
//!
//! The case it exists for is the paper's H9: a reading's layout written once
//! in the firmware and again in the web page, with only a hand-written test
//! between them.

use std::{path::Path, process::Command};

fn fid(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fid"))
        .args(args)
        .current_dir(dir)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to invoke fid")
}

fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn ok(root: &Path, args: &[&str]) {
    let out = fid(root, args);
    assert!(out.status.success(), "fid {args:?} failed: {}", text(&out));
}

const READING: &str = r#"
[[message]]
name   = "reading"
kind   = 1
fields = [
  { name = "centi_celsius", type = "i16" },
  { name = "centi_percent_rh", type = "u16" },
]
"#;

fn product(tmp: &Path) -> std::path::PathBuf {
    ok(tmp, &["new", "demo"]);
    let root = tmp.join("demo");
    ok(&root, &["add", "capability", "protocol"]);
    std::fs::write(root.join("protocol.toml"), READING).unwrap();
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
    root
}

#[test]
fn one_declaration_gives_both_sides_the_same_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let rs = std::fs::read_to_string(root.join("protocol/messages.rs")).unwrap();
    let ts = std::fs::read_to_string(root.join("protocol/messages.ts")).unwrap();
    assert!(
        rs.contains("centi_celsius: i16::from_le_bytes([b[1], b[2]])"),
        "{rs}"
    );
    assert!(ts.contains("centiCelsius: v.getInt16(1, true)"), "{ts}");

    // Swap the fields in the declaration: both sides move together.
    std::fs::write(
        root.join("protocol.toml"),
        READING.replace(
            "{ name = \"centi_celsius\", type = \"i16\" },\n  { name = \"centi_percent_rh\", type = \"u16\" },",
            "{ name = \"centi_percent_rh\", type = \"u16\" },\n  { name = \"centi_celsius\", type = \"i16\" },",
        ),
    )
    .unwrap();
    let out = fid(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "a moved field is drift until derived"
    );
    assert!(text(&out).contains("protocol.toml"), "{}", text(&out));
    ok(&root, &["derive"]);
    let rs = std::fs::read_to_string(root.join("protocol/messages.rs")).unwrap();
    let ts = std::fs::read_to_string(root.join("protocol/messages.ts")).unwrap();
    assert!(
        rs.contains("centi_celsius: i16::from_le_bytes([b[3], b[4]])"),
        "{rs}"
    );
    assert!(ts.contains("centiCelsius: v.getInt16(3, true)"), "{ts}");
}

#[test]
fn a_codec_edited_by_hand_fails_check() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let p = root.join("protocol/messages.ts");
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(&p, s.replace("getInt16(1, true)", "getInt16(3, true)")).unwrap();
    let out = fid(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "a hand-edited codec passed --check");
    assert!(
        text(&out).contains("protocol/messages.ts"),
        "{}",
        text(&out)
    );
}

#[test]
fn the_generated_rust_compiles_and_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let src = format!(
        "#[path = {:?}] mod messages;\nfn main() {{\n    use messages::reading::Reading;\n    let r = Reading {{ centi_celsius: -1234, centi_percent_rh: 4810 }};\n    assert_eq!(Reading::decode(&r.encode()), Some(r));\n    assert_eq!(Reading::decode(&[2, 0, 0, 0, 0]), None);\n}}\n",
        root.join("protocol/messages.rs")
    );
    let main = tmp.path().join("main.rs");
    std::fs::write(&main, src).unwrap();
    let bin = tmp.path().join("rt");
    let out = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&bin)
        .arg(&main)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(Command::new(&bin).status().unwrap().success());
}

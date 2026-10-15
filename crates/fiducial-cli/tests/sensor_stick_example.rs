//! The public example, changed the way round 5 of the paper's study changed
//! it: each datasheet fact the board depends on is refused when contradicted.
//!
//! The example is copied, so the repository's own copy is never touched.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fid(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fid"))
        .args(args)
        .current_dir(dir)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to invoke fid")
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let name = e.file_name();
        if ["target", "node_modules", "build"].contains(&name.to_string_lossy().as_ref()) {
            continue;
        }
        let (src, dst) = (e.path(), to.join(&name));
        if src.is_dir() {
            copy(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

fn example() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("stick");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sensor-stick"),
        &root,
    );
    (tmp, root)
}

fn edit(root: &Path, file: &str, from: &str, to: &str) {
    let p = root.join(file);
    let s = std::fs::read_to_string(&p).unwrap();
    assert!(s.contains(from), "{file} has no {from:?}");
    std::fs::write(&p, s.replacen(from, to, 1)).unwrap();
}

/// Derive fails, and says this.
fn refused(root: &Path, says: &str) {
    let out = fid(root, &["derive"]);
    assert!(
        !out.status.success() && text(&out).contains(says),
        "{}",
        text(&out)
    );
}

#[test]
fn a_crystal_the_mcu_cannot_use_is_refused_once_ordered() {
    let (_t, root) = example();
    edit(
        &root,
        "hardware/product.toml",
        r#"lcsc      = "C9002""#,
        r#"lcsc      = "C13738""#,
    );
    refused(&root, "LCSC C13738 is not in hardware/parts.lock");
    // Resolved, the catalog says 16 MHz, and the RP2040 needs 12.
    edit(
        &root,
        "hardware/parts.lock",
        r#"lcsc = "C9002""#,
        r#"lcsc = "C13738""#,
    );
    edit(
        &root,
        "hardware/parts.lock",
        "Crystal 12MHz",
        "Crystal 16MHz",
    );
    refused(&root, "runs at 16 MHz, but it needs 12 MHz");
}

#[test]
fn a_datasheet_wait_typed_into_firmware_is_named() {
    let (_t, root) = example();
    let main = "firmware/rp2040/src/main.rs";
    let p = root.join(main);
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(&p, format!("{s}\nconst MEASURE_MS: u64 = 8;\n")).unwrap();
    refused(&root, "writes out AHT20's `MEASURE_MS` wait by hand");
}

#[test]
fn a_light_behind_an_opaque_case_is_refused() {
    let (_t, root) = example();
    edit(
        &root,
        "hardware/product.toml",
        "\"#1d4ed8b3\"",
        "\"#111827\"",
    );
    refused(
        &root,
        "`led` is a light inside the case, and the case is opaque",
    );
    edit(
        &root,
        "hardware/product.toml",
        "\"#111827\"",
        "\"#15803db3\"",
    );
    let out = fid(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
}

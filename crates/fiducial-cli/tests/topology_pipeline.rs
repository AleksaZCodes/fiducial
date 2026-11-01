//! The `topology` capability: computers, the buses between them, their modules
//! and wiring and their settings, declared once in `topology.toml`.

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

fn product(tmp: &Path) -> std::path::PathBuf {
    ok(tmp, &["new", "site"]);
    let root = tmp.join("site");
    ok(&root, &["add", "capability", "topology"]);
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
    root
}

fn edit(root: &Path, file: &str, old: &str, new: &str) {
    let p = root.join(file);
    let s = std::fs::read_to_string(&p).unwrap();
    assert!(s.contains(old), "{old:?} not in {file}");
    std::fs::write(p, s.replacen(old, new, 1)).unwrap();
}

#[test]
fn a_setting_reaches_firmware_in_the_language_its_output_names() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let rs = std::fs::read_to_string(root.join("firmware/controller/generated/params.rs")).unwrap();
    assert!(rs.contains("Parameter::Threshold => 1000.0"), "{rs}");

    edit(
        &root,
        "pipelines/topology.toml",
        "  \"firmware/controller/generated/params.rs\",",
        "  \"firmware/controller/generated/params.rs\",\n  \"firmware/controller/generated/params.h\",\n  \"firmware/controller/generated/params.json\",",
    );
    ok(&root, &["derive"]);
    let h = std::fs::read_to_string(root.join("firmware/controller/generated/params.h")).unwrap();
    assert!(
        h.contains("#define PARAMETER_THRESHOLD_DEFAULT 1000.0f"),
        "{h}"
    );
    let j =
        std::fs::read_to_string(root.join("firmware/controller/generated/params.json")).unwrap();
    assert!(j.contains("\"threshold\""), "{j}");

    edit(&root, "topology.toml", "default = 1000", "default = 1200");
    let stale = fid(&root, &["derive", "--check"]);
    assert!(!stale.status.success(), "a changed default was not stale");
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
}

#[test]
fn a_bus_across_systems_fails_naming_both() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(
        &root,
        "topology.toml",
        "nodes = [\"controller\", \"server\"]",
        "nodes = [\"controller\"]\n\n[[system]]\nid    = \"office\"\nnodes = [\"server\"]",
    );
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("bus `lan` crosses systems `office` and `site`"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_new_firmware_role_fails_until_its_outputs_are_listed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(
        &root,
        "topology.toml",
        "nodes = [\"controller\", \"server\"]",
        "nodes = [\"controller\", \"other\", \"server\"]",
    );
    let p = root.join("topology.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(
        &p,
        s + "\n[[node]]\nid = \"other\"\nkind = \"mcu\"\nfirmware = \"other\"\n",
    )
    .unwrap();
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("firmware/other/…/params.<rs|h|json>"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_setting_outside_its_range_and_mqtt_without_tls_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(&root, "topology.toml", "default = 1000", "default = 5000");
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("default 5000 is outside its range"),
        "{}",
        text(&out)
    );
    edit(&root, "topology.toml", "default = 5000", "default = 1000");
    edit(&root, "topology.toml", "tls      = true\n", "");
    let out = fid(&root, &["derive"]);
    assert!(
        text(&out).contains("MQTT without `tls = true`"),
        "{}",
        text(&out)
    );
}

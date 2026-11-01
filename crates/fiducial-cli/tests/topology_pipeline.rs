//! The `topology` capability: several computers, the buses between them and
//! the points each measures and drives, declared once in `topology.toml`.

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

fn edit(root: &Path, old: &str, new: &str) {
    let p = root.join("topology.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    assert!(s.contains(old), "{old:?} not in topology.toml");
    std::fs::write(p, s.replacen(old, new, 1)).unwrap();
}

#[test]
fn a_new_point_reaches_firmware_and_web_in_one_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let humidity = "\n[[point]]\nid       = \"humidity\"\nnode     = \"controller\"\nkind     = \"sensor\"\nunit     = \"%\"\nrange    = [0, 100]\nbus      = \"sensors\"\naddress  = 0x62\nquantity = \"relative_humidity\"\ndisplay  = true\n";
    let p = root.join("topology.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(&p, s + humidity).unwrap();

    let stale = fid(&root, &["derive", "--check"]);
    assert!(!stale.status.success());
    assert!(text(&stale).contains("topology.toml"), "{}", text(&stale));

    ok(&root, &["derive"]);
    let rs =
        std::fs::read_to_string(root.join("firmware/controller/src/generated/points.rs")).unwrap();
    assert!(rs.contains("Humidity = 3,"), "{rs}");
    let cards = std::fs::read_to_string(root.join("apps/web/src/generated/cards.ts")).unwrap();
    assert!(
        cards.contains("{ point: \"humidity\", unit: \"%\", limits: [] }"),
        "{cards}"
    );
    let ts = std::fs::read_to_string(root.join("apps/web/src/generated/points.ts")).unwrap();
    assert!(ts.contains("name: \"humidity\""), "{ts}");
    ok(&root, &["derive", "--check"]);
}

#[test]
fn a_bus_across_systems_fails_naming_both() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(
        &root,
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
fn a_parameter_pointing_at_a_missing_rule_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(
        &root,
        "rule    = \"ventilation\"\nlimits  = \"co2\"\n\n[[parameter]]\nid      = \"co2_off\"",
        "rule    = \"airing\"\nlimits  = \"co2\"\n\n[[parameter]]\nid      = \"co2_off\"",
    );
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("parameter `co2_on` names rule `airing`, which is not declared"),
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
        "nodes = [\"controller\", \"server\"]",
        "nodes = [\"controller\", \"roof\", \"server\"]",
    );
    let p = root.join("topology.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(
        &p,
        s + "\n[[node]]\nid = \"roof\"\nkind = \"mcu\"\ntarget = \"riscv32imac-unknown-none-elf\"\nruntime = \"embassy-esp\"\nfirmware = \"roof\"\n",
    )
    .unwrap();
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("firmware/roof/…/points.<rs|h|json>"),
        "{}",
        text(&out)
    );
}

#[test]
fn mqtt_without_tls_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(&root, "tls      = true\n", "");
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("MQTT without `tls = true`"),
        "{}",
        text(&out)
    );
}

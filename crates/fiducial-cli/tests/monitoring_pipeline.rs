//! The `monitoring` capability: what a monitoring and control product measures,
//! drives and schedules, declared once in `monitoring.toml`.

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
    ok(&root, &["add", "capability", "monitoring"]);
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);
    root
}

fn append(root: &Path, file: &str, more: &str) {
    let p = root.join(file);
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(p, s + more).unwrap();
}

fn edit(root: &Path, file: &str, old: &str, new: &str) {
    let p = root.join(file);
    let s = std::fs::read_to_string(&p).unwrap();
    assert!(s.contains(old), "{old:?} not in {file}");
    std::fs::write(p, s.replacen(old, new, 1)).unwrap();
}

#[test]
fn monitoring_needs_topology() {
    let tmp = tempfile::tempdir().unwrap();
    ok(tmp.path(), &["new", "site"]);
    let root = tmp.path().join("site");
    let out = fid(&root, &["add", "capability", "monitoring"]);
    assert!(
        !out.status.success(),
        "monitoring installed without topology"
    );
}

#[test]
fn a_new_point_reaches_firmware_web_and_schema_in_one_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    append(
        &root,
        "monitoring.toml",
        "\n[[point]]\nid = \"humidity\"\nnode = \"controller\"\nkind = \"sensor\"\nunit = \"%\"\nrange = [0, 100]\nbus = \"sensors\"\naddress = 0x62\nquantity = \"relative_humidity\"\ndisplay = true\n",
    );
    let stale = fid(&root, &["derive", "--check"]);
    assert!(!stale.status.success());
    ok(&root, &["derive"]);
    let rs = std::fs::read_to_string(root.join("firmware/controller/generated/points.rs")).unwrap();
    assert!(rs.contains("Humidity = 3,"), "{rs}");
    let cards = std::fs::read_to_string(root.join("apps/web/src/generated/cards.ts")).unwrap();
    assert!(
        cards.contains("{ point: \"humidity\", unit: \"%\", limits: [] }"),
        "{cards}"
    );
    assert!(
        cards.contains("{ point: \"co2\", unit: \"ppm\", limits: [\"threshold\"] }"),
        "{cards}"
    );
    let sql = std::fs::read_to_string(root.join("migrations/0001_monitoring.sql")).unwrap();
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS reading "), "{sql}");
    ok(&root, &["derive", "--check"]);
}

#[test]
fn a_rule_using_a_missing_parameter_and_an_orphan_reading_fail() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    edit(
        &root,
        "monitoring.toml",
        "parameters = [\"threshold\"]",
        "parameters = [\"missing\"]",
    );
    let out = fid(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains(
            "rule `ventilation` reads parameter `missing`, which topology.toml does not declare"
        ),
        "{}",
        text(&out)
    );
    edit(
        &root,
        "monitoring.toml",
        "parameters = [\"missing\"]",
        "parameters = [\"threshold\"]",
    );
    append(
        &root,
        "monitoring.toml",
        "\n[[point]]\nid = \"dust\"\nnode = \"controller\"\nkind = \"sensor\"\nunit = \"%\"\nrange = [0, 100]\nbus = \"sensors\"\naddress = 0x70\n",
    );
    let out = fid(&root, &["derive"]);
    assert!(
        text(&out).contains("point `dust` is read by no rule"),
        "{}",
        text(&out)
    );
}

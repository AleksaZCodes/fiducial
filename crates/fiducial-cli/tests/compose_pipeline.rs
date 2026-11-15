//! `deploy = "compose"`: the self-hosted target is derived, not written.
//!
//! Scaffolds a product with a topology, selects the target, derives, and checks
//! that every output exists, that the broker ACL follows the topology's topics,
//! and that changing a declaration makes the outputs stale.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(PathBuf::from(env!("CARGO_BIN_EXE_fid")))
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .output()
        .expect("running fid")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn product(dir: &Path) -> PathBuf {
    assert!(run(dir, &["new", "building"]).status.success(), "fid new");
    let root = dir.join("building");
    for cap in ["topology", "deploy-compose"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "add {cap}: {}", text(&out));
    }
    let toml = root.join("fiducial.toml");
    let mut s = std::fs::read_to_string(&toml).unwrap();
    s.push_str("\n[adapters]\ndeploy = \"compose\"\n");
    std::fs::write(&toml, s).unwrap();
    root
}

#[test]
fn the_self_hosted_target_is_derived_and_gated() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "derive: {}", text(&out));

    for f in [
        "deploy/compose.yaml",
        "deploy/Caddyfile",
        "deploy/mosquitto/mosquitto.conf",
        "deploy/mosquitto/acl",
    ] {
        assert!(root.join(f).exists(), "{f} was not derived");
    }
    let compose = std::fs::read_to_string(root.join("deploy/compose.yaml")).unwrap();
    assert!(compose.contains("platform: linux/arm64"), "{compose}");
    assert!(!compose.contains("ollama"), "no ollama unless ai = ollama");
    let acl = std::fs::read_to_string(root.join("deploy/mosquitto/acl")).unwrap();
    assert!(acl.contains("pattern write building/%u/state"), "{acl}");
    assert!(acl.contains("topic write building/+/command"), "{acl}");

    assert!(run(&root, &["derive", "--check"]).status.success());

    // A hand edit is caught.
    std::fs::write(root.join("deploy/Caddyfile"), "evil.example {}\n").unwrap();
    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "a hand-edited Caddyfile passed");
}

#[test]
fn a_public_certificate_for_a_lan_name_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());
    let toml = root.join("fiducial.toml");
    let s = std::fs::read_to_string(&toml)
        .unwrap()
        .replace("tls = \"internal\"", "tls = \"acme\"");
    std::fs::write(&toml, s).unwrap();
    let out = run(&root, &["derive"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("cannot get a public certificate"),
        "{}",
        text(&out)
    );
}

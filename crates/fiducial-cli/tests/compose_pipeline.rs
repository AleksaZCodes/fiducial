//! `deploy = "compose"`: the self-hosted target is derived, not written.
//!
//! It is general: a product that is an app and a database needs no topology.
//! The building (controllers, a broker, a Raspberry Pi) is the use case that
//! exercises all of it, so the first test is that one: a topology with a broker,
//! the broker's outputs owed and derived, its ACL following the topology's
//! topics, and a hand edit failing the gate.

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
    select_compose(&root);
    root
}

fn select_compose(root: &Path) {
    let toml = root.join("fiducial.toml");
    let mut s = std::fs::read_to_string(&toml).unwrap();
    s.push_str("\n[adapters]\ndeploy = \"compose\"\n");
    std::fs::write(&toml, s).unwrap();
}

#[test]
fn the_building_use_case_is_derived_and_gated() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product(tmp.path());

    // The topology declares a broker, so its files are owed and named.
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "a broker with no outputs passed");
    assert!(
        text(&out).contains("deploy/mosquitto/acl"),
        "{}",
        text(&out)
    );
    let pipeline = root.join("pipelines/deploy-compose.toml");
    let p = std::fs::read_to_string(&pipeline).unwrap().replace(
        "  \"deploy/Caddyfile\",\n",
        "  \"deploy/Caddyfile\",\n  \"deploy/mosquitto/mosquitto.conf\",\n  \"deploy/mosquitto/acl\",\n",
    );
    std::fs::write(&pipeline, p).unwrap();

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
fn a_product_with_no_topology_is_an_app_a_database_and_a_proxy() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "shop"]).status.success());
    let root = tmp.path().join("shop");
    let out = run(&root, &["add", "capability", "deploy-compose"]);
    assert!(out.status.success(), "{}", text(&out));
    select_compose(&root);
    let toml = root.join("fiducial.toml");
    let mut s = std::fs::read_to_string(&toml).unwrap();
    s.push_str(
        "\n[[deploy.services]]\nname = \"db\"\nimage = \"postgres:17\"\nvolumes = [\"db-data:/var/lib/postgresql/data\"]\n",
    );
    std::fs::write(&toml, s).unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "derive: {}", text(&out));
    let compose = std::fs::read_to_string(root.join("deploy/compose.yaml")).unwrap();
    assert!(
        compose.contains("  server:") && compose.contains("  db:"),
        "{compose}"
    );
    assert!(!compose.contains("mosquitto"), "{compose}");
    assert!(!root.join("deploy/mosquitto").exists());
    assert!(run(&root, &["derive", "--check"]).status.success());
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

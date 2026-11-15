//! The `fid-compose` pipeline: the self-hosted deploy, derived.
//!
//! `[adapters] deploy = "compose"` ships a product to one Linux host running
//! Docker Compose, with no vendor account and no required internet. What runs
//! is already declared: the host nodes and the MQTT bus in `topology.toml`, the
//! vendors in `[adapters]`, the hostname in `[deploy]`. So the services, their
//! volumes and healthchecks, the reverse proxy and the broker's topic ACL are
//! derived from them rather than written a second time
//! (`docs/specs/2026-11-01-a-self-hosted-target.md`).
//!
//! Outputs, all derived and gated by `fid derive --check`:
//!
//! - `deploy/compose.yaml` — `server`, `mosquitto`, `caddy`, and `ollama` when
//!   `[adapters] ai = "ollama"`;
//! - `deploy/Caddyfile` — TLS and the reverse proxy to the server;
//! - `deploy/mosquitto/mosquitto.conf` and `deploy/mosquitto/acl` — the broker
//!   and which principal may publish or subscribe to which topic.
//!
//! **What the ACL is derived from.** Each topic in the topology has a direction
//! (`up`: a device publishes; `down`: the server publishes and the device
//! reads; `will`: the broker publishes on a device's behalf). A device may only
//! touch its own subtree, `<product>/<device>/<topic>`, which is `can()`'s rule
//! that a device may always act on itself and nothing else. The server may read
//! every device's `up` topics and write its `down` topics. Grants a product
//! stores beyond that are runtime data and are not enumerated here.

use std::path::Path;

use anyhow::{bail, Result};

use crate::config::Config;
use crate::topology::{Direction, NodeKind, Topology};

/// The port the product's server listens on, behind the proxy.
pub const SERVER_PORT: u16 = 8080;

/// Files this pipeline can derive, relative to the product root.
pub const OUTPUTS: [&str; 4] = [
    "deploy/compose.yaml",
    "deploy/Caddyfile",
    "deploy/mosquitto/mosquitto.conf",
    "deploy/mosquitto/acl",
];

/// The facts the derivation reads, gathered once.
pub struct Plan {
    pub product: String,
    pub host: String,
    pub registry: String,
    pub acme: bool,
    pub platform: Option<&'static str>,
    pub ollama: bool,
    /// The MQTT broker, when the topology declares one.
    pub broker: Option<Broker>,
    pub devices: Vec<String>,
    pub topics: Vec<(String, Direction)>,
    pub secrets: Vec<String>,
}

pub struct Broker {
    pub tls: bool,
}

/// `linux/arm64` or `linux/amd64`, from a free-text target triple.
fn platform(target: &str) -> Option<&'static str> {
    let t = target.to_ascii_lowercase();
    if t.contains("aarch64") || t.contains("arm64") {
        Some("linux/arm64")
    } else if t.contains("x86_64") || t.contains("amd64") {
        Some("linux/amd64")
    } else {
        None
    }
}

pub fn plan(config: &Config, t: &Topology) -> Result<Plan> {
    let vendor = config.adapters.get("deploy").unwrap_or("");
    if vendor != "compose" {
        bail!(
            "fid-compose: [adapters] deploy = \"{}\" — this executor derives the \
             self-hosted target. Set deploy = \"compose\", or remove \
             pipelines/deploy-compose.toml.",
            if vendor.is_empty() { "none" } else { vendor }
        );
    }
    let product = config.product.name.clone();
    let d = &config.deploy;
    let tls = match d.tls.as_str() {
        "" | "internal" => false,
        "acme" => true,
        other => bail!(
            "fid-compose: [deploy] tls = \"{other}\" is not a kind of certificate. \
             Known: internal (Caddy's own CA, for a LAN name; the default), acme (a \
             public domain)."
        ),
    };
    let host = if d.host.is_empty() {
        format!("{product}.local")
    } else {
        d.host.clone()
    };
    if tls && host.ends_with(".local") {
        bail!(
            "fid-compose: [deploy] tls = \"acme\" with host `{host}` — a `.local` name \
             cannot get a public certificate. Declare a public domain in [deploy] host, \
             or use tls = \"internal\"."
        );
    }
    let hosts: Vec<_> = t.node.iter().filter(|n| n.kind == NodeKind::Host).collect();
    let Some(server) = hosts.first() else {
        bail!("fid-compose: topology.toml declares no host node for the server to run on");
    };
    let broker = t
        .bus
        .iter()
        .find(|b| b.broker.is_some())
        .map(|b| Broker { tls: b.tls });
    let devices: Vec<String> = t
        .node
        .iter()
        .filter(|n| n.kind == NodeKind::Mcu)
        .map(|n| n.id.clone())
        .collect();
    let topics = t
        .topics()
        .into_iter()
        .map(|x| (x.id, x.direction))
        .collect();

    let mut secrets = Vec::new();
    if config.adapters.get("auth") == Some("native") {
        secrets.push("SESSION_SECRET".to_string());
    }
    if config.adapters.get("ai") == Some("openrouter") {
        secrets.push("OPENROUTER_API_KEY".to_string());
    }
    Ok(Plan {
        product,
        host,
        registry: d.registry.clone(),
        acme: tls,
        platform: server.target.as_deref().and_then(platform),
        ollama: config.adapters.get("ai") == Some("ollama"),
        broker,
        devices,
        topics,
        secrets,
    })
}

const HEADER: &str =
    "Derived by fid-compose from topology.toml, [adapters] and [deploy] — do not edit.";

pub fn render_compose(p: &Plan) -> String {
    let image = if p.registry.is_empty() {
        format!("{}-server:latest", p.product)
    } else {
        format!(
            "{}/{}-server:latest",
            p.registry.trim_end_matches('/'),
            p.product
        )
    };
    let mut s = format!("# {HEADER}\nname: {}\n\nservices:\n", p.product);
    s.push_str(&format!("  server:\n    image: {image}\n"));
    if let Some(pl) = p.platform {
        s.push_str(&format!("    platform: {pl}\n"));
    }
    s.push_str(
        "    restart: unless-stopped\n    environment:\n      DATABASE_PATH: /data/app.db\n",
    );
    if p.broker.is_some() {
        s.push_str("      MQTT_URL: mqtts://mosquitto:8883\n");
    }
    if p.ollama {
        s.push_str("      AI_BASE_URL: http://ollama:11434/v1\n");
    }
    if !p.secrets.is_empty() {
        s.push_str("      # Secrets, never written here — set them in deploy/.env:\n");
        for name in &p.secrets {
            s.push_str(&format!("      {name}: ${{{name}}}\n"));
        }
    }
    s.push_str("    volumes:\n      - server-data:/data\n");
    s.push_str(&format!(
        "    healthcheck:\n      test: [\"CMD-SHELL\", \"wget -q --spider http://127.0.0.1:{SERVER_PORT}/healthz || exit 1\"]\n      interval: 30s\n      timeout: 5s\n      retries: 3\n"
    ));
    let mut deps = Vec::new();
    if p.broker.is_some() {
        deps.push("mosquitto");
    }
    if p.ollama {
        deps.push("ollama");
    }
    if !deps.is_empty() {
        s.push_str("    depends_on:\n");
        for d in deps {
            s.push_str(&format!("      {d}:\n        condition: service_started\n"));
        }
    }
    if p.broker.is_some() {
        s.push_str(
            "\n  mosquitto:\n    image: eclipse-mosquitto:2\n    restart: unless-stopped\n    ports:\n      - \"8883:8883\"\n    volumes:\n      - ./mosquitto/mosquitto.conf:/mosquitto/config/mosquitto.conf:ro\n      - ./mosquitto/acl:/mosquitto/config/acl:ro\n      # Provisioned per device at first flash; never in the repository.\n      - ./mosquitto/passwd:/mosquitto/config/passwd:ro\n      - ./certs:/mosquitto/certs:ro\n      - mosquitto-data:/mosquitto/data\n    healthcheck:\n      test: [\"CMD-SHELL\", \"mosquitto_sub -h localhost -p 1883 -t '$$SYS/broker/uptime' -C 1 -W 3 || exit 1\"]\n      interval: 30s\n      timeout: 5s\n      retries: 3\n",
        );
    }
    if p.ollama {
        s.push_str(
            "\n  ollama:\n    image: ollama/ollama:latest\n    restart: unless-stopped\n    volumes:\n      - ollama-models:/root/.ollama\n    healthcheck:\n      test: [\"CMD\", \"ollama\", \"list\"]\n      interval: 30s\n      timeout: 10s\n      retries: 3\n",
        );
    }
    s.push_str(
        "\n  caddy:\n    image: caddy:2\n    restart: unless-stopped\n    ports:\n      - \"80:80\"\n      - \"443:443\"\n    volumes:\n      - ./Caddyfile:/etc/caddy/Caddyfile:ro\n      - caddy-data:/data\n      - caddy-config:/config\n    depends_on:\n      server:\n        condition: service_healthy\n",
    );
    s.push_str("\nvolumes:\n  server-data:\n  caddy-data:\n  caddy-config:\n");
    if p.broker.is_some() {
        s.push_str("  mosquitto-data:\n");
    }
    if p.ollama {
        s.push_str("  ollama-models:\n");
    }
    s
}

pub fn render_caddyfile(p: &Plan) -> String {
    let tls = if p.acme {
        "    # A public domain: Caddy gets and renews a certificate itself.\n"
    } else {
        "    tls internal\n"
    };
    format!(
        "# {HEADER}\n{} {{\n{tls}    encode zstd gzip\n    reverse_proxy server:{SERVER_PORT}\n}}\n",
        p.host
    )
}

pub fn render_mosquitto_conf(p: &Plan) -> String {
    let tls = p.broker.as_ref().is_some_and(|b| b.tls);
    let mut s = format!("# {HEADER}\n");
    s.push_str("persistence true\npersistence_location /mosquitto/data/\nallow_anonymous false\npassword_file /mosquitto/config/passwd\nacl_file /mosquitto/config/acl\n\n");
    s.push_str("listener 8883\n");
    if tls {
        s.push_str("cafile /mosquitto/certs/ca.crt\ncertfile /mosquitto/certs/server.crt\nkeyfile /mosquitto/certs/server.key\ntls_version tlsv1.2\n");
    }
    // Health check only, loopback only.
    s.push_str("\nlistener 1883 127.0.0.1\n");
    s
}

pub fn render_acl(p: &Plan) -> String {
    let root = &p.product;
    let mut s = format!(
        "# {HEADER}\n# A device (its MQTT username is its node id) touches only its own\n# subtree, {root}/<device>/<topic>.\n\n"
    );
    for (id, dir) in &p.topics {
        match dir {
            Direction::Up | Direction::Will => {
                s.push_str(&format!("pattern write {root}/%u/{id}\n"));
            }
            Direction::Down => s.push_str(&format!("pattern read {root}/%u/{id}\n")),
        }
    }
    s.push_str(
        "\n# The server reads what devices report and writes what they obey.\nuser server\n",
    );
    for (id, dir) in &p.topics {
        match dir {
            Direction::Up | Direction::Will => {
                s.push_str(&format!("topic read {root}/+/{id}\n"));
            }
            Direction::Down => s.push_str(&format!("topic write {root}/+/{id}\n")),
        }
    }
    if !p.devices.is_empty() {
        s.push_str(&format!(
            "\n# Devices declared in topology.toml: {}\n",
            p.devices.join(", ")
        ));
    }
    s
}

/// Dispatch one output path to its renderer.
pub fn render(p: &Plan, out: &str) -> Result<String> {
    let needs_broker = out.contains("mosquitto/");
    if needs_broker && p.broker.is_none() {
        bail!(
            "fid-compose: output `{out}` needs an MQTT broker, and topology.toml declares \
             no [[bus]] with `broker = \"<host>\"`"
        );
    }
    Ok(match out {
        "deploy/compose.yaml" => render_compose(p),
        "deploy/Caddyfile" => render_caddyfile(p),
        "deploy/mosquitto/mosquitto.conf" => render_mosquitto_conf(p),
        "deploy/mosquitto/acl" => render_acl(p),
        other => bail!(
            "fid-compose: unknown output `{other}` (supported: {})",
            OUTPUTS.join(", ")
        ),
    })
}

pub fn load(root: &Path, config: &Config, decl: &str) -> Result<Plan> {
    let t = crate::topology::load(root, decl)?;
    plan(config, &t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SITE: &str = r#"
[[system]]
id = "site"
nodes = ["room_1", "server"]

[[node]]
id = "room_1"
kind = "mcu"
firmware = "room"

[[node]]
id = "server"
kind = "host"
target = "aarch64-unknown-linux-gnu"
apps = ["server"]

[[bus]]
id = "lan"
type = "ethernet"
protocol = "mqtt"
tls = true
broker = "server"
connect = ["room_1.wifi", "server.eth0"]
"#;

    fn plan_for(ai: &str) -> Plan {
        let t: Topology = toml::from_str(SITE).unwrap();
        let config: Config = toml::from_str(&format!(
            "[product]\nname = \"building\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\nai = \"{ai}\"\nauth = \"native\"\n"
        ))
        .unwrap();
        plan(&config, &t).unwrap()
    }

    #[test]
    fn the_services_follow_the_declarations() {
        let c = render_compose(&plan_for("ollama"));
        for s in ["server:", "mosquitto:", "caddy:", "ollama:"] {
            assert!(c.contains(&format!("  {s}")), "{s}\n{c}");
        }
        assert!(c.contains("platform: linux/arm64"));
        assert!(c.contains("SESSION_SECRET: ${SESSION_SECRET}"));
        let c = render_compose(&plan_for("none"));
        assert!(!c.contains("ollama"));
    }

    #[test]
    fn a_device_touches_only_its_own_subtree() {
        let a = render_acl(&plan_for("none"));
        assert!(a.contains("pattern write building/%u/state"));
        assert!(a.contains("pattern read building/%u/command"));
        assert!(a.contains("topic write building/+/command"));
        assert!(!a.contains("pattern write building/%u/command"));
    }

    #[test]
    fn a_lan_name_cannot_ask_for_a_public_certificate() {
        let t: Topology = toml::from_str(SITE).unwrap();
        let config: Config = toml::from_str(
            "[product]\nname = \"b\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n\n[deploy]\ntls = \"acme\"\n",
        )
        .unwrap();
        let e = plan(&config, &t).err().unwrap().to_string();
        assert!(e.contains("cannot get a public certificate"), "{e}");
    }
}

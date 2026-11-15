//! The `fid-compose` pipeline: the self-hosted deploy, derived.
//!
//! `[adapters] deploy = "compose"` ships a product to one Linux host running
//! Docker Compose, with no vendor account and no required internet. What runs
//! is already declared, so the file is derived rather than written a second
//! time (`docs/specs/2026-11-01-a-self-hosted-target.md`).
//!
//! **A list of services, from several sources.** Nothing here is specific to one
//! kind of product:
//!
//! - **apps** — each app of each host node in `topology.toml`, when there is one
//!   (without a topology, one service named `server`);
//! - **the broker** — Mosquitto, only when the topology declares a bus with a
//!   `broker`; this is the building-controller use case, and what makes the
//!   broker's config and topic ACL derivable;
//! - **the model server** — Ollama, when `[adapters] ai = "ollama"`;
//! - **declared services** — `[[deploy.services]]`: a database, a cache, a
//!   worker, anything with an image.
//!
//! Caddy proxies one of them (`[deploy] proxy`, default the first app).
//!
//! Outputs, all derived and gated by `fid derive --check`:
//!
//! - `deploy/compose.yaml` and `deploy/Caddyfile`, always;
//! - `deploy/mosquitto/mosquitto.conf` and `deploy/mosquitto/acl`, when there is
//!   a broker.
//!
//! **What the ACL is derived from.** Each topic in the topology has a direction
//! (`up`: a device publishes; `down`: the server publishes and the device
//! reads; `will`: the broker publishes on a device's behalf). A device may only
//! touch its own subtree, `<product>/<device>/<topic>`, which is `can()`'s rule
//! that a device may always act on itself and nothing else. The server may read
//! every device's `up` topics and write its `down` topics. Grants a product
//! stores beyond that are runtime data and are not enumerated here.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Result};

use crate::config::{ComposeService, Config};
use crate::topology::{Direction, NodeKind, Topology};

/// The port the proxied service listens on unless `[deploy] proxy_port` says so.
pub const DEFAULT_PORT: u16 = 8080;

/// Files this pipeline can derive, relative to the product root.
pub const OUTPUTS: [&str; 4] = [
    "deploy/compose.yaml",
    "deploy/Caddyfile",
    "deploy/mosquitto/mosquitto.conf",
    "deploy/mosquitto/acl",
];

/// The outputs every product gets; the broker's are added when there is one.
pub const ALWAYS: [&str; 2] = ["deploy/compose.yaml", "deploy/Caddyfile"];

/// One service in the Compose file.
pub struct Service {
    pub name: String,
    pub image: String,
    pub platform: Option<&'static str>,
    pub ports: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub volumes: Vec<String>,
    pub depends_on: Vec<(String, &'static str)>,
    pub healthcheck: Option<String>,
    /// Extra lines, already indented, for what the generic fields cannot say.
    pub extra: String,
}

/// The facts the derivation reads, gathered once.
pub struct Plan {
    pub product: String,
    pub host: String,
    pub acme: bool,
    /// The web app's static export, served by Caddy, and the prefix still
    /// proxied to the server.
    pub static_dir: Option<String>,
    pub api_prefix: String,
    pub services: Vec<Service>,
    pub proxy: String,
    pub proxy_port: u16,
    /// The MQTT broker, when the topology declares one.
    pub broker: Option<Broker>,
    pub devices: Vec<String>,
    pub topics: Vec<(String, Direction)>,
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

fn image_of(registry: &str, product: &str, app: &str) -> String {
    if registry.is_empty() {
        format!("{product}-{app}:latest")
    } else {
        format!("{}/{product}-{app}:latest", registry.trim_end_matches('/'))
    }
}

pub fn plan(config: &Config, t: Option<&Topology>) -> Result<Plan> {
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

    let mut secrets: Vec<String> = Vec::new();
    if config.adapters.get("auth") == Some("native") {
        secrets.push("SESSION_SECRET".to_string());
    }
    if config.adapters.get("ai") == Some("openrouter") {
        secrets.push("OPENROUTER_API_KEY".to_string());
    }
    let broker = t
        .and_then(|t| t.bus.iter().find(|b| b.broker.is_some()))
        .map(|b| Broker { tls: b.tls });
    let ollama = config.adapters.get("ai") == Some("ollama");
    let sqlite = config.adapters.get("database") == Some("sqlite");

    // Apps: from the topology's host nodes, or one `server`.
    let mut apps: Vec<(String, Option<&'static str>)> = Vec::new();
    if let Some(t) = t {
        for n in t.node.iter().filter(|n| n.kind == NodeKind::Host) {
            let pf = n.target.as_deref().and_then(platform);
            for a in &n.apps {
                apps.push((a.clone(), pf));
            }
        }
    }
    if apps.is_empty() {
        apps.push(("server".to_string(), None));
    }
    let proxy = if d.proxy.is_empty() {
        apps[0].0.clone()
    } else {
        d.proxy.clone()
    };
    let proxy_port = if d.proxy_port == 0 {
        DEFAULT_PORT
    } else {
        d.proxy_port
    };

    let mut services: Vec<Service> = Vec::new();
    for (app, pf) in &apps {
        let mut env: Vec<(String, String)> = Vec::new();
        if sqlite {
            env.push(("DATABASE_PATH".into(), format!("/data/{app}.db")));
        }
        if broker.is_some() {
            env.push(("MQTT_URL".into(), "mqtts://mosquitto:8883".into()));
        }
        if ollama {
            let url = if config.ai.base_url.is_empty() {
                "http://ollama:11434/v1"
            } else {
                config.ai.base_url.as_str()
            };
            env.push(("AI_BASE_URL".into(), url.into()));
            env.push(("AI_MODEL".into(), config.ai.model.clone()));
        }
        let mut depends = Vec::new();
        if broker.is_some() {
            depends.push(("mosquitto".to_string(), "service_started"));
        }
        if ollama {
            depends.push(("ollama".to_string(), "service_started"));
        }
        let health = (*app == proxy)
            .then(|| format!("wget -q --spider http://127.0.0.1:{proxy_port}/healthz || exit 1"));
        let mut extra = String::new();
        if !secrets.is_empty() {
            extra.push_str("      # Secrets, never written here — set them in deploy/.env:\n");
            for name in &secrets {
                extra.push_str(&format!("      {name}: ${{{name}}}\n"));
            }
        }
        services.push(Service {
            name: app.clone(),
            image: image_of(&d.registry, &product, app),
            platform: *pf,
            ports: Vec::new(),
            environment: env,
            volumes: vec![format!("{app}-data:/data")],
            depends_on: depends,
            healthcheck: health,
            extra,
        });
    }

    if broker.is_some() {
        services.push(Service {
            name: "mosquitto".into(),
            image: "eclipse-mosquitto:2".into(),
            platform: None,
            ports: vec!["8883:8883".into()],
            environment: Vec::new(),
            volumes: vec![
                "./mosquitto/mosquitto.conf:/mosquitto/config/mosquitto.conf:ro".into(),
                "./mosquitto/acl:/mosquitto/config/acl:ro".into(),
                // Provisioned per device at first flash; never in the repository.
                "./mosquitto/passwd:/mosquitto/config/passwd:ro".into(),
                "./certs:/mosquitto/certs:ro".into(),
                "mosquitto-data:/mosquitto/data".into(),
            ],
            depends_on: Vec::new(),
            healthcheck: Some(
                "mosquitto_sub -h localhost -p 1883 -t '$$SYS/broker/uptime' -C 1 -W 3 || exit 1"
                    .into(),
            ),
            extra: String::new(),
        });
    }
    if ollama {
        services.push(Service {
            name: "ollama".into(),
            image: "ollama/ollama:latest".into(),
            platform: None,
            ports: Vec::new(),
            environment: Vec::new(),
            volumes: vec!["ollama-models:/root/.ollama".into()],
            depends_on: Vec::new(),
            healthcheck: Some("ollama list || exit 1".into()),
            extra: String::new(),
        });
    }
    for x in &d.services {
        services.push(declared(x));
    }
    let mut seen = BTreeSet::new();
    for sv in &services {
        if !seen.insert(sv.name.clone()) {
            bail!(
                "fid-compose: service `{}` is declared twice — an app, the broker or Ollama \
                 already derives it, and [[deploy.services]] lists it again. Rename one.",
                sv.name
            );
        }
    }
    if !services.iter().any(|sv| sv.name == proxy) {
        bail!(
            "fid-compose: [deploy] proxy = \"{proxy}\" names no service. Services: {}.",
            services
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for sv in &services {
        for (dep, _) in &sv.depends_on {
            if !services.iter().any(|o| &o.name == dep) {
                bail!(
                    "fid-compose: service `{}` depends_on `{dep}`, which is not a service.",
                    sv.name
                );
            }
        }
    }

    let devices = t
        .map(|t| {
            t.node
                .iter()
                .filter(|n| n.kind == NodeKind::Mcu)
                .map(|n| n.id.clone())
                .collect()
        })
        .unwrap_or_default();
    let topics = t
        .map(|t| {
            t.topics()
                .into_iter()
                .map(|x| (x.id, x.direction))
                .collect()
        })
        .unwrap_or_default();
    let static_dir = if d.static_dir.is_empty() {
        None
    } else {
        let dir = d.static_dir.trim_end_matches('/');
        if dir.starts_with('/') || dir.split('/').any(|c| c == "..") {
            bail!(
                "fid-compose: [deploy] static_dir = \"{}\" must be a path inside the product, \
                 relative to its root (e.g. apps/web/out).",
                d.static_dir
            );
        }
        Some(dir.to_string())
    };
    let api_prefix = if d.api_prefix.is_empty() {
        "/api".to_string()
    } else {
        d.api_prefix.trim_end_matches('/').to_string()
    };
    if !api_prefix.starts_with('/') {
        bail!(
            "fid-compose: [deploy] api_prefix = \"{}\" must start with `/`.",
            d.api_prefix
        );
    }
    Ok(Plan {
        product,
        host,
        acme: tls,
        static_dir,
        api_prefix,
        services,
        proxy,
        proxy_port,
        broker,
        devices,
        topics,
    })
}

fn declared(x: &ComposeService) -> Service {
    let mut extra = String::new();
    if !x.secrets.is_empty() {
        extra.push_str("      # Secrets, never written here — set them in deploy/.env:\n");
        for name in &x.secrets {
            extra.push_str(&format!("      {name}: ${{{name}}}\n"));
        }
    }
    Service {
        name: x.name.clone(),
        image: x.image.clone(),
        platform: None,
        ports: x.ports.clone(),
        environment: x
            .environment
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        volumes: x.volumes.clone(),
        depends_on: x
            .depends_on
            .iter()
            .map(|d| (d.clone(), "service_started"))
            .collect(),
        healthcheck: (!x.healthcheck.is_empty()).then(|| x.healthcheck.clone()),
        extra,
    }
}

const HEADER: &str =
    "Derived by fid-compose from topology.toml, [adapters] and [deploy] — do not edit.";

/// A value that must stay one YAML scalar.
fn q(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn render_compose(p: &Plan) -> String {
    let mut s = format!("# {HEADER}\nname: {}\n\nservices:\n", p.product);
    let mut named: BTreeSet<String> = BTreeSet::new();
    for (i, sv) in p.services.iter().enumerate() {
        if i > 0 {
            s.push('\n');
        }
        s.push_str(&format!("  {}:\n    image: {}\n", sv.name, sv.image));
        if let Some(pl) = sv.platform {
            s.push_str(&format!("    platform: {pl}\n"));
        }
        s.push_str("    restart: unless-stopped\n");
        if !sv.ports.is_empty() {
            s.push_str("    ports:\n");
            for port in &sv.ports {
                s.push_str(&format!("      - {}\n", q(port)));
            }
        }
        if !sv.environment.is_empty() || !sv.extra.is_empty() {
            s.push_str("    environment:\n");
            for (k, v) in &sv.environment {
                s.push_str(&format!("      {k}: {}\n", q(v)));
            }
            s.push_str(&sv.extra);
        }
        if !sv.volumes.is_empty() {
            s.push_str("    volumes:\n");
            for v in &sv.volumes {
                if let Some((left, _)) = v.split_once(':') {
                    if !left.starts_with('.') && !left.starts_with('/') {
                        named.insert(left.to_string());
                    }
                }
                s.push_str(&format!("      - {v}\n"));
            }
        }
        if let Some(h) = &sv.healthcheck {
            s.push_str(&format!(
                "    healthcheck:\n      test: [\"CMD-SHELL\", {}]\n      interval: 30s\n      timeout: 5s\n      retries: 3\n",
                q(h)
            ));
        }
        if !sv.depends_on.is_empty() {
            s.push_str("    depends_on:\n");
            for (d, cond) in &sv.depends_on {
                s.push_str(&format!("      {d}:\n        condition: {cond}\n"));
            }
        }
    }
    s.push_str(
        "\n  caddy:\n    image: caddy:2\n    restart: unless-stopped\n    ports:\n      - \"80:80\"\n      - \"443:443\"\n    volumes:\n      - ./Caddyfile:/etc/caddy/Caddyfile:ro\n      - caddy-data:/data\n      - caddy-config:/config\n",
    );
    if let Some(dir) = &p.static_dir {
        // Relative to deploy/, where this file lives.
        s.push_str(&format!("      - ../{dir}:/srv/web:ro\n"));
    }
    s.push_str(&format!(
        "    depends_on:\n      {}:\n        condition: service_healthy\n",
        p.proxy
    ));
    named.insert("caddy-data".into());
    named.insert("caddy-config".into());
    s.push_str("\nvolumes:\n");
    for v in &named {
        s.push_str(&format!("  {v}:\n"));
    }
    s
}

/// The response headers a static export cannot set for itself.
///
/// A Next.js static export has no server, so the `headers()` in its config do
/// not exist, and Caddy sends them instead. They are the same names as
/// `securityHeaders` in the web template's `next.config.ts`, and a test fails if
/// the two lists drift. Two values differ on purpose: HSTS is sent only over a
/// public certificate (a LAN name on Caddy's own CA has nothing to pin) and
/// without `preload`, which is a deliberate, slow-to-undo step.
pub const STATIC_HEADERS: [(&str, &str); 6] = [
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "strict-origin-when-cross-origin"),
    ("X-Frame-Options", "DENY"),
    (
        "Permissions-Policy",
        "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()",
    ),
    ("Cross-Origin-Opener-Policy", "same-origin"),
    (
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https:; font-src 'self' data:; connect-src 'self'; media-src 'self' https:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests",
    ),
];

pub const HSTS: (&str, &str) = (
    "Strict-Transport-Security",
    "max-age=63072000; includeSubDomains",
);

pub fn render_caddyfile(p: &Plan) -> String {
    let tls = if p.acme {
        "    # A public domain: Caddy gets and renews a certificate itself.\n"
    } else {
        "    tls internal\n"
    };
    let mut s = format!("# {HEADER}\n{} {{\n{tls}    encode zstd gzip\n", p.host);
    if p.static_dir.is_none() {
        s.push_str(&format!(
            "    reverse_proxy {}:{}\n}}\n",
            p.proxy, p.proxy_port
        ));
        return s;
    }
    s.push_str("    header {\n        -Server\n");
    let hsts = p.acme.then_some(HSTS);
    for (k, v) in hsts.iter().chain(STATIC_HEADERS.iter()) {
        s.push_str(&format!("        {k} \"{v}\"\n"));
    }
    s.push_str("    }\n\n");
    s.push_str(&format!(
        "    handle {}/* {{\n        reverse_proxy {}:{}\n    }}\n\n",
        p.api_prefix, p.proxy, p.proxy_port
    ));
    s.push_str(
        "    handle {\n        root * /srv/web\n        try_files {path} {path}/index.html\n        file_server\n    }\n\n    handle_errors {\n        root * /srv/web\n        rewrite * /404.html\n        file_server\n    }\n}\n",
    );
    s
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

/// The topology, when the product has one. A product that is only an app and a
/// database declares none.
pub fn topology_if_any(root: &Path, decl: &str) -> Result<Option<Topology>> {
    if root.join(decl).exists() {
        Ok(Some(crate::topology::load(root, decl)?))
    } else {
        Ok(None)
    }
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

    fn config(extra: &str) -> Config {
        toml::from_str(&format!(
            "[product]\nname = \"building\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n{extra}"
        ))
        .unwrap()
    }

    fn site() -> Topology {
        toml::from_str(SITE).unwrap()
    }

    #[test]
    fn the_building_use_case_gets_its_broker_and_acl() {
        let p = plan(
            &config(
                "ai = \"ollama\"\nauth = \"native\"\ndatabase = \"sqlite\"\n\n[ai]\nmodel = \"llama3.2\"\n",
            ),
            Some(&site()),
        )
        .unwrap();
        let c = render_compose(&p);
        for s in ["server:", "mosquitto:", "caddy:", "ollama:"] {
            assert!(c.contains(&format!("  {s}")), "{s}\n{c}");
        }
        assert!(c.contains("platform: linux/arm64"));
        assert!(c.contains("SESSION_SECRET: ${SESSION_SECRET}"));
        assert!(c.contains("DATABASE_PATH: \"/data/server.db\""), "{c}");
        assert!(c.contains("AI_MODEL: \"llama3.2\""), "{c}");
        assert!(c.contains("AI_BASE_URL: \"http://ollama:11434/v1\""), "{c}");
        let a = render_acl(&p);
        assert!(a.contains("pattern write building/%u/state"));
        assert!(a.contains("pattern read building/%u/command"));
        assert!(a.contains("topic write building/+/command"));
        assert!(!a.contains("pattern write building/%u/command"));
    }

    #[test]
    fn without_a_topology_it_is_one_app_and_a_proxy() {
        let p = plan(&config(""), None).unwrap();
        let c = render_compose(&p);
        assert!(c.contains("  server:") && c.contains("  caddy:"), "{c}");
        assert!(!c.contains("mosquitto") && !c.contains("ollama"), "{c}");
        assert!(render_caddyfile(&p).contains("reverse_proxy server:8080"));
        assert!(p.broker.is_none());
    }

    #[test]
    fn declared_services_sit_beside_the_derived_ones() {
        let cfg: Config = toml::from_str(
            r#"
[product]
name = "shop"
version = "0.1.0"

[adapters]
deploy = "compose"

[deploy]
proxy = "web"
proxy_port = 3000

[[deploy.services]]
name = "db"
image = "postgres:17"
volumes = ["db-data:/var/lib/postgresql/data"]
secrets = ["POSTGRES_PASSWORD"]
healthcheck = "pg_isready -U postgres"

[[deploy.services]]
name = "web"
image = "ghcr.io/me/shop-web:latest"
depends_on = ["db"]
"#,
        )
        .unwrap();
        let p = plan(&cfg, None).unwrap();
        let c = render_compose(&p);
        assert!(c.contains("  db:\n    image: postgres:17"), "{c}");
        assert!(c.contains("POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}"), "{c}");
        assert!(c.contains("  db-data:"), "named volume declared: {c}");
        assert!(render_caddyfile(&p).contains("reverse_proxy web:3000"));
    }

    #[test]
    fn a_static_export_is_served_by_caddy_with_the_api_proxied() {
        let cfg: Config = toml::from_str(
            "[product]\nname = \"site\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n\n[deploy]\nstatic_dir = \"apps/web/out/\"\n",
        )
        .unwrap();
        let p = plan(&cfg, None).unwrap();
        let caddy = render_caddyfile(&p);
        assert!(
            caddy.contains("handle /api/* {\n        reverse_proxy server:8080"),
            "{caddy}"
        );
        assert!(
            caddy.contains("root * /srv/web") && caddy.contains("file_server"),
            "{caddy}"
        );
        assert!(caddy.contains("X-Frame-Options \"DENY\""), "{caddy}");
        assert!(
            !caddy.contains("Strict-Transport-Security"),
            "no HSTS on a LAN name: {caddy}"
        );
        assert!(render_compose(&p).contains("      - ../apps/web/out:/srv/web:ro"));
        let e = plan(
            &toml::from_str::<Config>(
                "[product]\nname = \"s\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n\n[deploy]\nstatic_dir = \"../elsewhere\"\n",
            )
            .unwrap(),
            None,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(e.contains("inside the product"), "{e}");
    }

    #[test]
    fn a_public_domain_gets_hsts_and_the_headers_match_the_web_template() {
        let cfg: Config = toml::from_str(
            "[product]\nname = \"site\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n\n[deploy]\nhost = \"example.org\"\ntls = \"acme\"\nstatic_dir = \"apps/web/out\"\n",
        )
        .unwrap();
        let caddy = render_caddyfile(&plan(&cfg, None).unwrap());
        assert!(
            caddy.contains("Strict-Transport-Security \"max-age=63072000; includeSubDomains\""),
            "{caddy}"
        );
        assert!(!caddy.contains("preload"), "{caddy}");

        // The header names are the web template's own, from its source: a name
        // added there and not here (or the reverse) is a header the static
        // export silently lacks.
        let next = include_str!("../capabilities/web-next/apps/web/next.config.ts");
        let mut in_next: Vec<&str> = next
            .split("key: \"")
            .skip(1)
            .filter_map(|r| r.split('"').next())
            .collect();
        in_next.sort_unstable();
        let mut here: Vec<&str> = std::iter::once(HSTS.0)
            .chain(STATIC_HEADERS.iter().map(|(k, _)| *k))
            .collect();
        here.sort_unstable();
        assert_eq!(
            in_next, here,
            "next.config.ts securityHeaders vs fid-compose STATIC_HEADERS"
        );

        // The Astro template's `_headers` is the third copy: the same names and,
        // for every header the platform shares, the same value.
        let astro = include_str!("../capabilities/web-astro/apps/web/public/_headers");
        let mut in_astro: Vec<&str> = astro
            .lines()
            .filter(|l| l.starts_with("  "))
            .filter_map(|l| l.trim().split(':').next())
            .collect();
        in_astro.sort_unstable();
        assert_eq!(
            in_astro, here,
            "web-astro _headers vs fid-compose STATIC_HEADERS"
        );
        for (k, v) in STATIC_HEADERS.iter().chain(std::iter::once(&HSTS)) {
            assert!(
                astro.contains(&format!("{k}: {v}")),
                "web-astro _headers lacks `{k}: {v}`"
            );
        }

        // And the React Router template's `app/security.ts`, the fourth: the same
        // names and the same values, written as `["Name", "value"]` pairs.
        let router = include_str!("../capabilities/web-router/apps/web/app/security.ts");
        for (k, v) in STATIC_HEADERS.iter().chain(std::iter::once(&HSTS)) {
            assert!(
                router.contains(&format!("\"{k}\"")) && router.contains(&format!("\"{v}\"")),
                "web-router app/security.ts lacks `{k}: {v}`"
            );
        }
    }

    #[test]
    fn a_duplicate_or_unknown_service_is_named() {
        let cfg: Config = toml::from_str(
            "[product]\nname = \"p\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n\n[[deploy.services]]\nname = \"server\"\nimage = \"x\"\n",
        )
        .unwrap();
        let e = plan(&cfg, None).err().unwrap().to_string();
        assert!(e.contains("`server` is declared twice"), "{e}");
        let cfg: Config = toml::from_str(
            "[product]\nname = \"p\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n\n[deploy]\nproxy = \"nope\"\n",
        )
        .unwrap();
        let e = plan(&cfg, None).err().unwrap().to_string();
        assert!(e.contains("names no service"), "{e}");
    }

    #[test]
    fn a_lan_name_cannot_ask_for_a_public_certificate() {
        let cfg: Config = toml::from_str(
            "[product]\nname = \"b\"\nversion = \"0.1.0\"\n\n[adapters]\ndeploy = \"compose\"\n\n[deploy]\ntls = \"acme\"\n",
        )
        .unwrap();
        let e = plan(&cfg, Some(&site())).err().unwrap().to_string();
        assert!(e.contains("cannot get a public certificate"), "{e}");
    }
}

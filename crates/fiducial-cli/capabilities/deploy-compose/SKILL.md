# Skill: deploy-compose

**Capability:** `deploy-compose` · **Platform:** Fiducial {{version}}

---

## What this capability adds

The second deploy target: one Linux host (a Raspberry Pi 4 or any x86-64 box)
running Docker Compose, with no vendor account and no required internet. Set
`[adapters] deploy = "compose"` and `fid derive` writes:

| File | Holds |
|---|---|
| `deploy/compose.yaml` | every service, with volumes, healthchecks, restart policy and the image platform |
| `deploy/Caddyfile` | TLS and the reverse proxy to one service |
| `deploy/mosquitto/mosquitto.conf`, `deploy/mosquitto/acl` | only when `topology.toml` declares an MQTT broker: the listener with TLS and the topic ACL |

All are derived: edit the declarations, never the files. `fid derive --check`
fails on a stale or hand-edited one. A product with a broker lists the two
Mosquitto files in `pipelines/deploy-compose.toml`; derive names them if they are
missing.

## It is a list of services, from four sources

Nothing here assumes one kind of product.

| Source | Services |
|---|---|
| **Apps** | each `apps` entry of each `host` node in `topology.toml` (an app named `server` with no topology) |
| **The broker** | `mosquitto`, when a `[[bus]]` has `broker = "<host>"` |
| **The model server** | `ollama`, when `[adapters] ai = "ollama"` |
| **Declared** | `[[deploy.services]]`: a database, a cache, a worker, anything with an image |

```toml
[adapters]
deploy = "compose"

[deploy]
host       = "shop.example.com"   # a LAN name or a public domain; default <product>.local
registry   = "ghcr.io/you"         # where the app images are pulled from
tls        = "acme"                # Caddy's own CA is "internal" (the default)
proxy      = "web"                 # which service Caddy proxies; default the first app
proxy_port = 3000                  # default 8080

[[deploy.services]]
name        = "db"
image       = "postgres:17"
volumes     = ["db-data:/var/lib/postgresql/data"]   # a name before the colon is a named volume
secrets     = ["POSTGRES_PASSWORD"]                  # written as ${POSTGRES_PASSWORD}, set in deploy/.env
healthcheck = "pg_isready -U postgres"
```

`tls = "acme"` with a `.local` host is refused: nothing can issue a public
certificate for it. A service declared twice, a `proxy` that names no service and
a `depends_on` that names none are each refused by name.

## The use case that exercises all of it: a building

Controllers in each room, a Raspberry Pi in the cabinet, one web app. The
topology declares the controllers (`mcu`), the Pi (`host`, `target =
"aarch64-unknown-linux-gnu"`, `apps = ["server"]`) and a `lan` bus with
`broker = "server"`. From that alone:

- `server` (`platform: linux/arm64`), `mosquitto` and `caddy`, plus `ollama` if
  chosen, in `compose.yaml`;
- the broker's ACL: a device's MQTT username is its node id, and it may touch only
  `<product>/<device>/<topic>`, writing the topics the topology marks `up` or
  `will` and reading the ones marked `down`. The `server` user reads every device's
  `up` topics and writes every device's `down` topics. That is the rule `can()`
  states for a device acting on itself. Grants a product stores beyond it are
  runtime data and are not enumerated into the ACL.

The integration test `tests/compose_pipeline.rs` is this product.

## The Rust vendors that go with it

Three `[adapters]` selections are implemented on the Rust side only, for the
product's own server (`fiducial-adapters`, each behind the cargo feature of the
same name):

| Selection | What it is |
|---|---|
| `database = "sqlite"` | one SQLite file in WAL mode, through `sqlx`; the same dialect as `d1`, so a migration runs on both. Sets `DATABASE_PATH` for the server |
| `auth = "native"` | accounts declared in server configuration with argon2id hashes, sessions as HMAC-signed tokens; no sign-up and no reset email. The key is the secret `SESSION_SECRET`, at least 32 bytes |
| `ai = "ollama"` | the OpenAI-compatible endpoint of the `ollama` service; `[ai] model` is required and `[ai] base_url` defaults to `http://ollama:11434/v1`. Sets `AI_BASE_URL` and `AI_MODEL`. Plain http only |

They have **no TypeScript class**: the generated `adapters.generated.ts` throws a
message saying so if it is read, rather than falling back to `none`. In the server:

```toml
# Cargo.toml
fiducial-adapters = { version = "…", features = ["sqlite", "native", "ollama"] }
```

## What it does not derive

- **Passwords and certificates.** Per-device credentials are provisioned at first
  flash into `deploy/mosquitto/passwd`, and the broker's certificates go in
  `deploy/certs/`. Neither is committed.
- **Secrets.** Their names appear in `compose.yaml` as `${NAME}`; the values go in
  `deploy/.env`.
- **The app images.** `/healthz` on the proxy port is the contract the healthcheck
  reads; building the images is the product's.
- **Backups.** A disk filling and a certificate expiring are this target's failure
  modes; back up the named volumes nightly.

## Guard rules activated

None.

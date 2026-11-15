# Skill: deploy-compose

**Capability:** `deploy-compose` · **Platform:** Fiducial {{version}}

---

## What this capability adds

The second deploy target: one Linux host (a Raspberry Pi 4 or any x86-64 box)
running Docker Compose, with no vendor account and no required internet. Set
`[adapters] deploy = "compose"` and `fid derive` writes:

| File | Holds |
|---|---|
| `deploy/compose.yaml` | `server`, `mosquitto`, `caddy`, and `ollama` when `[adapters] ai = "ollama"` — with volumes, healthchecks, restart policy and the image platform from the host node's `target` |
| `deploy/Caddyfile` | TLS and the reverse proxy to the server on port 8080 |
| `deploy/mosquitto/mosquitto.conf` | the MQTT listener on 8883 with TLS, no anonymous clients |
| `deploy/mosquitto/acl` | which principal may publish or subscribe to which topic |

All four are derived: edit `topology.toml`, `[adapters]` or `[deploy]`, never the
files. `fid derive --check` fails on a stale or hand-edited one.

## What you declare

```toml
[adapters]
deploy = "compose"

[deploy]
host     = "building.local"      # a LAN name or a public domain; default <product>.local
registry = "ghcr.io/you"         # where the server image is pulled from
tls      = "internal"            # Caddy's own CA; "acme" for a public domain
```

`tls = "acme"` with a `.local` host is refused: nothing can issue a public
certificate for it.

## The broker ACL

A device's MQTT username is its node id. It may touch only
`<product>/<device>/<topic>`: write the topics the topology marks `up` or `will`,
read the ones marked `down`. The `server` user reads every device's `up` topics and
writes every device's `down` topics. That is the same rule `can()` states for a
device acting on itself. Grants a product stores beyond it are runtime data and are
not enumerated into the ACL.

## What it does not derive

- **Passwords and certificates.** Per-device credentials are provisioned at first
  flash into `deploy/mosquitto/passwd`, and the broker's certificates go in
  `deploy/certs/`. Neither is committed.
- **Secrets.** Their names are in `compose.yaml` as `${NAME}`; the values go in
  `deploy/.env`.
- **The server image.** `/healthz` on port 8080 is the contract the healthcheck
  reads; building the image is the product's.
- **Backups.** A disk filling and a certificate expiring are this target's failure
  modes; back up the `server-data` volume nightly (`docker run --rm -v
  <product>_server-data:/data -v $PWD:/backup alpine tar czf /backup/data.tgz /data`).

## Guard rules activated

None.

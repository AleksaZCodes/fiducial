# A Linux host running Docker Compose is the second deploy target

**Date:** 2026-11-01
**Status:** accepted
**Supersedes:** nothing

---

## Context

Every server-side adapter assumes Cloudflare or Supabase:

| Contract | Selectable today |
|---|---|
| `deploy` | `cloudflare` only |
| `auth` | `supabase` only |
| `ai` | `openrouter`, implemented in TypeScript only "because there is no Rust-side consumer" |

A school building has to run on a Raspberry Pi 4 on the school network. It
must have no account at any vendor and keep working without the internet. A
contract with one implementation has never been tested as a contract. This
product is the second implementation of `database`, `auth`, `ai` and `deploy`.

ARCHITECTURE §5.1 puts server glue in TypeScript.

## Decision

- **`deploy = "compose"`** derives `deploy/compose.yaml` and
  `deploy/Caddyfile`, following `deploy-config-is-derived`:

  | Derived from adapters and topology | Declared in `[deploy]` |
  |---|---|
  | services (`server`, `mosquitto`, `caddy`, plus `ollama` when `ai = "ollama"`) | domain or LAN hostname |
  | volumes, healthchecks and ports | ACME DNS provider or internal CA |
  | image platforms, from the host nodes' target triples | image registry |
  | secret names, as comments | |

- **`database = "sqlite"`** uses `sqlx` with WAL. It maps to the existing
  `SqlDialect::Sqlite`, which `d1` already uses.
- **`auth = "native"`**:
  - argon2 hashes;
  - accounts declared in server configuration, with secrets in SOPS;
  - cookie sessions.
- **`ai = "ollama"`** points at an OpenAI-compatible local endpoint, with the
  base URL in `[ai]`. `openrouter` gains its Rust half.
- **The server is Rust** (`axum` + `tokio` + `sqlx`). This is a recorded
  exception to ARCHITECTURE §5.1. The server:
  - runs on a Pi next to a local model;
  - decodes postcard natively;
  - runs the product's `no_std` rules crate to explain states;
  - calls `can()` natively.

  It is the first Rust-side consumer of `database`, `auth` and `ai`, which is
  what the Rust half of `fiducial-adapters` exists for. The web app is a
  static export, so no server-side TypeScript remains.
- **Public read is a declaration.** `[access] public_read = true` lists the
  resources an anonymous request may read without entering `can()`. Every
  write goes through `can()`. One user principal kind holds full rights;
  devices hold grants on their own topics.
- **The broker ACL is derived** by enumerating `can()` over every declared
  topic and device principal, and it joins the identity conformance vectors.

## Why not the alternatives

- **A TypeScript server on Node.** It would decode postcard through WASM on a
  4 GB Pi beside a model. It would re-implement the rules crate, or call it
  through WASM, to explain a state. It would also leave the Rust half of the
  adapters without a consumer for another product.
- **A fake identity for every viewer.** It makes "anonymous" a principal that
  `can()` grants, so a missing grant on a write fails open in a way the
  conformance vectors cannot see.
- **Kubernetes or Nomad.** One host and four services. Compose is the
  smallest thing that runs them with healthchecks and restarts.

## Consequences

- Switching `[adapters] ai` between `openrouter` and `ollama` changes
  `compose.yaml` and the generated factory and nothing else.
- `fid doctor` treats `compose`, `sqlite`, `native` and `ollama` like every
  other implementation, with conformance tests.
- The platform gains a deploy target with no vendor account. Its failure
  modes are a disk filling up and a certificate expiring, so the nightly
  backup and Caddy's renewal are part of the scaffold, not advice.

## What this deliberately does not do

- No roles, user-management screen or audit log. The first product defines
  none; the last command and its outcome per device is state, not an audit.
- No over-the-air firmware update.
- No public-internet exposure by default: the hostname is a LAN name unless
  `[deploy]` declares a domain.

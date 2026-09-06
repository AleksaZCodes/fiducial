# Skill: tauri

**Capability:** `tauri` · **Platform:** Fiducial {{version}}

---

## What this capability adds

A Tauri 2 desktop + mobile application at `apps/desktop/`, backed by a Rust
`src-tauri/` core that shares crates with firmware and geometry.

- **Tauri 2** — Rust backend, platform-native window, signed updater
- **Shared Rust crates** — `fiducial-core`, `fiducial-protocol`, and any product
  crates compile natively; no WASM boundary needed
- **`tauri-plugin-updater`** — signed binary OTA, A/B semantics on desktop
- **Mobile** — same codebase targets iOS and Android via Tauri; store release +
  signed binary via the updater

## Development

```sh
pnpm tauri dev                  # desktop dev (hot-reload)
pnpm tauri android dev          # Android dev (requires Android SDK)
pnpm tauri ios dev              # iOS dev (requires Xcode on macOS)
pnpm tauri build                # release build (all platforms via CI)
```

## Sharing code with the spine

```toml
# apps/desktop/src-tauri/Cargo.toml
[dependencies]
fiducial-core = { path = "../../../crates/fiducial-core", features = ["std"] }
```

## OTA updates

Tauri compiles web assets into the binary — there is no web-only OTA. The full
signed binary is the update unit. `tauri-plugin-updater` handles download,
verification (`ed25519`), and install. The private signing key lives in the
portfolio secret store; the public key is baked into `tauri.conf.json`.

## Key constraints

- Sign every release binary — the updater verifies the signature before installing.
- Never ship a debug binary as an update.
- Test the updater path in CI with a staging signing key.

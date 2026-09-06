# Skill: firmware-rp2040

**Capability:** `firmware-rp2040` · **Platform:** Fiducial {{version}}

---

## What this capability adds

RP2040 Embassy firmware at `firmware/rp2040/`, using:

- **Embassy 0.10** async executor
- **embassy-rp 0.10** HAL
- **defmt** structured logging over USB-serial / RTT
- **probe-rs** for flashing and debugging

The firmware workspace is a **separate** Cargo workspace (excluded from the host
workspace) because embedded targets require a different toolchain. It lives at
`firmware/` and has its own `rust-toolchain.toml`.

## Build & flash

```sh
# Build (requires thumbv6m-none-eabi target)
cd firmware
cargo build --release

# Flash to connected RP2040 (requires probe-rs)
cargo run --release

# Read defmt logs
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/release/<binary>

# Or with cargo-embed
cargo embed --release
```

## Sharing code with the host

The `fiducial-core` and `fiducial-protocol` crates are `#![no_std]` and compile
for `thumbv6m-none-eabi`. Import them directly:

```toml
# firmware/rp2040/Cargo.toml
[dependencies]
fiducial-core = { path = "../../crates/fiducial-core" }
```

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-direct-flash-without-check` | Flashing firmware that has not passed its build + defmt self-test |
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |

## Key constraints

- Firmware is built in the `firmware/` workspace, **not** the host workspace.
- Never `cargo install` tooling without pinning the version in `rust-toolchain.toml`
  or a Cargo `.lock`.
- Hardware CI is manual — no RP2040 in the runner. Mark hardware-only checks with
  `// MANUAL` comments and a corresponding entry in `docs/manual-checks.md`.

# Skill: firmware-stm32

**Capability:** `firmware-stm32` · **Platform:** Fiducial {{version}}

---

## What this capability adds

STM32F401 Embassy firmware at `firmware/stm32/`, using:

- **Embassy 0.10** async executor
- **embassy-stm32 0.10** HAL — chip: `stm32f401cc` (blackpill v2 default)
- **defmt** structured logging over RTT
- **probe-rs** for flashing and debugging (ST-Link, J-Link, DAPLink)

The firmware workspace is a **separate** Cargo workspace (excluded from the host
workspace) at `firmware/`.

## Build & flash

```sh
# Check (fast, no binary)
cd firmware
cargo check --target thumbv7em-none-eabihf

# Build release binary (from firmware/stm32/)
cargo build --release

# Flash (from firmware/stm32/)
cargo run --release

# Or with probe-rs directly:
probe-rs run --chip STM32F401CCUx \
  target/thumbv7em-none-eabihf/release/firmware-stm32
```

## Chip targeting

Default chip: `stm32f401cc` — 256K flash, 64K RAM (blackpill v2).
To retarget: change the `embassy-stm32` feature in `firmware/stm32/Cargo.toml`
and update `firmware/stm32/memory.x` with the correct FLASH/RAM lengths.

| Variant      | Flash | RAM  | Feature flag  |
|---|---|---|---|
| STM32F401CB  | 128K  | 64K  | `stm32f401cb` |
| STM32F401CC  | 256K  | 64K  | `stm32f401cc` |
| STM32F401CD  | 384K  | 96K  | `stm32f401cd` |
| STM32F401CE  | 512K  | 96K  | `stm32f401ce` |

## Sharing L0 crates

`fiducial-core` and `fiducial-protocol` are `#![no_std]` and compile for
`thumbv7em-none-eabihf`. They are declared in the firmware workspace deps
(`firmware/Cargo.toml`) and re-exported from `firmware-shared`.

## LoRa

Enable LoRa PHY constants by adding `features = ["lora"]` to `firmware-shared`
in `firmware/stm32/Cargo.toml`. The `firmware::lora` module exposes EU868
channel frequencies and spreading-factor/bandwidth pairs via `lora-modulation`.

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-direct-flash-without-check` | Flashing before build + defmt self-test passes |
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |

## Key constraints

- Firmware is built in the `firmware/` workspace, **not** the host workspace.
- Hardware CI is manual — no STM32 in the runner. Mark hardware-only checks
  with `// MANUAL` comments.

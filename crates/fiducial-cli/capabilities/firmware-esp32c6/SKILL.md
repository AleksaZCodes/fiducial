# Skill: firmware-esp32c6

**Capability:** `firmware-esp32c6` · **Platform:** Fiducial {{version}}

---

## What this capability adds

ESP32-C6 firmware at `firmware/esp32c6/`, using:

- **esp-hal 1.x** — Espressif's own bare-metal HAL
- **esp-rtos** with its `embassy` feature — the scheduler and the async executor
- **embassy-executor 0.10** tasks, **embassy-time** timers
- **defmt** logging, carried over the serial port by `esp-println` and read by `espflash`
- **esp-backtrace** as the panic handler
- **espflash** for flashing and the serial monitor

It is an optional template. A product that already has firmware in another
language reads the same derived tables (`.rs`, `.h` or `.json`) from
`topology.toml`; this capability only gives a Rust product a place to start.

The firmware workspace is a **separate** Cargo workspace, at `firmware/`, with
its own `rust-toolchain.toml`, because an embedded target needs a different one.

## Build & flash

```sh
cd firmware/esp32c6

# Build (the riscv32imac-unknown-none-elf target is in rust-toolchain.toml)
cargo build --release

# Flash and open the serial monitor (needs `cargo install espflash`)
cargo run --release
```

`.cargo/config.toml` sets the target, the linker scripts (`linkall.x` from
esp-hal and `defmt.x`) and the `espflash flash --monitor` runner.

## Versions move together

`esp-hal`, `esp-rtos`, `esp-backtrace`, `esp-println` and
`esp-bootloader-esp-idf` share generated chip metadata. They are pinned in one
place, the `[workspace.dependencies]` table of `firmware/Cargo.toml`, and a
mismatch fails at link time rather than at resolve time. Move them together.

Do **not** enable an `arch-*` feature on `embassy-executor`: `esp-rtos` supplies
the executor, and a second one fails to link.

## Radio

Wi-Fi, BLE and 802.15.4 are in `esp-radio`, which is not a dependency here: it
needs a heap and a decision about which radios the product uses. Add it, with
`esp-alloc`, when the product has a radio to use, and enable `esp-rtos`'s
`esp-radio` and `esp-alloc` features with it.

## Pins from the declaration

When `topology.toml` declares a module-built node, `fid derive` writes a `board`
table for the node's firmware role in the language the output's extension names.
Put it at `firmware/esp32c6/src/board.rs` and read the pins from it instead of
writing `GPIO15` in the source. The LED in the template is GPIO 15; change it to
the pin of the board in use.

## Sharing code with the host

`fiducial-core` and `fiducial-protocol` are `#![no_std]` and compile for
`riscv32imac-unknown-none-elf`; `firmware/shared` re-exports them
(`version`, `DeviceId`, `encode`, `FrameDecoder`).

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |

Flashing a board is your judgement, not a guard rule: nothing here stops a flash
that has not passed its build.

## Verified

`scripts/verify-firmware.sh` in the platform repository scaffolds a product, runs
`fid add firmware esp32c6` and builds and links it for the real target on every
change to a firmware template. It does not flash a board.

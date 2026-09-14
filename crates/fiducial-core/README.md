# fiducial-core

**The `no_std` spine every Fiducial runtime shares.**

[![crates.io](https://img.shields.io/crates/v/fiducial-core.svg)](https://crates.io/crates/fiducial-core)
[![docs.rs](https://docs.rs/fiducial-core/badge.svg)](https://docs.rs/fiducial-core)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

The reference point everything else aligns against. Firmware, the browser
(WASM), the desktop app (Tauri), the edge (Workers) and host tooling all depend
on this same compiled artifact — so a fact declared here cannot disagree with
itself across runtimes, because there is only one copy of it.

This is [principle 2](../../MISSION.md) made concrete: *push behavior down to the
layer with the longest reach.* Logic placed here runs everywhere and outlives
every framework above it.

## Install

```sh
cargo add fiducial-core
```

## Use

```rust
use fiducial_core::{version, DeviceId};

// The platform version, injected at compile time by Cargo.
assert!(!version().is_empty());

// An 8-byte opaque device identifier — the same type on the MCU and in the browser.
let id = DeviceId::new([0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33]);
```

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Pure `no_std`. What embedded targets use. |
| `alloc` | | Enables owned types that need a heap. |
| `std` | | Implies `alloc`. What WASM and host tooling use. |

The crate is unconditionally `#![no_std]`. `std` is opt-in, never assumed.

## Supported targets

CI compiles this crate for all four **on every commit**, so a target breaks the
day it breaks rather than the day it is needed:

| Target | Triple | Notes |
|---|---|---|
| Host | `x86_64-unknown-linux-gnu` | tests run here |
| WASM | `wasm32-unknown-unknown` | browser + edge |
| RP2040 | `thumbv6m-none-eabi` | Cortex-M0+ |
| STM32 | `thumbv7em-none-eabihf` | Cortex-M4F |

## License

MIT

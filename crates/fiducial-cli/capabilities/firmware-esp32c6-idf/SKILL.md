# Skill: firmware-esp32c6-idf

**Capability:** `firmware-esp32c6-idf` · **Platform:** Fiducial {{version}}

---

## What this capability adds

ESP32-C6 firmware at `firmware/esp32c6-idf/` that runs **on ESP-IDF**: Espressif's
own SDK with FreeRTOS, the Wi-Fi, BLE, TLS and MQTT stacks, NVS storage and OTA. It
is `std` Rust, through:

- **esp-idf-svc** — safe wrappers for the SDK's services (Wi-Fi, MQTT, HTTP, NVS, OTA)
- **esp-idf-hal / esp-idf-sys** — re-exported by it; drivers and the raw bindings
- **embuild** — downloads and builds the SDK for you
- **ldproxy** and **espflash** — linking and flashing

## Which one to choose

| | `esp32c6` (esp-hal + Embassy) | `esp32c6-idf` (this) |
|---|---|---|
| Language runtime | `no_std`, async | `std`, threads, FreeRTOS |
| SDK | none; Espressif's Rust HAL | ESP-IDF |
| Wi-Fi, TLS, MQTT, OTA | add `esp-radio` and write more | in the SDK, one call each |
| Toolchain | stable | a **pinned nightly** with `build-std` |
| First build | seconds | the SDK is downloaded and built: minutes, and gigabytes |
| Footprint | smallest | larger |

Pick this one when the node needs a networking stack, TLS or over-the-air updates
the SDK already provides, or when the team knows ESP-IDF. Pick the other for
small, hard-real-time firmware. A product can have both, in different
directories: this one is its own Cargo workspace precisely so the two toolchains
never meet.

## Build & flash

```sh
cargo install ldproxy espflash        # once
cd firmware/esp32c6-idf
cargo build --release                 # the first build fetches the SDK
cargo run --release                   # flash and open the serial monitor
```

The target, linker, nightly `build-std` and the pinned SDK version
(`ESP_IDF_VERSION`) are in `.cargo/config.toml` and `rust-toolchain.toml`.
Kernel and SDK options go in `sdkconfig.defaults`. Move the SDK version on
purpose, not by tracking a branch.

## Sharing code with the host

`fiducial-core` is a dependency here (`version()` and the identifiers).
`fiducial-protocol` compiles on this target too: add it and decode frames the way
the host does. Tables derived from `topology.toml` (`.rs`) are plain Rust and
`include!` straight in.

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |

Flashing a board is your judgement, not a guard rule.

## The toolchain is pinned

`rust-toolchain.toml` names a dated nightly, not `nightly`. ESP-IDF's `std` is
built from source and tracks `libc` closely: on 2026-11-13 the then-current
nightly's `std` failed to compile for this target. A floating channel would make
the same firmware stop building on a day nobody chose.

## Verified

`TARGETS=esp32c6-idf scripts/verify-firmware.sh` scaffolds a product, adds this
capability and builds it for the real target. It is not in the default set: the
first build downloads the ESP-IDF SDK and its toolchain. See that script's header.

**What has and has not been checked.** On the pinned nightly, `std` and every crate
through `esp-idf-sys` compile, and the SDK clones. The SDK's own tool installer then
needs `dl.espressif.com`, which the environment this was written in blocks, so a
full build and link have not been run, and nothing was flashed. Treat the first
`cargo build` on a networked machine as the real test and report what it says.

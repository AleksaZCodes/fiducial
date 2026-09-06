# {{name}} — RP2040 Firmware

Embassy async firmware for the RP2040, part of the `{{name}}` product.

## Build

```sh
cd firmware

# Check (fast, no binary produced)
cargo check --target thumbv6m-none-eabi

# Build release binary
cargo build --release

# Flash via probe-rs (connect RP2040 via SWD)
cargo run --release
```

## Toolchain

Pin declared in `firmware/rust-toolchain.toml`. Do not change it without
updating the CI matrix.

## Sharing L0 crates

This firmware imports `fiducial-core` (and optionally `fiducial-protocol`,
`fiducial-quantity`) from the host workspace via relative paths. All L0 crates
are `#![no_std]` and compile for `thumbv6m-none-eabi` on every commit.

## Logging

Uses `defmt` over USB-serial (default) or RTT. View logs with:

```sh
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/release/firmware
# or
cargo embed --release
```

//! Firmware adapter boundary.
//!
//! Cloud adapter contracts — database, storage, email, diagnostics — **do not
//! apply to `no_std` firmware targets** (RP2040, STM32, and future bare-metal
//! boards). Attaching a database client to a microcontroller makes no sense:
//! the CPU has no OS scheduler, no heap allocator in many configurations, no
//! TCP/IP stack in user code, and typically no persistent network connection.
//!
//! # What firmware uses instead
//!
//! | Cloud contract | Firmware equivalent |
//! |---|---|
//! | `database`     | embedded-storage flash traits (`ReadStorage`, `NorFlash`) |
//! | `storage`      | same, plus optional FATFS/littlefs on SPI flash |
//! | `email`        | not applicable; logging goes over UART or RTT |
//! | `diagnostics`  | `defmt` logger + panic handler; no network path |
//! | `deploy`       | `fiducial-ota` — OTA manifest signed with ed25519, delivered over LoRa or USB |
//!
//! Embassy provides the async runtime and HAL traits. `fiducial-ota` provides
//! the OTA protocol. Neither depends on this crate.
//!
//! # The boundary, stated plainly
//!
//! A product that targets firmware uses:
//! - `firmware/*` — the bare-metal crates, excluded from the workspace
//! - `crates/fiducial-ota` — OTA delivery
//! - `crates/fiducial-protocol` — the wire format (no_std postcard)
//!
//! A product that targets web/server/edge uses:
//! - This crate (`fiducial-adapters`) for runtime contracts
//! - `packages/adapters` for the TypeScript equivalents
//!
//! A Tauri product that bridges both worlds uses:
//! - This crate in the Rust backend (over Tauri commands)
//! - `packages/adapters` in the frontend
//! - `crates/fiducial-tauri` for the serial/USB transport to connected devices

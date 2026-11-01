# ESP32-C6 firmware is `esp-hal` + Embassy, flashed over USB with probe-rs

**Date:** 2026-11-01
**Status:** accepted
**Supersedes:** nothing

---

## Context

The first distributed product's controllers are ESP32-C6 boards, either the
DevKitC (WROOM-1) or the Seeed XIAO. The input design proposed `esp-idf-svc`
(std on FreeRTOS), `espflash`, and OTA with rollback. Records already in this
repository decide otherwise:

- the design spec §7.4 names `esp-hal` (`no_std`) with Embassy, with
  `esp-idf-hal` as the escape hatch only, and prefers the RISC-V C6;
- STACK §6 names probe-rs and defmt;
- `riscv32imac-unknown-none-elf` is already compiled on every commit.

The product owner flashes over USB-C and wants no OTA in this product.

## Decision

- **Runtime:** `esp-hal` + Embassy, `esp-radio` (Wi-Fi), `esp-rtos` and
  `embassy-net`. This is one concurrency model across STM32, RP2040 and ESP32.
- **Flashing:** `probe-rs` over the C6's built-in USB-JTAG, with `defmt`
  logging. No OTA.
- **Provisioning:** Wi-Fi credentials, the broker password and the server CA
  are written into flash at first flash by a `fid` command over USB. They are
  never in source. The guard rule `no-hardcoded-credentials` refuses an SSID,
  a password or PEM text under `firmware/`.
- **Boot:** outputs idle before anything else runs; then a task watchdog and
  brown-out handling start.
- **Persistence:** thresholds and the timetable arrive as retained MQTT
  messages and are persisted with `sequential-storage`. Occupancy is computed
  from the node's own clock (SNTP, then free-running), so a controller with no
  network keeps running its rules.
- **Pin rules are board data:**
  - strapping pins GPIO8, 9 and 15;
  - USB D−/D+ on GPIO12/13;
  - UART0 on GPIO16/17, driven by the boot ROM;
  - ADC-capable pins.

  Each module carries these in its library entry. The XIAO's D6 (UART0 TX)
  can never take a contact to GND.

## Why not the alternatives

- **`esp-idf-svc`.** A second concurrency model (FreeRTOS tasks) and a C
  toolchain in the build, for a controller whose work is a few sensors, two
  relays and one TLS connection. Embassy already covers the other two MCU
  families.
- **`espflash`.** It works, but the platform already flashes and logs every
  other target through probe-rs and defmt, and the C6 exposes USB-JTAG.
- **OTA now.** `embassy-boot` has no ESP backend, so OTA means
  `esp-bootloader-esp-idf` partitions plus `fiducial-ota`. Nobody needs it for
  this product. It stays a STACK candidate for the first product that does.

## Consequences

- `fid add firmware esp32c6` joins `rp2040` and `stm32`, and CI builds every
  `embassy-esp` member for `riscv32imac-unknown-none-elf`.
- Several details have to be verified against current documentation before
  the template is written:
  - which `esp-radio` and `esp-rtos` versions match the current `esp-hal`;
  - TLS 1.3 memory use with `rust-mqtt` and `embedded-tls`;
  - ADC linearity from 0 to 3.0 V;
  - RTC drift.

## What this deliberately does not do

- No Xtensa targets (ESP32, ESP32-S3) in this decision.
- No BLE, LoRa or Meshtastic bindings.

---
"@fiducial/transport-web": minor
---

Phase 13: `@fiducial/transport-web` — Web Serial, WebUSB, and BLE transports speaking the Fiducial frame protocol.

Exports a shared `Transport` interface and three browser implementations:
- `WebSerialTransport` — Chrome/Edge Web Serial API
- `WebUsbTransport` — Chrome/Edge WebUSB API
- `BleTransport` — Chrome/Edge Web Bluetooth API

All three encode with the same `encode()`/`FrameDecoder` codec as the Rust `fiducial-protocol` crate, ensuring byte-exact compatibility with firmware. Codec includes `crc8`, `encode`, `encodedLen`, and `FrameDecoder` (24 tests; green).

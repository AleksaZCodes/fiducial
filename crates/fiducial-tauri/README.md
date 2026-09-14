# fiducial-tauri

**USB serial transport for Fiducial desktop apps.**

[![crates.io](https://img.shields.io/crates/v/fiducial-tauri.svg)](https://crates.io/crates/fiducial-tauri)
[![docs.rs](https://docs.rs/fiducial-tauri/badge.svg)](https://docs.rs/fiducial-tauri)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

`SerialTransport` — open a USB serial port, send a Fiducial frame, receive one
back — plus `list_ports()` for device discovery and a `TransportError` that says
which stage failed.

It is one implementation of the transport contract below the
[`fiducial-protocol`](../fiducial-protocol) waist. The browser
([`@fiducial/transport-web`](../../packages/transport-web)) implements the same
contract over Web Serial, WebUSB and BLE. Both speak the identical frame format,
byte for byte, because both are checked against the same committed conformance
vectors — so the desktop app and the web app reach the same device with the same
codec and no translation layer between them.

## Install

```sh
cargo add fiducial-tauri
```

On Linux this needs `libudev`:

```sh
sudo apt-get install libudev-dev
```

## Use

```rust,ignore
use fiducial_tauri::{list_ports, SerialTransport};

for port in list_ports()? {
    println!("{}", port.name);
}

let mut transport = SerialTransport::open("/dev/ttyACM0", 115_200)?;
transport.send(b"ping")?;
let reply = transport.recv()?;
```

## With Tauri

`fid add app tauri` scaffolds a desktop app wired to this crate, exposing
`cmd_list_ports`, `cmd_connect`, `cmd_disconnect`, `cmd_send` and `cmd_recv` to
the frontend.

## License

MIT

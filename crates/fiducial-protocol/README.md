# fiducial-protocol

**The waist of the hourglass: one frame format, every transport.**

[![crates.io](https://img.shields.io/crates/v/fiducial-protocol.svg)](https://crates.io/crates/fiducial-protocol)
[![docs.rs](https://docs.rs/fiducial-protocol/badge.svg)](https://docs.rs/fiducial-protocol)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

A `no_std` frame codec that turns a byte stream into delimited, checksummed
messages. It is deliberately narrow: framing, checksums and version negotiation,
and nothing else.

That narrowness is the point. Above the waist sit sub-protocols
([`fiducial-ota`](../fiducial-ota) is the first); below it sits any transport
that moves bytes — USB serial, Web Serial, WebUSB, BLE, LoRa. Neither side knows
about the other, so adding a transport costs nothing at the message layer and
adding a message type costs nothing at the transport layer.

## Wire format

```text
┌────────┬────────┬────────┬───────────────┬─────────────────┐
│ MAGIC  │ LEN_LO │ LEN_HI │   PAYLOAD     │     CRC-32      │
│ 1 byte │ 1 byte │ 1 byte │  LEN bytes    │     4 bytes     │
└────────┴────────┴────────┴───────────────┴─────────────────┘
```

The **canonical specification** is
[`docs/protocol/README.md`](../../docs/protocol/README.md), and conformance
vectors live in [`docs/protocol/vectors.json`](../../docs/protocol/vectors.json).
Those vectors are generated from this crate, committed, and asserted by **both**
the Rust and TypeScript suites — so the browser codec
([`@fiducial/transport-web`](../../packages/transport-web)) cannot drift from
this one without failing a build.

## Install

```sh
cargo add fiducial-protocol
```

## Use

```rust
use fiducial_protocol::{encode, FrameDecoder};

let mut out = [0u8; 64];
let written = encode(b"hello", &mut out).unwrap();

let mut decoder: FrameDecoder<64> = FrameDecoder::new();
for &byte in &out[..written] {
    if let Some(frame) = decoder.feed(byte) {
        assert_eq!(frame, b"hello");
    }
}
```

The decoder is a state machine: it resynchronises after a bad magic byte,
rejects oversized frames, and never allocates.

## Version skew

`WIRE_VERSION` is declared once here and embedded in every compiled artifact.
`assert_compatible(local, remote, min_compatible)` rejects a peer below the
declared minimum, and the committed policy lives in
[`docs/compat/matrix.toml`](../../docs/compat/matrix.toml). `fid release check`
fails the build when the matrix and `WIRE_VERSION` disagree.

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Pure `no_std`. |
| `alloc` | | Owned frame buffers. |
| `std` | | Implies `alloc`. |

## License

MIT

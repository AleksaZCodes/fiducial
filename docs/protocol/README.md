# The Fiducial Protocol

**Wire version 2.** Canonical specification. Conformance vectors:
[`vectors.json`](./vectors.json).

This document describes the wire format completely enough to implement it in
any language. It is the reference; `vectors.json` is the test suite.

---

## 1. The shape: an hourglass with one waist

```
    ┌──────────────────────────────────────────────────────────┐
    │  Application   OTA · telemetry · commands                │  product-defined
    ├──────────────────────────────────────────────────────────┤
    │  Messages      one declared Rust type                    │  ← THE WAIST
    │                serialized with postcard                  │
    ├──────────────────────────────────────────────────────────┤
    │  Framing       MAGIC | LEN | PAYLOAD | CRC32             │  fiducial-protocol
    ├──────────────────────────────────────────────────────────┤
    │  Transport     UART · USB · BLE · LoRa · TCP             │  product-declared
    └──────────────────────────────────────────────────────────┘
```

**One rule governs everything below.**

> The **declaration** is unified. The **encodings** are derived, and are
> deliberately different per boundary.

A single encoding cannot serve a 0.3 kbps radio and a SQL column without making
both worse. So the same declared type becomes postcard on a device link, JSON at
a web boundary, and columns in a database — three derivations, one declaration.

Adding an encoding is not a violation. **Adding a second declaration is.**

| Layer | Fixed | Free to vary |
|---|---|---|
| Messages | the Rust type | — |
| Serialization | postcard on device links | JSON/SQL/CBOR at other boundaries |
| Framing | this document | — |
| Transport | — | anything that moves bytes |

---

## 2. Frame format

```
 0        1        2        3                  3+N              3+N+4
 ├────────┼────────┼────────┼──────────────────┼────────────────────┤
 │ MAGIC  │ LEN_LO │ LEN_HI │   PAYLOAD (N)    │   CRC32 (4, LE)    │
 └────────┴────────┴────────┴──────────────────┴────────────────────┘
```

| Field | Size | Value |
|---|---|---|
| `MAGIC` | 1 | `0xFD` |
| `LEN` | 2 | payload byte count, **little-endian** |
| `PAYLOAD` | N | 0 … 65 535 bytes |
| `CRC32` | 4 | CRC-32 of `PAYLOAD` only, **little-endian** |

Overhead: **7 bytes**. `encoded_len(n) == n + 7`.

The CRC covers **the payload only** — not `MAGIC`, not `LEN`. A corrupted `LEN`
is caught by the resulting CRC failure, and the decoder resynchronises.

### Encoding

```
out[0]           = 0xFD
out[1]           = len & 0xFF
out[2]           = (len >> 8) & 0xFF
out[3 .. 3+N]    = payload
out[3+N .. +4]   = crc32(payload)   little-endian
```

### Decoding — a five-state machine

```
   ┌─────────┐  byte==0xFD   ┌────────┐              ┌────────┐
   │  MAGIC  │──────────────▶│ LEN_LO │─────────────▶│ LEN_HI │
   └─────────┘               └────────┘              └────────┘
        ▲  │ byte!=0xFD                                   │
        │  └─ skip (resync)                               │
        │                                                 ▼
        │                           len==0        ┌───────────────┐
        │                    ┌────────────────────│  dispatch on  │
        │                    │                    │      len      │
        │                    │                    └───────────────┘
        │                    ▼                       │         │
        │              ┌──────────┐   N bytes        │ len>CAP │ 0<len<=CAP
        │              │   CRC    │◀─────────────────┼─────────┘
        │              │ (4 bytes)│                  │         │
        │              └──────────┘                  ▼         ▼
        │                    │                 (drop, resync) PAYLOAD
        └────────────────────┘
           emit payload if CRC matches, else drop
```

**Required behaviours:**

| Situation | Behaviour |
|---|---|
| Byte that is not `MAGIC` while idle | Skip it. Resynchronisation is silent. |
| `LEN` exceeds the receive buffer | Drop the frame, return to `MAGIC`. Never overflow. |
| CRC mismatch | Drop the frame silently, return to `MAGIC`. |
| Payload containing `0xFD` | Ordinary payload data. Never a frame start mid-payload. |

A decoder must not panic, allocate, or block on any input — including hostile
input. It is expected to run in an interrupt handler.

---

## 3. CRC-32

**CRC-32/ISO-HDLC** — the variant used by zlib, gzip, PNG and Ethernet FCS.

| Parameter | Value |
|---|---|
| Polynomial | `0x04C11DB7` (reflected: `0xEDB88320`) |
| Init | `0xFFFFFFFF` |
| RefIn / RefOut | `true` / `true` |
| XorOut | `0xFFFFFFFF` |
| **Check** (`"123456789"`) | **`0xCBF43926`** |

```rust
crc = 0xFFFFFFFF
for byte in data:
    crc ^= byte
    repeat 8: crc = (crc >> 1) ^ (0xEDB88320 if crc & 1 else 0)
return ~crc
```

**Why a standard variant.** A frame produced here is verifiable by any
conforming implementation in any language, without shipping our polynomial
alongside it. The check value is asserted in every implementation.

**Why not a lookup table.** The wire is the bottleneck on every transport this
runs over — UART at 115 200 baud is 11.5 KB/s; LoRa is orders slower. A 1 KB
table would buy throughput no transport can consume. Footprint and
`const`-evaluability won instead. **Speed was not the selection criterion.**

---

## 4. Version negotiation

`WIRE_VERSION` is a `u8` compiled into every artifact. Endpoints exchange it at
handshake, before application frames.

```rust
assert_compatible(local, remote, min_compatible) -> Result<(), VersionSkewError>
// Ok when remote >= min_compatible
```

Policy lives in [`../compat/matrix.toml`](../compat/matrix.toml):

```toml
[wire]
current        = 2   # what this build speaks
min_compatible = 2   # oldest remote accepted
```

`fid release check` fails CI when `wire.current` and `WIRE_VERSION` disagree, so
neither can move without the other.

| Bump | Effect | Old artifacts |
|---|---|---|
| `--bump breaking` | `current+1`, `min_compatible = current` | **rejected** |
| `--bump compatible` | `current+1`, `min_compatible` unchanged | accepted |

### History

| Version | Change |
|---|---|
| 1 | Initial. `CRC8` = XOR fold. Overhead 4 bytes. |
| **2** | **CRC-32/ISO-HDLC. Overhead 7 bytes.** An XOR fold is a parity byte, not a CRC — blind to reordering, duplication, and even-length bursts; and any 8-bit check over 64 KB passes ~1 in 256 corrupted frames. |

---

## 5. Messages

Frame payloads are **postcard**-serialized Rust types. Postcard is the declared
protocol contract for device links.

Rules that keep the waist stable:

1. **Declare the type once, in Rust.** Every other language's view is generated
   (`ts-rs` → TypeScript today).
2. **Never hand-write a second copy.** The walkie-talkie project declared a wire
   protocol three times by hand; typecheck and integration tests stayed green
   while every client was broken.
3. **A signed message is not postcard-encoded.** See §6.2.

---

## 6. OTA sub-protocol

The first application protocol on the waist, and the proof it works: declared
once in `fiducial-ota`, postcard-serialized, framed here, transport-agnostic.

### 6.1 Messages

| Message | Direction | Purpose |
|---|---|---|
| `Offer { manifest, signature }` | host → device | An image is available |
| `Resume { offset }` | device → host | Send from here (`0` = start) |
| `Chunk { offset, data }` | host → device | Image bytes |
| `Ack { offset }` | device → host | Bytes durably held |
| `Status(OtaStatus)` | device → host | Outcome |

```
host                                    device
 │─── Offer{manifest, signature} ──────▶│  verify sig, version, size, power
 │◀── Resume{offset: 0} ────────────────│
 │─── Chunk{0, ...} ───────────────────▶│
 │◀── Ack{512} ─────────────────────────│
 │            ✂ link drops ✂            │
 │─── Offer{...} (reconnect) ──────────▶│
 │◀── Resume{offset: 512} ──────────────│  ← resumes, does not restart
 │─── Chunk{512, ...} ─────────────────▶│
 │◀── Status(Staged) ───────────────────│  digest matched
```

Chunks are capped at **512 bytes** so a chunk plus its postcard envelope plus
the frame header fits a 1 KiB transport buffer.

### 6.2 Signing

The **manifest** is signed, not the image. The manifest commits to the image via
SHA-256, so verifying 43 bytes transitively authenticates a 400 KB image — and a
bad offer is rejected before a single flash write or radio second is spent.

Signed bytes are **fixed-width big-endian in fixed order**, deliberately *not*
postcard:

```
[0..4)   version     u32 BE
[4..8)   length      u32 BE
[8..40)  digest      SHA-256
[40..42) chunk_size  u16 BE
[42]     cohort      u8
```

Postcard's varint encoding is a *serialization*, not a *canonicalization*. Tying
signature validity to it would let a future encoding optimisation silently
invalidate every deployed public key.

Verification is a trait (`SignatureVerifier`), because the three real
implementations differ: software on a host, hardware PKA on an STM32WLE5, and
neither in a transfer-logic test. The default is `RejectAll` — an unconfigured
device installs **nothing**, rather than anything.

### 6.3 Guarantees

| Property | Mechanism |
|---|---|
| Never brick | A/B slots, trial boot, rollback when the budget expires unconfirmed |
| Booting ≠ healthy | `mark_booted(self_test_passed)` — only a passing self-test confirms |
| Signed, always | No code path reaches `Staged` without a verified signature |
| Resumable | `resume_offset()` after a dropped link |
| Idempotent | A duplicate chunk is **acknowledged**, not rejected — a lost ACK is the ordinary cause, and failing there live-locks a flaky link |
| No downgrade | Offers at or below the running version are refused |
| Power-safe | `PowerPolicy` — a declared fact, not a buried constant |
| Staged rollout | `Cohort` is inside the signed bytes, so a canary image cannot be replayed at the fleet |

---

## 7. Conformance

[`vectors.json`](./vectors.json) is the test suite. It carries, for each case,
the payload, its CRC-32, and the complete frame — all as hex.

Both the Rust crate and the TypeScript port assert against it. So can yours.

```
✔ crc32 matches Rust for "check_string"
✔ encodes byte-for-byte like Rust for "len_256"
✔ decodes the Rust-generated frame for "payload_contains_magic"
```

Cases are chosen for properties, not coverage theatre: the empty payload,
`"ab"` vs `"ba"` (reordering — which the v1 XOR fold could not distinguish), a
payload containing `MAGIC`, all-zeros and all-ones, and lengths straddling the
`LEN` byte boundary.

**Regenerate** (only when the wire format legitimately changes):

```sh
FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-protocol --test vectors
```

CI runs it without that variable, so a stale file fails the build.

### Implementation checklist

- [ ] `crc32("123456789") == 0xCBF43926`
- [ ] `encoded_len(n) == n + 7`
- [ ] Every `vectors.json` frame encodes and decodes byte-for-byte
- [ ] Decoder skips non-`MAGIC` bytes while idle
- [ ] Decoder drops oversized `LEN` without overflowing
- [ ] Decoder drops CRC failures silently and resynchronises
- [ ] Decoder never panics, allocates, or blocks
- [ ] Version exchanged at handshake; `remote < min_compatible` refused

---

## 8. Deliberately not in this protocol

Recorded so they are not re-proposed. Full reasoning:
[`2026-09-10-protocol-crc32-and-serialization-boundaries.md`](../specs/2026-09-10-protocol-crc32-and-serialization-boundaries.md).

| Not included | Why |
|---|---|
| Encryption / Noise | No link needs it. BLE: no declared part has it. LoRaWAN: carries its own session crypto. USB: physical access already required. |
| COSE | ed25519 via `embassy-boot` already owns signing. A second scheme for one job. |
| CBOR / minicbor | Zero non-Rust consumers on any device link. The web boundary is already served by generated TypeScript. |
| Per-frame version byte | Version is negotiated at handshake. A byte per frame is not free on links that cannot spare it. |
| Fragmentation / reassembly | `MAX_PAYLOAD` is 64 KB. No product has needed more. |
| Retransmission / ordering | Transport-dependent. USB and TCP provide it; a product over a lossy radio declares its own. |

Each returns to the table when a real product needs it — not before.

# Protocol: CRC-32, and the serialization / security boundaries

_Date: 2026-09-10 | Wire version 2 | Appended — supersedes nothing by edit_

Outcome of a review of the framing, serialization and security layers. Four
options were evaluated and three were **rejected or deferred**. Recording the
rejections matters more than recording the acceptance: without them the same
proposals return every six months and get re-argued from zero.

---

## 1. The waist — what is fixed, and what is allowed to vary

This is the load-bearing decision; everything below is a consequence of it.

Fiducial's protocol layer is an hourglass. The narrow waist is **one declared
Rust type**. Everything below it (UART, USB, BLE, LoRa) and everything above it
(desktop, browser, simulation, database) is permitted to vary — and *must* be,
because a single encoding cannot serve a 0.3 kbps radio and a SQL column at the
same time without making both worse.

```
                   ONE declared Rust type          ← the waist. Fixed.
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
    postcard             ts-rs → JSON        (future: SQL DDL, CBOR)
    device link          web boundary         when a product needs one
```

**The rule:** the *declaration* is unified; the *encodings* are derived and are
deliberately different. Adding an encoding is not a violation. Adding a second
*declaration* is.

This is the existing rule from the 2026-09-06 design spec (§"Commit to the
contract. Never commit to the tool that satisfies it."), stated in the form the
protocol layer needs it. It is restated here because the review turned up three
separate proposals that would each have added a second declaration, and the
existing wording did not make that consequence obvious enough to stop them.

---

## 2. Accepted: CRC-8 XOR fold → CRC-32/ISO-HDLC

**Wire version 1 → 2. Breaking.**

| | v1 | v2 |
|---|---|---|
| Frame | `MAGIC \| LEN_LO \| LEN_HI \| PAYLOAD \| CRC8` | `MAGIC \| LEN_LO \| LEN_HI \| PAYLOAD \| CRC32(4, LE)` |
| Overhead | 4 bytes | 7 bytes |
| Check | XOR fold (parity byte) | CRC-32/ISO-HDLC |

**Why.** The v1 check was `data.iter().fold(0, ^)` — a parity byte, not a
polynomial CRC. It does not detect byte reordering, byte duplication, or any
even-length burst error. And independent of that weakness, *any* 8-bit check
over a payload of up to 65 535 bytes lets roughly 1 in 256 corrupted frames
through. The v1 doc comment argued the transport's own error detection (USB,
UART FIFO) made this sufficient — defensible for USB serial, not defensible for
a radio link, and the protocol is explicitly transport-agnostic.

**Why this variant.** `CRC-32/ISO-HDLC` (`poly=0x04C11DB7`, `init=0xFFFFFFFF`,
`refin/refout=true`, `xorout=0xFFFFFFFF`) is the variant used by zlib, gzip, PNG
and Ethernet FCS. A frame produced here is verifiable by any conforming
implementation on any platform, in any language, without shipping our
polynomial alongside it. Its standard check value over `b"123456789"` is
`0xCBF43926`, and **that constant is asserted in both the Rust crate and the
TypeScript port** — it is the anchor that detects drift between them.

**Implementation.** Bitwise, table-free, `const fn`. No heap, no static table.
A 1 KB lookup table would buy throughput that no transport here can consume:
UART at 115 200 baud is 11.5 KB/s and LoRa is three orders of magnitude slower.
The wire is the bottleneck, so the codec is optimised for flash footprint and
`const`-evaluability instead — **speed was explicitly not the selection
criterion, and should not become one retroactively.**

**Mirrored, not reimplemented.** `packages/transport-web/src/codec.ts` changed
in the same commit. The two are declared byte-exact; a change to one without the
other is the exact failure this platform exists to delete.

---

## 3. Reaffirmed: postcard remains the sole protocol contract

No change. Recorded because the review re-opened it.

The 2026-09-06 design spec already declares `postcard` as the protocol contract
(§"Each domain has exactly one contract" — `| Protocol | The postcard schema |
any transport |`, and §7: *"A product defines its messages on top with `serde` +
`postcard`"*). That stands.

Products **inherit** this. A product does not choose its message contract; it
uses the platform's. This is stated explicitly so a product-level decision
record cannot silently fork it.

---

## 4. Deferred: minicbor / CBOR — fails the Rule of Two today

**Not rejected on merit. Rejected on timing.**

CBOR with integer field keys is a good fit for a polyglot device link, and if a
non-Rust consumer ever reads a device directly it is the right answer.

There is no such consumer. Zero non-Rust readers exist on any device link today.
The web boundary — the one place a non-Rust consumer actually exists — is already
served by `ts-rs` generating TypeScript from the Rust types (`fiducial-wasm` has
`ts-rs = "10"`; `packages/wasm-bridge/src/generated.ts` exists and is generated).

CLAUDE.md's bar is *"a real product needs it."* Nothing needs it.

**Revisit trigger, stated so it is checkable rather than a matter of opinion:**
a second product, or a Python simulation, reads a device link directly without
going through Rust. Until then, adding it would mean two serialization paths to
keep byte-exact, and serde's derive machinery monomorphises per type — so the
cost is paid on *every message type carried on both paths*, not once.

---

## 5. Rejected: COSE for OTA image signing — redundant

The OTA signing mechanism is **already decided**: the 2026-09-06 design spec
§12.2/§12.3.2 specifies `embassy-boot` with `ed25519` signature verification,
public key baked into the bootloader, private key in the portfolio secret store.

COSE would be a *second* signing scheme for a job that already has one, and the
weaker option here: `embassy-boot`'s ed25519 path is built in, COSE would be
bolted on. COSE's advantage is a standard, self-describing, CBOR-native envelope
— which buys nothing when the verifier is a bootloader answering exactly one
question: *is this signature valid for this image?*

**ed25519 remains the only OTA signing mechanism.**

---

## 6. Rejected: Noise Protocol — no link needs it

`no_std` Noise implementations do exist and are usable (`noise-protocol` with
default features off, static dispatch, no alloc; also `clatter`). The
recommendation did not fail on ecosystem availability. It failed on having no
link to protect:

| Link | Why Noise is not needed |
| --- | --- |
| BLE | No declared part has both LoRa and BLE (design spec §"Pairing note"). No current product has BLE at all. |
| LoRa / LoRaWAN | LoRaWAN carries its own session crypto (NwkSKey / AppSKey). A second encryption layer is redundant. |
| USB serial | Local, physical-access-required, desktop end is trusted. |

Adopting it would mean carrying a handshake, key management and a second crypto
dependency for zero protected links. **Rejected until a link exists that needs
it.**

---

## 7. What this does not decide

- Per-device identity and key provisioning. Distinct from image signing (see the
  Phase 16c rescope, appended same day) and not required by it.
- Any LoRaWAN policy. No LoRaWAN crate exists in this repo — only
  `lora-modulation` re-exporting `Bandwidth`/`CodingRate`/`SpreadingFactor`.
  LoRaWAN decisions belong to a product until a second product needs them
  generalised.
- Whether to add a version byte to the frame header. Wire version is exchanged
  at handshake (see `2026-09-10-release-and-version-skew.md`); putting it in
  every frame costs a byte per frame on links that cannot spare it. Not changed,
  and deliberately not changed as part of a checksum fix.

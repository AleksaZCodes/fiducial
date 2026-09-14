# fiducial-ota

**Firmware updates that cannot brick the device.**

[![crates.io](https://img.shields.io/crates/v/fiducial-ota.svg)](https://crates.io/crates/fiducial-ota)
[![docs.rs](https://docs.rs/fiducial-ota/badge.svg)](https://docs.rs/fiducial-ota)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

A `no_std`, **no-`alloc`** over-the-air update protocol: signed manifests,
resumable transfer, trial boot with automatic rollback, and staged rollout. It is
the first sub-protocol built on the [`fiducial-protocol`](../fiducial-protocol)
waist, carried as postcard-serialized messages inside ordinary frames.

## What makes it testable

Everything here is **pure**. `Receiver` is a state machine that touches no flash,
no radio and no clock. That is the entire design trick: it means every failure
mode below is a host unit test rather than a field incident.

| Owned here | Owned elsewhere |
|---|---|
| Messages, transfer state, resumption, digest | Framing (`fiducial-protocol`) |
| Signature **verification** | Signing (maintainer key store) |
| Deciding an image is complete and authentic | Flash writes, partition swap (`embassy-boot`) |

## The six that bite

Each one is an assertion in this crate, not a comment:

1. **Never brick** — `TrialState` models trial boot, a boot budget, and automatic
   rollback when an image is never confirmed.
2. **Booting ≠ healthy** — `mark_booted(self_test_passed)`. An image that boots
   and then fails its self-test still rolls back.
3. **Signed, always** — no code path reaches `Phase::Staged` without a verified
   signature. The default verifier is `RejectAll`, so an unconfigured device
   installs *nothing* rather than *anything*.
4. **Resumable** — a 1 KB transfer dropped at 384 bytes continues from 384.
5. **Idempotent** — a duplicate chunk is *acknowledged*, not rejected. Rejecting
   there live-locks a flaky link.
6. **No downgrade or replay** — offers at or below the running version are
   refused, and `Cohort` sits inside the signed bytes so a canary image cannot be
   replayed at the whole fleet.

## Why the manifest is 43 bytes

`ImageManifest` commits to the image by SHA-256, so those bytes authenticate a
400 KB image — and a bad offer costs **zero flash writes**.

`signing_bytes()` is a fixed-width big-endian canonical form, deliberately **not**
postcard. Postcard is a serialization, not a canonicalization; tying signatures
to it would let an encoding change silently invalidate every deployed key.

## Install

```sh
cargo add fiducial-ota
# software ed25519 verification:
cargo add fiducial-ota --features ed25519
```

`SignatureVerifier` is a trait with three real implementations — software
ed25519, STM32WLE5 hardware PKA, and a test stub — which is why the transfer
state machine builds with no crypto dependency at all on every target.

## Features

| Feature | Default | Effect |
|---|---|---|
| *(none)* | ✅ | Pure `no_std`, no `alloc`, no crypto. |
| `alloc` | | Owned buffers. |
| `std` | | Implies `alloc`. |
| `ed25519` | | Software signature verification. |

## License

MIT

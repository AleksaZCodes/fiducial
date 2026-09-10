# Phase 16c rescope: OTA transport is USB-first, written transport-agnostically

_Date: 2026-09-10 | Appended — supersedes by record, not by edit_

## Supersession notice

This decision supersedes the OTA **transport** assumption in:

| Document | Line | Superseded text |
|---|---|---|
| `PHASES.md` | Phase 16c row | "Device updates over **BLE**, self-tests, marks booted" |
| `docs/specs/2026-09-06-fiducial-design.md` | §12.3.3 | "Restarting a 400 KB image over **BLE** because of one dropout…" |

Those lines are **not edited** — per CLAUDE.md, decisions are appended and the
history is the record. They remain as written; this document is what governs.

**Everything else in §12.2 and §12.3 stands unchanged**: A/B partitions, trial
boot, automatic rollback, `mark_booted` only after self-test, ed25519 signing,
resumability, the battery threshold, staged rollout, and §12.3.5's rule that
LoRa does not carry firmware. Only the transport changes.

---

## The change

**From:** OTA over BLE.
**To:** OTA over **USB serial** as the v1 transport, with the transfer logic
written transport-agnostically so a future product with BLE can add it as a
second transport rather than a rewrite.

## Why it changed

The BLE assumption was written in the 2026-09-06 design spec without a concrete
product behind it, and it has since become load-bearing for a phase that is next
to execute. Three facts, all already on record in the same spec, contradict it:

1. **No declared part has both LoRa and BLE.** §"Pairing note": *"STM32WL gives
   LoRa without BLE; ESP32 gives BLE and Wi-Fi without LoRa."* §"No single
   declared part does both LoRa and BLE."
2. **LoRa cannot carry firmware.** §12.3.5, explicitly: *"OTA over LoRa is
   impractical for a full image… LoRa carries commands and telemetry; it does
   not carry firmware."*
3. **No current or planned product has BLE hardware.** The first real product's
   bench and service transport is USB.

Together these mean the phase as written **cannot be executed on the hardware
that exists**. Keeping "BLE" in the phase commits the platform to building a
capability that zero products need — which is precisely what MISSION.md
anti-goal 2 forbids: *"The platform must never become the project. Nothing is
added speculatively."*

This is the Rule of Two applied correctly rather than applied once, in the
original spec, and never revisited. The original spec was not wrong to name a
transport; it was wrong to keep naming it after the hardware answer changed.

## What "transport-agnostically" obliges, and what it does not

**Obliges:** the resumable-transfer logic (offset tracking, chunk
acknowledgement, retry, verification) is written against a byte-stream interface,
not against USB APIs. Adding BLE later means implementing that interface, not
rewriting the transfer.

**Does not oblige:** building, testing, or shipping any BLE support now. No BLE
abstraction is speculatively added "so it's ready." One transport is
implemented. The interface exists because the transfer logic needs *some*
boundary anyway, not as a hedge.

The distinction matters: a transport-agnostic interface with one implementation
is good design. A transport-agnostic interface with one implementation *and* a
half-built second one is the anti-goal.

## Revised "done when"

> A device receives a firmware update over USB serial, verifies its ed25519
> signature, boots the new image on trial, runs its self-test, and calls
> `mark_booted`. An image that fails self-test rolls back automatically without
> intervention. The transfer resumes from its offset after an interrupted link.

## Not a prerequisite: per-device identity provisioning

Recorded because a review proposed it as a blocker and it is not one. The two
key problems are distinct:

| | Proves | Keys required |
|---|---|---|
| **Image signing** (Phase 16c) | this *image* is genuine | **one** keypair, held by the maintainer; the private key never goes on a device |
| **Device identity** (not 16c) | this *device* is genuine to a backend | a **per-device** secret, provisioned at manufacture |

Phase 16c needs only the first. The device holds the *public* key, which
requires integrity, not secrecy. **Phase 16c is not blocked on provisioning**;
per-device identity is a backend concern for Phase 17 or a product.

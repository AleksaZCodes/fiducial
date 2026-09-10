//! Over-the-air firmware update protocol — `no_std`.
//!
//! This is the first message-layer protocol built on the Fiducial waist, and it
//! is deliberately shaped to prove that the waist works: every message here is
//! **one declared Rust type**, serialized with `postcard`, carried inside a
//! `fiducial-protocol` frame, over any transport that moves bytes.
//!
//! # What this crate owns, and what it does not
//!
//! | Owned here | Owned elsewhere |
//! |---|---|
//! | Message types, transfer state machine, resumption, digest | Framing ([`fiducial-protocol`]) |
//! | Manifest signature *verification* | Signing (maintainer secret store) |
//! | Deciding an image is complete and authentic | Flash writes, partition swap (`embassy-boot`) |
//!
//! The split matters: everything in this crate is **pure**. It touches no
//! flash, no radio, no clock. That is what makes the whole update path testable
//! on a host, which is the only reason the six failure modes below can be
//! asserted rather than hoped for.
//!
//! # The six that bite
//!
//! From the design spec §12.3, and each one is a test in this crate:
//!
//! 1. **Never brick** — [`TrialState`] models trial boot and automatic
//!    rollback. `mark_booted` is reachable only after a self-test passes.
//! 2. **Signed, always** — [`Receiver`] refuses to reach [`Phase::Staged`]
//!    without a verified signature. There is no code path that stages an
//!    unverified image; the type system enforces it, not a comment.
//! 3. **Resumable** — [`Receiver::resume_offset`] reports how much survived a
//!    dropped link. Transfer continues from there.
//! 4. **Power-safe** — [`PowerPolicy`] refuses to begin below a declared
//!    threshold. A fact, asserted, not a constant buried in firmware.
//! 5. **Bandwidth honesty** — the transport is chosen by the caller. This crate
//!    names none, which is what lets a product declare its own.
//! 6. **Staged rollout** — [`Cohort`] tags an image offer, so a canary cohort
//!    can be served before a fleet.
//!
//! # Idempotency
//!
//! Design spec §12.4: *"every state-mutating action tolerates being called
//! twice."* A re-sent [`OtaMessage::Chunk`] at an already-written offset is
//! **acknowledged, not rejected** — because the common cause is a lost ACK, not
//! a misbehaving host, and failing there is how a transfer live-locks on a
//! flaky link.
//!
//! [`fiducial-protocol`]: https://docs.rs/fiducial-protocol

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

#[cfg(any(test, feature = "std"))]
extern crate std;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Length of an image digest (SHA-256).
pub const DIGEST_LEN: usize = 32;

/// Length of an ed25519 signature.
pub const SIGNATURE_LEN: usize = 64;

/// Length of an ed25519 public key.
pub const PUBLIC_KEY_LEN: usize = 32;

/// Largest chunk payload this protocol permits, in bytes.
///
/// Bounded well below `fiducial_protocol::MAX_PAYLOAD` so a chunk plus its
/// postcard envelope plus the frame header always fits a 1 KiB transport
/// buffer — the smallest a declared transport is expected to provide.
pub const MAX_CHUNK: usize = 512;

// ── Manifest ──────────────────────────────────────────────────────────────────

/// Signed metadata describing a firmware image.
///
/// The manifest — not the image — is what gets signed. It commits to the image
/// through [`ImageManifest::digest`], so verifying 96 bytes of manifest
/// transitively authenticates a 400 KB image, and the device can reject a bad
/// offer **before** spending a single flash write or radio second on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageManifest {
    /// Monotonic image version. A device refuses anything not strictly newer.
    pub version: u32,
    /// Image length in bytes.
    pub length: u32,
    /// SHA-256 of the complete image.
    pub digest: [u8; DIGEST_LEN],
    /// Chunk size the sender intends to use.
    pub chunk_size: u16,
    /// Rollout cohort this image is offered to.
    pub cohort: Cohort,
}

impl ImageManifest {
    /// Canonical byte encoding used as the signed message.
    ///
    /// Fixed-width big-endian fields in a fixed order — deliberately **not**
    /// postcard. A signature must be computed over bytes whose layout can never
    /// shift; postcard's varint encoding is a serialization format, not a
    /// canonicalization, and tying signature validity to it would make a future
    /// encoding optimisation silently invalidate every deployed public key.
    pub fn signing_bytes(&self) -> [u8; 4 + 4 + DIGEST_LEN + 2 + 1] {
        let mut out = [0u8; 4 + 4 + DIGEST_LEN + 2 + 1];
        out[0..4].copy_from_slice(&self.version.to_be_bytes());
        out[4..8].copy_from_slice(&self.length.to_be_bytes());
        out[8..8 + DIGEST_LEN].copy_from_slice(&self.digest);
        out[40..42].copy_from_slice(&self.chunk_size.to_be_bytes());
        out[42] = self.cohort as u8;
        out
    }
}

/// Staged-rollout cohort. Canaries take an image before the fleet does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Cohort {
    /// First cohort — a small group, watched before going wider.
    Canary = 0,
    /// Second cohort — broader, after canary health telemetry holds.
    Broad = 1,
    /// Everyone.
    Fleet = 2,
}

// ── Wire messages ─────────────────────────────────────────────────────────────

/// A message in the OTA sub-protocol.
///
/// Serialized with `postcard`, carried as the payload of a
/// `fiducial-protocol` frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OtaMessage<'a> {
    /// Host → device: an image is available.
    Offer {
        /// Metadata for the offered image.
        manifest: ImageManifest,
        /// ed25519 signature over [`ImageManifest::signing_bytes`].
        #[serde(with = "serde_bytes_64")]
        signature: [u8; SIGNATURE_LEN],
    },
    /// Device → host: send from this offset. `0` means "from the start".
    ///
    /// This is the resumption point. A device that already holds a prefix of
    /// the offered image answers with the length of that prefix.
    Resume {
        /// Byte offset the device wants the next chunk from.
        offset: u32,
    },
    /// Host → device: image bytes at `offset`.
    Chunk {
        /// Offset of the first byte of `data` within the image.
        offset: u32,
        /// Image bytes. At most [`MAX_CHUNK`].
        data: &'a [u8],
    },
    /// Device → host: bytes up to `offset` are durably held.
    Ack {
        /// Total bytes the device now holds.
        offset: u32,
    },
    /// Device → host: terminal outcome.
    Status(OtaStatus),
}

/// Terminal or near-terminal state reported by the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OtaStatus {
    /// Image complete, digest matched, signature verified. Ready to swap.
    Staged,
    /// Offer refused — the device already runs this version or newer.
    NotNewer,
    /// Offer refused — battery below the declared threshold.
    PowerTooLow,
    /// Offer refused — image larger than the receiving slot.
    TooLarge,
    /// Transfer failed — assembled bytes did not match the manifest digest.
    DigestMismatch,
    /// Offer refused — manifest signature did not verify.
    BadSignature,
    /// Transfer failed — a chunk arrived beyond the end of the image, or a gap
    /// was left in the middle.
    BadOffset,
}

// ── Signature verification ────────────────────────────────────────────────────

/// Verifies a detached signature over a message.
///
/// A trait rather than a hard dependency because the three real implementations
/// differ: a host verifies in software, a device with a public-key accelerator
/// (STM32WLE5 PKA) verifies in hardware, and a transfer-logic test needs
/// neither. Feature `ed25519` supplies the software one.
pub trait SignatureVerifier {
    /// Return `true` only if `signature` is a valid signature over `message`.
    fn verify(&self, message: &[u8], signature: &[u8; SIGNATURE_LEN]) -> bool;
}

/// Software ed25519 verification.
#[cfg(feature = "ed25519")]
pub struct Ed25519Verifier {
    key: ed25519_dalek::VerifyingKey,
}

#[cfg(feature = "ed25519")]
impl Ed25519Verifier {
    /// Build a verifier from a 32-byte ed25519 public key.
    ///
    /// Returns `None` if the bytes are not a valid curve point.
    pub fn new(public_key: &[u8; PUBLIC_KEY_LEN]) -> Option<Self> {
        ed25519_dalek::VerifyingKey::from_bytes(public_key)
            .ok()
            .map(|key| Self { key })
    }
}

#[cfg(feature = "ed25519")]
impl SignatureVerifier for Ed25519Verifier {
    fn verify(&self, message: &[u8], signature: &[u8; SIGNATURE_LEN]) -> bool {
        use ed25519_dalek::Verifier as _;
        let sig = ed25519_dalek::Signature::from_bytes(signature);
        self.key.verify(message, &sig).is_ok()
    }
}

/// A verifier that rejects everything.
///
/// The default when no verifier is supplied. It exists so that "forgot to
/// configure a key" fails closed — an unconfigured device installs nothing,
/// rather than installing anything.
pub struct RejectAll;

impl SignatureVerifier for RejectAll {
    fn verify(&self, _message: &[u8], _signature: &[u8; SIGNATURE_LEN]) -> bool {
        false
    }
}

// ── Power policy ──────────────────────────────────────────────────────────────

/// Declared minimum battery state of charge required to begin an update.
///
/// A fact carried in the type, not a constant buried in a flashing routine —
/// so the threshold a product declares is the threshold that is enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerPolicy {
    /// Minimum state of charge, in percent, to begin an update.
    pub min_percent: u8,
}

impl PowerPolicy {
    /// A policy requiring at least `min_percent` state of charge.
    pub const fn new(min_percent: u8) -> Self {
        Self { min_percent }
    }

    /// Whether an update may begin at the given state of charge.
    pub const fn permits(&self, state_of_charge: u8) -> bool {
        state_of_charge >= self.min_percent
    }
}

impl Default for PowerPolicy {
    /// 30% — enough headroom to complete a transfer and a swap on a device
    /// whose radio and flash are its two largest consumers.
    fn default() -> Self {
        Self::new(30)
    }
}

// ── Receiver ──────────────────────────────────────────────────────────────────

/// Where a transfer currently stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No offer accepted.
    Idle,
    /// Offer accepted and verified; receiving chunks.
    Receiving,
    /// All bytes received, digest matched, signature verified.
    Staged,
    /// Transfer ended in a failure. Carries the reason.
    Failed(OtaStatus),
}

/// The device side of an OTA transfer.
///
/// Pure: it decides, it does not act. The caller performs flash writes when
/// [`Receiver::accept_chunk`] returns `Ok(Some(bytes))`, and performs the
/// partition swap when [`Receiver::phase`] reaches [`Phase::Staged`].
///
/// `V` is the signature verifier; `SLOT` is the receiving slot size in bytes,
/// which bounds what this device can accept regardless of what is offered.
pub struct Receiver<V: SignatureVerifier, const SLOT: usize> {
    verifier: V,
    power: PowerPolicy,
    current_version: u32,
    manifest: Option<ImageManifest>,
    offset: u32,
    hasher: Sha256,
    phase: Phase,
}

impl<V: SignatureVerifier, const SLOT: usize> Receiver<V, SLOT> {
    /// Create a receiver for a device currently running `current_version`.
    pub fn new(verifier: V, power: PowerPolicy, current_version: u32) -> Self {
        Self {
            verifier,
            power,
            current_version,
            manifest: None,
            offset: 0,
            hasher: Sha256::new(),
            phase: Phase::Idle,
        }
    }

    /// Current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Byte offset the next chunk should start at.
    ///
    /// This is the answer to [`OtaMessage::Resume`]: after a dropped link, a
    /// host asks, and continues from here rather than from zero.
    pub fn resume_offset(&self) -> u32 {
        self.offset
    }

    /// The accepted manifest, if an offer is in progress.
    pub fn manifest(&self) -> Option<&ImageManifest> {
        self.manifest.as_ref()
    }

    /// Consider an offer.
    ///
    /// Checks, in the order that costs least first: version, size, power, then
    /// signature. Signature verification is last because it is the most
    /// expensive, and there is no reason to spend it on an image this device
    /// was never going to take.
    pub fn offer(
        &mut self,
        manifest: &ImageManifest,
        signature: &[u8; SIGNATURE_LEN],
        state_of_charge: u8,
    ) -> OtaStatus {
        if manifest.version <= self.current_version {
            return self.fail(OtaStatus::NotNewer);
        }
        if manifest.length as usize > SLOT || manifest.chunk_size as usize > MAX_CHUNK {
            return self.fail(OtaStatus::TooLarge);
        }
        if !self.power.permits(state_of_charge) {
            return self.fail(OtaStatus::PowerTooLow);
        }
        if !self.verifier.verify(&manifest.signing_bytes(), signature) {
            return self.fail(OtaStatus::BadSignature);
        }

        self.manifest = Some(*manifest);
        self.offset = 0;
        self.hasher = Sha256::new();
        self.phase = Phase::Receiving;
        OtaStatus::Staged // provisional: accepted, transfer may begin
    }

    /// Feed a chunk.
    ///
    /// Returns `Ok(Some(data))` when `data` should be written to flash at
    /// `offset`, and `Ok(None)` when the chunk was a **duplicate** of bytes
    /// already held — which is acknowledged rather than rejected, because a
    /// lost ACK is the ordinary cause and failing here live-locks the transfer
    /// on a flaky link (design spec §12.4).
    ///
    /// Returns `Err` on a gap or an overrun, both of which are unrecoverable
    /// for this transfer because the running digest cannot be rewound.
    pub fn accept_chunk<'a>(
        &mut self,
        offset: u32,
        data: &'a [u8],
    ) -> Result<Option<&'a [u8]>, OtaStatus> {
        if self.phase != Phase::Receiving {
            return Err(OtaStatus::BadOffset);
        }
        let manifest = match self.manifest {
            Some(m) => m,
            None => return Err(OtaStatus::BadOffset),
        };

        // Duplicate: a chunk fully behind the write head. Idempotent no-op.
        if offset < self.offset {
            let end = offset as u64 + data.len() as u64;
            if end <= self.offset as u64 {
                return Ok(None);
            }
            // Partially-overlapping chunk. The digest is a running hash and
            // cannot absorb a partial rewrite, so this is not recoverable.
            self.phase = Phase::Failed(OtaStatus::BadOffset);
            return Err(OtaStatus::BadOffset);
        }

        // Gap: the sender skipped bytes we never received.
        if offset > self.offset {
            self.phase = Phase::Failed(OtaStatus::BadOffset);
            return Err(OtaStatus::BadOffset);
        }

        // Overrun past the declared image length.
        let end = self.offset as u64 + data.len() as u64;
        if end > manifest.length as u64 || data.len() > MAX_CHUNK {
            self.phase = Phase::Failed(OtaStatus::BadOffset);
            return Err(OtaStatus::BadOffset);
        }

        self.hasher.update(data);
        self.offset = end as u32;
        Ok(Some(data))
    }

    /// Finish the transfer: check length and digest.
    ///
    /// Reaching [`Phase::Staged`] requires that the signature verified at
    /// [`Receiver::offer`] **and** the assembled bytes hash to the digest that
    /// signature committed to. There is no other path to `Staged`.
    pub fn finish(&mut self) -> OtaStatus {
        let manifest = match self.manifest {
            Some(m) => m,
            None => return self.fail(OtaStatus::BadOffset),
        };
        if self.phase != Phase::Receiving || self.offset != manifest.length {
            return self.fail(OtaStatus::BadOffset);
        }

        let digest: [u8; DIGEST_LEN] = self.hasher.clone().finalize().into();
        if digest != manifest.digest {
            return self.fail(OtaStatus::DigestMismatch);
        }

        self.phase = Phase::Staged;
        OtaStatus::Staged
    }

    fn fail(&mut self, status: OtaStatus) -> OtaStatus {
        self.phase = Phase::Failed(status);
        status
    }
}

// ── Trial boot ────────────────────────────────────────────────────────────────

/// Outcome of a trial boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootOutcome {
    /// Image confirmed. The new slot is now the durable one.
    Confirmed,
    /// Self-test failed, or the boot budget was exhausted. Roll back.
    RollBack,
    /// Still on trial — more boots remain in the budget.
    StillTrying,
}

/// Trial-boot bookkeeping: the "never brick" half of OTA.
///
/// A staged image boots on trial. It becomes durable **only** when its own
/// self-test passes and [`TrialState::mark_booted`] is called. Merely booting
/// is not evidence of health — a device that boots and then fails to talk to
/// its radio is bricked in every sense that matters to a user.
///
/// If the budget is exhausted without confirmation, the bootloader reverts to
/// the previous slot. That revert is `embassy-boot`'s job; this type is the
/// declaration of *when* it should happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialState {
    budget: u8,
    attempts: u8,
    confirmed: bool,
}

impl TrialState {
    /// Begin a trial allowing `budget` boots to confirm.
    pub const fn new(budget: u8) -> Self {
        Self {
            budget,
            attempts: 0,
            confirmed: false,
        }
    }

    /// Whether the image has been confirmed healthy.
    pub const fn is_confirmed(&self) -> bool {
        self.confirmed
    }

    /// Record a boot attempt and report what the bootloader should do.
    pub fn record_boot(&mut self) -> BootOutcome {
        if self.confirmed {
            return BootOutcome::Confirmed;
        }
        self.attempts = self.attempts.saturating_add(1);
        if self.attempts >= self.budget {
            BootOutcome::RollBack
        } else {
            BootOutcome::StillTrying
        }
    }

    /// Confirm the image after its self-test passed.
    ///
    /// `self_test_passed` is taken as an argument rather than assumed, so that
    /// the only way to reach [`BootOutcome::Confirmed`] is to have actually run
    /// a self-test and passed it.
    pub fn mark_booted(&mut self, self_test_passed: bool) -> BootOutcome {
        if self_test_passed {
            self.confirmed = true;
            BootOutcome::Confirmed
        } else {
            BootOutcome::RollBack
        }
    }
}

// ── serde helper: fixed 64-byte array ─────────────────────────────────────────

/// `[u8; 64]` has no `Serialize`/`Deserialize` in serde's stock impls, and a
/// `Vec<u8>` would need `alloc`. Encoding it as a fixed tuple keeps the crate
/// `no_std`-and-no-`alloc` on every target.
mod serde_bytes_64 {
    use super::SIGNATURE_LEN;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; SIGNATURE_LEN], s: S) -> Result<S::Ok, S::Error> {
        // Two 32-byte halves: serde implements the traits for [u8; 32].
        let (a, b) = bytes.split_at(32);
        let a: [u8; 32] = a.try_into().unwrap_or([0u8; 32]);
        let b: [u8; 32] = b.try_into().unwrap_or([0u8; 32]);
        (a, b).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; SIGNATURE_LEN], D::Error> {
        let (a, b): ([u8; 32], [u8; 32]) = Deserialize::deserialize(d)?;
        let mut out = [0u8; SIGNATURE_LEN];
        out[..32].copy_from_slice(&a);
        out[32..].copy_from_slice(&b);
        Ok(out)
    }
}

//! OTA transfer tests — one per failure mode in design spec §12.3.
//!
//! These run on the host with no hardware, which is the point: the update path
//! is pure logic, so "never brick" and "signed, always" are assertions rather
//! than hopes. The flash writes and the partition swap are the caller's job and
//! are the only parts that need a device.

use fiducial_ota::*;
use sha2::{Digest, Sha256};

// ── Helpers ───────────────────────────────────────────────────────────────────

const SLOT: usize = 4096;

/// A verifier that accepts everything — for tests about *transfer*, not crypto.
struct AcceptAll;
impl SignatureVerifier for AcceptAll {
    fn verify(&self, _m: &[u8], _s: &[u8; SIGNATURE_LEN]) -> bool {
        true
    }
}

fn digest_of(data: &[u8]) -> [u8; DIGEST_LEN] {
    Sha256::digest(data).into()
}

fn manifest_for(image: &[u8], version: u32) -> ImageManifest {
    ImageManifest {
        version,
        length: image.len() as u32,
        digest: digest_of(image),
        chunk_size: 64,
        cohort: Cohort::Canary,
    }
}

fn receiver<V: SignatureVerifier>(v: V, running: u32) -> Receiver<V, SLOT> {
    Receiver::new(v, PowerPolicy::default(), running)
}

/// Drive a complete transfer in `chunk` sized pieces.
fn transfer<V: SignatureVerifier>(
    rx: &mut Receiver<V, SLOT>,
    image: &[u8],
    chunk: usize,
) -> Result<(), OtaStatus> {
    let mut off = 0usize;
    while off < image.len() {
        let end = (off + chunk).min(image.len());
        rx.accept_chunk(off as u32, &image[off..end])?;
        off = end;
    }
    Ok(())
}

// ── 1. Never brick: trial boot and rollback ───────────────────────────────────

#[test]
fn unconfirmed_image_rolls_back_when_budget_exhausted() {
    let mut trial = TrialState::new(3);
    assert_eq!(trial.record_boot(), BootOutcome::StillTrying);
    assert_eq!(trial.record_boot(), BootOutcome::StillTrying);
    // Third boot exhausts the budget with no confirmation — revert.
    assert_eq!(trial.record_boot(), BootOutcome::RollBack);
    assert!(!trial.is_confirmed());
}

#[test]
fn booting_is_not_enough_only_a_passing_self_test_confirms() {
    let mut trial = TrialState::new(3);
    trial.record_boot();
    // Booted fine, but the self-test failed: roll back anyway.
    assert_eq!(trial.mark_booted(false), BootOutcome::RollBack);
    assert!(!trial.is_confirmed());
}

#[test]
fn passing_self_test_confirms_and_is_durable() {
    let mut trial = TrialState::new(3);
    trial.record_boot();
    assert_eq!(trial.mark_booted(true), BootOutcome::Confirmed);
    assert!(trial.is_confirmed());
    // Later boots stay confirmed — the budget no longer applies.
    assert_eq!(trial.record_boot(), BootOutcome::Confirmed);
    assert_eq!(trial.record_boot(), BootOutcome::Confirmed);
}

// ── 2. Signed, always ─────────────────────────────────────────────────────────

#[test]
fn bad_signature_is_refused_before_any_transfer() {
    let image = [0xABu8; 512];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(RejectAll, 1);

    assert_eq!(
        rx.offer(&m, &[0u8; SIGNATURE_LEN], 100),
        OtaStatus::BadSignature
    );
    assert_eq!(rx.phase(), Phase::Failed(OtaStatus::BadSignature));
    // And no chunk is accepted afterwards.
    assert!(rx.accept_chunk(0, &image[..64]).is_err());
}

#[test]
fn default_posture_fails_closed() {
    // An unconfigured device (RejectAll) installs nothing. "Forgot to set a
    // key" must not mean "accepts anything".
    let image = [0x11u8; 128];
    let m = manifest_for(&image, 9);
    let mut rx = receiver(RejectAll, 0);
    assert_eq!(
        rx.offer(&m, &[0xFFu8; SIGNATURE_LEN], 100),
        OtaStatus::BadSignature
    );
}

#[test]
fn staged_is_unreachable_without_a_verified_signature() {
    let image = [0x5Au8; 256];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(RejectAll, 1);
    rx.offer(&m, &[0u8; SIGNATURE_LEN], 100);
    // Even a complete, digest-correct transfer cannot reach Staged.
    let _ = transfer(&mut rx, &image, 64);
    assert_ne!(rx.finish(), OtaStatus::Staged);
    assert_ne!(rx.phase(), Phase::Staged);
}

// ── 3. Resumable ──────────────────────────────────────────────────────────────

#[test]
fn transfer_resumes_from_offset_after_a_dropped_link() {
    let image: Vec<u8> = (0..1024u32).map(|i| (i % 251) as u8).collect();
    let m = manifest_for(&image, 2);
    let mut rx = receiver(AcceptAll, 1);
    rx.offer(&m, &[0u8; SIGNATURE_LEN], 100);

    // Send 384 bytes, then the link drops.
    for off in (0..384).step_by(64) {
        rx.accept_chunk(off as u32, &image[off..off + 64]).unwrap();
    }
    assert_eq!(rx.resume_offset(), 384);

    // Host reconnects, asks, and continues from there — not from zero.
    let resume = rx.resume_offset() as usize;
    let mut off = resume;
    while off < image.len() {
        let end = (off + 64).min(image.len());
        rx.accept_chunk(off as u32, &image[off..end]).unwrap();
        off = end;
    }

    assert_eq!(rx.finish(), OtaStatus::Staged);
    assert_eq!(rx.phase(), Phase::Staged);
}

#[test]
fn duplicate_chunk_is_acknowledged_not_rejected() {
    // §12.4: every state-mutating action tolerates being called twice. A lost
    // ACK makes the host resend; failing here would live-lock the transfer.
    let image = [0x77u8; 256];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(AcceptAll, 1);
    rx.offer(&m, &[0u8; SIGNATURE_LEN], 100);

    rx.accept_chunk(0, &image[0..64]).unwrap();
    rx.accept_chunk(64, &image[64..128]).unwrap();
    assert_eq!(rx.resume_offset(), 128);

    // Resend both — accepted as no-ops, offset unchanged, digest unpolluted.
    assert_eq!(rx.accept_chunk(0, &image[0..64]), Ok(None));
    assert_eq!(rx.accept_chunk(64, &image[64..128]), Ok(None));
    assert_eq!(rx.resume_offset(), 128);

    rx.accept_chunk(128, &image[128..192]).unwrap();
    rx.accept_chunk(192, &image[192..256]).unwrap();
    assert_eq!(rx.finish(), OtaStatus::Staged);
}

#[test]
fn a_gap_in_the_stream_is_rejected() {
    let image = [0x33u8; 256];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(AcceptAll, 1);
    rx.offer(&m, &[0u8; SIGNATURE_LEN], 100);

    rx.accept_chunk(0, &image[0..64]).unwrap();
    // Skip 64..128 entirely — a running digest cannot absorb a hole.
    assert_eq!(
        rx.accept_chunk(128, &image[128..192]),
        Err(OtaStatus::BadOffset)
    );
    assert_eq!(rx.phase(), Phase::Failed(OtaStatus::BadOffset));
}

#[test]
fn overrun_past_declared_length_is_rejected() {
    let image = [0x44u8; 128];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(AcceptAll, 1);
    rx.offer(&m, &[0u8; SIGNATURE_LEN], 100);

    rx.accept_chunk(0, &image).unwrap();
    assert_eq!(
        rx.accept_chunk(128, &[0xFFu8; 16]),
        Err(OtaStatus::BadOffset)
    );
}

// ── 4. Power-safe ─────────────────────────────────────────────────────────────

#[test]
fn update_refused_below_declared_battery_threshold() {
    let image = [0x22u8; 256];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(AcceptAll, 1);
    // Default policy is 30%; offer at 12%.
    assert_eq!(
        rx.offer(&m, &[0u8; SIGNATURE_LEN], 12),
        OtaStatus::PowerTooLow
    );
}

#[test]
fn threshold_is_the_declared_one_not_a_hardcoded_one() {
    let image = [0x22u8; 256];
    let m = manifest_for(&image, 2);

    // A product declaring 80% gets 80% enforced.
    let mut strict: Receiver<_, SLOT> = Receiver::new(AcceptAll, PowerPolicy::new(80), 1);
    assert_eq!(
        strict.offer(&m, &[0u8; SIGNATURE_LEN], 50),
        OtaStatus::PowerTooLow
    );

    // A product declaring 10% gets 10%.
    let mut lax: Receiver<_, SLOT> = Receiver::new(AcceptAll, PowerPolicy::new(10), 1);
    assert_ne!(
        lax.offer(&m, &[0u8; SIGNATURE_LEN], 50),
        OtaStatus::PowerTooLow
    );
}

// ── Digest, version, size ─────────────────────────────────────────────────────

#[test]
fn digest_mismatch_fails_even_when_length_is_right() {
    let image = [0x01u8; 256];
    let mut m = manifest_for(&image, 2);
    m.digest[0] ^= 0xFF; // manifest commits to a different image

    let mut rx = receiver(AcceptAll, 1);
    rx.offer(&m, &[0u8; SIGNATURE_LEN], 100);
    transfer(&mut rx, &image, 64).unwrap();

    assert_eq!(rx.finish(), OtaStatus::DigestMismatch);
    assert_eq!(rx.phase(), Phase::Failed(OtaStatus::DigestMismatch));
}

#[test]
fn finish_before_all_bytes_arrive_is_rejected() {
    let image = [0x09u8; 256];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(AcceptAll, 1);
    rx.offer(&m, &[0u8; SIGNATURE_LEN], 100);
    rx.accept_chunk(0, &image[0..64]).unwrap();
    assert_eq!(rx.finish(), OtaStatus::BadOffset);
}

#[test]
fn downgrade_and_replay_are_refused() {
    let image = [0x02u8; 128];
    let mut rx = receiver(AcceptAll, 7);
    // Older
    assert_eq!(
        rx.offer(&manifest_for(&image, 6), &[0u8; SIGNATURE_LEN], 100),
        OtaStatus::NotNewer
    );
    // Same version replayed
    assert_eq!(
        rx.offer(&manifest_for(&image, 7), &[0u8; SIGNATURE_LEN], 100),
        OtaStatus::NotNewer
    );
}

#[test]
fn image_larger_than_the_slot_is_refused() {
    let image = vec![0u8; SLOT + 1];
    let m = manifest_for(&image, 2);
    let mut rx = receiver(AcceptAll, 1);
    assert_eq!(
        rx.offer(&m, &[0u8; SIGNATURE_LEN], 100),
        OtaStatus::TooLarge
    );
}

// ── 6. Staged rollout ─────────────────────────────────────────────────────────

#[test]
fn cohort_travels_with_the_manifest_and_is_signed_over() {
    let image = [0x08u8; 64];
    let canary = ImageManifest {
        cohort: Cohort::Canary,
        ..manifest_for(&image, 2)
    };
    let fleet = ImageManifest {
        cohort: Cohort::Fleet,
        ..manifest_for(&image, 2)
    };
    // Changing the cohort changes the signed bytes, so a canary image cannot
    // be replayed at the fleet with the same signature.
    assert_ne!(canary.signing_bytes(), fleet.signing_bytes());
}

// ── Wire round-trip ───────────────────────────────────────────────────────────

#[test]
fn messages_round_trip_through_postcard() {
    let image = [0x5Cu8; 96];
    let m = manifest_for(&image, 3);
    let msgs = [
        OtaMessage::Offer {
            manifest: m,
            signature: [0xA5u8; SIGNATURE_LEN],
        },
        OtaMessage::Resume { offset: 384 },
        OtaMessage::Ack { offset: 512 },
        OtaMessage::Status(OtaStatus::Staged),
    ];
    for msg in &msgs {
        let bytes = postcard::to_allocvec(msg).unwrap();
        let back: OtaMessage = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(&back, msg);
    }
}

#[test]
fn a_chunk_fits_inside_one_protocol_frame() {
    // MAX_CHUNK plus postcard envelope plus frame overhead must fit the
    // smallest transport buffer a product is expected to provide (1 KiB).
    let data = [0u8; MAX_CHUNK];
    let msg = OtaMessage::Chunk {
        offset: 0xDEAD,
        data: &data,
    };
    let encoded = postcard::to_allocvec(&msg).unwrap();
    let framed = fiducial_protocol::encoded_len(encoded.len());
    assert!(
        framed <= 1024,
        "chunk frame is {framed} bytes, over the 1 KiB budget"
    );
}

// ── Signing-bytes canonicalisation ────────────────────────────────────────────

#[test]
fn signing_bytes_are_fixed_width_and_order_stable() {
    let image = [0u8; 16];
    let m = manifest_for(&image, 0x01020304);
    let b = m.signing_bytes();
    assert_eq!(b.len(), 43);
    // Big-endian version, first four bytes — a layout that cannot drift with a
    // serialization change.
    assert_eq!(&b[0..4], &[0x01, 0x02, 0x03, 0x04]);
    assert_eq!(&b[8..40], &m.digest);
}

#[test]
fn any_manifest_field_change_changes_the_signed_bytes() {
    let image = [0u8; 32];
    let base = manifest_for(&image, 2);
    let variants = [
        ImageManifest { version: 3, ..base },
        ImageManifest { length: 33, ..base },
        ImageManifest {
            chunk_size: 128,
            ..base
        },
        ImageManifest {
            cohort: Cohort::Broad,
            ..base
        },
    ];
    for v in &variants {
        assert_ne!(v.signing_bytes(), base.signing_bytes());
    }
}

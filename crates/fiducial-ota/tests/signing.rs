//! Real ed25519 signing and verification — not a stub verifier.
//!
//! The transfer tests use `AcceptAll`/`RejectAll` to isolate transfer logic.
//! This file closes the loop with actual crypto: a manifest signed by a real
//! key verifies, and every tampering of it fails.

#![cfg(feature = "ed25519")]

use ed25519_dalek::{Signer, SigningKey};
use fiducial_ota::*;
use sha2::{Digest, Sha256};

const SLOT: usize = 4096;

/// Deterministic test key. A real deployment's private key lives in the
/// maintainer secret store and never appears in a repository.
fn test_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn signed(m: &ImageManifest) -> [u8; SIGNATURE_LEN] {
    test_key().sign(&m.signing_bytes()).to_bytes()
}

fn manifest_for(image: &[u8], version: u32) -> ImageManifest {
    ImageManifest {
        version,
        length: image.len() as u32,
        digest: Sha256::digest(image).into(),
        chunk_size: 64,
        cohort: Cohort::Canary,
    }
}

fn verifier() -> Ed25519Verifier {
    Ed25519Verifier::new(&test_key().verifying_key().to_bytes()).expect("valid key")
}

#[test]
fn genuine_signature_verifies_and_image_stages() {
    let image: Vec<u8> = (0..512u32).map(|i| (i % 251) as u8).collect();
    let m = manifest_for(&image, 2);
    let sig = signed(&m);

    let mut rx: Receiver<_, SLOT> = Receiver::new(verifier(), PowerPolicy::default(), 1);
    assert_ne!(rx.offer(&m, &sig, 100), OtaStatus::BadSignature);
    assert_eq!(rx.phase(), Phase::Receiving);

    let mut off = 0usize;
    while off < image.len() {
        let end = (off + 64).min(image.len());
        rx.accept_chunk(off as u32, &image[off..end]).unwrap();
        off = end;
    }
    assert_eq!(rx.finish(), OtaStatus::Staged);
}

#[test]
fn signature_from_the_wrong_key_is_refused() {
    let image = [0x31u8; 256];
    let m = manifest_for(&image, 2);
    let attacker = SigningKey::from_bytes(&[9u8; 32]);
    let sig = attacker.sign(&m.signing_bytes()).to_bytes();

    let mut rx: Receiver<_, SLOT> = Receiver::new(verifier(), PowerPolicy::default(), 1);
    assert_eq!(rx.offer(&m, &sig, 100), OtaStatus::BadSignature);
}

#[test]
fn tampering_with_any_signed_field_invalidates_the_signature() {
    let image = [0x62u8; 256];
    let m = manifest_for(&image, 2);
    let sig = signed(&m);

    // Each of these is covered by signing_bytes, so each must break the sig.
    let tampered = [
        ImageManifest { version: 3, ..m },
        ImageManifest { length: 257, ..m },
        ImageManifest {
            chunk_size: 128,
            ..m
        },
        ImageManifest {
            cohort: Cohort::Fleet,
            ..m
        },
        ImageManifest {
            digest: {
                let mut d = m.digest;
                d[0] ^= 0xFF;
                d
            },
            ..m
        },
    ];

    for t in &tampered {
        let mut rx: Receiver<_, SLOT> = Receiver::new(verifier(), PowerPolicy::default(), 1);
        assert_eq!(
            rx.offer(t, &sig, 100),
            OtaStatus::BadSignature,
            "tampered manifest accepted: {t:?}"
        );
    }
}

#[test]
fn a_flipped_bit_in_the_signature_is_refused() {
    let image = [0x8Fu8; 128];
    let m = manifest_for(&image, 2);
    let mut sig = signed(&m);
    sig[0] ^= 0x01;

    let mut rx: Receiver<_, SLOT> = Receiver::new(verifier(), PowerPolicy::default(), 1);
    assert_eq!(rx.offer(&m, &sig, 100), OtaStatus::BadSignature);
}

#[test]
fn a_genuine_signature_does_not_authorise_a_different_image() {
    // The manifest commits to the image via its digest, so a valid signature
    // over manifest A cannot be replayed to install image B.
    let image_a = [0xAAu8; 256];
    let image_b = [0xBBu8; 256];
    let m_a = manifest_for(&image_a, 2);
    let sig_a = signed(&m_a);

    let mut rx: Receiver<_, SLOT> = Receiver::new(verifier(), PowerPolicy::default(), 1);
    rx.offer(&m_a, &sig_a, 100);
    // Feed image B against manifest A: length matches, digest will not.
    let mut off = 0usize;
    while off < image_b.len() {
        let end = (off + 64).min(image_b.len());
        rx.accept_chunk(off as u32, &image_b[off..end]).unwrap();
        off = end;
    }
    assert_eq!(rx.finish(), OtaStatus::DigestMismatch);
    assert_ne!(rx.phase(), Phase::Staged);
}

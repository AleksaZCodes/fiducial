//! Token issuance — how a principal *proves* it is that principal.
//!
//! `can()` answers "may they". It assumes the caller already knows *who is
//! asking*. For a user that is the `auth` adapter's job. For a device or a
//! service there was nothing: `docs/specs/2026-09-15-identity-at-every-level.md`
//! named it and stopped, because "how a device proves it is that device needs
//! its own pass with real cryptographic choices."
//!
//! # The choices, and why
//!
//! **Ed25519.** Already in this workspace for signed OTA manifests, already
//! understood by the firmware that has to verify offline, small keys and small
//! signatures. Deterministic — no per-signature entropy, which matters on a
//! microcontroller where the RNG is the least trustworthy peripheral.
//!
//! **A fixed-layout binary token, not JWT.** A JWT is JSON, base64 and a
//! header that names its own algorithm. The header is the problem: `alg: none`
//! and algorithm-confusion attacks exist because the token gets to say how it
//! should be checked. Here the layout is fixed, the algorithm is not in the
//! token, and the verifier decides. It is also 96 bytes rather than several
//! hundred, which is the difference between fitting in a radio frame and not.
//!
//! **The signature covers a domain separator.** A token and an OTA manifest are
//! both ed25519 signatures by the same platform key over some bytes. Without a
//! separator, a blob that is valid as one may be replayed as the other.
//!
//! **Expiry is mandatory, not optional.** A token with no expiry is a password
//! that cannot be changed. Unlike a [`Grant`](crate::Grant), where permanence
//! is a legitimate choice an owner makes, a bearer credential that never
//! expires has no legitimate use here.
//!
//! # What this is not
//!
//! Not a session, not a refresh flow, and not a revocation list. A token is
//! short-lived and that is its whole revocation story: the window is the
//! lifetime. See the spec for why a revocation list was rejected rather than
//! forgotten.

use crate::{Principal, Timestamp};
use fiducial_core::DeviceId;

/// Bytes of an ed25519 signature.
pub const SIGNATURE_LEN: usize = 64;
/// Bytes of an ed25519 public key.
pub const PUBLIC_KEY_LEN: usize = 32;

/// The signed part of a token, in bytes.
///
/// 1 kind + 16 id + 8 issued + 8 expires + 8 nonce = 41.
pub const CLAIMS_LEN: usize = 41;

/// A complete token: the claims, then the signature over them.
pub const TOKEN_LEN: usize = CLAIMS_LEN + SIGNATURE_LEN;

/// Domain separator, prefixed to the signed bytes.
///
/// A token and an OTA manifest are both ed25519 signatures by the platform key
/// over some bytes. Without this, a blob valid as one could be presented as the
/// other, and the verifier would be right to accept it.
const DOMAIN: &[u8] = b"fiducial-identity-token-v1";

/// Why a token was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenError {
    /// Not `TOKEN_LEN` bytes.
    MalformedLength,
    /// The principal kind byte is not one this version defines.
    UnknownPrincipalKind,
    /// The signature does not verify against the expected key.
    BadSignature,
    /// `now` is at or after the expiry.
    Expired,
    /// `now` is before the issue time — a clock disagreement large enough
    /// that nothing about the token can be trusted.
    NotYetValid,
    /// The verifier has no clock, so the expiry cannot be honoured.
    NoClock,
    /// The token is for a different principal than the one being checked.
    WrongPrincipal,
}

/// What a token asserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Claims {
    /// Who the bearer is. Never `Anonymous` — there is nothing to assert.
    pub principal: Principal,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    /// Issuer-chosen, unique per token.
    ///
    /// Two tokens for the same principal over the same second are otherwise
    /// byte-identical, which makes a captured token indistinguishable from a
    /// fresh one in any log that records it.
    pub nonce: u64,
}

impl Claims {
    /// The canonical bytes that get signed.
    ///
    /// Fixed-width, big-endian, no length prefixes and no optional fields, so
    /// one claims value has exactly one encoding. A format where two encodings
    /// mean the same thing is a format where a signature can be moved between
    /// them.
    pub fn encode(&self) -> Option<[u8; CLAIMS_LEN]> {
        let mut out = [0u8; CLAIMS_LEN];
        let (kind, id): (u8, [u8; 16]) = match self.principal {
            // Anonymous is refused rather than encoded: a token asserting
            // "nobody" is a token that can only be misused.
            Principal::Anonymous => return None,
            Principal::User(u) => (1, *u.as_bytes()),
            Principal::Device(d) => {
                let mut id = [0u8; 16];
                id[..8].copy_from_slice(d.as_bytes());
                (2, id)
            }
            Principal::Service(s) => {
                let mut id = [0u8; 16];
                id[..8].copy_from_slice(s.as_bytes());
                (3, id)
            }
        };
        out[0] = kind;
        out[1..17].copy_from_slice(&id);
        out[17..25].copy_from_slice(&self.issued_at.0.to_be_bytes());
        out[25..33].copy_from_slice(&self.expires_at.0.to_be_bytes());
        out[33..41].copy_from_slice(&self.nonce.to_be_bytes());
        Some(out)
    }

    /// Read claims back out of their canonical bytes.
    pub fn decode(bytes: &[u8; CLAIMS_LEN]) -> Result<Self, TokenError> {
        let mut id16 = [0u8; 16];
        id16.copy_from_slice(&bytes[1..17]);
        let mut id8 = [0u8; 8];
        id8.copy_from_slice(&bytes[1..9]);

        let principal = match bytes[0] {
            1 => Principal::User(crate::UserId::new(id16)),
            2 => Principal::Device(DeviceId::new(id8)),
            3 => Principal::Service(crate::ServiceId::new(id8)),
            _ => return Err(TokenError::UnknownPrincipalKind),
        };

        let u64_at = |o: usize| {
            let mut b = [0u8; 8];
            b.copy_from_slice(&bytes[o..o + 8]);
            u64::from_be_bytes(b)
        };
        Ok(Self {
            principal,
            issued_at: Timestamp(u64_at(17)),
            expires_at: Timestamp(u64_at(25)),
            nonce: u64_at(33),
        })
    }
}

/// Verify an ed25519 signature.
///
/// A trait rather than a concrete type, mirroring `fiducial_ota`: a device with
/// a hardware crypto accelerator implements this against that, and never links
/// a software implementation it would not use.
pub trait SignatureVerifier {
    /// `true` only if `signature` is valid over `message` for this key.
    fn verify(&self, message: &[u8], signature: &[u8; SIGNATURE_LEN]) -> bool;
}

/// Produce an ed25519 signature. Held by the issuer only.
pub trait Signer {
    fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LEN];
}

/// The bytes that are actually signed: domain separator, then claims.
///
/// Returned as a fixed buffer so this works with no allocator.
fn signing_input(claims: &[u8; CLAIMS_LEN]) -> ([u8; DOMAIN.len() + CLAIMS_LEN], usize) {
    let mut buf = [0u8; DOMAIN.len() + CLAIMS_LEN];
    buf[..DOMAIN.len()].copy_from_slice(DOMAIN);
    buf[DOMAIN.len()..].copy_from_slice(claims);
    let len = buf.len();
    (buf, len)
}

/// Issue a token for `claims`.
///
/// Returns `None` when the claims cannot be encoded — an `Anonymous` principal,
/// or an expiry at or before the issue time, which would be a token born dead.
pub fn issue<S: Signer>(signer: &S, claims: &Claims) -> Option<[u8; TOKEN_LEN]> {
    if claims.expires_at <= claims.issued_at {
        return None;
    }
    let encoded = claims.encode()?;
    let (input, len) = signing_input(&encoded);
    let signature = signer.sign(&input[..len]);

    let mut token = [0u8; TOKEN_LEN];
    token[..CLAIMS_LEN].copy_from_slice(&encoded);
    token[CLAIMS_LEN..].copy_from_slice(&signature);
    Some(token)
}

/// Verify a token and return what it asserts.
///
/// **Fails closed without a clock**, exactly as [`crate::Grant::is_live`] does:
/// a verifier that cannot tell the time cannot honour an expiry, and a bearer
/// credential whose expiry is unenforceable is a permanent one.
pub fn verify<V: SignatureVerifier>(
    verifier: &V,
    token: &[u8],
    now: Option<Timestamp>,
) -> Result<Claims, TokenError> {
    if token.len() != TOKEN_LEN {
        return Err(TokenError::MalformedLength);
    }
    let mut encoded = [0u8; CLAIMS_LEN];
    encoded.copy_from_slice(&token[..CLAIMS_LEN]);
    let mut signature = [0u8; SIGNATURE_LEN];
    signature.copy_from_slice(&token[CLAIMS_LEN..]);

    // Signature first, then claims. Reading claims out of an unverified token
    // and acting on them — even to report a better error — is how a parser
    // becomes the attack surface.
    let (input, len) = signing_input(&encoded);
    if !verifier.verify(&input[..len], &signature) {
        return Err(TokenError::BadSignature);
    }

    let claims = Claims::decode(&encoded)?;
    let Some(now) = now else {
        return Err(TokenError::NoClock);
    };
    if now >= claims.expires_at {
        return Err(TokenError::Expired);
    }
    if now < claims.issued_at {
        return Err(TokenError::NotYetValid);
    }
    Ok(claims)
}

/// Verify a token and require it to be for `expected`.
///
/// The check a caller almost always wants and would otherwise write itself.
/// Forgetting it means accepting any validly-signed token as any principal —
/// the token is authentic and the binding to *who is asking* is missing.
pub fn verify_as<V: SignatureVerifier>(
    verifier: &V,
    token: &[u8],
    expected: &Principal,
    now: Option<Timestamp>,
) -> Result<Claims, TokenError> {
    let claims = verify(verifier, token, now)?;
    if claims.principal != *expected {
        return Err(TokenError::WrongPrincipal);
    }
    Ok(claims)
}

// ── Software ed25519 ─────────────────────────────────────────────────────────

/// Software ed25519 verification.
#[cfg(feature = "ed25519")]
pub struct Ed25519Verifier {
    key: ed25519_dalek::VerifyingKey,
}

#[cfg(feature = "ed25519")]
impl Ed25519Verifier {
    /// Build a verifier from a 32-byte public key, or `None` if the bytes are
    /// not a valid curve point.
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
        self.key
            .verify(message, &ed25519_dalek::Signature::from_bytes(signature))
            .is_ok()
    }
}

/// Software ed25519 signing. The platform holds this; a device never does.
#[cfg(feature = "ed25519")]
pub struct Ed25519Signer {
    key: ed25519_dalek::SigningKey,
}

#[cfg(feature = "ed25519")]
impl Ed25519Signer {
    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        Self {
            key: ed25519_dalek::SigningKey::from_bytes(secret),
        }
    }

    pub fn public_key(&self) -> [u8; PUBLIC_KEY_LEN] {
        self.key.verifying_key().to_bytes()
    }
}

#[cfg(feature = "ed25519")]
impl Signer for Ed25519Signer {
    fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LEN] {
        use ed25519_dalek::Signer as _;
        self.key.sign(message).to_bytes()
    }
}

#[cfg(all(test, feature = "ed25519"))]
mod tests {
    use super::*;
    use crate::UserId;

    const NOW: Timestamp = Timestamp(1_000_000);
    const LATER: Timestamp = Timestamp(1_060_000);

    fn signer() -> Ed25519Signer {
        Ed25519Signer::from_bytes(&[7u8; 32])
    }

    fn verifier(s: &Ed25519Signer) -> Ed25519Verifier {
        Ed25519Verifier::new(&s.public_key()).expect("valid key")
    }

    fn device_claims() -> Claims {
        Claims {
            principal: Principal::Device(DeviceId::new([1, 2, 3, 4, 5, 6, 7, 8])),
            issued_at: NOW,
            expires_at: LATER,
            nonce: 42,
        }
    }

    #[test]
    fn a_device_token_round_trips() {
        let s = signer();
        let token = issue(&s, &device_claims()).expect("issued");
        let got = verify(&verifier(&s), &token, Some(NOW)).expect("verified");
        assert_eq!(got, device_claims());
    }

    #[test]
    fn a_token_signed_by_another_key_is_refused() {
        let token = issue(&signer(), &device_claims()).unwrap();
        let other = Ed25519Signer::from_bytes(&[9u8; 32]);
        assert_eq!(
            verify(&verifier(&other), &token, Some(NOW)),
            Err(TokenError::BadSignature)
        );
    }

    #[test]
    fn any_tampering_invalidates_the_signature() {
        let s = signer();
        let token = issue(&s, &device_claims()).unwrap();
        // Every signed byte, one at a time — a signature covering only part of
        // the claims would pass most tests and fail this one.
        for i in 0..CLAIMS_LEN {
            let mut bad = token;
            bad[i] ^= 0x01;
            assert!(
                verify(&verifier(&s), &bad, Some(NOW)).is_err(),
                "byte {i} is not covered by the signature"
            );
        }
    }

    #[test]
    fn an_expired_token_is_refused() {
        let s = signer();
        let token = issue(&s, &device_claims()).unwrap();
        assert_eq!(
            verify(&verifier(&s), &token, Some(Timestamp(LATER.0 + 1))),
            Err(TokenError::Expired)
        );
        // And exactly at the expiry: a token is valid *until* it expires.
        assert_eq!(
            verify(&verifier(&s), &token, Some(LATER)),
            Err(TokenError::Expired)
        );
    }

    #[test]
    fn a_verifier_with_no_clock_refuses() {
        // Same rule as a grant's expiry. A bearer credential whose expiry is
        // unenforceable is a permanent one.
        let s = signer();
        let token = issue(&s, &device_claims()).unwrap();
        assert_eq!(
            verify(&verifier(&s), &token, None),
            Err(TokenError::NoClock)
        );
    }

    #[test]
    fn a_token_from_the_future_is_refused() {
        let s = signer();
        let token = issue(&s, &device_claims()).unwrap();
        assert_eq!(
            verify(&verifier(&s), &token, Some(Timestamp(NOW.0 - 1))),
            Err(TokenError::NotYetValid)
        );
    }

    #[test]
    fn a_token_that_expires_before_it_is_issued_is_never_created() {
        let mut claims = device_claims();
        claims.expires_at = claims.issued_at;
        assert!(issue(&signer(), &claims).is_none());
    }

    #[test]
    fn anonymous_cannot_be_asserted() {
        let claims = Claims {
            principal: Principal::Anonymous,
            issued_at: NOW,
            expires_at: LATER,
            nonce: 1,
        };
        assert!(issue(&signer(), &claims).is_none());
    }

    #[test]
    fn a_valid_token_for_another_principal_is_refused_by_verify_as() {
        // Without this check a caller accepts any authentic token as any
        // principal: the signature is real and the binding to who is asking
        // is simply missing.
        let s = signer();
        let token = issue(&s, &device_claims()).unwrap();
        let someone_else = Principal::User(UserId::new([3; 16]));
        assert_eq!(
            verify_as(&verifier(&s), &token, &someone_else, Some(NOW)),
            Err(TokenError::WrongPrincipal)
        );
        assert!(verify_as(&verifier(&s), &token, &device_claims().principal, Some(NOW)).is_ok());
    }

    #[test]
    fn a_truncated_token_is_rejected_before_anything_is_parsed() {
        let s = signer();
        let token = issue(&s, &device_claims()).unwrap();
        assert_eq!(
            verify(&verifier(&s), &token[..TOKEN_LEN - 1], Some(NOW)),
            Err(TokenError::MalformedLength)
        );
    }

    #[test]
    fn user_device_and_service_tokens_are_distinct() {
        // The kind byte is part of the signed claims, so a device token cannot
        // be re-read as a service token with the same id bytes.
        let s = signer();
        let id = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut dev = device_claims();
        dev.principal = Principal::Device(DeviceId::new(id));
        let mut svc = device_claims();
        svc.principal = Principal::Service(crate::ServiceId::new(id));

        let dev_token = issue(&s, &dev).unwrap();
        let svc_token = issue(&s, &svc).unwrap();
        assert_ne!(dev_token, svc_token);

        let got = verify(&verifier(&s), &svc_token, Some(NOW)).unwrap();
        assert_eq!(got.principal, svc.principal);
    }

    #[test]
    fn the_domain_separator_stops_a_signature_being_reused() {
        // A signature over the bare claims — what another subsystem signing the
        // same bytes would produce — must not verify as a token.
        use ed25519_dalek::Signer as _;
        let s = signer();
        let encoded = device_claims().encode().unwrap();
        let raw = s.key.sign(&encoded).to_bytes();

        let mut forged = [0u8; TOKEN_LEN];
        forged[..CLAIMS_LEN].copy_from_slice(&encoded);
        forged[CLAIMS_LEN..].copy_from_slice(&raw);

        assert_eq!(
            verify(&verifier(&s), &forged, Some(NOW)),
            Err(TokenError::BadSignature)
        );
    }

    #[test]
    fn claims_encode_to_exactly_one_representation() {
        let c = device_claims();
        assert_eq!(c.encode().unwrap(), c.encode().unwrap());
        assert_eq!(Claims::decode(&c.encode().unwrap()).unwrap(), c);
    }
}

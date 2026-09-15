//! Token conformance vectors — the format, pinned across two implementations.
//!
//! `token.rs` verifies tokens on a device; `packages/identity/src/token.ts`
//! verifies the same tokens in a Worker. Two implementations of one wire format
//! is the drift this platform exists to prevent, and a token format is a
//! particularly bad place for it: a divergence does not look like a bug, it
//! looks like either an outage or a bypass.
//!
//! So the tokens are generated here, with a fixed key and fixed timestamps, and
//! replayed by the TypeScript suite.
//!
//! Regenerate with:
//!
//! ```sh
//! FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity --features ed25519 --test token_vectors
//! ```

#![cfg(feature = "ed25519")]

use fiducial_core::DeviceId;
use fiducial_identity::token::{issue, Claims, Ed25519Signer, TOKEN_LEN};
use fiducial_identity::{Principal, ServiceId, Timestamp, UserId};

const VECTORS_PATH: &str = "../../docs/identity/token-vectors.json";

/// A fixed key. Test material, and it is in the vectors file by design — a
/// verifier needs the public half, and the private half is what makes the
/// vectors reproducible on another machine.
const SECRET: [u8; 32] = [7u8; 32];

const ISSUED: u64 = 1_789_516_800_000;
const EXPIRES: u64 = 1_789_516_800_000 + 3_600_000;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

struct Case {
    name: &'static str,
    why: &'static str,
    claims: Claims,
    /// `now` the verifier is given, and what it must conclude.
    checks: Vec<(Option<u64>, &'static str)>,
}

fn cases() -> Vec<Case> {
    let device = Principal::Device(DeviceId::new([0xda; 8]));
    let service = Principal::Service(ServiceId::new([0x5c; 8]));
    let user = Principal::User(UserId::new([0x0a; 16]));

    let claims = |principal, nonce| Claims {
        principal,
        issued_at: Timestamp(ISSUED),
        expires_at: Timestamp(EXPIRES),
        nonce,
    };

    // Midway through the lifetime, plus each boundary. A token is valid *until*
    // it expires, and "until" is the half of an interval implementations
    // disagree about.
    let lifecycle = || {
        vec![
            (Some(ISSUED), "ok"),
            (Some(ISSUED + 1), "ok"),
            (Some(EXPIRES - 1), "ok"),
            (Some(EXPIRES), "expired"),
            (Some(EXPIRES + 1), "expired"),
            (Some(ISSUED - 1), "not_yet_valid"),
            (None, "no_clock"),
        ]
    };

    vec![
        Case {
            name: "device_token",
            why: "a device proving it is that device, offline",
            claims: claims(device, 1),
            checks: lifecycle(),
        },
        Case {
            name: "service_token",
            why: "a backend service; same id bytes as a device would be a \
                  different token, because the kind is signed",
            claims: claims(service, 2),
            checks: lifecycle(),
        },
        Case {
            name: "user_token",
            why: "a 16-byte user id, where device and service ids are 8",
            claims: claims(user, 3),
            checks: lifecycle(),
        },
        Case {
            name: "same_principal_different_nonce",
            why: "two tokens for one principal over one second must not be \
                  byte-identical, or a captured token is indistinguishable \
                  from a fresh one",
            claims: claims(device, 999),
            checks: vec![(Some(ISSUED), "ok")],
        },
    ]
}

fn render() -> String {
    let signer = Ed25519Signer::from_bytes(&SECRET);

    let cases: Vec<serde_json::Value> = cases()
        .iter()
        .map(|c| {
            let token = issue(&signer, &c.claims).expect("issuable");
            assert_eq!(token.len(), TOKEN_LEN);
            serde_json::json!({
                "name": c.name,
                "why": c.why,
                "token": hex(&token),
                "claims": {
                    "principal": match c.claims.principal {
                        Principal::Anonymous => serde_json::json!({ "kind": "anonymous" }),
                        Principal::User(u) =>
                            serde_json::json!({ "kind": "user", "id": hex(u.as_bytes()) }),
                        Principal::Device(d) =>
                            serde_json::json!({ "kind": "device", "id": hex(d.as_bytes()) }),
                        Principal::Service(s) =>
                            serde_json::json!({ "kind": "service", "id": hex(s.as_bytes()) }),
                    },
                    "issued_at": c.claims.issued_at.0,
                    "expires_at": c.claims.expires_at.0,
                    "nonce": c.claims.nonce.to_string(),
                },
                "checks": c.checks.iter().map(|(now, expected)| serde_json::json!({
                    "now": now,
                    "expect": expected,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    let doc = serde_json::json!({
        "note": "Generated from crates/fiducial-identity. Do not edit by hand — run \
                 FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity \
                 --features ed25519 --test token_vectors",
        "format": {
            "domain": "fiducial-identity-token-v1",
            "claims_len": 41,
            "signature_len": 64,
            "token_len": TOKEN_LEN,
            "layout": "kind:u8 | id:16 | issued_at:u64be | expires_at:u64be | nonce:u64be | sig:64",
            "principal_kinds": { "user": 1, "device": 2, "service": 3 },
        },
        "public_key": hex(&signer.public_key()),
        "cases": cases,
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&doc).expect("serialize")
    )
}

#[test]
fn token_vectors_are_fresh() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(VECTORS_PATH);
    let rendered = render();

    if std::env::var("FIDUCIAL_WRITE_VECTORS").is_ok() {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, &rendered).expect("write vectors");
        eprintln!("wrote {}", path.display());
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        committed, rendered,
        "\ndocs/identity/token-vectors.json is out of date with the token format.\n\
         A format change that skips regeneration is one the TypeScript verifier \
         never learns about.\n\
         Run FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity \
         --features ed25519 --test token_vectors\n"
    );
}

#[test]
fn every_token_is_distinct() {
    // The nonce is what makes this true. Without it, two tokens for one
    // principal over one second are the same bytes.
    let signer = Ed25519Signer::from_bytes(&SECRET);
    let mut seen: Vec<[u8; TOKEN_LEN]> = Vec::new();
    for c in cases() {
        let token = issue(&signer, &c.claims).expect("issuable");
        assert!(!seen.contains(&token), "duplicate token for `{}`", c.name);
        seen.push(token);
    }
}

//! Generates and verifies `docs/protocol/vectors.json` — the conformance
//! vectors for the Fiducial wire protocol.
//!
//! # Why this file exists
//!
//! The Rust crate and the TypeScript port are declared byte-exact. "Declared"
//! is not "checked": two hand-written implementations of one spec drift, and
//! the drift stays silent until a device stops talking to a browser. That is
//! precisely the failure the walkie-talkie project hit — a network stack
//! written twice, in two languages, that diverged.
//!
//! So the vectors are a **third thing both implementations answer to**. They
//! are generated from the Rust implementation, committed, and asserted against
//! by both test suites. Any change that alters the wire either updates this
//! file — visibly, in review — or fails CI.
//!
//! They are also a conformance suite for a *third-party* implementation.
//! Anyone writing a Fiducial codec in C, Python or Go checks it against this
//! file without reading a line of Rust. That is what makes the waist portable
//! rather than merely documented.
//!
//! # Regenerating
//!
//! ```sh
//! FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-protocol --test vectors
//! ```
//!
//! CI runs it *without* that variable, so the committed file must already
//! match — the same freshness contract as every other derived artifact.

use fiducial_protocol::{
    crc32, encode, encoded_len, FrameDecoder, MAGIC, MAX_PAYLOAD, WIRE_VERSION,
};
use std::{fmt::Write as _, fs, path::PathBuf};

fn vectors_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs/protocol/vectors.json")
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// The vector corpus. Each case is here for a property, not for coverage
/// theatre — the comment says which.
fn cases() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        // Boundary: a zero-length payload still carries a full trailer.
        ("empty", vec![]),
        // Smallest non-trivial payload.
        ("single_byte", vec![0x42]),
        // Reordering sensitivity: "ab" and "ba" must produce different CRCs.
        // The v1 XOR fold could not tell these apart.
        ("ab", b"ab".to_vec()),
        ("ba", b"ba".to_vec()),
        // The CRC-32/ISO-HDLC standard check string.
        ("check_string", b"123456789".to_vec()),
        // A payload containing MAGIC, which the decoder must not mistake for a
        // frame start while mid-payload.
        ("payload_contains_magic", vec![MAGIC, MAGIC, 0x00, MAGIC]),
        // All-zero and all-ones: inputs whose own reorderings an XOR fold
        // cannot distinguish.
        ("zeros_16", vec![0x00; 16]),
        ("ones_16", vec![0xFF; 16]),
        // Length field crossing the low byte — catches endianness errors.
        ("len_255", vec![0xA5; 255]),
        ("len_256", vec![0x5A; 256]),
        // Non-repeating multi-byte pattern.
        ("counting_64", (0..64u16).map(|i| (i % 251) as u8).collect()),
    ]
}

fn render() -> String {
    let mut s = String::new();
    s.push_str("{\n");
    let _ = writeln!(s, "  \"wire_version\": {WIRE_VERSION},");
    let _ = writeln!(s, "  \"magic\": \"0x{MAGIC:02x}\",");
    let _ = writeln!(s, "  \"max_payload\": {MAX_PAYLOAD},");
    s.push_str("  \"frame_overhead\": 7,\n");
    s.push_str(
        "  \"frame_layout\": \"MAGIC(1) | LEN_LO(1) | LEN_HI(1) | PAYLOAD(N) | CRC32(4, LE)\",\n",
    );
    s.push_str("  \"crc\": {\n");
    s.push_str("    \"algorithm\": \"CRC-32/ISO-HDLC\",\n");
    s.push_str("    \"poly\": \"0x04C11DB7\",\n");
    s.push_str("    \"init\": \"0xFFFFFFFF\",\n");
    s.push_str("    \"refin\": true,\n");
    s.push_str("    \"refout\": true,\n");
    s.push_str("    \"xorout\": \"0xFFFFFFFF\",\n");
    s.push_str("    \"check\": \"0xCBF43926\"\n");
    s.push_str("  },\n");
    s.push_str("  \"vectors\": [\n");

    let cases = cases();
    for (i, (name, payload)) in cases.iter().enumerate() {
        let mut buf = vec![0u8; encoded_len(payload.len())];
        let n = encode(payload, &mut buf).expect("encode");
        buf.truncate(n);
        s.push_str("    {\n");
        let _ = writeln!(s, "      \"name\": \"{name}\",");
        let _ = writeln!(s, "      \"payload\": \"{}\",", hex(payload));
        let _ = writeln!(s, "      \"payload_len\": {},", payload.len());
        let _ = writeln!(s, "      \"crc32\": \"0x{:08X}\",", crc32(payload));
        let _ = writeln!(s, "      \"frame\": \"{}\"", hex(&buf));
        s.push_str(if i + 1 == cases.len() {
            "    }\n"
        } else {
            "    },\n"
        });
    }
    s.push_str("  ]\n}\n");
    s
}

#[test]
fn vectors_are_fresh() {
    let path = vectors_path();
    let rendered = render();

    if std::env::var("FIDUCIAL_WRITE_VECTORS").is_ok() {
        fs::create_dir_all(path.parent().unwrap()).expect("mkdir docs/protocol");
        fs::write(&path, &rendered).expect("write vectors.json");
        eprintln!("wrote {}", path.display());
        return;
    }

    let committed = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing.\nRegenerate with:\n  \
             FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-protocol --test vectors",
            path.display()
        )
    });

    assert_eq!(
        committed, rendered,
        "\ndocs/protocol/vectors.json is stale — the wire format changed but the \
         committed vectors did not.\nRegenerate with:\n  \
         FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-protocol --test vectors\n"
    );
}

/// Every case round-trips through the *decoder*.
///
/// Generating vectors from the encoder and then asserting them against the
/// encoder would be circular. Closing the loop through the decoder means the
/// committed file pins a round-trip, not one direction.
#[test]
fn every_vector_round_trips_through_the_decoder() {
    for (name, payload) in cases() {
        let mut buf = vec![0u8; encoded_len(payload.len())];
        let n = encode(&payload, &mut buf).expect("encode");
        let mut dec: FrameDecoder<512> = FrameDecoder::new();
        let mut got = None;
        for &b in &buf[..n] {
            if let Some(p) = dec.feed(b) {
                got = Some(p.to_vec());
            }
        }
        assert_eq!(got.as_deref(), Some(payload.as_slice()), "vector {name}");
    }
}

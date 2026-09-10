//! Transport-agnostic protocol framing — `no_std`.
//!
//! # Frame format
//!
//! ```text
//! [ MAGIC(1) | LEN_LO(1) | LEN_HI(1) | PAYLOAD(N) | CRC32(4) ]
//! ```
//!
//! - `MAGIC` = `0xFD` — identifies a Fiducial frame
//! - `LEN` = payload byte count, 2-byte little-endian
//! - `CRC32` = CRC-32/ISO-HDLC over the payload, 4-byte little-endian
//!
//! Total overhead: 7 bytes per frame. Maximum payload: 65 535 bytes.
//!
//! # Why CRC-32
//!
//! Wire version 1 used an 8-bit XOR fold. Over a payload of up to 65 535 bytes
//! that is not adequate: an XOR fold is a parity byte, not a polynomial CRC —
//! it misses byte reordering, byte duplication, and any even-length burst — and
//! even a *true* 8-bit CRC lets roughly 1 in 256 corrupted frames through.
//!
//! `CRC-32/ISO-HDLC` is the variant used by zlib, gzip, PNG and Ethernet FCS,
//! so a frame can be verified against any other implementation on any platform.
//! Its check value over `b"123456789"` is `0xCBF4_3926`, asserted in the tests
//! here and mirrored in the TypeScript port.
//!
//! # Usage
//!
//! **Encoding** into a caller-provided buffer:
//!
//! ```rust
//! use fiducial_protocol::{encode, encoded_len, MAX_PAYLOAD};
//!
//! let payload = b"hello";
//! let mut buf = [0u8; 64];
//! let n = encode(payload, &mut buf).unwrap();
//! assert_eq!(n, encoded_len(payload.len()));
//! ```
//!
//! **Decoding** byte-by-byte (for a serial ISR or async read loop):
//!
//! ```rust
//! use fiducial_protocol::FrameDecoder;
//!
//! let mut dec: FrameDecoder<256> = FrameDecoder::new();
//! let frame_bytes = {
//!     use fiducial_protocol::encode;
//!     let mut buf = [0u8; 32];
//!     let n = encode(b"hi", &mut buf).unwrap();
//!     buf[..n].to_vec()
//! };
//! for &b in &frame_bytes {
//!     if let Some(payload) = dec.feed(b) {
//!         assert_eq!(payload, b"hi");
//!     }
//! }
//! ```

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

#[cfg(test)]
extern crate std;

#[cfg(test)]
use std::vec;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Frame magic byte — every Fiducial frame starts with this.
pub const MAGIC: u8 = 0xFD;

/// Maximum payload length in bytes (u16::MAX).
pub const MAX_PAYLOAD: usize = u16::MAX as usize;

/// Wire protocol version — a monotonically increasing integer embedded in every
/// artifact (firmware, desktop, WASM) that speaks this protocol.
///
/// The version is exchanged during the connection handshake and checked against
/// the compatibility matrix before any application frames are sent.
///
/// This constant is the *only* declaration of the wire version in the Rust
/// workspace. The committed `docs/compat/matrix.toml` records the same number
/// plus the range of older versions still accepted — `fid release check`
/// enforces that they agree.
pub const WIRE_VERSION: u8 = 2;

// ── Version-skew ──────────────────────────────────────────────────────────────

/// Error returned when two endpoints cannot interoperate due to a protocol
/// version mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionSkewError {
    /// The wire version this endpoint speaks.
    pub local: u8,
    /// The wire version the remote endpoint advertised.
    pub remote: u8,
    /// The oldest remote version this endpoint will accept.
    pub min_compatible: u8,
}

impl core::fmt::Display for VersionSkewError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "protocol version skew: local={} remote={} (min acceptable={})",
            self.local, self.remote, self.min_compatible
        )
    }
}

// `core::error::Error` was stabilised in Rust 1.81 (our MSRV is 1.82).
impl core::error::Error for VersionSkewError {}

/// Check whether a remote endpoint speaking `remote` can interoperate with a
/// local endpoint speaking `local` that accepts versions down to `min_compatible`.
///
/// Returns `Ok(())` when `remote >= min_compatible`, `Err` otherwise.
///
/// # Example
///
/// ```rust
/// use fiducial_protocol::{WIRE_VERSION, assert_compatible};
///
/// // Same version always works.
/// assert!(assert_compatible(WIRE_VERSION, WIRE_VERSION, WIRE_VERSION).is_ok());
///
/// // A remote on version 1 is rejected when the minimum is 2.
/// assert!(assert_compatible(2, 1, 2).is_err());
/// ```
pub fn assert_compatible(
    local: u8,
    remote: u8,
    min_compatible: u8,
) -> Result<(), VersionSkewError> {
    if remote >= min_compatible {
        Ok(())
    } else {
        Err(VersionSkewError {
            local,
            remote,
            min_compatible,
        })
    }
}

/// Convenience form of [`assert_compatible`] using `WIRE_VERSION` as `local` and
/// `min_compatible`.
///
/// This matches the policy recorded in `docs/compat/matrix.toml` when
/// `min_compatible = current`, i.e. no backward compatibility is declared.
///
/// For backward-compatible bumps, call [`assert_compatible`] directly with the
/// `min_compatible` value from the matrix.
pub fn is_current_compatible(remote: u8) -> Result<(), VersionSkewError> {
    assert_compatible(WIRE_VERSION, remote, WIRE_VERSION)
}

// ── CRC-32 ────────────────────────────────────────────────────────────────────

/// Reflected polynomial for CRC-32/ISO-HDLC (`0x04C1_1DB7` reflected).
const CRC32_POLY: u32 = 0xEDB8_8320;

/// Compute the CRC-32/ISO-HDLC of a byte slice.
///
/// Parameters: `poly=0x04C11DB7`, `init=0xFFFFFFFF`, `refin=true`,
/// `refout=true`, `xorout=0xFFFFFFFF`. This is the variant used by zlib, gzip,
/// PNG and Ethernet — `crc32(b"123456789") == 0xCBF4_3926`.
///
/// Bitwise and table-free: no heap, no static table, `const`-evaluable. The
/// wire is the bottleneck on every link this protocol runs over (UART at
/// 115200 baud is 11.5 KB/s; LoRa is slower still), so a 1 KB lookup table
/// would buy throughput that no transport here can use.
pub const fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    let mut i = 0;
    while i < data.len() {
        crc ^= data[i] as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ CRC32_POLY
            } else {
                crc >> 1
            };
            bit += 1;
        }
        i += 1;
    }
    !crc
}

// ── Encoding ──────────────────────────────────────────────────────────────────

/// Error returned by [`encode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// The output buffer is too small.
    BufferTooSmall,
    /// The payload exceeds [`MAX_PAYLOAD`] bytes.
    PayloadTooLarge,
}

/// Number of bytes produced by [`encode`] for the given payload length.
#[inline]
pub const fn encoded_len(payload_len: usize) -> usize {
    payload_len + 7 // magic + len_lo + len_hi + crc32(4)
}

/// Encode `payload` as a framed message into `out`.
///
/// Returns the number of bytes written, or an [`EncodeError`].
pub fn encode(payload: &[u8], out: &mut [u8]) -> Result<usize, EncodeError> {
    if payload.len() > MAX_PAYLOAD {
        return Err(EncodeError::PayloadTooLarge);
    }
    let total = encoded_len(payload.len());
    if out.len() < total {
        return Err(EncodeError::BufferTooSmall);
    }
    let len = payload.len() as u16;
    out[0] = MAGIC;
    out[1] = (len & 0xFF) as u8;
    out[2] = (len >> 8) as u8;
    out[3..3 + payload.len()].copy_from_slice(payload);
    let crc = crc32(payload).to_le_bytes();
    out[3 + payload.len()..3 + payload.len() + 4].copy_from_slice(&crc);
    Ok(total)
}

// ── Decoding ──────────────────────────────────────────────────────────────────

/// Error returned when a decoded frame fails its CRC check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcError {
    /// CRC computed over the received payload.
    pub computed: u32,
    /// CRC received in the frame trailer.
    pub received: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodeState {
    Magic,
    LenLo,
    LenHi {
        len_lo: u8,
    },
    Payload {
        remaining: u16,
    },
    /// Accumulating the 4-byte little-endian CRC trailer.
    Crc {
        got: u8,
        acc: u32,
    },
}

/// Byte-by-byte frame decoder with a fixed-size internal buffer.
///
/// `N` is the maximum payload size in bytes. Feed one byte at a time via
/// [`FrameDecoder::feed`]; it returns a slice of the decoded payload when a
/// complete, valid frame has arrived.
///
/// Designed for use in an interrupt handler or an async read loop — no heap,
/// no panics on malformed input.
pub struct FrameDecoder<const N: usize> {
    buf: [u8; N],
    pos: usize,
    expected: u16,
    state: DecodeState,
}

impl<const N: usize> FrameDecoder<N> {
    /// Create a new decoder in the initial state.
    pub const fn new() -> Self {
        Self {
            buf: [0u8; N],
            pos: 0,
            expected: 0,
            state: DecodeState::Magic,
        }
    }

    /// Reset to the initial state, discarding any partially accumulated frame.
    pub fn reset(&mut self) {
        self.pos = 0;
        self.expected = 0;
        self.state = DecodeState::Magic;
    }

    /// Feed one byte into the decoder.
    ///
    /// Returns `Some(&[u8])` containing the decoded payload when a complete,
    /// CRC-valid frame has been received. Returns `None` otherwise — including
    /// on CRC failure (the frame is silently dropped and the decoder resets).
    ///
    /// Bytes with the wrong magic are silently skipped (resynchronisation).
    pub fn feed(&mut self, byte: u8) -> Option<&[u8]> {
        match self.state {
            DecodeState::Magic => {
                if byte == MAGIC {
                    self.state = DecodeState::LenLo;
                }
                None
            }
            DecodeState::LenLo => {
                self.state = DecodeState::LenHi { len_lo: byte };
                None
            }
            DecodeState::LenHi { len_lo } => {
                let len = u16::from_le_bytes([len_lo, byte]);
                self.pos = 0;
                self.expected = len;
                if len == 0 {
                    self.state = DecodeState::Crc { got: 0, acc: 0 };
                } else if len as usize > N {
                    self.state = DecodeState::Magic;
                } else {
                    self.state = DecodeState::Payload { remaining: len };
                }
                None
            }
            DecodeState::Payload { remaining } => {
                if self.pos < N {
                    self.buf[self.pos] = byte;
                    self.pos += 1;
                }
                if remaining == 1 {
                    self.state = DecodeState::Crc { got: 0, acc: 0 };
                } else {
                    self.state = DecodeState::Payload {
                        remaining: remaining - 1,
                    };
                }
                None
            }
            DecodeState::Crc { got, acc } => {
                // Little-endian: first byte received is the least significant.
                let acc = acc | ((byte as u32) << (8 * got as u32));
                let got = got + 1;
                if got < 4 {
                    self.state = DecodeState::Crc { got, acc };
                    return None;
                }
                let payload = &self.buf[..self.expected as usize];
                let computed = crc32(payload);
                self.state = DecodeState::Magic;
                if computed == acc {
                    Some(payload)
                } else {
                    None
                }
            }
        }
    }
}

impl<const N: usize> Default for FrameDecoder<N> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn encode_to_vec(payload: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; encoded_len(payload.len())];
        let n = encode(payload, &mut buf).unwrap();
        buf[..n].to_vec()
    }

    fn decode_all<const N: usize>(bytes: &[u8]) -> Option<Vec<u8>> {
        let mut dec: FrameDecoder<N> = FrameDecoder::new();
        for &b in bytes {
            if let Some(payload) = dec.feed(b) {
                return Some(payload.to_vec());
            }
        }
        None
    }

    #[test]
    fn crc32_standard_check_value() {
        // The CRC-32/ISO-HDLC check value. Any conforming implementation on any
        // platform produces this for b"123456789" — this is what makes the frame
        // verifiable outside Rust, and what the TypeScript port asserts too.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn crc32_empty_is_zero() {
        assert_eq!(crc32(&[]), 0);
    }

    #[test]
    fn crc32_detects_byte_reorder() {
        // The property the old XOR fold did not have: order matters.
        assert_ne!(crc32(b"ab"), crc32(b"ba"));
    }

    #[test]
    fn crc32_is_const_evaluable() {
        const CHECK: u32 = crc32(b"123456789");
        assert_eq!(CHECK, 0xCBF4_3926);
    }

    #[test]
    fn encode_empty_payload() {
        let bytes = encode_to_vec(&[]);
        assert_eq!(bytes.len(), 7);
        assert_eq!(bytes[0], MAGIC);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 0);
        assert_eq!(&bytes[3..7], &[0, 0, 0, 0]); // crc32([]) = 0
    }

    #[test]
    fn encode_known_payload() {
        let payload = b"hi";
        let bytes = encode_to_vec(payload);
        assert_eq!(bytes[0], MAGIC);
        assert_eq!(bytes[1], 2);
        assert_eq!(bytes[2], 0);
        assert_eq!(bytes[3], b'h');
        assert_eq!(bytes[4], b'i');
        assert_eq!(&bytes[5..9], &crc32(payload).to_le_bytes());
    }

    #[test]
    fn roundtrip_short_payload() {
        let payload = b"hello";
        let bytes = encode_to_vec(payload);
        let decoded = decode_all::<64>(&bytes).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn roundtrip_empty_payload() {
        let bytes = encode_to_vec(&[]);
        let decoded = decode_all::<64>(&bytes).unwrap();
        assert_eq!(decoded, &[] as &[u8]);
    }

    #[test]
    fn crc_failure_drops_frame() {
        let mut bytes = encode_to_vec(b"test");
        *bytes.last_mut().unwrap() ^= 0xFF;
        assert!(decode_all::<64>(&bytes).is_none());
    }

    #[test]
    fn decoder_resyncs_after_noise() {
        let mut input = vec![0x00, 0x12, 0x34];
        input.extend_from_slice(&encode_to_vec(b"sync"));
        let decoded = decode_all::<64>(&input).unwrap();
        assert_eq!(decoded, b"sync");
    }

    #[test]
    fn encode_buffer_too_small() {
        let payload = b"hello";
        let mut buf = [0u8; 2];
        assert_eq!(encode(payload, &mut buf), Err(EncodeError::BufferTooSmall));
    }

    #[test]
    fn encoded_len_formula() {
        assert_eq!(encoded_len(0), 7);
        assert_eq!(encoded_len(10), 17);
        assert_eq!(encoded_len(100), 107);
    }

    // ── Version-skew tests ────────────────────────────────────────────────────

    #[test]
    fn same_version_compatible() {
        assert!(assert_compatible(1, 1, 1).is_ok());
        assert!(assert_compatible(WIRE_VERSION, WIRE_VERSION, WIRE_VERSION).is_ok());
    }

    #[test]
    fn newer_remote_accepted_when_min_is_old() {
        // Local speaks v1 but min_compatible=1, remote is v2 — allowed.
        assert!(assert_compatible(1, 2, 1).is_ok());
    }

    #[test]
    fn old_artifact_rejected_after_breaking_bump() {
        // A device still running protocol v1 connects to a host that has bumped
        // to v2 with a breaking change (min_compatible=2). It must be rejected.
        let local: u8 = 2;
        let remote: u8 = 1; // old artifact
        let min_compatible: u8 = 2;
        let err = assert_compatible(local, remote, min_compatible).unwrap_err();
        assert_eq!(err.local, 2);
        assert_eq!(err.remote, 1);
        assert_eq!(err.min_compatible, 2);
    }

    #[test]
    fn is_current_compatible_self() {
        assert!(is_current_compatible(WIRE_VERSION).is_ok());
    }

    #[test]
    fn is_current_compatible_rejects_old() {
        // WIRE_VERSION is 1; any version < 1 (i.e. 0) is rejected.
        // This also proves that after a breaking bump to N, version N-1 is refused.
        if WIRE_VERSION > 0 {
            assert!(is_current_compatible(WIRE_VERSION - 1).is_err());
        }
    }

    #[test]
    fn decoder_skips_oversized_frame() {
        let claimed_len: u16 = 300;
        let mut input = vec![MAGIC, (claimed_len & 0xFF) as u8, (claimed_len >> 8) as u8];
        input.extend_from_slice(&[0u8; 300]);
        input.extend_from_slice(&[0u8; 4]); // 4-byte CRC trailer
        input.extend_from_slice(&encode_to_vec(b"ok"));
        let decoded = decode_all::<64>(&input).unwrap();
        assert_eq!(decoded, b"ok");
    }
}

//! Transport-agnostic protocol framing — `no_std`.
//!
//! # Frame format
//!
//! ```text
//! [ MAGIC(1) | LEN_LO(1) | LEN_HI(1) | PAYLOAD(N) | CRC8(1) ]
//! ```
//!
//! - `MAGIC` = `0xFD` — identifies a Fiducial frame
//! - `LEN` = payload byte count, 2-byte little-endian
//! - `CRC8` = XOR fold of all payload bytes (fast, sufficient for a framed link)
//!
//! Total overhead: 4 bytes per frame. Maximum payload: 65 535 bytes.
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

// ── CRC-8 ─────────────────────────────────────────────────────────────────────

/// Compute the CRC-8 of a byte slice (XOR fold).
///
/// Fast and allocation-free. Sufficient for a framed point-to-point link
/// where the transport already has its own error detection (USB, UART FIFO).
pub fn crc8(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc ^ b)
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
    payload_len + 4 // magic + len_lo + len_hi + crc8
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
    out[3 + payload.len()] = crc8(payload);
    Ok(total)
}

// ── Decoding ──────────────────────────────────────────────────────────────────

/// Error returned when a decoded frame fails its CRC check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrcError {
    /// CRC computed over the received payload.
    pub computed: u8,
    /// CRC received in the frame trailer.
    pub received: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodeState {
    Magic,
    LenLo,
    LenHi { len_lo: u8 },
    Payload { remaining: u16 },
    Crc,
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
                    self.state = DecodeState::Crc;
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
                    self.state = DecodeState::Crc;
                } else {
                    self.state = DecodeState::Payload {
                        remaining: remaining - 1,
                    };
                }
                None
            }
            DecodeState::Crc => {
                let payload = &self.buf[..self.expected as usize];
                let computed = crc8(payload);
                self.state = DecodeState::Magic;
                if computed == byte {
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
    fn encode_empty_payload() {
        let bytes = encode_to_vec(&[]);
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes[0], MAGIC);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 0);
        assert_eq!(bytes[3], 0); // crc8([]) = 0
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
        assert_eq!(bytes[5], b'h' ^ b'i');
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
        assert_eq!(encoded_len(0), 4);
        assert_eq!(encoded_len(10), 14);
        assert_eq!(encoded_len(100), 104);
    }

    #[test]
    fn decoder_skips_oversized_frame() {
        let claimed_len: u16 = 300;
        let mut input = vec![MAGIC, (claimed_len & 0xFF) as u8, (claimed_len >> 8) as u8];
        input.extend_from_slice(&[0u8; 300]);
        input.push(0);
        input.extend_from_slice(&encode_to_vec(b"ok"));
        let decoded = decode_all::<64>(&input).unwrap();
        assert_eq!(decoded, b"ok");
    }
}

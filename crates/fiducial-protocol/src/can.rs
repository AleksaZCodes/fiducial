//! CAN 2.0 and CAN-FD frames, `no_std`.
//!
//! A frame here is what a controller sends and receives, with the rules the bus
//! itself enforces checked at construction rather than discovered on the wire:
//!
//! - an identifier is 11 bits (standard) or 29 bits (extended);
//! - a classic frame carries 0 to 8 bytes;
//! - a CAN-FD frame carries one of **sixteen** lengths (0 to 8, then 12, 16, 20,
//!   24, 32, 48 or 64), because its 4-bit length code cannot say 9, 10 or 11. A
//!   payload of any other length must be padded up, which is
//!   [`padded_len`] for a sender and what [`crate::segment`] does.
//!
//! The bit rate, the sample point and the transceiver are not here: they are
//! `[[bus]]` facts in `topology.toml`, and the controller driver owns them.

/// The most a classic CAN frame carries.
pub const CLASSIC_MAX: usize = 8;

/// The most a CAN-FD frame carries.
pub const FD_MAX: usize = 64;

/// Every payload length a CAN-FD frame can have, indexed by its length code.
pub const FD_LENGTHS: [usize; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64];

/// The payload length a length code stands for. Codes 0 to 8 are literal; 9 to
/// 15 are 12, 16, 20, 24, 32, 48 and 64. Only the low four bits are read.
pub const fn dlc_to_len(dlc: u8) -> usize {
    FD_LENGTHS[(dlc & 0x0F) as usize]
}

/// The length code for a payload of exactly `len` bytes, or `None` when no CAN-FD
/// frame has that length (`len` of 9, 10, 11, 13 and so on).
pub const fn len_to_dlc(len: usize) -> Option<u8> {
    let mut i = 0;
    while i < FD_LENGTHS.len() {
        if FD_LENGTHS[i] == len {
            return Some(i as u8);
        }
        i += 1;
    }
    None
}

/// The smallest valid CAN-FD payload length that holds `len` bytes, or `None` when
/// `len` is more than [`FD_MAX`].
pub const fn padded_len(len: usize) -> Option<usize> {
    let mut i = 0;
    while i < FD_LENGTHS.len() {
        if FD_LENGTHS[i] >= len {
            return Some(FD_LENGTHS[i]);
        }
        i += 1;
    }
    None
}

/// Why a frame could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A standard identifier is 11 bits and an extended one 29.
    IdOutOfRange,
    /// More than 8 bytes in a classic frame, or more than 64 in an FD frame.
    TooLong,
    /// An FD frame's payload is not one of its sixteen lengths. Pad it first.
    NotAnFdLength,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::IdOutOfRange => f.write_str("CAN identifier out of range"),
            Error::TooLong => f.write_str("CAN payload too long for the frame kind"),
            Error::NotAnFdLength => f.write_str("not a CAN-FD payload length: pad it first"),
        }
    }
}

impl core::error::Error for Error {}

/// A CAN identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id {
    /// 11 bits.
    Standard(u16),
    /// 29 bits.
    Extended(u32),
}

impl Id {
    /// A standard (11-bit) identifier.
    pub const fn standard(raw: u16) -> Result<Id, Error> {
        if raw > 0x7FF {
            Err(Error::IdOutOfRange)
        } else {
            Ok(Id::Standard(raw))
        }
    }

    /// An extended (29-bit) identifier.
    pub const fn extended(raw: u32) -> Result<Id, Error> {
        if raw > 0x1FFF_FFFF {
            Err(Error::IdOutOfRange)
        } else {
            Ok(Id::Extended(raw))
        }
    }

    /// The identifier as a number.
    pub const fn raw(self) -> u32 {
        match self {
            Id::Standard(r) => r as u32,
            Id::Extended(r) => r,
        }
    }
}

/// Classic CAN, or CAN-FD with or without the faster data phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Up to 8 bytes at the arbitration bit rate.
    Classic,
    /// Up to 64 bytes; `brs` switches to the data bit rate after arbitration.
    Fd {
        /// Bit-rate switch: the data phase runs at `data_bitrate`.
        brs: bool,
    },
}

/// One CAN frame.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    id: Id,
    kind: Kind,
    len: u8,
    data: [u8; FD_MAX],
}

impl core::fmt::Debug for Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Frame")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("data", &self.data())
            .finish()
    }
}

impl Frame {
    /// A classic frame, 0 to 8 bytes.
    pub fn classic(id: Id, data: &[u8]) -> Result<Frame, Error> {
        if data.len() > CLASSIC_MAX {
            return Err(Error::TooLong);
        }
        Ok(Frame::build(id, Kind::Classic, data))
    }

    /// A CAN-FD frame. `data.len()` must be one of the sixteen valid lengths; a
    /// payload of another length is [`Error::NotAnFdLength`], not silently padded,
    /// because padding changes what the receiver sees.
    pub fn fd(id: Id, data: &[u8], brs: bool) -> Result<Frame, Error> {
        if data.len() > FD_MAX {
            return Err(Error::TooLong);
        }
        if len_to_dlc(data.len()).is_none() {
            return Err(Error::NotAnFdLength);
        }
        Ok(Frame::build(id, Kind::Fd { brs }, data))
    }

    fn build(id: Id, kind: Kind, data: &[u8]) -> Frame {
        let mut buf = [0u8; FD_MAX];
        buf[..data.len()].copy_from_slice(data);
        Frame {
            id,
            kind,
            len: data.len() as u8,
            data: buf,
        }
    }

    /// The identifier.
    pub const fn id(&self) -> Id {
        self.id
    }

    /// Classic or FD.
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// The payload.
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }

    /// The 4-bit length code on the wire.
    pub fn dlc(&self) -> u8 {
        // Construction guarantees the length is valid.
        len_to_dlc(self.len as usize).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_length_codes_are_literal_to_eight_then_jump() {
        let expected = [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64];
        for (code, len) in expected.iter().enumerate() {
            assert_eq!(dlc_to_len(code as u8), *len);
            assert_eq!(len_to_dlc(*len), Some(code as u8));
        }
        // Only the low four bits are a code.
        assert_eq!(dlc_to_len(0x1F), 64);
    }

    #[test]
    fn a_length_with_no_code_is_refused_and_padding_goes_up_to_the_next() {
        for bad in [9, 10, 11, 13, 17, 25, 49, 63] {
            assert_eq!(len_to_dlc(bad), None, "{bad}");
        }
        assert_eq!(padded_len(9), Some(12));
        assert_eq!(padded_len(13), Some(16));
        assert_eq!(padded_len(49), Some(64));
        assert_eq!(padded_len(64), Some(64));
        assert_eq!(padded_len(65), None);
        assert_eq!(padded_len(0), Some(0));
        // Padding always lands on a valid length.
        for n in 0..=64 {
            let p = padded_len(n).unwrap();
            assert!(p >= n && len_to_dlc(p).is_some());
        }
    }

    #[test]
    fn identifiers_are_range_checked() {
        assert!(Id::standard(0x7FF).is_ok());
        assert_eq!(Id::standard(0x800), Err(Error::IdOutOfRange));
        assert!(Id::extended(0x1FFF_FFFF).is_ok());
        assert_eq!(Id::extended(0x2000_0000), Err(Error::IdOutOfRange));
        assert_eq!(Id::standard(0x123).unwrap().raw(), 0x123);
    }

    #[test]
    fn a_frame_holds_what_its_kind_allows() {
        let id = Id::standard(0x100).unwrap();
        assert_eq!(Frame::classic(id, &[0; 8]).unwrap().data().len(), 8);
        assert_eq!(Frame::classic(id, &[0; 9]), Err(Error::TooLong));

        assert_eq!(Frame::fd(id, &[0; 64], true).unwrap().dlc(), 15);
        assert_eq!(Frame::fd(id, &[0; 65], false), Err(Error::TooLong));
        assert_eq!(Frame::fd(id, &[0; 10], false), Err(Error::NotAnFdLength));
        let f = Frame::fd(id, &[7; 12], true).unwrap();
        assert_eq!(f.dlc(), 9);
        assert_eq!(f.kind(), Kind::Fd { brs: true });
        assert_eq!(f.data(), &[7; 12]);
    }
}

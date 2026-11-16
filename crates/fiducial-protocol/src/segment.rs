//! ISO 15765-2 (ISO-TP) segmentation over CAN and CAN-FD, `no_std`.
//!
//! A CAN frame carries 8 bytes (64 on CAN-FD); a message may be up to 4 GiB.
//! ISO-TP splits one into frames and puts it back together, with the receiver
//! pacing the sender so a slow node is not flooded:
//!
//! ```text
//! sender                                   receiver
//!   |-- FF  first frame: total length + first bytes -->|
//!   |<-- FC  flow control: go / wait / overflow, block size, STmin --|
//!   |-- CF  consecutive frames, sequence 1,2,..15,0,1.. -->|
//!   |      (every `block size` frames: wait for another FC)         |
//! ```
//!
//! A message that fits one frame is a **single frame** (SF) and needs no flow
//! control. Normal addressing only: the identifier carries the addressing, and
//! the frame's data begins with the protocol-control byte.
//!
//! # Frame layouts
//!
//! | Frame | Bytes | Notes |
//! |---|---|---|
//! | SF, up to 7 bytes | `0L d..` | `L` is the length |
//! | SF, CAN-FD, up to 62 | `00 LL d..` | the escape form, only when the frame is longer than 8 |
//! | FF, up to 4095 | `1H LL d..` | 12-bit length |
//! | FF, longer | `10 00 L4 L3 L2 L1 d..` | the 32-bit escape |
//! | CF | `2N d..` | `N` is the sequence number, wrapping 15 to 0 |
//! | FC | `3S BS STmin` | `S`: 0 go, 1 wait, 2 overflow |
//!
//! # What this does not do
//!
//! It owns no clock and no CAN controller. The caller passes elapsed time to
//! [`Sender::tick`] and [`Receiver::tick`], and moves frames to and from the
//! controller (SocketCAN on Linux, FDCAN on an MCU). That is what lets it be
//! tested here over an in-memory bus: this machine's kernel has no CAN.

use crate::can::{len_to_dlc, padded_len, CLASSIC_MAX, FD_MAX};

/// Frames carry 8 bytes on classic CAN and up to 64 on CAN-FD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Classic CAN: 8-byte frames.
    Classic,
    /// CAN-FD: up to 64-byte frames.
    Fd,
}

impl Mode {
    /// The largest frame this mode sends.
    pub const fn frame_len(self) -> usize {
        match self {
            Mode::Classic => CLASSIC_MAX,
            Mode::Fd => FD_MAX,
        }
    }
}

/// The separation time a receiver asks for between consecutive frames, in the
/// byte the flow-control frame carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StMin(pub u8);

impl StMin {
    /// The gap in microseconds. `0x00..=0x7F` are milliseconds, `0xF1..=0xF9` are
    /// 100 to 900 microseconds, and every other value is reserved, which the
    /// standard says to treat as the largest valid one (127 ms).
    pub const fn micros(self) -> u32 {
        match self.0 {
            ms @ 0x00..=0x7F => ms as u32 * 1000,
            n @ 0xF1..=0xF9 => (n as u32 - 0xF0) * 100,
            _ => 127_000,
        }
    }
}

/// Timing and pacing, read by whichever side each field concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    /// Receiver: frames per block before asking again. 0 means "all of them".
    pub block_size: u8,
    /// Receiver: the separation time it asks the sender to keep.
    pub st_min: StMin,
    /// Sender: how long to wait for a flow-control frame (N_Bs), in microseconds.
    pub n_bs_us: u32,
    /// Receiver: how long to wait for the next consecutive frame (N_Cr), in
    /// microseconds.
    pub n_cr_us: u32,
    /// The byte a frame is padded with. `None` sends short classic frames as they
    /// are; a CAN-FD frame is padded up to a valid length regardless, and uses
    /// `0xCC` when this is `None`.
    pub padding: Option<u8>,
    /// Sender: how many WAIT flow-control frames it tolerates in a row.
    pub max_wait_frames: u8,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            block_size: 0,
            st_min: StMin(0),
            n_bs_us: 1_000_000,
            n_cr_us: 1_000_000,
            padding: Some(0xCC),
            max_wait_frames: 10,
        }
    }
}

/// What can go wrong sending or receiving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A message must have at least one byte.
    EmptyMessage,
    /// The message does not fit a 32-bit length.
    MessageTooLong,
    /// A frame that is not a valid ISO-TP frame in this position.
    InvalidFrame,
    /// A frame that cannot be accepted now: a consecutive frame with no first
    /// frame, or flow control when none was expected.
    UnexpectedFrame,
    /// A consecutive frame out of order: a frame was lost or repeated.
    WrongSequence {
        /// The sequence number that was due.
        expected: u8,
        /// The one that arrived.
        got: u8,
    },
    /// The message is larger than the receiver's buffer.
    Overflow,
    /// The other side did not answer in time (N_Bs or N_Cr).
    Timeout,
    /// The receiver kept asking the sender to wait.
    TooManyWaits,
    /// The transfer already failed.
    Aborted,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::EmptyMessage => f.write_str("empty ISO-TP message"),
            Error::MessageTooLong => f.write_str("ISO-TP message longer than 4 GiB"),
            Error::InvalidFrame => f.write_str("invalid ISO-TP frame"),
            Error::UnexpectedFrame => f.write_str("unexpected ISO-TP frame"),
            Error::WrongSequence { expected, got } => {
                write!(f, "ISO-TP sequence error: expected {expected}, got {got}")
            }
            Error::Overflow => f.write_str("ISO-TP message larger than the receive buffer"),
            Error::Timeout => f.write_str("ISO-TP timeout"),
            Error::TooManyWaits => f.write_str("ISO-TP receiver kept asking to wait"),
            Error::Aborted => f.write_str("ISO-TP transfer already failed"),
        }
    }
}

impl core::error::Error for Error {}

/// A flow-control verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowStatus {
    /// Clear to send.
    ContinueToSend,
    /// Hold on and expect another flow-control frame.
    Wait,
    /// The message does not fit: stop.
    Overflow,
}

/// A flow-control frame's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowControl {
    /// Go, wait or overflow.
    pub status: FlowStatus,
    /// Frames to send before the next flow control. 0 means no more are needed.
    pub block_size: u8,
    /// The gap to keep between consecutive frames.
    pub st_min: StMin,
}

/// Bytes a frame of `used` meaningful bytes occupies on the wire, padded as the
/// mode and the padding setting require. `buf` is filled past `used` with the pad.
fn finish(buf: &mut [u8; FD_MAX], used: usize, mode: Mode, padding: Option<u8>) -> usize {
    let pad = padding.unwrap_or(0xCC);
    let target = match mode {
        Mode::Classic => {
            if padding.is_some() {
                CLASSIC_MAX
            } else {
                used
            }
        }
        // CAN-FD lengths are discrete, so padding up is not optional.
        Mode::Fd => padded_len(used).unwrap_or(used),
    };
    for b in &mut buf[used..target] {
        *b = pad;
    }
    target
}

/// Write a flow-control frame. Returns its length.
pub fn encode_flow_control(
    fc: FlowControl,
    mode: Mode,
    padding: Option<u8>,
    out: &mut [u8; FD_MAX],
) -> usize {
    let status = match fc.status {
        FlowStatus::ContinueToSend => 0,
        FlowStatus::Wait => 1,
        FlowStatus::Overflow => 2,
    };
    out[0] = 0x30 | status;
    out[1] = fc.block_size;
    out[2] = fc.st_min.0;
    finish(out, 3, mode, padding)
}

fn decode_flow_control(frame: &[u8]) -> Result<FlowControl, Error> {
    if frame.len() < 3 || frame[0] >> 4 != 3 {
        return Err(Error::InvalidFrame);
    }
    let status = match frame[0] & 0x0F {
        0 => FlowStatus::ContinueToSend,
        1 => FlowStatus::Wait,
        2 => FlowStatus::Overflow,
        _ => return Err(Error::InvalidFrame),
    };
    Ok(FlowControl {
        status,
        block_size: frame[1],
        st_min: StMin(frame[2]),
    })
}

// ── Sender ───────────────────────────────────────────────────────────────────

/// What a sender wants next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// Send this many bytes of the buffer you passed.
    Frame(usize),
    /// Nothing to send yet: a flow-control frame is due, or the separation time
    /// has not passed. Call [`Sender::tick`] and poll again.
    Wait,
    /// The whole message has been sent.
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxState {
    Single,
    First,
    AwaitFlowControl,
    Sending,
    Done,
    Failed,
}

/// Sends one message.
pub struct Sender<'a> {
    data: &'a [u8],
    offset: usize,
    sn: u8,
    mode: Mode,
    params: Params,
    state: TxState,
    block_left: u16,
    st_us: u32,
    gap_us: u32,
    timer_us: u32,
    waits: u8,
}

impl<'a> Sender<'a> {
    /// Prepare to send `data`.
    pub fn new(data: &'a [u8], mode: Mode, params: Params) -> Result<Sender<'a>, Error> {
        if data.is_empty() {
            return Err(Error::EmptyMessage);
        }
        if data.len() as u64 > u32::MAX as u64 {
            return Err(Error::MessageTooLong);
        }
        let single_max = match mode {
            Mode::Classic => 7,
            Mode::Fd => FD_MAX - 2,
        };
        let state = if data.len() <= single_max {
            TxState::Single
        } else {
            TxState::First
        };
        Ok(Sender {
            data,
            offset: 0,
            sn: 1,
            mode,
            params,
            state,
            block_left: 0,
            st_us: 0,
            gap_us: 0,
            timer_us: 0,
            waits: 0,
        })
    }

    /// The next frame, written to `out`, or why there is none yet.
    pub fn poll(&mut self, out: &mut [u8; FD_MAX]) -> Result<Poll, Error> {
        match self.state {
            TxState::Single => {
                let n = self.data.len();
                let used = if n <= 7 {
                    out[0] = n as u8;
                    out[1..1 + n].copy_from_slice(self.data);
                    1 + n
                } else {
                    out[0] = 0x00;
                    out[1] = n as u8;
                    out[2..2 + n].copy_from_slice(self.data);
                    2 + n
                };
                self.offset = n;
                self.state = TxState::Done;
                Ok(Poll::Frame(finish(
                    out,
                    used,
                    self.mode,
                    self.params.padding,
                )))
            }
            TxState::First => {
                let total = self.data.len();
                let header = if total <= 0xFFF {
                    out[0] = 0x10 | (total >> 8) as u8;
                    out[1] = total as u8;
                    2
                } else {
                    out[0] = 0x10;
                    out[1] = 0x00;
                    out[2..6].copy_from_slice(&(total as u32).to_be_bytes());
                    6
                };
                let frame = self.mode.frame_len();
                let take = frame - header;
                out[header..frame].copy_from_slice(&self.data[..take]);
                self.offset = take;
                self.state = TxState::AwaitFlowControl;
                self.timer_us = 0;
                Ok(Poll::Frame(frame))
            }
            TxState::AwaitFlowControl => Ok(Poll::Wait),
            TxState::Sending => {
                if self.gap_us > 0 {
                    return Ok(Poll::Wait);
                }
                let cap = self.mode.frame_len() - 1;
                let take = cap.min(self.data.len() - self.offset);
                out[0] = 0x20 | self.sn;
                out[1..1 + take].copy_from_slice(&self.data[self.offset..self.offset + take]);
                self.offset += take;
                self.sn = (self.sn + 1) & 0x0F;
                let used = 1 + take;
                if self.offset >= self.data.len() {
                    self.state = TxState::Done;
                } else if self.block_left > 0 {
                    self.block_left -= 1;
                    if self.block_left == 0 {
                        // The block is spent: another flow-control frame is due.
                        self.state = TxState::AwaitFlowControl;
                        self.timer_us = 0;
                    } else {
                        self.gap_us = self.st_us;
                    }
                } else {
                    self.gap_us = self.st_us;
                }
                Ok(Poll::Frame(finish(
                    out,
                    used,
                    self.mode,
                    self.params.padding,
                )))
            }
            TxState::Done => Ok(Poll::Done),
            TxState::Failed => Err(Error::Aborted),
        }
    }

    /// Whether the sender is waiting for a flow-control frame.
    pub fn awaiting_flow_control(&self) -> bool {
        self.state == TxState::AwaitFlowControl
    }

    /// Hand the sender a flow-control frame the receiver sent.
    pub fn on_flow_control(&mut self, frame: &[u8]) -> Result<(), Error> {
        if self.state != TxState::AwaitFlowControl {
            return Err(Error::UnexpectedFrame);
        }
        let fc = match decode_flow_control(frame) {
            Ok(fc) => fc,
            Err(e) => {
                self.state = TxState::Failed;
                return Err(e);
            }
        };
        match fc.status {
            FlowStatus::ContinueToSend => {
                self.block_left = fc.block_size as u16;
                self.st_us = fc.st_min.micros();
                self.gap_us = 0;
                self.waits = 0;
                self.state = TxState::Sending;
                Ok(())
            }
            FlowStatus::Wait => {
                self.waits += 1;
                if self.waits > self.params.max_wait_frames {
                    self.state = TxState::Failed;
                    return Err(Error::TooManyWaits);
                }
                self.timer_us = 0;
                Ok(())
            }
            FlowStatus::Overflow => {
                self.state = TxState::Failed;
                Err(Error::Overflow)
            }
        }
    }

    /// Tell the sender that `elapsed_us` microseconds have passed.
    pub fn tick(&mut self, elapsed_us: u32) -> Result<(), Error> {
        match self.state {
            TxState::AwaitFlowControl => {
                self.timer_us = self.timer_us.saturating_add(elapsed_us);
                if self.timer_us >= self.params.n_bs_us {
                    self.state = TxState::Failed;
                    return Err(Error::Timeout);
                }
                Ok(())
            }
            TxState::Sending => {
                self.gap_us = self.gap_us.saturating_sub(elapsed_us);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Whether every byte has been sent.
    pub fn is_done(&self) -> bool {
        self.state == TxState::Done
    }
}

// ── Receiver ─────────────────────────────────────────────────────────────────

/// What a received frame did.
#[derive(Debug, PartialEq, Eq)]
pub enum Event<'a> {
    /// Mid-message, nothing to do yet.
    Pending,
    /// Send this flow-control frame back (see [`encode_flow_control`]).
    SendFlowControl(FlowControl),
    /// A whole message. It borrows the receiver's buffer until the next call.
    Message(&'a [u8]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RxState {
    Idle,
    Receiving,
}

/// Reassembles messages of up to `N` bytes.
pub struct Receiver<const N: usize> {
    buf: [u8; N],
    total: usize,
    filled: usize,
    sn: u8,
    rx_dl: usize,
    block_count: u8,
    timer_us: u32,
    params: Params,
    state: RxState,
}

impl<const N: usize> Receiver<N> {
    /// A receiver with the pacing in `params`.
    pub const fn new(params: Params) -> Self {
        Receiver {
            buf: [0; N],
            total: 0,
            filled: 0,
            sn: 0,
            rx_dl: 0,
            block_count: 0,
            timer_us: 0,
            params,
            state: RxState::Idle,
        }
    }

    fn reset(&mut self) {
        self.state = RxState::Idle;
        self.filled = 0;
        self.total = 0;
    }

    fn clear_to_send(&self) -> FlowControl {
        FlowControl {
            status: FlowStatus::ContinueToSend,
            block_size: self.params.block_size,
            st_min: self.params.st_min,
        }
    }

    /// Whether a multi-frame message is being reassembled.
    pub fn is_receiving(&self) -> bool {
        self.state == RxState::Receiving
    }

    /// Hand the receiver one frame.
    pub fn feed<'a>(&'a mut self, frame: &[u8]) -> Result<Event<'a>, Error> {
        let Some(&pci) = frame.first() else {
            return Err(Error::InvalidFrame);
        };
        match pci >> 4 {
            0 => self.single(frame),
            1 => self.first(frame),
            2 => self.consecutive(frame),
            3 => Err(Error::UnexpectedFrame),
            _ => Err(Error::InvalidFrame),
        }
    }

    fn single<'a>(&'a mut self, frame: &[u8]) -> Result<Event<'a>, Error> {
        // A single frame mid-message abandons the old one.
        self.reset();
        let low = (frame[0] & 0x0F) as usize;
        let (len, start) = if low != 0 {
            (low, 1)
        } else {
            // The escape form exists only when the frame is longer than 8 bytes.
            if frame.len() <= CLASSIC_MAX || frame.len() < 2 {
                return Err(Error::InvalidFrame);
            }
            (frame[1] as usize, 2)
        };
        if len == 0 || frame.len() < start + len {
            return Err(Error::InvalidFrame);
        }
        if len > N {
            return Err(Error::Overflow);
        }
        self.buf[..len].copy_from_slice(&frame[start..start + len]);
        Ok(Event::Message(&self.buf[..len]))
    }

    fn first<'a>(&'a mut self, frame: &[u8]) -> Result<Event<'a>, Error> {
        self.reset();
        // A first frame is always a full frame: 8 bytes, or a valid CAN-FD length.
        if frame.len() < CLASSIC_MAX || len_to_dlc(frame.len()).is_none() || frame.len() < 2 {
            return Err(Error::InvalidFrame);
        }
        let short = (((frame[0] & 0x0F) as usize) << 8) | frame[1] as usize;
        let (total, header) = if short != 0 {
            (short, 2)
        } else {
            if frame.len() < 6 {
                return Err(Error::InvalidFrame);
            }
            let long = u32::from_be_bytes([frame[2], frame[3], frame[4], frame[5]]) as usize;
            // The escape is for lengths the 12-bit field cannot hold.
            if long <= 0xFFF {
                return Err(Error::InvalidFrame);
            }
            (long, 6)
        };
        let carried = frame.len() - header;
        // A message that fits the first frame is a single frame, not this.
        if total <= carried {
            return Err(Error::InvalidFrame);
        }
        if total > N {
            return Ok(Event::SendFlowControl(FlowControl {
                status: FlowStatus::Overflow,
                block_size: 0,
                st_min: StMin(0),
            }));
        }
        self.buf[..carried].copy_from_slice(&frame[header..]);
        self.total = total;
        self.filled = carried;
        self.rx_dl = frame.len();
        self.sn = 1;
        self.block_count = 0;
        self.timer_us = 0;
        self.state = RxState::Receiving;
        Ok(Event::SendFlowControl(self.clear_to_send()))
    }

    fn consecutive<'a>(&'a mut self, frame: &[u8]) -> Result<Event<'a>, Error> {
        if self.state != RxState::Receiving {
            return Err(Error::UnexpectedFrame);
        }
        let got = frame[0] & 0x0F;
        if got != self.sn {
            let expected = self.sn;
            self.reset();
            return Err(Error::WrongSequence { expected, got });
        }
        let remaining = self.total - self.filled;
        let cap = self.rx_dl - 1;
        let take = if remaining > cap {
            // Not the last: it must be as long as the first frame was.
            if frame.len() != self.rx_dl {
                self.reset();
                return Err(Error::InvalidFrame);
            }
            cap
        } else {
            // The last may carry padding after its data.
            if frame.len() < 1 + remaining {
                self.reset();
                return Err(Error::InvalidFrame);
            }
            remaining
        };
        self.buf[self.filled..self.filled + take].copy_from_slice(&frame[1..1 + take]);
        self.filled += take;
        self.sn = (self.sn + 1) & 0x0F;
        self.timer_us = 0;
        if self.filled == self.total {
            let total = self.total;
            self.state = RxState::Idle;
            self.filled = 0;
            self.total = 0;
            return Ok(Event::Message(&self.buf[..total]));
        }
        if self.params.block_size != 0 {
            self.block_count += 1;
            if self.block_count == self.params.block_size {
                self.block_count = 0;
                return Ok(Event::SendFlowControl(self.clear_to_send()));
            }
        }
        Ok(Event::Pending)
    }

    /// Tell the receiver that `elapsed_us` microseconds have passed.
    pub fn tick(&mut self, elapsed_us: u32) -> Result<(), Error> {
        if self.state != RxState::Receiving {
            return Ok(());
        }
        self.timer_us = self.timer_us.saturating_add(elapsed_us);
        if self.timer_us >= self.params.n_cr_us {
            self.reset();
            return Err(Error::Timeout);
        }
        Ok(())
    }
}

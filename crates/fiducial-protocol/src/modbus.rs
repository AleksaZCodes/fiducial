//! Modbus RTU framing and RS-485 direction control, `no_std`.
//!
//! RS-485 is a pair of wires that one node drives at a time; Modbus RTU is the
//! framing most vendor sensors on it speak (the soil and temperature probes in the
//! module library are two). This is the part that is the same for every one:
//!
//! - **frames**: `[unit][function][data…][CRC-16 low][CRC-16 high]`, with the
//!   CRC-16/MODBUS check (poly `0xA001` reflected, init `0xFFFF`);
//! - **requests** for the four functions a sensor network uses: read holding
//!   registers (`0x03`), read input registers (`0x04`), write single register
//!   (`0x06`) and write multiple registers (`0x10`);
//! - **responses**, parsed against the request they answer, with the device's
//!   exception codes surfaced as errors;
//! - **timing**: a frame ends after 3.5 character times of silence, and a gap of
//!   more than 1.5 inside one means it is corrupt ([`Timing`], [`FrameReceiver`]);
//! - **direction**: the transceiver is switched to transmit around a frame and
//!   back to receive after, even when the write failed ([`send`]).
//!
//! The register map of a device, its baud rate and which pin is the driver enable
//! are declarations (`topology.toml` and the module library), not code here.
//! Coils and discrete inputs (`0x01`, `0x02`, `0x05`, `0x0F`) are not here: no
//! sensor in the library uses them.

/// The most registers one read may ask for (the PDU holds 250 data bytes).
pub const MAX_READ_REGISTERS: u16 = 125;
/// The most registers one write-multiple may carry.
pub const MAX_WRITE_REGISTERS: u16 = 123;
/// The longest frame: unit, function, 252 bytes of data, and the CRC.
pub const MAX_FRAME: usize = 256;

/// CRC-16/MODBUS over `data`. Sent low byte first.
pub const fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    let mut i = 0;
    while i < data.len() {
        crc ^= data[i] as u16;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
            bit += 1;
        }
        i += 1;
    }
    crc
}

/// A function this module speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    /// `0x03`.
    ReadHoldingRegisters,
    /// `0x04`.
    ReadInputRegisters,
    /// `0x06`.
    WriteSingleRegister,
    /// `0x10`.
    WriteMultipleRegisters,
}

impl Function {
    /// The function code on the wire.
    pub const fn code(self) -> u8 {
        match self {
            Function::ReadHoldingRegisters => 0x03,
            Function::ReadInputRegisters => 0x04,
            Function::WriteSingleRegister => 0x06,
            Function::WriteMultipleRegisters => 0x10,
        }
    }

    /// The function a code stands for, if this module speaks it.
    pub const fn from_code(code: u8) -> Option<Function> {
        match code {
            0x03 => Some(Function::ReadHoldingRegisters),
            0x04 => Some(Function::ReadInputRegisters),
            0x06 => Some(Function::WriteSingleRegister),
            0x10 => Some(Function::WriteMultipleRegisters),
            _ => None,
        }
    }
}

/// A device's reason for refusing a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionCode {
    /// `1`: the function is not supported.
    IllegalFunction,
    /// `2`: the address, or the address plus quantity, does not exist.
    IllegalDataAddress,
    /// `3`: a value in the request is not allowed.
    IllegalDataValue,
    /// `4`: the device failed while handling it.
    ServerDeviceFailure,
    /// `5`: accepted, and it will take a while.
    Acknowledge,
    /// `6`: busy; try again.
    ServerDeviceBusy,
    /// `8`: a parity error in the device's memory.
    MemoryParityError,
    /// `10`: a gateway has no route.
    GatewayPathUnavailable,
    /// `11`: a gateway's target did not answer.
    GatewayTargetFailed,
    /// Any other code.
    Other(u8),
}

impl ExceptionCode {
    /// The code on the wire.
    pub const fn code(self) -> u8 {
        match self {
            ExceptionCode::IllegalFunction => 1,
            ExceptionCode::IllegalDataAddress => 2,
            ExceptionCode::IllegalDataValue => 3,
            ExceptionCode::ServerDeviceFailure => 4,
            ExceptionCode::Acknowledge => 5,
            ExceptionCode::ServerDeviceBusy => 6,
            ExceptionCode::MemoryParityError => 8,
            ExceptionCode::GatewayPathUnavailable => 10,
            ExceptionCode::GatewayTargetFailed => 11,
            ExceptionCode::Other(c) => c,
        }
    }

    const fn from_code(code: u8) -> ExceptionCode {
        match code {
            1 => ExceptionCode::IllegalFunction,
            2 => ExceptionCode::IllegalDataAddress,
            3 => ExceptionCode::IllegalDataValue,
            4 => ExceptionCode::ServerDeviceFailure,
            5 => ExceptionCode::Acknowledge,
            6 => ExceptionCode::ServerDeviceBusy,
            8 => ExceptionCode::MemoryParityError,
            10 => ExceptionCode::GatewayPathUnavailable,
            11 => ExceptionCode::GatewayTargetFailed,
            other => ExceptionCode::Other(other),
        }
    }
}

/// What can go wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A unit address is 1 to 247, or 0 to broadcast a write.
    BadUnit,
    /// A quantity of 0, or more than a frame can carry.
    BadQuantity,
    /// The output buffer cannot hold the frame.
    BufferTooSmall,
    /// The frame is too short or too long to be a Modbus frame.
    BadLength,
    /// The CRC does not match.
    BadCrc,
    /// The frame answers a different unit than the request was for.
    WrongUnit,
    /// The frame answers a different function than the request was for.
    WrongFunction,
    /// The response says a different number of bytes than the request asked for.
    WrongByteCount,
    /// The response does not echo the request's address or value.
    WrongEcho,
    /// A function this module does not speak.
    UnknownFunction(u8),
    /// The device refused, and said why.
    Exception(ExceptionCode),
    /// A byte arrived too long after the last one inside a frame (more than 1.5
    /// character times), or the frame overran the buffer.
    CorruptFrame,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::BadUnit => f.write_str("unit address out of range"),
            Error::BadQuantity => f.write_str("register quantity out of range"),
            Error::BufferTooSmall => f.write_str("buffer too small for the frame"),
            Error::BadLength => f.write_str("not a Modbus frame length"),
            Error::BadCrc => f.write_str("Modbus CRC mismatch"),
            Error::WrongUnit => f.write_str("response from another unit"),
            Error::WrongFunction => f.write_str("response to another function"),
            Error::WrongByteCount => f.write_str("response byte count does not match the request"),
            Error::WrongEcho => f.write_str("response does not echo the request"),
            Error::UnknownFunction(c) => write!(f, "unsupported Modbus function {c:#04x}"),
            Error::Exception(e) => write!(f, "device exception {}", e.code()),
            Error::CorruptFrame => f.write_str("corrupt Modbus frame"),
        }
    }
}

impl core::error::Error for Error {}

/// A request to a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    /// The device: 1 to 247 (0 broadcasts a write, and nothing answers it).
    pub unit: u8,
    /// What to do.
    pub function: Function,
    /// The first register.
    pub address: u16,
    /// How many registers (reads and write-multiple); for write-single, the value.
    pub quantity_or_value: u16,
}

fn check_unit(unit: u8, broadcast_ok: bool) -> Result<(), Error> {
    if unit > 247 || (unit == 0 && !broadcast_ok) {
        Err(Error::BadUnit)
    } else {
        Ok(())
    }
}

fn seal(out: &mut [u8], len: usize) -> Result<usize, Error> {
    if out.len() < len + 2 {
        return Err(Error::BufferTooSmall);
    }
    let crc = crc16(&out[..len]);
    out[len] = crc as u8;
    out[len + 1] = (crc >> 8) as u8;
    Ok(len + 2)
}

/// Write a read request (`0x03` or `0x04`). Returns the frame length.
pub fn encode_read(
    out: &mut [u8],
    unit: u8,
    function: Function,
    address: u16,
    quantity: u16,
) -> Result<usize, Error> {
    check_unit(unit, false)?;
    if !matches!(
        function,
        Function::ReadHoldingRegisters | Function::ReadInputRegisters
    ) {
        return Err(Error::WrongFunction);
    }
    if quantity == 0 || quantity > MAX_READ_REGISTERS {
        return Err(Error::BadQuantity);
    }
    if out.len() < 8 {
        return Err(Error::BufferTooSmall);
    }
    out[0] = unit;
    out[1] = function.code();
    out[2..4].copy_from_slice(&address.to_be_bytes());
    out[4..6].copy_from_slice(&quantity.to_be_bytes());
    seal(out, 6)
}

/// Write a write-single-register request (`0x06`). `unit` 0 broadcasts.
pub fn encode_write_single(
    out: &mut [u8],
    unit: u8,
    address: u16,
    value: u16,
) -> Result<usize, Error> {
    check_unit(unit, true)?;
    if out.len() < 8 {
        return Err(Error::BufferTooSmall);
    }
    out[0] = unit;
    out[1] = Function::WriteSingleRegister.code();
    out[2..4].copy_from_slice(&address.to_be_bytes());
    out[4..6].copy_from_slice(&value.to_be_bytes());
    seal(out, 6)
}

/// Write a write-multiple-registers request (`0x10`). `unit` 0 broadcasts.
pub fn encode_write_multiple(
    out: &mut [u8],
    unit: u8,
    address: u16,
    values: &[u16],
) -> Result<usize, Error> {
    check_unit(unit, true)?;
    if values.is_empty() || values.len() > MAX_WRITE_REGISTERS as usize {
        return Err(Error::BadQuantity);
    }
    let n = values.len();
    let len = 7 + 2 * n;
    if out.len() < len + 2 {
        return Err(Error::BufferTooSmall);
    }
    out[0] = unit;
    out[1] = Function::WriteMultipleRegisters.code();
    out[2..4].copy_from_slice(&address.to_be_bytes());
    out[4..6].copy_from_slice(&(n as u16).to_be_bytes());
    out[6] = (2 * n) as u8;
    for (i, v) in values.iter().enumerate() {
        out[7 + 2 * i..9 + 2 * i].copy_from_slice(&v.to_be_bytes());
    }
    seal(out, len)
}

fn verify(frame: &[u8]) -> Result<(), Error> {
    if frame.len() < 4 || frame.len() > MAX_FRAME {
        return Err(Error::BadLength);
    }
    let (body, tail) = frame.split_at(frame.len() - 2);
    let want = crc16(body);
    if tail[0] != want as u8 || tail[1] != (want >> 8) as u8 {
        return Err(Error::BadCrc);
    }
    Ok(())
}

/// The registers a read response carried, as big-endian byte pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registers<'a>(&'a [u8]);

impl Registers<'_> {
    /// How many registers.
    pub const fn len(&self) -> usize {
        self.0.len() / 2
    }

    /// Whether there are none.
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Register `i`, counting from the requested address.
    pub fn get(&self, i: usize) -> Option<u16> {
        let b = self.0.get(2 * i..2 * i + 2)?;
        Some(u16::from_be_bytes([b[0], b[1]]))
    }

    /// All of them, in order.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        self.0
            .chunks_exact(2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
    }
}

/// A device's answer to a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response<'a> {
    /// The registers a read asked for.
    Registers(Registers<'a>),
    /// A write was accepted: the address, and the value or the count, echoed.
    Written {
        /// The first register written.
        address: u16,
        /// The value (write-single) or the number of registers (write-multiple).
        quantity_or_value: u16,
    },
}

/// Parse `frame` as the answer to `request`.
///
/// Checked in this order, because each check makes the next one meaningful: the
/// CRC, the unit, the function (an exception sets the top bit and is reported as
/// [`Error::Exception`]), and then the shape of the data against what was asked.
pub fn decode_response<'a>(frame: &'a [u8], request: &Request) -> Result<Response<'a>, Error> {
    verify(frame)?;
    if frame[0] != request.unit {
        return Err(Error::WrongUnit);
    }
    let code = frame[1];
    if code == request.function.code() | 0x80 {
        if frame.len() != 5 {
            return Err(Error::BadLength);
        }
        return Err(Error::Exception(ExceptionCode::from_code(frame[2])));
    }
    if code != request.function.code() {
        return Err(Error::WrongFunction);
    }
    let body = &frame[2..frame.len() - 2];
    match request.function {
        Function::ReadHoldingRegisters | Function::ReadInputRegisters => {
            let Some((&count, data)) = body.split_first() else {
                return Err(Error::BadLength);
            };
            let want = 2 * request.quantity_or_value as usize;
            if count as usize != want || data.len() != want {
                return Err(Error::WrongByteCount);
            }
            Ok(Response::Registers(Registers(data)))
        }
        Function::WriteSingleRegister | Function::WriteMultipleRegisters => {
            if body.len() != 4 {
                return Err(Error::BadLength);
            }
            let address = u16::from_be_bytes([body[0], body[1]]);
            let qv = u16::from_be_bytes([body[2], body[3]]);
            if address != request.address || qv != request.quantity_or_value {
                return Err(Error::WrongEcho);
            }
            Ok(Response::Written {
                address,
                quantity_or_value: qv,
            })
        }
    }
}

// ── The device side ──────────────────────────────────────────────────────────

/// A request as a device sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Incoming<'a> {
    /// Who it is for (0 is a broadcast).
    pub unit: u8,
    /// What it asks.
    pub function: Function,
    /// The first register.
    pub address: u16,
    /// The count (reads, write-multiple) or the value (write-single).
    pub quantity_or_value: u16,
    /// For write-multiple, the values as big-endian byte pairs.
    pub values: Registers<'a>,
}

/// Parse a request frame, for firmware that is the device on the bus.
pub fn parse_request(frame: &[u8]) -> Result<Incoming<'_>, Error> {
    verify(frame)?;
    let Some(function) = Function::from_code(frame[1]) else {
        return Err(Error::UnknownFunction(frame[1]));
    };
    let body = &frame[2..frame.len() - 2];
    let want = |n: usize| {
        if body.len() == n {
            Ok(())
        } else {
            Err(Error::BadLength)
        }
    };
    let (address, qv, values) = match function {
        Function::ReadHoldingRegisters
        | Function::ReadInputRegisters
        | Function::WriteSingleRegister => {
            want(4)?;
            let q = u16::from_be_bytes([body[2], body[3]]);
            if function != Function::WriteSingleRegister && (q == 0 || q > MAX_READ_REGISTERS) {
                return Err(Error::BadQuantity);
            }
            (u16::from_be_bytes([body[0], body[1]]), q, Registers(&[]))
        }
        Function::WriteMultipleRegisters => {
            if body.len() < 5 {
                return Err(Error::BadLength);
            }
            let q = u16::from_be_bytes([body[2], body[3]]);
            if q == 0 || q > MAX_WRITE_REGISTERS {
                return Err(Error::BadQuantity);
            }
            let bytes = body[4] as usize;
            if bytes != 2 * q as usize || body.len() != 5 + bytes {
                return Err(Error::WrongByteCount);
            }
            (
                u16::from_be_bytes([body[0], body[1]]),
                q,
                Registers(&body[5..]),
            )
        }
    };
    Ok(Incoming {
        unit: frame[0],
        function,
        address,
        quantity_or_value: qv,
        values,
    })
}

/// Write a read response carrying `registers`.
pub fn encode_read_response(
    out: &mut [u8],
    unit: u8,
    function: Function,
    registers: &[u16],
) -> Result<usize, Error> {
    check_unit(unit, false)?;
    if registers.is_empty() || registers.len() > MAX_READ_REGISTERS as usize {
        return Err(Error::BadQuantity);
    }
    let len = 3 + 2 * registers.len();
    if out.len() < len + 2 {
        return Err(Error::BufferTooSmall);
    }
    out[0] = unit;
    out[1] = function.code();
    out[2] = (2 * registers.len()) as u8;
    for (i, r) in registers.iter().enumerate() {
        out[3 + 2 * i..5 + 2 * i].copy_from_slice(&r.to_be_bytes());
    }
    seal(out, len)
}

/// Write the echo a device answers a write with.
pub fn encode_write_response(
    out: &mut [u8],
    unit: u8,
    function: Function,
    address: u16,
    quantity_or_value: u16,
) -> Result<usize, Error> {
    check_unit(unit, false)?;
    if out.len() < 8 {
        return Err(Error::BufferTooSmall);
    }
    out[0] = unit;
    out[1] = function.code();
    out[2..4].copy_from_slice(&address.to_be_bytes());
    out[4..6].copy_from_slice(&quantity_or_value.to_be_bytes());
    seal(out, 6)
}

/// Write an exception response: the function with its top bit set, and the code.
pub fn encode_exception(
    out: &mut [u8],
    unit: u8,
    function: Function,
    code: ExceptionCode,
) -> Result<usize, Error> {
    check_unit(unit, false)?;
    if out.len() < 5 {
        return Err(Error::BufferTooSmall);
    }
    out[0] = unit;
    out[1] = function.code() | 0x80;
    out[2] = code.code();
    seal(out, 3)
}

// ── Timing ───────────────────────────────────────────────────────────────────

/// The silences that frame an RTU message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// A gap longer than this inside a frame means the frame is corrupt (1.5
    /// character times).
    pub inter_char_us: u32,
    /// A silence this long ends a frame (3.5 character times).
    pub inter_frame_us: u32,
}

impl Timing {
    /// The timing for a baud rate. A character is 11 bits (start, 8 data, parity
    /// or a second stop, stop). Above 19200 baud the standard fixes the values at
    /// 750 and 1750 microseconds, because the arithmetic would otherwise ask for
    /// gaps shorter than most operating systems can measure.
    pub const fn for_baud(baud: u32) -> Timing {
        if baud > 19_200 {
            return Timing {
                inter_char_us: 750,
                inter_frame_us: 1750,
            };
        }
        // 11 bits per character, in microseconds, times 1.5 and 3.5.
        let char_us_x2 = 2 * 11 * 1_000_000 / baud;
        Timing {
            inter_char_us: char_us_x2 * 3 / 4,
            inter_frame_us: char_us_x2 * 7 / 4,
        }
    }
}

/// Collects bytes into frames by the silence between them.
pub struct FrameReceiver<const N: usize> {
    buf: [u8; N],
    len: usize,
    last_us: u64,
    corrupt: bool,
    timing: Timing,
}

impl<const N: usize> FrameReceiver<N> {
    /// A receiver using `timing`.
    pub const fn new(timing: Timing) -> Self {
        FrameReceiver {
            buf: [0; N],
            len: 0,
            last_us: 0,
            corrupt: false,
            timing,
        }
    }

    /// A byte arrived at `now_us` (a monotonic clock, in microseconds).
    pub fn feed(&mut self, byte: u8, now_us: u64) {
        if self.len > 0 {
            let gap = now_us.saturating_sub(self.last_us);
            if gap >= self.timing.inter_frame_us as u64 {
                // The old frame ended before this byte and nobody polled: it is
                // lost, and this byte starts a new one.
                self.len = 0;
                self.corrupt = false;
            } else if gap > self.timing.inter_char_us as u64 {
                self.corrupt = true;
            }
        }
        if self.len < N {
            self.buf[self.len] = byte;
            self.len += 1;
        } else {
            self.corrupt = true;
        }
        self.last_us = now_us;
    }

    /// Whether a frame has ended by `now_us`. Returns it once, or the reason it
    /// is unusable.
    pub fn poll(&mut self, now_us: u64) -> Option<Result<&[u8], Error>> {
        if self.len == 0 || now_us.saturating_sub(self.last_us) < self.timing.inter_frame_us as u64
        {
            return None;
        }
        let len = self.len;
        let corrupt = self.corrupt;
        self.len = 0;
        self.corrupt = false;
        Some(if corrupt {
            Err(Error::CorruptFrame)
        } else {
            Ok(&self.buf[..len])
        })
    }
}

// ── RS-485 direction ─────────────────────────────────────────────────────────

/// The driver-enable line of an RS-485 transceiver.
pub trait Transceiver {
    /// Drive the bus: the transceiver's transmitter is on.
    fn set_transmit(&mut self);
    /// Listen: the transmitter is off, so another node can drive.
    fn set_receive(&mut self);
}

/// A serial port that can write and say when it has finished.
pub trait SerialWrite {
    /// What a write can fail with.
    type Error;
    /// Queue `bytes`.
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
    /// Return only when the last bit has left the shifter. Releasing the bus
    /// before then cuts the last character off.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Send `frame` on a half-duplex bus: transmit, write, flush, and **always**
/// return to receive, including when the write failed. A node left in transmit
/// holds the bus and silences every other.
pub fn send<W: SerialWrite, D: Transceiver>(
    port: &mut W,
    direction: &mut D,
    frame: &[u8],
) -> Result<(), W::Error> {
    direction.set_transmit();
    let result = port.write_all(frame).and_then(|()| port.flush());
    direction.set_receive();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crc_matches_the_published_check_value() {
        // CRC-16/MODBUS of "123456789" is 0x4B37 (the catalogue check value).
        assert_eq!(crc16(b"123456789"), 0x4B37);
        assert_eq!(crc16(&[]), 0xFFFF);
    }

    #[test]
    fn a_read_request_is_the_documented_frame() {
        // Unit 1, read 3 holding registers from address 0: small enough to check by hand.
        let mut b = [0u8; 16];
        let n = encode_read(&mut b, 1, Function::ReadHoldingRegisters, 0, 3).unwrap();
        assert_eq!(&b[..n], &[0x01, 0x03, 0x00, 0x00, 0x00, 0x03, 0x05, 0xCB]);
    }

    #[test]
    fn a_widely_published_frame_has_the_published_crc() {
        // Read 10 holding registers from address 0 on unit 1: `01 03 00 00 00 0A C5 CD`.
        let mut b = [0u8; 16];
        let n = encode_read(&mut b, 1, Function::ReadHoldingRegisters, 0, 10).unwrap();
        assert_eq!(&b[..n], &[0x01, 0x03, 0x00, 0x00, 0x00, 0x0A, 0xC5, 0xCD]);
    }

    #[test]
    fn timing_follows_the_standard() {
        assert_eq!(
            Timing::for_baud(115_200),
            Timing {
                inter_char_us: 750,
                inter_frame_us: 1750
            }
        );
        let t = Timing::for_baud(9600);
        // 11 bits at 9600 baud is 1145.8 us a character: 1.5 is 1718 and 3.5 is 4010.
        assert!((1700..=1720).contains(&t.inter_char_us), "{t:?}");
        assert!((4000..=4020).contains(&t.inter_frame_us), "{t:?}");
        // Slower is longer.
        assert!(Timing::for_baud(4800).inter_frame_us > t.inter_frame_us);
    }
}

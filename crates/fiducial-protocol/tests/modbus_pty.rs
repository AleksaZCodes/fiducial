//! Modbus RTU over RS-485, through a real (pseudo) serial port.
//!
//! `openpty` makes a virtual serial cable: what one end writes the other reads,
//! through real file descriptors, with real blocking and real timing. A master
//! and a simulated device talk over it using only the public API. This machine has
//! no RS-485 adapter, so the line driver and the wires are out of reach; the
//! framing, the silence-based frame boundaries, the CRC, the exceptions and the
//! direction-control sequence are not.
#![cfg(unix)]

use std::fs::File;
use std::io::Write;
use std::os::fd::{FromRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fiducial_protocol::modbus::{
    decode_response, encode_exception, encode_read, encode_read_response, encode_write_multiple,
    encode_write_response, encode_write_single, parse_request, send, Error, ExceptionCode,
    FrameReceiver, Function, Request, Response, SerialWrite, Timing, Transceiver,
};

/// Generous on purpose: the test runs on a loaded machine, and a frame is one
/// burst of a dozen bytes, so these only have to be longer than a scheduling hiccup
/// and shorter than a human would wait.
const TIMING: Timing = Timing {
    inter_char_us: 20_000,
    inter_frame_us: 40_000,
};

fn pty_pair() -> (File, File) {
    let (mut master, mut slave): (RawFd, RawFd) = (0, 0);
    // SAFETY: `openpty` writes two file descriptors into the integers we pass, and
    // we take ownership of each exactly once below.
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    assert_eq!(rc, 0, "openpty failed");
    for fd in [master, slave] {
        // SAFETY: both are valid descriptors from openpty; termios is plain data.
        unsafe {
            let mut t: libc::termios = std::mem::zeroed();
            assert_eq!(libc::tcgetattr(fd, &mut t), 0);
            libc::cfmakeraw(&mut t);
            assert_eq!(libc::tcsetattr(fd, libc::TCSANOW, &t), 0);
        }
    }
    // SAFETY: each descriptor is valid and owned by nothing else.
    unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) }
}

/// Wait up to `timeout` for bytes; read what is there.
fn read_burst(file: &File, timeout: Duration) -> Vec<u8> {
    use std::os::fd::AsRawFd;
    let fd = file.as_raw_fd();
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: `pfd` is a valid pollfd for one descriptor.
    let ready = unsafe { libc::poll(&mut pfd, 1, timeout.as_millis() as i32) };
    if ready <= 0 {
        return Vec::new();
    }
    let mut buf = [0u8; 512];
    // SAFETY: `buf` is valid for `buf.len()` bytes.
    let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
    if n <= 0 {
        return Vec::new();
    }
    buf[..n as usize].to_vec()
}

fn micros(start: Instant) -> u64 {
    start.elapsed().as_micros() as u64
}

/// What the device saw, for the test to check.
#[derive(Debug, PartialEq)]
enum Seen {
    Served(Function, u16),
    Ignored(Error),
}

/// A device on unit 7 with 64 holding registers.
fn spawn_device(
    port: File,
    stop: Arc<AtomicBool>,
) -> (thread::JoinHandle<()>, mpsc::Receiver<Seen>) {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let start = Instant::now();
        let mut regs = [0u16; 64];
        for (i, r) in regs.iter_mut().enumerate() {
            *r = 1000 + i as u16;
        }
        let mut frames: FrameReceiver<260> = FrameReceiver::new(TIMING);
        let mut out = [0u8; 300];
        while !stop.load(Ordering::Relaxed) {
            for b in read_burst(&port, Duration::from_millis(5)) {
                frames.feed(b, micros(start));
            }
            let Some(frame) = frames.poll(micros(start)) else {
                continue;
            };
            let reply: Option<usize> = match frame.and_then(parse_request) {
                Err(e) => {
                    let _ = tx.send(Seen::Ignored(e));
                    None
                }
                Ok(req) if req.unit != 7 => None, // another device's frame
                Ok(req) => {
                    let _ = tx.send(Seen::Served(req.function, req.address));
                    let end = req.address as usize + req.quantity_or_value as usize;
                    match req.function {
                        Function::ReadHoldingRegisters | Function::ReadInputRegisters => {
                            if end > regs.len() {
                                encode_exception(
                                    &mut out,
                                    7,
                                    req.function,
                                    ExceptionCode::IllegalDataAddress,
                                )
                                .ok()
                            } else {
                                let s = &regs[req.address as usize..end];
                                encode_read_response(&mut out, 7, req.function, s).ok()
                            }
                        }
                        Function::WriteSingleRegister => {
                            regs[req.address as usize] = req.quantity_or_value;
                            encode_write_response(
                                &mut out,
                                7,
                                req.function,
                                req.address,
                                req.quantity_or_value,
                            )
                            .ok()
                        }
                        Function::WriteMultipleRegisters => {
                            for (i, v) in req.values.iter().enumerate() {
                                regs[req.address as usize + i] = v;
                            }
                            encode_write_response(
                                &mut out,
                                7,
                                req.function,
                                req.address,
                                req.quantity_or_value,
                            )
                            .ok()
                        }
                    }
                }
            };
            if let Some(n) = reply {
                (&port).write_all(&out[..n]).unwrap();
            }
        }
    });
    (handle, rx)
}

/// Send `frame` and read the reply, framed by silence like the device does.
fn transact(master: &File, frame: &[u8], request: &Request) -> Result<Vec<u16>, Error> {
    let mut m = master;
    m.write_all(frame).unwrap();
    let start = Instant::now();
    let mut frames: FrameReceiver<260> = FrameReceiver::new(TIMING);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        for b in read_burst(master, Duration::from_millis(5)) {
            frames.feed(b, micros(start));
        }
        if let Some(reply) = frames.poll(micros(start)) {
            return match decode_response(reply?, request)? {
                Response::Registers(r) => Ok(r.iter().collect()),
                Response::Written { .. } => Ok(Vec::new()),
            };
        }
    }
    Err(Error::BadLength) // no reply: this stands for "timed out" in the assertions
}

fn read_request(function: Function, address: u16, quantity: u16) -> (Vec<u8>, Request) {
    let mut b = [0u8; 16];
    let n = encode_read(&mut b, 7, function, address, quantity).unwrap();
    (
        b[..n].to_vec(),
        Request {
            unit: 7,
            function,
            address,
            quantity_or_value: quantity,
        },
    )
}

#[test]
fn a_master_reads_and_writes_a_device_over_a_serial_port() {
    let (master, slave) = pty_pair();
    let stop = Arc::new(AtomicBool::new(false));
    let (device, seen) = spawn_device(slave, stop.clone());

    // Read three holding registers.
    let (frame, req) = read_request(Function::ReadHoldingRegisters, 4, 3);
    assert_eq!(transact(&master, &frame, &req), Ok(vec![1004, 1005, 1006]));
    // The input-register function reaches the same map in this device.
    let (frame, req) = read_request(Function::ReadInputRegisters, 0, 2);
    assert_eq!(transact(&master, &frame, &req), Ok(vec![1000, 1001]));

    // Write one register and read it back.
    let mut b = [0u8; 16];
    let n = encode_write_single(&mut b, 7, 10, 0xBEEF).unwrap();
    let req = Request {
        unit: 7,
        function: Function::WriteSingleRegister,
        address: 10,
        quantity_or_value: 0xBEEF,
    };
    assert_eq!(transact(&master, &b[..n], &req), Ok(vec![]));
    let (frame, req) = read_request(Function::ReadHoldingRegisters, 10, 1);
    assert_eq!(transact(&master, &frame, &req), Ok(vec![0xBEEF]));

    // Write several, and read them back.
    let mut b = [0u8; 64];
    let n = encode_write_multiple(&mut b, 7, 20, &[1, 2, 3, 4]).unwrap();
    let req = Request {
        unit: 7,
        function: Function::WriteMultipleRegisters,
        address: 20,
        quantity_or_value: 4,
    };
    assert_eq!(transact(&master, &b[..n], &req), Ok(vec![]));
    let (frame, req) = read_request(Function::ReadHoldingRegisters, 19, 6);
    assert_eq!(
        transact(&master, &frame, &req),
        Ok(vec![1019, 1, 2, 3, 4, 1024])
    );

    stop.store(true, Ordering::Relaxed);
    device.join().unwrap();
    let served: Vec<_> = seen.try_iter().collect();
    assert_eq!(served.len(), 6, "{served:?}");
    assert!(
        served.iter().all(|s| matches!(s, Seen::Served(..))),
        "{served:?}"
    );
}

#[test]
fn a_device_refuses_an_address_it_does_not_have_and_says_why() {
    let (master, slave) = pty_pair();
    let stop = Arc::new(AtomicBool::new(false));
    let (device, _) = spawn_device(slave, stop.clone());

    let (frame, req) = read_request(Function::ReadHoldingRegisters, 60, 10); // past register 63
    assert_eq!(
        transact(&master, &frame, &req),
        Err(Error::Exception(ExceptionCode::IllegalDataAddress))
    );
    stop.store(true, Ordering::Relaxed);
    device.join().unwrap();
}

#[test]
fn a_damaged_frame_is_ignored_by_the_device_and_never_answered() {
    let (master, slave) = pty_pair();
    let stop = Arc::new(AtomicBool::new(false));
    let (device, seen) = spawn_device(slave, stop.clone());

    let (mut frame, req) = read_request(Function::ReadHoldingRegisters, 0, 2);
    frame[3] ^= 0x40; // a flipped bit in the address
    assert_eq!(
        transact(&master, &frame, &req),
        Err(Error::BadLength),
        "no reply expected"
    );
    assert_eq!(
        seen.recv_timeout(Duration::from_secs(1)),
        Ok(Seen::Ignored(Error::BadCrc))
    );

    // The bus is not wedged: the next good request is served.
    let (frame, req) = read_request(Function::ReadHoldingRegisters, 0, 2);
    assert_eq!(transact(&master, &frame, &req), Ok(vec![1000, 1001]));
    stop.store(true, Ordering::Relaxed);
    device.join().unwrap();
}

#[test]
fn a_pause_inside_a_frame_corrupts_it_and_a_request_for_another_unit_is_not_answered() {
    let (master, slave) = pty_pair();
    let stop = Arc::new(AtomicBool::new(false));
    let (device, seen) = spawn_device(slave, stop.clone());

    // Half a frame, a pause longer than 1.5 character times but shorter than 3.5,
    // then the rest: the standard says the frame is corrupt.
    let (frame, _) = read_request(Function::ReadHoldingRegisters, 0, 2);
    let mut m = &master;
    m.write_all(&frame[..4]).unwrap();
    thread::sleep(Duration::from_millis(30));
    m.write_all(&frame[4..]).unwrap();
    assert_eq!(
        seen.recv_timeout(Duration::from_secs(1)),
        Ok(Seen::Ignored(Error::CorruptFrame))
    );

    // A well-formed request for unit 9 is on the wire but is not this device's.
    let mut b = [0u8; 16];
    let n = encode_read(&mut b, 9, Function::ReadHoldingRegisters, 0, 1).unwrap();
    let req = Request {
        unit: 9,
        function: Function::ReadHoldingRegisters,
        address: 0,
        quantity_or_value: 1,
    };
    assert_eq!(
        transact(&master, &b[..n], &req),
        Err(Error::BadLength),
        "unit 9 must not be answered by unit 7"
    );
    stop.store(true, Ordering::Relaxed);
    device.join().unwrap();
}

#[test]
fn a_response_is_checked_against_the_request_it_answers() {
    let mut b = [0u8; 32];
    let req = Request {
        unit: 7,
        function: Function::ReadHoldingRegisters,
        address: 0,
        quantity_or_value: 2,
    };
    let n = encode_read_response(&mut b, 7, Function::ReadHoldingRegisters, &[1, 2]).unwrap();
    assert!(decode_response(&b[..n], &req).is_ok());
    // Another unit's reply, the wrong function, the wrong number of registers.
    let n = encode_read_response(&mut b, 8, Function::ReadHoldingRegisters, &[1, 2]).unwrap();
    assert_eq!(decode_response(&b[..n], &req), Err(Error::WrongUnit));
    let n = encode_read_response(&mut b, 7, Function::ReadInputRegisters, &[1, 2]).unwrap();
    assert_eq!(decode_response(&b[..n], &req), Err(Error::WrongFunction));
    let n = encode_read_response(&mut b, 7, Function::ReadHoldingRegisters, &[1, 2, 3]).unwrap();
    assert_eq!(decode_response(&b[..n], &req), Err(Error::WrongByteCount));
    // A write echo that does not echo.
    let w = Request {
        unit: 7,
        function: Function::WriteSingleRegister,
        address: 5,
        quantity_or_value: 9,
    };
    let n = encode_write_response(&mut b, 7, Function::WriteSingleRegister, 5, 10).unwrap();
    assert_eq!(decode_response(&b[..n], &w), Err(Error::WrongEcho));
}

#[test]
fn requests_that_cannot_exist_are_refused_before_they_are_sent() {
    let mut b = [0u8; 300];
    let r = Function::ReadHoldingRegisters;
    assert_eq!(
        encode_read(&mut b, 0, r, 0, 1),
        Err(Error::BadUnit),
        "a read cannot be broadcast"
    );
    assert_eq!(encode_read(&mut b, 248, r, 0, 1), Err(Error::BadUnit));
    assert_eq!(encode_read(&mut b, 1, r, 0, 0), Err(Error::BadQuantity));
    assert_eq!(encode_read(&mut b, 1, r, 0, 126), Err(Error::BadQuantity));
    assert!(encode_read(&mut b, 1, r, 0, 125).is_ok());
    assert_eq!(
        encode_write_multiple(&mut b, 1, 0, &[]),
        Err(Error::BadQuantity)
    );
    assert_eq!(
        encode_write_multiple(&mut b, 1, 0, &[0; 124]),
        Err(Error::BadQuantity)
    );
    assert!(
        encode_write_multiple(&mut b, 0, 0, &[0; 123]).is_ok(),
        "a write may be broadcast"
    );
    assert_eq!(
        encode_read(&mut [0u8; 4], 1, r, 0, 1),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        encode_read(&mut b, 1, Function::WriteSingleRegister, 0, 1),
        Err(Error::WrongFunction)
    );
}

// ── Direction control ────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum Step {
    Transmit,
    Write(Vec<u8>),
    Flush,
    Receive,
}

struct Log(std::cell::RefCell<Vec<Step>>);

struct Mock<'a> {
    log: &'a Log,
    fail_write: bool,
}
impl SerialWrite for Mock<'_> {
    type Error = &'static str;
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        self.log.0.borrow_mut().push(Step::Write(bytes.to_vec()));
        if self.fail_write {
            Err("write failed")
        } else {
            Ok(())
        }
    }
    fn flush(&mut self) -> Result<(), &'static str> {
        self.log.0.borrow_mut().push(Step::Flush);
        Ok(())
    }
}
struct Dir<'a>(&'a Log);
impl Transceiver for Dir<'_> {
    fn set_transmit(&mut self) {
        self.0 .0.borrow_mut().push(Step::Transmit);
    }
    fn set_receive(&mut self) {
        self.0 .0.borrow_mut().push(Step::Receive);
    }
}

#[test]
fn the_bus_is_driven_only_while_a_frame_is_going_out() {
    let log = Log(Default::default());
    send(
        &mut Mock {
            log: &log,
            fail_write: false,
        },
        &mut Dir(&log),
        &[1, 2, 3],
    )
    .unwrap();
    assert_eq!(
        *log.0.borrow(),
        vec![
            Step::Transmit,
            Step::Write(vec![1, 2, 3]),
            Step::Flush,
            Step::Receive
        ]
    );
}

#[test]
fn a_failed_write_still_releases_the_bus() {
    // A node stuck in transmit silences every other node on the wire.
    let log = Log(Default::default());
    let r = send(
        &mut Mock {
            log: &log,
            fail_write: true,
        },
        &mut Dir(&log),
        &[9],
    );
    assert_eq!(r, Err("write failed"));
    assert_eq!(
        *log.0.borrow(),
        vec![Step::Transmit, Step::Write(vec![9]), Step::Receive]
    );
}

/// A real descriptor behind the same trait: `flush` is `tcdrain`, which returns
/// when the last byte has been sent, and the line is released only after it.
struct Port<'a> {
    file: &'a File,
    flushed: &'a AtomicBool,
}
impl SerialWrite for Port<'_> {
    type Error = std::io::Error;
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut f = self.file;
        f.write_all(bytes)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        use std::os::fd::AsRawFd;
        // SAFETY: the descriptor is valid for the life of `self`.
        let rc = unsafe { libc::tcdrain(self.file.as_raw_fd()) };
        self.flushed.store(true, Ordering::SeqCst);
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}
struct Flagged<'a>(&'a AtomicBool, &'a AtomicBool);
impl Transceiver for Flagged<'_> {
    fn set_transmit(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
    fn set_receive(&mut self) {
        // Released only after the drain.
        assert!(
            self.0.load(Ordering::SeqCst),
            "the bus was released before the frame had drained"
        );
        self.1.store(true, Ordering::SeqCst);
    }
}

#[test]
fn a_real_port_is_released_after_it_drains() {
    let (master, slave) = pty_pair();
    let flushed = AtomicBool::new(false);
    let released = AtomicBool::new(false);
    let mut frame = [0u8; 16];
    let n = encode_read(&mut frame, 7, Function::ReadHoldingRegisters, 0, 2).unwrap();
    send(
        &mut Port {
            file: &master,
            flushed: &flushed,
        },
        &mut Flagged(&flushed, &released),
        &frame[..n],
    )
    .unwrap();
    assert!(released.load(Ordering::SeqCst));
    // The bytes are on the cable.
    let got = read_burst(&slave, Duration::from_secs(1));
    assert_eq!(got, &frame[..n]);
}

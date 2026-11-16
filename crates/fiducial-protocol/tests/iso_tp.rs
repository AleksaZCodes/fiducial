//! ISO-TP over an in-memory bus.
//!
//! This machine's kernel has no CAN (no `vcan`, no SocketCAN), so a real bus is
//! out of reach here. What can be proved is the protocol: a sender and a receiver
//! exchanging frames through nothing but the public API, with the bus's rules
//! (frame sizes, CAN-FD lengths) checked on every frame, and the exact bytes of
//! each frame kind pinned.

use fiducial_protocol::can::{len_to_dlc, FD_MAX};
use fiducial_protocol::segment::{
    encode_flow_control, Error, Event, FlowControl, FlowStatus, Mode, Params, Poll, Receiver,
    Sender, StMin,
};

struct Run {
    /// Every frame the sender put on the bus.
    frames: Vec<Vec<u8>>,
    /// Every flow-control frame the receiver sent back.
    flow_control: Vec<Vec<u8>>,
    message: Vec<u8>,
    /// Microseconds the sender spent waiting out separation times.
    waited_us: u64,
}

fn data(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + 3) as u8).collect()
}

fn transfer<const N: usize>(msg: &[u8], mode: Mode, tx: Params, rx: Params) -> Result<Run, Error> {
    let mut sender = Sender::new(msg, mode, tx)?;
    let mut receiver: Receiver<N> = Receiver::new(rx);
    let mut out = [0u8; FD_MAX];
    let mut run = Run {
        frames: Vec::new(),
        flow_control: Vec::new(),
        message: Vec::new(),
        waited_us: 0,
    };
    let mut guard = 0u32;
    loop {
        guard += 1;
        assert!(guard < 2_000_000, "the transfer does not terminate");
        match sender.poll(&mut out)? {
            Poll::Frame(n) => {
                let frame = out[..n].to_vec();
                match mode {
                    Mode::Classic => assert!(n <= 8, "a classic frame of {n} bytes"),
                    Mode::Fd => assert!(
                        len_to_dlc(n).is_some(),
                        "a CAN-FD frame of {n} bytes has no length code"
                    ),
                }
                run.frames.push(frame.clone());
                match receiver.feed(&frame)? {
                    Event::Pending => {}
                    Event::Message(m) => run.message = m.to_vec(),
                    Event::SendFlowControl(fc) => {
                        let mut buf = [0u8; FD_MAX];
                        let k = encode_flow_control(fc, mode, rx.padding, &mut buf);
                        run.flow_control.push(buf[..k].to_vec());
                        sender.on_flow_control(&buf[..k])?;
                    }
                }
            }
            Poll::Wait => {
                sender.tick(100)?;
                receiver.tick(100)?;
                run.waited_us += 100;
            }
            Poll::Done => break,
        }
    }
    Ok(run)
}

fn params(block_size: u8, st_min: u8, padding: Option<u8>) -> Params {
    Params {
        block_size,
        st_min: StMin(st_min),
        padding,
        ..Params::default()
    }
}

// ── The exact bytes of each frame kind ───────────────────────────────────────

#[test]
fn a_short_message_is_a_single_frame() {
    let run = transfer::<64>(
        &[0x22, 0xF1, 0x90],
        Mode::Classic,
        Params::default(),
        Params::default(),
    )
    .unwrap();
    assert_eq!(
        run.frames,
        vec![vec![0x03, 0x22, 0xF1, 0x90, 0xCC, 0xCC, 0xCC, 0xCC]]
    );
    assert!(
        run.flow_control.is_empty(),
        "a single frame needs no flow control"
    );
    assert_eq!(run.message, [0x22, 0xF1, 0x90]);
}

#[test]
fn padding_is_optional_on_classic_can_and_required_on_fd() {
    let none = params(0, 0, None);
    let run = transfer::<64>(&[1, 2, 3], Mode::Classic, none, none).unwrap();
    assert_eq!(run.frames, vec![vec![0x03, 1, 2, 3]], "unpadded classic");

    // Five bytes plus the length byte is six, a valid FD length: no padding needed.
    let run = transfer::<64>(&[1, 2, 3, 4, 5], Mode::Fd, none, none).unwrap();
    assert_eq!(run.frames[0].len(), 6);
    // Twenty bytes use the escape form (00 LL) and pad 22 up to 24.
    let run = transfer::<64>(&data(20), Mode::Fd, none, none).unwrap();
    assert_eq!(run.frames.len(), 1);
    assert_eq!(&run.frames[0][..2], &[0x00, 0x14]);
    assert_eq!(run.frames[0].len(), 24);
    assert!(run.frames[0][22..].iter().all(|&b| b == 0xCC));
}

#[test]
fn a_first_frame_carries_the_length_and_a_consecutive_frame_the_sequence() {
    let msg = data(20);
    let run = transfer::<64>(&msg, Mode::Classic, Params::default(), Params::default()).unwrap();
    // 6 bytes in the first frame, then 7 and 7.
    assert_eq!(run.frames.len(), 3);
    assert_eq!(run.frames[0], [&[0x10, 0x14][..], &msg[..6]].concat());
    assert_eq!(run.frames[1], [&[0x21][..], &msg[6..13]].concat());
    assert_eq!(run.frames[2], [&[0x22][..], &msg[13..20]].concat());
    assert_eq!(
        run.flow_control,
        vec![vec![0x30, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]]
    );
    assert_eq!(run.message, msg);
}

#[test]
fn a_message_too_long_for_twelve_bits_uses_the_escape_length() {
    let msg = data(5000);
    let run = transfer::<8192>(&msg, Mode::Classic, Params::default(), Params::default()).unwrap();
    // 10 00, the 32-bit length 5000 = 0x1388, then 2 data bytes.
    assert_eq!(&run.frames[0][..6], &[0x10, 0x00, 0x00, 0x00, 0x13, 0x88]);
    assert_eq!(&run.frames[0][6..], &msg[..2]);
    assert_eq!(run.message, msg);

    // The largest the short form can say is 4095; 4096 needs the escape.
    let at = transfer::<8192>(
        &data(4095),
        Mode::Classic,
        Params::default(),
        Params::default(),
    )
    .unwrap();
    assert_eq!(&at.frames[0][..2], &[0x1F, 0xFF]);
    let over = transfer::<8192>(
        &data(4096),
        Mode::Classic,
        Params::default(),
        Params::default(),
    )
    .unwrap();
    assert_eq!(&over.frames[0][..6], &[0x10, 0x00, 0x00, 0x00, 0x10, 0x00]);
}

#[test]
fn the_sequence_number_wraps_from_fifteen_to_zero() {
    let msg = data(6 + 7 * 20);
    let run = transfer::<256>(&msg, Mode::Classic, Params::default(), Params::default()).unwrap();
    let sns: Vec<u8> = run.frames[1..].iter().map(|f| f[0] & 0x0F).collect();
    assert_eq!(
        sns[..17],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1]
    );
    assert_eq!(run.message, msg);
}

#[test]
fn can_fd_carries_more_per_frame() {
    let msg = data(1000);
    let classic =
        transfer::<1024>(&msg, Mode::Classic, Params::default(), Params::default()).unwrap();
    let fd = transfer::<1024>(&msg, Mode::Fd, Params::default(), Params::default()).unwrap();
    assert_eq!(fd.message, msg);
    // 62 bytes in the first frame, then 63 in each: 1 + ceil(938 / 63).
    assert_eq!(fd.frames.len(), 1 + (1000usize - 62).div_ceil(63));
    assert!(
        fd.frames.len() * 8 < classic.frames.len(),
        "{} vs {}",
        fd.frames.len(),
        classic.frames.len()
    );
}

// ── Many transfers, with the counts held exactly ─────────────────────────────

#[test]
fn every_length_round_trips_in_every_mode_with_the_expected_frames_and_gaps() {
    let mut lengths: Vec<usize> = (1..=130).collect();
    lengths.extend([254, 255, 256, 257, 4094, 4095, 4096, 4097, 5000]);
    let mut transfers = 0u32;
    for mode in [Mode::Classic, Mode::Fd] {
        for padding in [Some(0xCC), None, Some(0x55)] {
            for bs in [0u8, 1, 3, 15, 255] {
                for stmin in [0u8, 5, 0xF3] {
                    for &len in &lengths {
                        let msg = data(len);
                        let rx = params(bs, stmin, padding);
                        let tx = Params {
                            padding,
                            ..Params::default()
                        };
                        let run = transfer::<8192>(&msg, mode, tx, rx).unwrap();
                        assert_eq!(run.message, msg, "{mode:?} len {len} bs {bs}");
                        transfers += 1;

                        // How many frames the standard says.
                        let (single_max, ff_data, cf_data) = match mode {
                            Mode::Classic => (7usize, if len <= 4095 { 6 } else { 2 }, 7usize),
                            Mode::Fd => (62, if len <= 4095 { 62 } else { 58 }, 63),
                        };
                        let expected_cf = if len <= single_max {
                            0
                        } else {
                            (len - ff_data).div_ceil(cf_data)
                        };
                        let expected_frames = if len <= single_max {
                            1
                        } else {
                            1 + expected_cf
                        };
                        assert_eq!(run.frames.len(), expected_frames, "{mode:?} len {len}");

                        // One flow control after the first frame, and one more at
                        // the end of every block that leaves data to send.
                        let expected_fc = if expected_cf == 0 {
                            0
                        } else if bs == 0 {
                            1
                        } else {
                            expected_cf.div_ceil(bs as usize)
                        };
                        assert_eq!(
                            run.flow_control.len(),
                            expected_fc,
                            "{mode:?} len {len} bs {bs}"
                        );

                        // The separation time is kept between every pair of
                        // consecutive frames inside a block, and nowhere else.
                        let gaps = expected_cf.saturating_sub(expected_fc);
                        assert_eq!(
                            run.waited_us,
                            gaps as u64 * StMin(stmin).micros() as u64,
                            "{mode:?} len {len} bs {bs} stmin {stmin:#x}"
                        );
                    }
                }
            }
        }
    }
    assert!(transfers > 10_000, "only {transfers} transfers ran");
}

#[test]
fn the_separation_time_encoding_is_the_standards() {
    assert_eq!(StMin(0x00).micros(), 0);
    assert_eq!(StMin(0x7F).micros(), 127_000);
    assert_eq!(StMin(0xF1).micros(), 100);
    assert_eq!(StMin(0xF9).micros(), 900);
    // 0x80..=0xF0 and 0xFA..=0xFF are reserved: the largest valid value.
    for reserved in [0x80u8, 0xF0, 0xFA, 0xFF] {
        assert_eq!(StMin(reserved).micros(), 127_000, "{reserved:#x}");
    }
}

// ── What goes wrong ──────────────────────────────────────────────────────────

fn frames_of(msg: &[u8]) -> Vec<Vec<u8>> {
    transfer::<256>(msg, Mode::Classic, Params::default(), Params::default())
        .unwrap()
        .frames
}

#[test]
fn a_lost_frame_is_a_sequence_error_and_the_receiver_recovers() {
    let frames = frames_of(&data(40));
    let mut rx: Receiver<64> = Receiver::new(Params::default());
    assert!(matches!(rx.feed(&frames[0]), Ok(Event::SendFlowControl(_))));
    assert!(matches!(rx.feed(&frames[1]), Ok(Event::Pending)));
    // frames[2] is lost.
    assert_eq!(
        rx.feed(&frames[3]),
        Err(Error::WrongSequence {
            expected: 2,
            got: 3
        })
    );
    // The transfer is abandoned: a stray consecutive frame is now unexpected.
    assert_eq!(rx.feed(&frames[4]), Err(Error::UnexpectedFrame));
    // And a fresh message goes through.
    assert!(matches!(rx.feed(&frames[0]), Ok(Event::SendFlowControl(_))));
}

#[test]
fn a_repeated_frame_is_a_sequence_error() {
    let frames = frames_of(&data(40));
    let mut rx: Receiver<64> = Receiver::new(Params::default());
    rx.feed(&frames[0]).unwrap();
    rx.feed(&frames[1]).unwrap();
    assert_eq!(
        rx.feed(&frames[1]),
        Err(Error::WrongSequence {
            expected: 2,
            got: 1
        })
    );
}

#[test]
fn a_consecutive_frame_with_no_first_frame_is_unexpected() {
    let frames = frames_of(&data(40));
    let mut rx: Receiver<64> = Receiver::new(Params::default());
    assert_eq!(rx.feed(&frames[1]), Err(Error::UnexpectedFrame));
    // So is flow control: a receiver never expects one.
    assert_eq!(
        rx.feed(&[0x30, 0, 0, 0, 0, 0, 0, 0]),
        Err(Error::UnexpectedFrame)
    );
}

#[test]
fn a_message_too_big_for_the_receiver_is_refused_with_overflow() {
    let r = transfer::<8>(
        &data(20),
        Mode::Classic,
        Params::default(),
        Params::default(),
    );
    assert!(
        matches!(r, Err(Error::Overflow)),
        "{:?}",
        r.map(|r| r.frames.len())
    );

    // The receiver's verdict is on the bus as a flow-control frame.
    let frames = frames_of(&data(20));
    let mut rx: Receiver<8> = Receiver::new(Params::default());
    match rx.feed(&frames[0]).unwrap() {
        Event::SendFlowControl(fc) => assert_eq!(fc.status, FlowStatus::Overflow),
        other => panic!("{other:?}"),
    }
    assert!(!rx.is_receiving());
    // A single frame too big for the buffer is an error too.
    let single: Receiver<2> = Receiver::new(Params::default());
    let mut single = single;
    assert_eq!(
        single.feed(&[0x03, 1, 2, 3, 0, 0, 0, 0]),
        Err(Error::Overflow)
    );
}

#[test]
fn silence_times_out_on_both_sides_and_the_sender_stays_failed() {
    let msg = data(40);
    let mut tx = Sender::new(&msg, Mode::Classic, Params::default()).unwrap();
    let mut out = [0u8; FD_MAX];
    assert!(matches!(tx.poll(&mut out), Ok(Poll::Frame(8))));
    assert!(tx.awaiting_flow_control());
    assert_eq!(tx.poll(&mut out), Ok(Poll::Wait));
    tx.tick(999_999).unwrap();
    assert_eq!(tx.tick(1), Err(Error::Timeout));
    assert_eq!(tx.poll(&mut out), Err(Error::Aborted));

    let frames = frames_of(&msg);
    let mut rx: Receiver<64> = Receiver::new(Params::default());
    rx.feed(&frames[0]).unwrap();
    rx.tick(500_000).unwrap();
    // A frame arriving restarts the clock.
    rx.feed(&frames[1]).unwrap();
    rx.tick(999_999).unwrap();
    assert_eq!(rx.tick(1), Err(Error::Timeout));
    assert!(!rx.is_receiving());
    assert_eq!(rx.feed(&frames[2]), Err(Error::UnexpectedFrame));
}

#[test]
fn the_receiver_can_ask_the_sender_to_wait_but_not_for_ever() {
    let msg = data(40);
    let fc = |status| {
        let mut b = [0u8; FD_MAX];
        let n = encode_flow_control(
            FlowControl {
                status,
                block_size: 0,
                st_min: StMin(0),
            },
            Mode::Classic,
            Some(0xCC),
            &mut b,
        );
        b[..n].to_vec()
    };
    let mut out = [0u8; FD_MAX];

    let mut tx = Sender::new(&msg, Mode::Classic, Params::default()).unwrap();
    tx.poll(&mut out).unwrap();
    for _ in 0..10 {
        tx.on_flow_control(&fc(FlowStatus::Wait)).unwrap();
    }
    assert_eq!(
        tx.on_flow_control(&fc(FlowStatus::Wait)),
        Err(Error::TooManyWaits)
    );
    assert_eq!(tx.poll(&mut out), Err(Error::Aborted));

    // A wait restarts the N_Bs clock, and a go after waits proceeds.
    let mut tx = Sender::new(&msg, Mode::Classic, Params::default()).unwrap();
    tx.poll(&mut out).unwrap();
    tx.tick(900_000).unwrap();
    tx.on_flow_control(&fc(FlowStatus::Wait)).unwrap();
    tx.tick(900_000).unwrap();
    tx.on_flow_control(&fc(FlowStatus::ContinueToSend)).unwrap();
    assert!(matches!(tx.poll(&mut out), Ok(Poll::Frame(_))));

    // Flow control nobody asked for is refused.
    let mut idle = Sender::new(&msg, Mode::Classic, Params::default()).unwrap();
    assert_eq!(
        idle.on_flow_control(&fc(FlowStatus::ContinueToSend)),
        Err(Error::UnexpectedFrame)
    );
}

#[test]
fn frames_that_are_not_iso_tp_are_refused_not_guessed_at() {
    let mut rx: Receiver<64> = Receiver::new(Params::default());
    let bad: &[&[u8]] = &[
        &[],                          // nothing at all
        &[0x40, 1, 2, 3, 4, 5, 6, 7], // a reserved frame type
        &[0xF0, 1, 2, 3, 4, 5, 6, 7],
        &[0x00, 1, 2, 3, 4, 5, 6, 7],    // a single frame of length 0
        &[0x07, 1, 2],                   // a length the frame cannot hold
        &[0x00, 5, 1, 2, 3, 4, 5, 0],    // the escape form in an 8-byte frame
        &[0x10, 0x05, 1, 2, 3, 4, 5, 6], // a "first frame" of 5 bytes: it fits one frame
        &[0x10, 0x00, 0, 0, 0x0F, 0xFF, 1, 2], // the escape for a length the short form holds
        &[0x10, 0x14, 1, 2, 3],          // a first frame that is not a full frame
    ];
    for frame in bad {
        assert_eq!(rx.feed(frame), Err(Error::InvalidFrame), "{frame:02X?}");
    }
    // A first frame of an invalid CAN-FD length (10 bytes has no length code).
    let mut ten = vec![0x10, 0x40];
    ten.extend([0u8; 8]);
    assert_eq!(rx.feed(&ten), Err(Error::InvalidFrame));
    // An empty message cannot be sent.
    assert!(matches!(
        Sender::new(&[], Mode::Classic, Params::default()),
        Err(Error::EmptyMessage)
    ));
}

#[test]
fn a_new_single_frame_abandons_a_message_in_progress() {
    let frames = frames_of(&data(40));
    let mut rx: Receiver<64> = Receiver::new(Params::default());
    rx.feed(&frames[0]).unwrap();
    assert!(rx.is_receiving());
    match rx.feed(&[0x02, 0xAA, 0xBB, 0, 0, 0, 0, 0]).unwrap() {
        Event::Message(m) => assert_eq!(m, [0xAA, 0xBB]),
        other => panic!("{other:?}"),
    }
    assert!(!rx.is_receiving());
    assert_eq!(rx.feed(&frames[1]), Err(Error::UnexpectedFrame));
}

#[test]
fn a_receiver_that_asks_for_blocks_gets_them() {
    let msg = data(100);
    let rx = params(4, 0, Some(0xCC));
    let run = transfer::<256>(&msg, Mode::Classic, Params::default(), rx).unwrap();
    assert_eq!(run.message, msg);
    // Every flow control carries the receiver's block size and separation time.
    for fc in &run.flow_control {
        assert_eq!(&fc[..3], &[0x30, 4, 0]);
    }
}

/// Random garbage must never panic the receiver or the sender's flow-control
/// parser, whatever length and bytes arrive.
#[test]
fn garbage_frames_never_panic() {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut rx: Receiver<64> = Receiver::new(Params::default());
    let msg = data(300);
    let mut out = [0u8; FD_MAX];
    for _ in 0..100_000 {
        let len = (next() % 72) as usize;
        let frame: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        let _ = rx.feed(&frame);
        let _ = rx.tick((next() % 2_000_000) as u32);

        let mut tx = Sender::new(&msg, Mode::Fd, Params::default()).unwrap();
        tx.poll(&mut out).unwrap();
        let _ = tx.on_flow_control(&frame);
        let _ = tx.poll(&mut out);
    }
}

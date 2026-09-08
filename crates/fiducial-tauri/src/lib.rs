//! Serial transport and device discovery for Fiducial Tauri desktop apps.
//!
//! Wraps `serialport` with the Fiducial frame protocol from `fiducial-protocol`,
//! providing a typed interface for discovering and communicating with Fiducial
//! devices over USB serial.
//!
//! # Usage
//!
//! ```rust,no_run
//! use fiducial_tauri::{list_ports, SerialTransport};
//!
//! for port in list_ports().unwrap() {
//!     println!("{}: {}", port.name, port.description);
//! }
//!
//! let mut transport = SerialTransport::open("/dev/ttyUSB0", 115200).unwrap();
//! transport.send(b"ping").unwrap();
//! ```

use fiducial_protocol::{encode, encoded_len, FrameDecoder};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::time::Duration;
use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors from the serial transport.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Serial port I/O error.
    #[error("serial port error: {0}")]
    Serial(#[from] serialport::Error),
    /// I/O error on read or write.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Frame encoding failed (payload too large).
    #[error("frame encode error: payload too large")]
    EncodeTooLarge,
    /// No complete frame received within the timeout.
    #[error("read timeout — no frame received")]
    Timeout,
}

impl From<fiducial_protocol::EncodeError> for TransportError {
    fn from(_: fiducial_protocol::EncodeError) -> Self {
        TransportError::EncodeTooLarge
    }
}

// ── PortInfo ──────────────────────────────────────────────────────────────────

/// Information about an available serial port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    /// System port name, e.g. `/dev/ttyUSB0` or `COM3`.
    pub name: String,
    /// Human-readable description from the OS.
    pub description: String,
    /// USB vendor ID, if known.
    pub vid: Option<u16>,
    /// USB product ID, if known.
    pub pid: Option<u16>,
}

/// List all available serial ports on this system.
pub fn list_ports() -> Result<Vec<PortInfo>, TransportError> {
    let ports = serialport::available_ports()?;
    Ok(ports
        .into_iter()
        .map(|p| {
            let (description, vid, pid) = match &p.port_type {
                serialport::SerialPortType::UsbPort(info) => (
                    info.product
                        .clone()
                        .unwrap_or_else(|| "USB Serial Device".into()),
                    Some(info.vid),
                    Some(info.pid),
                ),
                _ => (p.port_name.clone(), None, None),
            };
            PortInfo {
                name: p.port_name,
                description,
                vid,
                pid,
            }
        })
        .collect())
}

// ── SerialTransport ───────────────────────────────────────────────────────────

/// An open serial connection that speaks the Fiducial frame protocol.
pub struct SerialTransport {
    port: Box<dyn serialport::SerialPort>,
    decoder: FrameDecoder<4096>,
}

impl SerialTransport {
    /// Open a serial port at the given path and baud rate.
    ///
    /// Sets a 500 ms read timeout; adjust with [`SerialTransport::set_timeout`].
    pub fn open(path: &str, baud_rate: u32) -> Result<Self, TransportError> {
        let port = serialport::new(path, baud_rate)
            .timeout(Duration::from_millis(500))
            .open()?;
        Ok(Self {
            port,
            decoder: FrameDecoder::new(),
        })
    }

    /// Change the read timeout.
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), TransportError> {
        self.port.set_timeout(timeout)?;
        Ok(())
    }

    /// Send `payload` as a framed message.
    pub fn send(&mut self, payload: &[u8]) -> Result<(), TransportError> {
        let mut buf = vec![0u8; encoded_len(payload.len())];
        let n = encode(payload, &mut buf)?;
        self.port.write_all(&buf[..n])?;
        Ok(())
    }

    /// Read bytes until one complete frame is decoded, then return its payload.
    ///
    /// Returns [`TransportError::Timeout`] if no frame arrives within the port
    /// read timeout.
    pub fn recv(&mut self) -> Result<Vec<u8>, TransportError> {
        let mut byte = [0u8; 1];
        loop {
            match self.port.read(&mut byte) {
                Ok(0) => return Err(TransportError::Timeout),
                Ok(_) => {
                    if let Some(payload) = self.decoder.feed(byte[0]) {
                        return Ok(payload.to_vec());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    return Err(TransportError::Timeout);
                }
                Err(e) => return Err(TransportError::Io(e)),
            }
        }
    }
}

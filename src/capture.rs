//! modbusio's own traffic while a capture runs, for Wireshark (concept section 11). Kept in
//! memory; `pcap.rs` writes it as a file.

use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::endpoint::Endpoint;

/// Frames kept at most, so a forgotten capture cannot fill the memory.
pub const MOST_FRAMES: usize = 200_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub time: SystemTime,
    pub direction: Direction,
    /// Local and remote address on TCP; `None` on a serial port.
    pub addresses: Option<(SocketAddr, SocketAddr)>,
    pub bytes: Vec<u8>,
    /// RTU frames (serial, or RTU over TCP) rather than Modbus TCP.
    pub rtu: bool,
}

#[derive(Default)]
pub struct Capture {
    frames: Mutex<Option<Vec<Frame>>>,
}

impl Capture {
    pub fn start(&self) {
        *self.frames.lock().expect("capture lock") = Some(Vec::new());
    }

    /// Ends the capture and hands over what it holds; `None` when none ran.
    pub fn stop(&self) -> Option<Vec<Frame>> {
        self.frames.lock().expect("capture lock").take()
    }

    /// How many frames the running capture holds; `None` when none runs.
    pub fn count(&self) -> Option<usize> {
        self.frames
            .lock()
            .expect("capture lock")
            .as_ref()
            .map(Vec::len)
    }

    pub fn record(
        &self,
        endpoint: &Endpoint,
        addresses: Option<(SocketAddr, SocketAddr)>,
        direction: Direction,
        bytes: &[u8],
    ) {
        let mut frames = self.frames.lock().expect("capture lock");
        let Some(frames) = frames.as_mut() else {
            return;
        };
        if frames.len() >= MOST_FRAMES {
            return;
        }
        frames.push(Frame {
            time: SystemTime::now(),
            direction,
            addresses,
            bytes: bytes.to_vec(),
            rtu: endpoint.is_serial_line(),
        });
    }
}

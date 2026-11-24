//! The Modbus protocol without I/O: PDUs, TCP and RTU framing. Bytes in, bytes out, so every
//! case is testable without a device (concept section 9).
//!
//! A crate of its own so that iecio, the controller runtime, speaks Modbus with the same code
//! (concept section 14, answer 6).

pub mod pdu;
pub mod rtu;
pub mod tcp;

pub use pdu::{Area, Exception, Identity, PduError, Request, Response};

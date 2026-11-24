//! Profiles of Modbus devices (concept section 6): what the registers of a kind of device mean,
//! and how registers become values and back. No I/O. Shared with iecio, which reads a profile as
//! the description of a module (concept section 14, answer 6).

pub mod profile;
pub mod values;

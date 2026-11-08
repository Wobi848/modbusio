//! Reading and writing one block of a device, and what modbusio learnt about the device's limits
//! on the way (concept section 6: never more than the device takes).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::endpoint::Target;
use crate::link::LinkError;
use crate::links::Links;
use crate::protocol::{Area, Exception, Request, Response, pdu};

/// What a device takes, learnt from its refusals. Kept with the device on the site.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Limits {
    pub most_registers: u16,
    pub most_bits: u16,
    /// Reading across an address the device does not have fails: read only what the profile names.
    pub no_holes: bool,
    /// The device refuses 16 for one register; 06 works.
    pub single_register_write: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            most_registers: pdu::MOST_REGISTERS,
            most_bits: pdu::MOST_BITS,
            no_holes: false,
            single_register_write: false,
        }
    }
}

impl Limits {
    pub fn most(&self, area: Area) -> u16 {
        if area.is_bits() {
            self.most_bits
        } else {
            self.most_registers
        }
    }

    /// The device refused or did not answer `count` items: ask for at most half next time.
    pub fn lower(&mut self, area: Area, count: u16) {
        let half = (count / 2).max(1);
        if area.is_bits() {
            self.most_bits = self.most_bits.min(half);
        } else {
            self.most_registers = self.most_registers.min(half);
        }
    }
}

/// Reads `count` items of `area` from `address`. Bits come as 0 and 1, so every area reads alike.
pub async fn read(
    links: &Links,
    target: &Target,
    area: Area,
    address: u16,
    count: u16,
    timeout: Duration,
) -> Result<Vec<u16>, LinkError> {
    let request = Request::Read {
        area,
        address,
        count,
    };
    match links.exchange(target, &request, timeout).await? {
        Response::Registers(registers) => Ok(registers),
        Response::Bits(bits) => Ok(bits.into_iter().map(u16::from).collect()),
        other => Err(LinkError::Malformed(format!("{other:?}"))),
    }
}

/// Writes registers (06 for one where the device wants it, else 16) or coils (05 for one, else 15).
/// Learns `single_register_write` when the device refuses 16 for one register.
pub async fn write(
    links: &Links,
    target: &Target,
    area: Area,
    address: u16,
    values: &[u16],
    limits: &mut Limits,
    timeout: Duration,
) -> Result<(), LinkError> {
    let request = match (area, values) {
        (Area::Coil, [value]) => Request::WriteCoil {
            address,
            value: *value != 0,
        },
        (Area::Coil, _) => Request::WriteCoils {
            address,
            values: values.iter().map(|value| *value != 0).collect(),
        },
        (Area::HoldingRegister, [value]) if limits.single_register_write => {
            Request::WriteRegister {
                address,
                value: *value,
            }
        }
        (Area::HoldingRegister, _) => Request::WriteRegisters {
            address,
            values: values.to_vec(),
        },
        _ => return Err(LinkError::Malformed(format!("{area} cannot be written"))),
    };
    match links.exchange(target, &request, timeout).await {
        Err(LinkError::Exception(Exception::ILLEGAL_FUNCTION))
            if area == Area::HoldingRegister
                && values.len() == 1
                && !limits.single_register_write =>
        {
            limits.single_register_write = true;
            let single = Request::WriteRegister {
                address,
                value: values[0],
            };
            links.exchange(target, &single, timeout).await.map(drop)
        }
        other => other.map(drop),
    }
}

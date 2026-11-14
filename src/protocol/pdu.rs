//! Modbus PDUs (Modbus Application Protocol 1.1b3): requests encoded, responses decoded. No I/O.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Most registers one read may ask for (spec 6.3, 6.4).
pub const MOST_REGISTERS: u16 = 125;
/// Most coils or discrete inputs one read may ask for (spec 6.1, 6.2).
pub const MOST_BITS: u16 = 2000;
/// Most registers one write may carry (spec 6.12).
pub const MOST_WRITE_REGISTERS: u16 = 123;
/// Most coils one write may carry (spec 6.11).
pub const MOST_WRITE_BITS: u16 = 1968;

/// The four data areas of the Modbus data model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Coil,
    DiscreteInput,
    InputRegister,
    HoldingRegister,
}

impl Area {
    pub const ALL: [Area; 4] = [
        Area::Coil,
        Area::DiscreteInput,
        Area::InputRegister,
        Area::HoldingRegister,
    ];

    pub fn is_bits(self) -> bool {
        matches!(self, Area::Coil | Area::DiscreteInput)
    }

    pub fn writable(self) -> bool {
        matches!(self, Area::Coil | Area::HoldingRegister)
    }

    /// Most items one read of this area may ask for.
    pub fn most_per_read(self) -> u16 {
        if self.is_bits() {
            MOST_BITS
        } else {
            MOST_REGISTERS
        }
    }

    fn read_function(self) -> u8 {
        match self {
            Area::Coil => 0x01,
            Area::DiscreteInput => 0x02,
            Area::HoldingRegister => 0x03,
            Area::InputRegister => 0x04,
        }
    }

    fn of_read_function(function: u8) -> Option<Area> {
        Area::ALL
            .into_iter()
            .find(|area| area.read_function() == function)
    }

    /// The short name the trade uses: "HR", "IR", "coil", "DI".
    pub fn short(self) -> &'static str {
        match self {
            Area::Coil => "coil",
            Area::DiscreteInput => "DI",
            Area::InputRegister => "IR",
            Area::HoldingRegister => "HR",
        }
    }

    /// Parses the short names, the long names and the old reference digits (0, 1, 3, 4).
    pub fn parse(text: &str) -> Option<Area> {
        match text
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "_")
            .as_str()
        {
            "coil" | "coils" | "co" | "0" | "0x" => Some(Area::Coil),
            "di" | "discrete_input" | "discrete_inputs" | "input_status" | "1" | "1x" => {
                Some(Area::DiscreteInput)
            }
            "ir" | "input_register" | "input_registers" | "3" | "3x" => Some(Area::InputRegister),
            "hr" | "holding_register" | "holding_registers" | "register" | "4" | "4x" => {
                Some(Area::HoldingRegister)
            }
            _ => None,
        }
    }
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Area::Coil => "coils",
            Area::DiscreteInput => "discrete inputs",
            Area::InputRegister => "input registers",
            Area::HoldingRegister => "holding registers",
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Read {
        area: Area,
        address: u16,
        count: u16,
    },
    WriteCoil {
        address: u16,
        value: bool,
    },
    WriteRegister {
        address: u16,
        value: u16,
    },
    WriteCoils {
        address: u16,
        values: Vec<bool>,
    },
    WriteRegisters {
        address: u16,
        values: Vec<u16>,
    },
    /// 43/14 Read Device Identification, from `object` on, at access level `code` (1 basic,
    /// 2 regular, 3 extended).
    Identify {
        code: u8,
        object: u8,
    },
    /// 08/00 Return Query Data: the device sends `data` back.
    Echo {
        data: u16,
    },
}

impl Request {
    pub fn function(&self) -> u8 {
        match self {
            Request::Read { area, .. } => area.read_function(),
            Request::WriteCoil { .. } => 0x05,
            Request::WriteRegister { .. } => 0x06,
            Request::WriteCoils { .. } => 0x0F,
            Request::WriteRegisters { .. } => 0x10,
            Request::Identify { .. } => 0x2B,
            Request::Echo { .. } => 0x08,
        }
    }

    /// The request changes nothing in the device: sending it twice does no harm.
    pub fn only_reads(&self) -> bool {
        matches!(self, Request::Read { .. } | Request::Identify { .. })
    }

    /// The normal answer repeats the request byte for byte (spec 6.5, 6.6, 6.8).
    pub fn answer_repeats_it(&self) -> bool {
        matches!(
            self,
            Request::WriteCoil { .. } | Request::WriteRegister { .. } | Request::Echo { .. }
        )
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut pdu = vec![self.function()];
        match self {
            Request::Read { address, count, .. } => {
                push_u16(&mut pdu, *address);
                push_u16(&mut pdu, *count);
            }
            Request::WriteCoil { address, value } => {
                push_u16(&mut pdu, *address);
                push_u16(&mut pdu, if *value { 0xFF00 } else { 0x0000 });
            }
            Request::WriteRegister { address, value } => {
                push_u16(&mut pdu, *address);
                push_u16(&mut pdu, *value);
            }
            Request::WriteCoils { address, values } => {
                push_u16(&mut pdu, *address);
                push_u16(&mut pdu, values.len() as u16);
                let packed = pack_bits(values);
                pdu.push(packed.len() as u8);
                pdu.extend(packed);
            }
            Request::WriteRegisters { address, values } => {
                push_u16(&mut pdu, *address);
                push_u16(&mut pdu, values.len() as u16);
                pdu.push((values.len() * 2) as u8);
                for value in values {
                    push_u16(&mut pdu, *value);
                }
            }
            Request::Identify { code, object } => pdu.extend([0x0E, *code, *object]),
            Request::Echo { data } => {
                push_u16(&mut pdu, 0x0000);
                push_u16(&mut pdu, *data);
            }
        }
        pdu
    }

    /// Checks the limits of the spec before anything is sent.
    pub fn check(&self) -> Result<(), String> {
        let span = |address: u16, count: usize, most: u16| {
            if count == 0 || count > most as usize {
                return Err(format!("count must be 1 to {most}, not {count}"));
            }
            if address as usize + count > 0x1_0000 {
                return Err(format!("{count} from {address} runs past address 65535"));
            }
            Ok(())
        };
        match self {
            Request::Read {
                area,
                address,
                count,
            } => span(*address, *count as usize, area.most_per_read()),
            Request::WriteCoils { address, values } => {
                span(*address, values.len(), MOST_WRITE_BITS)
            }
            Request::WriteRegisters { address, values } => {
                span(*address, values.len(), MOST_WRITE_REGISTERS)
            }
            _ => Ok(()),
        }
    }
}

/// What a device answered, decoded for the request it answers.
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    Bits(Vec<bool>),
    Registers(Vec<u16>),
    /// A write the device confirmed.
    Written,
    Identity(Identity),
    Echo(u16),
}

/// The objects of a Read Device Identification answer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Identity {
    pub objects: Vec<(u8, String)>,
    /// The device has more objects than fit in one answer; ask again from this one.
    pub next: Option<u8>,
}

impl Identity {
    /// Object 0 vendor, 1 product code, 2 revision, 3 vendor URL, 4 product name, 5 model.
    pub fn get(&self, object: u8) -> Option<&str> {
        self.objects
            .iter()
            .find(|(id, _)| *id == object)
            .map(|(_, text)| text.as_str())
    }
}

/// An exception the device answered (spec 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exception(pub u8);

impl Exception {
    pub const ILLEGAL_FUNCTION: Exception = Exception(0x01);
    pub const ILLEGAL_ADDRESS: Exception = Exception(0x02);
    pub const ILLEGAL_VALUE: Exception = Exception(0x03);
    pub const DEVICE_FAILURE: Exception = Exception(0x04);
    pub const BUSY: Exception = Exception(0x06);
    pub const GATEWAY_PATH: Exception = Exception(0x0A);
    pub const GATEWAY_TARGET: Exception = Exception(0x0B);

    /// A gateway answered for a device behind it.
    pub fn from_gateway(self) -> bool {
        matches!(self, Exception::GATEWAY_PATH | Exception::GATEWAY_TARGET)
    }
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self.0 {
            0x01 => "the device does not know this function",
            0x02 => "the device has no data at this address",
            0x03 => "the device refuses this value or count",
            0x04 => "the device reports a fault of its own",
            0x05 => "the device accepted and works on it",
            0x06 => "the device is busy",
            0x08 => "the device reports a memory parity error",
            0x0A => "the gateway has no path to this unit",
            0x0B => "the device behind the gateway does not answer",
            _ => "the device answered an unknown exception",
        };
        write!(f, "exception {:02X}: {text}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PduError {
    Exception(Exception),
    /// The answer does not fit the request.
    Malformed(String),
}

impl fmt::Display for PduError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PduError::Exception(exception) => exception.fmt(f),
            PduError::Malformed(reason) => write!(f, "answer does not fit the request: {reason}"),
        }
    }
}

/// Decodes the device's answer to `request`.
pub fn decode_response(request: &Request, pdu: &[u8]) -> Result<Response, PduError> {
    let malformed = |reason: &str| Err(PduError::Malformed(reason.to_string()));
    let Some(&function) = pdu.first() else {
        return malformed("empty");
    };
    if function == request.function() | 0x80 {
        return match pdu.get(1) {
            Some(&code) => Err(PduError::Exception(Exception(code))),
            None => malformed("exception without code"),
        };
    }
    if function != request.function() {
        return malformed(&format!(
            "function {function:02X} instead of {:02X}",
            request.function()
        ));
    }
    match request {
        Request::Read { area, count, .. } => {
            let Some(&bytes) = pdu.get(1) else {
                return malformed("no byte count");
            };
            let data = &pdu[2..];
            if data.len() != bytes as usize {
                return malformed("byte count does not match the data");
            }
            if area.is_bits() {
                if data.len() != (*count as usize).div_ceil(8) {
                    return malformed("wrong number of bits");
                }
                Ok(Response::Bits(unpack_bits(data, *count as usize)))
            } else {
                if data.len() != *count as usize * 2 {
                    return malformed("wrong number of registers");
                }
                Ok(Response::Registers(
                    data.chunks(2)
                        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                        .collect(),
                ))
            }
        }
        Request::WriteCoil { .. }
        | Request::WriteRegister { .. }
        | Request::WriteCoils { .. }
        | Request::WriteRegisters { .. } => {
            if pdu.len() != 5 {
                return malformed("write confirmation of wrong length");
            }
            let echoed = &request.encode()[1..5];
            if &pdu[1..5] != echoed {
                return malformed("write confirmation names another address or value");
            }
            Ok(Response::Written)
        }
        Request::Identify { .. } => decode_identity(pdu).map(Response::Identity),
        Request::Echo { data } => {
            if pdu.len() != 5 || pdu[1..3] != [0, 0] {
                return malformed("echo of wrong form");
            }
            let echoed = u16::from_be_bytes([pdu[3], pdu[4]]);
            if echoed != *data {
                return malformed("echo with other data");
            }
            Ok(Response::Echo(echoed))
        }
    }
}

/// Decodes a request another master sent, as heard on a trunk; `None` for a function modbusio
/// does not know or a request of the wrong form.
pub fn decode_request(pdu: &[u8]) -> Option<Request> {
    let (&function, body) = pdu.split_first()?;
    let word = |at: usize| Some(u16::from_be_bytes([*body.get(at)?, *body.get(at + 1)?]));
    let fixed = |length: usize| (body.len() == length).then_some(());
    match function {
        0x01..=0x04 => {
            fixed(4)?;
            Some(Request::Read {
                area: Area::of_read_function(function)?,
                address: word(0)?,
                count: word(2)?,
            })
        }
        0x05 => {
            fixed(4)?;
            let value = match word(2)? {
                0xFF00 => true,
                0x0000 => false,
                _ => return None,
            };
            Some(Request::WriteCoil {
                address: word(0)?,
                value,
            })
        }
        0x06 => {
            fixed(4)?;
            Some(Request::WriteRegister {
                address: word(0)?,
                value: word(2)?,
            })
        }
        0x0F => {
            let count = word(2)? as usize;
            let data = body.get(5..)?;
            (data.len() == *body.get(4)? as usize && data.len() == count.div_ceil(8))
                .then_some(())?;
            Some(Request::WriteCoils {
                address: word(0)?,
                values: unpack_bits(data, count),
            })
        }
        0x10 => {
            let count = word(2)? as usize;
            let data = body.get(5..)?;
            (data.len() == *body.get(4)? as usize && data.len() == count * 2).then_some(())?;
            Some(Request::WriteRegisters {
                address: word(0)?,
                values: (0..count)
                    .map(|index| word(5 + 2 * index))
                    .collect::<Option<_>>()?,
            })
        }
        0x2B => {
            fixed(3)?;
            (body[0] == 0x0E).then_some(())?;
            Some(Request::Identify {
                code: body[1],
                object: body[2],
            })
        }
        0x08 => {
            fixed(4)?;
            (word(0)? == 0).then_some(())?;
            Some(Request::Echo { data: word(2)? })
        }
        _ => None,
    }
}

fn decode_identity(pdu: &[u8]) -> Result<Identity, PduError> {
    let malformed = |reason: &str| Err(PduError::Malformed(reason.to_string()));
    if pdu.len() < 7 || pdu[1] != 0x0E {
        return malformed("identification too short");
    }
    let more = pdu[4] == 0xFF;
    let next = pdu[5];
    let count = pdu[6] as usize;
    let mut objects = Vec::with_capacity(count);
    let mut at = 7;
    for _ in 0..count {
        let (Some(&id), Some(&length)) = (pdu.get(at), pdu.get(at + 1)) else {
            return malformed("identification object cut off");
        };
        let Some(value) = pdu.get(at + 2..at + 2 + length as usize) else {
            return malformed("identification text cut off");
        };
        objects.push((id, String::from_utf8_lossy(value).trim().to_string()));
        at += 2 + length as usize;
    }
    Ok(Identity {
        objects,
        next: more.then_some(next),
    })
}

/// How long the PDU at the start of `bytes` is, from what has arrived so far: `Ok(None)` while
/// more bytes are needed. For framing answers on a serial line, which have no length field.
pub fn response_length(bytes: &[u8]) -> Result<Option<usize>, String> {
    let Some(&function) = bytes.first() else {
        return Ok(None);
    };
    let need = |length: usize| Ok((bytes.len() >= length).then_some(length));
    if function & 0x80 != 0 {
        return need(2);
    }
    match function {
        0x01..=0x04 => match bytes.get(1) {
            Some(&count) => need(2 + count as usize),
            None => Ok(None),
        },
        0x05 | 0x06 | 0x08 | 0x0F | 0x10 => need(5),
        0x2B => identity_length(bytes),
        _ => Err(format!("unknown function {function:02X}")),
    }
}

/// How long the request PDU at the start of `bytes` is, as [`response_length`] for answers: for
/// listening to a trunk, where requests and answers pass alike.
pub fn request_length(bytes: &[u8]) -> Result<Option<usize>, String> {
    let Some(&function) = bytes.first() else {
        return Ok(None);
    };
    let need = |length: usize| Ok((bytes.len() >= length).then_some(length));
    match function {
        0x01..=0x06 | 0x08 => need(5),
        0x0F | 0x10 => match bytes.get(5) {
            Some(&count) => need(6 + count as usize),
            None => Ok(None),
        },
        0x2B => need(4),
        _ => Err(format!("unknown function {function:02X}")),
    }
}

fn identity_length(bytes: &[u8]) -> Result<Option<usize>, String> {
    if bytes.len() < 7 {
        return Ok(None);
    }
    let mut at = 7;
    for _ in 0..bytes[6] {
        let Some(&length) = bytes.get(at + 1) else {
            return Ok(None);
        };
        at += 2 + length as usize;
    }
    Ok((bytes.len() >= at).then_some(at))
}

fn push_u16(pdu: &mut Vec<u8>, value: u16) {
    pdu.extend(value.to_be_bytes());
}

fn pack_bits(values: &[bool]) -> Vec<u8> {
    values
        .chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0u8, |byte, (bit, &on)| byte | (u8::from(on) << bit))
        })
        .collect()
}

fn unpack_bits(data: &[u8], count: usize) -> Vec<bool> {
    (0..count)
        .map(|bit| data[bit / 8] & (1 << (bit % 8)) != 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_holding_register_read_is_encoded_as_the_spec_shows() {
        // spec 6.3: read registers 108 to 110
        let request = Request::Read {
            area: Area::HoldingRegister,
            address: 0x006B,
            count: 3,
        };
        assert_eq!(request.encode(), [0x03, 0x00, 0x6B, 0x00, 0x03]);
        let answer = [0x03, 0x06, 0x02, 0x2B, 0x00, 0x00, 0x00, 0x64];
        assert_eq!(
            decode_response(&request, &answer),
            Ok(Response::Registers(vec![0x022B, 0x0000, 0x0064]))
        );
    }

    #[test]
    fn coils_are_unpacked_low_bit_first() {
        // spec 6.1: coils 20 to 38
        let request = Request::Read {
            area: Area::Coil,
            address: 0x0013,
            count: 19,
        };
        let answer = [0x01, 0x03, 0xCD, 0x6B, 0x05];
        let Ok(Response::Bits(bits)) = decode_response(&request, &answer) else {
            panic!("bits expected");
        };
        assert_eq!(bits.len(), 19);
        assert_eq!(
            &bits[..8],
            &[true, false, true, true, false, false, true, true]
        );
        assert_eq!(&bits[16..], &[true, false, true]);
    }

    #[test]
    fn writing_coils_packs_them_as_the_spec_shows() {
        // spec 6.11: ten coils from 20
        let values = [
            true, false, true, true, false, false, true, true, true, false,
        ];
        let request = Request::WriteCoils {
            address: 0x0013,
            values: values.to_vec(),
        };
        assert_eq!(
            request.encode(),
            [0x0F, 0x00, 0x13, 0x00, 0x0A, 0x02, 0xCD, 0x01]
        );
        assert_eq!(
            decode_response(&request, &[0x0F, 0x00, 0x13, 0x00, 0x0A]),
            Ok(Response::Written)
        );
    }

    #[test]
    fn an_exception_is_reported_with_its_code() {
        let request = Request::Read {
            area: Area::InputRegister,
            address: 9,
            count: 1,
        };
        assert_eq!(
            decode_response(&request, &[0x84, 0x02]),
            Err(PduError::Exception(Exception::ILLEGAL_ADDRESS))
        );
    }

    #[test]
    fn a_write_confirmation_for_another_value_is_refused() {
        let request = Request::WriteRegister {
            address: 1,
            value: 3,
        };
        assert!(matches!(
            decode_response(&request, &[0x06, 0x00, 0x01, 0x00, 0x04]),
            Err(PduError::Malformed(_))
        ));
    }

    #[test]
    fn device_identification_is_decoded() {
        let answer = [
            0x2B, 0x0E, 0x01, 0x01, 0x00, 0x00, 0x03, 0x00, 0x04, b'A', b'c', b'm', b'e', 0x01,
            0x02, b'M', b'9', 0x02, 0x03, b'1', b'.', b'2',
        ];
        let request = Request::Identify { code: 1, object: 0 };
        let Ok(Response::Identity(identity)) = decode_response(&request, &answer) else {
            panic!("identity expected");
        };
        assert_eq!(identity.get(0), Some("Acme"));
        assert_eq!(identity.get(2), Some("1.2"));
        assert_eq!(identity.next, None);
        assert_eq!(response_length(&answer), Ok(Some(answer.len())));
        assert_eq!(response_length(&answer[..10]), Ok(None));
    }

    #[test]
    fn the_length_of_an_answer_follows_from_its_first_bytes() {
        assert_eq!(response_length(&[0x03]), Ok(None));
        assert_eq!(response_length(&[0x03, 0x04, 1, 2]), Ok(None));
        assert_eq!(response_length(&[0x03, 0x04, 1, 2, 3, 4]), Ok(Some(6)));
        assert_eq!(response_length(&[0x83, 0x02]), Ok(Some(2)));
        assert_eq!(response_length(&[0x10, 0, 1, 0, 2]), Ok(Some(5)));
        assert!(response_length(&[0x55]).is_err());
    }

    #[test]
    fn requests_beyond_the_spec_limits_are_refused_before_sending() {
        let read = |count| Request::Read {
            area: Area::HoldingRegister,
            address: 0,
            count,
        };
        assert!(read(125).check().is_ok());
        assert!(read(126).check().is_err());
        assert!(read(0).check().is_err());
        let past_end = Request::Read {
            area: Area::Coil,
            address: 65535,
            count: 2,
        };
        assert!(past_end.check().is_err());
    }

    #[test]
    fn every_request_modbusio_sends_is_decoded_back() {
        let requests = [
            Request::Read {
                area: Area::DiscreteInput,
                address: 196,
                count: 22,
            },
            Request::Read {
                area: Area::InputRegister,
                address: 8,
                count: 1,
            },
            Request::WriteCoil {
                address: 172,
                value: true,
            },
            Request::WriteRegister {
                address: 1,
                value: 3,
            },
            Request::WriteCoils {
                address: 19,
                values: vec![
                    true, false, true, true, false, false, true, true, true, false,
                ],
            },
            Request::WriteRegisters {
                address: 1,
                values: vec![0x000A, 0x0102],
            },
            Request::Identify { code: 1, object: 0 },
            Request::Echo { data: 0xA537 },
        ];
        for request in requests {
            assert_eq!(decode_request(&request.encode()), Some(request));
        }
    }

    #[test]
    fn requests_of_another_form_are_not_decoded() {
        // spec 6.5: a coil is written with FF00 or 0000 only
        assert_eq!(decode_request(&[0x05, 0, 1, 0x12, 0x34]), None);
        // a byte count that does not match the registers
        assert_eq!(decode_request(&[0x10, 0, 1, 0, 2, 3, 0, 1, 0]), None);
        assert_eq!(decode_request(&[0x03, 0, 1, 0]), None);
        assert_eq!(decode_request(&[0x41, 0, 1]), None);
        assert_eq!(decode_request(&[]), None);
    }

    #[test]
    fn areas_are_read_from_the_names_lists_use() {
        assert_eq!(Area::parse("Holding Register"), Some(Area::HoldingRegister));
        assert_eq!(Area::parse("IR"), Some(Area::InputRegister));
        assert_eq!(Area::parse("4x"), Some(Area::HoldingRegister));
        assert_eq!(Area::parse("discrete-inputs"), Some(Area::DiscreteInput));
        assert_eq!(Area::parse("tank"), None);
    }
}

//! Modbus RTU framing (Modbus over Serial Line 1.02, section 2.5): unit, PDU, CRC. No I/O.

use super::pdu;

/// CRC-16/MODBUS (polynomial 0xA001 reflected, start 0xFFFF), sent low byte first.
pub fn crc(bytes: &[u8]) -> u16 {
    bytes
        .iter()
        .fold(CRC_START, |crc, &byte| crc_step(crc, byte))
}

pub const CRC_START: u16 = 0xFFFF;

/// The CRC after one more byte, for searching a frame's end byte by byte.
pub fn crc_step(crc: u16, byte: u8) -> u16 {
    (0..8).fold(crc ^ u16::from(byte), |crc, _| {
        if crc & 1 != 0 {
            (crc >> 1) ^ 0xA001
        } else {
            crc >> 1
        }
    })
}

pub fn frame(unit: u8, pdu: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(pdu.len() + 3);
    frame.push(unit);
    frame.extend_from_slice(pdu);
    let crc = crc(&frame);
    frame.extend(crc.to_le_bytes());
    frame
}

/// What the bytes received so far hold.
#[derive(Debug, PartialEq)]
pub enum Received {
    /// More bytes are needed.
    Incomplete,
    /// A whole answer of this length (unit, PDU, CRC) with a good CRC.
    Frame { unit: u8, length: usize },
    /// A whole answer whose CRC does not match: damaged on the line.
    BadCrc { length: usize },
    /// Bytes that cannot start an answer.
    Garbage(String),
}

/// Frames an answer from a serial line by the form of its PDU.
pub fn parse_answer(bytes: &[u8]) -> Received {
    let Some(&unit) = bytes.first() else {
        return Received::Incomplete;
    };
    match pdu::response_length(&bytes[1..]) {
        Ok(None) => Received::Incomplete,
        Err(reason) => Received::Garbage(reason),
        Ok(Some(pdu_length)) => {
            let length = 1 + pdu_length + 2;
            if bytes.len() < length {
                return Received::Incomplete;
            }
            if check(&bytes[..length]) {
                Received::Frame { unit, length }
            } else {
                Received::BadCrc { length }
            }
        }
    }
}

/// The CRC at the end of `frame` matches the bytes before it.
pub fn check(frame: &[u8]) -> bool {
    if frame.len() < 4 {
        return false;
    }
    let (body, sent) = frame.split_at(frame.len() - 2);
    crc(body).to_le_bytes() == [sent[0], sent[1]]
}

/// How long one character takes on the line: start bit, 8 data bits, parity or second stop bit,
/// stop bit: 11 bits (spec 2.5.1).
pub fn character_micros(baud: u32) -> u64 {
    11_000_000 / u64::from(baud.max(1))
}

/// The silence that ends a frame: 3.5 characters, at least 1.75 ms above 19200 baud (spec 2.5.1.1).
pub fn frame_gap_micros(baud: u32) -> u64 {
    if baud > 19_200 {
        1_750
    } else {
        character_micros(baud) * 7 / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crc_matches_the_spec_example() {
        // Modbus over Serial Line, appendix B: read 2 registers from 0x0000 at unit 1 is 01 03 00 00 00 02 C4 0B
        assert_eq!(
            frame(1, &[0x03, 0x00, 0x00, 0x00, 0x02]),
            [0x01, 0x03, 0x00, 0x00, 0x00, 0x02, 0xC4, 0x0B]
        );
    }

    #[test]
    fn an_answer_is_framed_by_its_form() {
        let answer = frame(7, &[0x03, 0x02, 0x12, 0x34]);
        assert_eq!(parse_answer(&answer[..4]), Received::Incomplete);
        assert_eq!(
            parse_answer(&answer),
            Received::Frame {
                unit: 7,
                length: answer.len()
            }
        );
        let mut damaged = answer.clone();
        damaged[3] ^= 0x01;
        assert_eq!(
            parse_answer(&damaged),
            Received::BadCrc {
                length: answer.len()
            }
        );
        assert!(matches!(parse_answer(&[7, 0x55]), Received::Garbage(_)));
    }

    #[test]
    fn the_frame_gap_follows_the_speed() {
        assert_eq!(frame_gap_micros(9600), 4_007);
        assert_eq!(frame_gap_micros(115_200), 1_750);
    }
}

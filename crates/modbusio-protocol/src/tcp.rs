//! Modbus TCP framing (Modbus Messaging on TCP/IP 1.0b, section 3.1.3): the MBAP header. No I/O.

pub const MODBUS_PORT: u16 = 502;
/// Transaction (2), protocol (2), length (2), unit (1).
pub const HEADER: usize = 7;

pub fn frame(transaction: u16, unit: u8, pdu: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(HEADER + pdu.len());
    frame.extend(transaction.to_be_bytes());
    frame.extend([0, 0]);
    frame.extend(((pdu.len() + 1) as u16).to_be_bytes());
    frame.push(unit);
    frame.extend_from_slice(pdu);
    frame
}

#[derive(Debug, PartialEq)]
pub struct Header {
    pub transaction: u16,
    pub unit: u8,
    /// Bytes of the PDU that follow the header.
    pub pdu_length: usize,
}

pub fn parse_header(bytes: &[u8; HEADER]) -> Result<Header, String> {
    let protocol = u16::from_be_bytes([bytes[2], bytes[3]]);
    if protocol != 0 {
        return Err(format!("protocol {protocol} is not Modbus"));
    }
    let length = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if !(2..=254).contains(&length) {
        return Err(format!("length {length} out of range"));
    }
    Ok(Header {
        transaction: u16::from_be_bytes([bytes[0], bytes[1]]),
        unit: bytes[6],
        pdu_length: length - 1,
    })
}

/// Takes the first whole frame off `received`: its header and all its bytes. `Ok(None)` while
/// bytes are missing; an error when the bytes are no Modbus TCP, so no frame can be found in them.
pub fn take_frame(received: &mut Vec<u8>) -> Result<Option<(Header, Vec<u8>)>, String> {
    let Some(start) = received.first_chunk::<HEADER>() else {
        return Ok(None);
    };
    let header = parse_header(start)?;
    let length = HEADER + header.pdu_length;
    if received.len() < length {
        return Ok(None);
    }
    Ok(Some((header, received.drain(..length).collect())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_carries_transaction_unit_and_length() {
        let frame = frame(0x1234, 17, &[0x03, 0, 0, 0, 1]);
        assert_eq!(frame[..7], [0x12, 0x34, 0, 0, 0, 6, 17]);
        let header = parse_header(frame[..7].try_into().expect("seven bytes"));
        assert_eq!(
            header,
            Ok(Header {
                transaction: 0x1234,
                unit: 17,
                pdu_length: 5
            })
        );
    }

    #[test]
    fn frames_are_taken_one_by_one_as_their_bytes_arrive() {
        let first = frame(1, 17, &[0x03, 2, 0, 9]);
        let second = frame(2, 17, &[0x83, 2]);
        let mut received = first[..9].to_vec();
        assert_eq!(take_frame(&mut received), Ok(None));
        received.extend_from_slice(&first[9..]);
        received.extend_from_slice(&second);
        let (header, bytes) = take_frame(&mut received).expect("Modbus").expect("whole");
        assert_eq!((header.transaction, bytes), (1, first));
        let (header, bytes) = take_frame(&mut received).expect("Modbus").expect("whole");
        assert_eq!((header.transaction, bytes), (2, second));
        assert_eq!(take_frame(&mut received), Ok(None));
        assert!(take_frame(&mut b"HTTP/1.1 400".to_vec()).is_err());
    }

    #[test]
    fn a_header_of_another_protocol_is_refused() {
        assert!(parse_header(&[0, 1, 0, 1, 0, 6, 1]).is_err());
        assert!(parse_header(&[0, 1, 0, 0, 0x10, 0, 1]).is_err());
    }
}

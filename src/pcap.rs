//! A capture as a pcapng file for Wireshark. Modbus TCP goes out as IPv4 packets with a TCP header
//! made up from the connection's addresses, so Wireshark decodes it on port 502 by itself. RTU
//! frames (serial, or RTU over TCP) use the user link type 147, which Wireshark decodes once
//! *DLT_USER* maps it to `mbrtu` (README, *Capture for Wireshark*).

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::UNIX_EPOCH;

use crate::capture::{Direction, Frame};

const LINKTYPE_IPV4: u16 = 228;
const LINKTYPE_USER0: u16 = 147;
const INTERFACE_TCP: u32 = 0;
const INTERFACE_RTU: u32 = 1;

pub fn write(frames: &[Frame]) -> Vec<u8> {
    let mut file = Vec::new();
    section_header(&mut file);
    interface(&mut file, LINKTYPE_IPV4);
    interface(&mut file, LINKTYPE_USER0);
    let mut sequences: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
    for frame in frames {
        match frame.addresses.filter(|_| !frame.rtu) {
            Some((local, remote)) => {
                let (from, to) = match frame.direction {
                    Direction::Sent => (local, remote),
                    Direction::Received => (remote, local),
                };
                let sequence = *sequences.get(&(from, to)).unwrap_or(&1);
                let acknowledged = *sequences.get(&(to, from)).unwrap_or(&1);
                let packet = ipv4_tcp(from, to, sequence, acknowledged, &frame.bytes);
                sequences.insert((from, to), sequence.wrapping_add(frame.bytes.len() as u32));
                packet_block(&mut file, INTERFACE_TCP, frame, &packet);
            }
            None => packet_block(&mut file, INTERFACE_RTU, frame, &frame.bytes),
        }
    }
    file
}

fn section_header(file: &mut Vec<u8>) {
    let mut body = Vec::new();
    body.extend(0x1A2B_3C4Du32.to_le_bytes());
    body.extend(1u16.to_le_bytes());
    body.extend(0u16.to_le_bytes());
    body.extend((-1i64).to_le_bytes());
    block(file, 0x0A0D_0D0A, &body);
}

fn interface(file: &mut Vec<u8>, link_type: u16) {
    let mut body = Vec::new();
    body.extend(link_type.to_le_bytes());
    body.extend(0u16.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    // option if_tsresol (9): microseconds, then the end of options
    body.extend(9u16.to_le_bytes());
    body.extend(1u16.to_le_bytes());
    body.extend([6, 0, 0, 0]);
    body.extend([0, 0, 0, 0]);
    block(file, 1, &body);
}

fn packet_block(file: &mut Vec<u8>, interface: u32, frame: &Frame, packet: &[u8]) {
    let micros = frame
        .time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_micros() as u64);
    let mut body = Vec::new();
    body.extend(interface.to_le_bytes());
    body.extend(((micros >> 32) as u32).to_le_bytes());
    body.extend((micros as u32).to_le_bytes());
    body.extend((packet.len() as u32).to_le_bytes());
    body.extend((packet.len() as u32).to_le_bytes());
    body.extend_from_slice(packet);
    while body.len() % 4 != 0 {
        body.push(0);
    }
    block(file, 6, &body);
}

fn block(file: &mut Vec<u8>, kind: u32, body: &[u8]) {
    let length = (body.len() + 12) as u32;
    file.extend(kind.to_le_bytes());
    file.extend(length.to_le_bytes());
    file.extend_from_slice(body);
    file.extend(length.to_le_bytes());
}

fn ipv4_tcp(
    from: SocketAddr,
    to: SocketAddr,
    sequence: u32,
    acknowledged: u32,
    payload: &[u8],
) -> Vec<u8> {
    let octets = |address: SocketAddr| match address.ip() {
        IpAddr::V4(ip) => ip.octets(),
        IpAddr::V6(_) => [0, 0, 0, 0],
    };
    let total = 20 + 20 + payload.len();
    let mut packet = Vec::with_capacity(total);
    packet.extend([0x45, 0]);
    packet.extend((total as u16).to_be_bytes());
    packet.extend([0, 0, 0x40, 0, 64, 6, 0, 0]);
    packet.extend(octets(from));
    packet.extend(octets(to));
    let checksum = ip_checksum(&packet);
    packet[10..12].copy_from_slice(&checksum.to_be_bytes());
    packet.extend(from.port().to_be_bytes());
    packet.extend(to.port().to_be_bytes());
    packet.extend(sequence.to_be_bytes());
    packet.extend(acknowledged.to_be_bytes());
    // header length 5 words; flags PSH and ACK; window; checksum 0 (Wireshark does not check it by default)
    packet.extend([0x50, 0x18, 0xFF, 0xFF, 0, 0, 0, 0]);
    packet.extend_from_slice(payload);
    packet
}

fn ip_checksum(header: &[u8]) -> u16 {
    let sum = header
        .chunks(2)
        .map(|pair| u32::from(u16::from_be_bytes([pair[0], pair[1]])))
        .sum::<u32>();
    let folded = (sum & 0xFFFF) + (sum >> 16);
    !((folded & 0xFFFF) + (folded >> 16)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn frame(rtu: bool, direction: Direction, bytes: &[u8]) -> Frame {
        Frame {
            time: SystemTime::UNIX_EPOCH,
            direction,
            addresses: (!rtu).then(|| {
                (
                    "10.0.0.1:50000".parse().expect("address"),
                    "10.0.0.5:502".parse().expect("address"),
                )
            }),
            bytes: bytes.to_vec(),
            rtu,
        }
    }

    #[test]
    fn the_file_has_a_section_two_interfaces_and_a_block_per_frame() {
        let file = write(&[
            frame(
                false,
                Direction::Sent,
                &[0, 1, 0, 0, 0, 6, 1, 3, 0, 0, 0, 1],
            ),
            frame(
                false,
                Direction::Received,
                &[0, 1, 0, 0, 0, 5, 1, 3, 2, 0, 7],
            ),
            frame(true, Direction::Sent, &[1, 3, 0, 0, 0, 2, 0xC4, 0x0B]),
        ]);
        let mut kinds = Vec::new();
        let mut at = 0;
        while at < file.len() {
            let kind = u32::from_le_bytes(file[at..at + 4].try_into().expect("four bytes"));
            let length = u32::from_le_bytes(file[at + 4..at + 8].try_into().expect("four bytes"));
            assert_eq!(length % 4, 0);
            kinds.push(kind);
            at += length as usize;
        }
        assert_eq!(at, file.len());
        assert_eq!(kinds, [0x0A0D_0D0A, 1, 1, 6, 6, 6]);
    }

    #[test]
    fn the_ip_header_checksum_is_valid() {
        let packet = ipv4_tcp(
            "10.0.0.1:50000".parse().expect("address"),
            "10.0.0.5:502".parse().expect("address"),
            1,
            1,
            &[1, 2, 3],
        );
        assert_eq!(ip_checksum(&packet[..20]), 0);
    }
}

//! The protocol core against tokio-modbus' servers: TCP, RTU over TCP and RTU on a
//! pseudo-terminal (concept section 9, M0).

mod common;

use std::time::Duration;

use modbusio::endpoint::Endpoint;
use modbusio::link::LinkError;
use modbusio::protocol::{Area, Exception, Request, Response};
use modbusio::values::{DataType, WordOrder, decode};

const WAIT: Duration = Duration::from_millis(800);

fn read(area: Area, address: u16, count: u16) -> Request {
    Request::Read {
        area,
        address,
        count,
    }
}

fn registers(response: Response) -> Vec<u16> {
    match response {
        Response::Registers(registers) => registers,
        other => panic!("registers expected, got {other:?}"),
    }
}

#[tokio::test]
async fn floats_of_a_tcp_meter_read_in_abcd() {
    let address = common::meter(false).await;
    let links = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    let answer = links
        .exchange(&target, &read(Area::InputRegister, 0, 2), WAIT)
        .await
        .expect("an answer");
    let voltage = decode(&registers(answer), DataType::F32, WordOrder::Abcd)
        .as_f64()
        .expect("a number");
    assert!((voltage - 230.1).abs() < 1e-3);
}

#[tokio::test]
async fn a_gap_in_the_map_is_exception_02_and_too_many_registers_03() {
    let address = common::gateway().await;
    let links = common::links();
    let drive = common::target(Endpoint::Tcp(address), 2);
    let gap = links
        .exchange(&drive, &read(Area::HoldingRegister, 0, 10), WAIT)
        .await;
    assert_eq!(gap, Err(LinkError::Exception(Exception::ILLEGAL_ADDRESS)));
    let heat = common::target(Endpoint::Tcp(address), 1);
    let many = links
        .exchange(&heat, &read(Area::HoldingRegister, 0, 100), WAIT)
        .await;
    assert_eq!(many, Err(LinkError::Exception(Exception::ILLEGAL_VALUE)));
}

#[tokio::test]
async fn a_gateway_answers_0b_for_a_unit_it_does_not_reach() {
    let address = common::gateway().await;
    let links = common::links();
    let missing = common::target(Endpoint::Tcp(address), 9);
    let answer = links
        .exchange(&missing, &read(Area::HoldingRegister, 0, 1), WAIT)
        .await;
    assert_eq!(answer, Err(LinkError::Exception(Exception::GATEWAY_TARGET)));
}

#[tokio::test]
async fn writes_are_confirmed_and_read_back() {
    let address = common::gateway().await;
    let links = common::links();
    let drive = common::target(Endpoint::Tcp(address), 2);
    let write = Request::WriteRegister {
        address: 1,
        value: 3000,
    };
    assert_eq!(
        links.exchange(&drive, &write, WAIT).await,
        Ok(Response::Written)
    );
    let back = links
        .exchange(&drive, &read(Area::HoldingRegister, 1, 1), WAIT)
        .await;
    assert_eq!(back.map(registers), Ok(vec![3000]));
    let coil = Request::WriteCoil {
        address: 0,
        value: true,
    };
    assert_eq!(
        links.exchange(&drive, &coil, WAIT).await,
        Ok(Response::Written)
    );
    let bits = links.exchange(&drive, &read(Area::Coil, 0, 2), WAIT).await;
    assert_eq!(bits, Ok(Response::Bits(vec![true, false])));
}

#[tokio::test]
async fn identification_is_read_where_the_device_offers_it() {
    let address = common::meter(false).await;
    let links = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    let answer = links
        .exchange(&target, &Request::Identify { code: 1, object: 0 }, WAIT)
        .await;
    let Ok(Response::Identity(identity)) = answer else {
        panic!("identity expected, got {answer:?}");
    };
    assert_eq!(identity.get(1), Some("EM-340"));
}

#[tokio::test]
async fn rtu_frames_pass_over_tcp() {
    let address = common::rtu_gateway().await;
    let links = common::links();
    let heat = common::target(Endpoint::RtuOverTcp(address), 1);
    let answer = links
        .exchange(&heat, &read(Area::HoldingRegister, 2, 2), WAIT)
        .await
        .expect("an answer");
    let flow = decode(&registers(answer), DataType::F32, WordOrder::Cdab)
        .as_f64()
        .expect("a number");
    assert!((flow - 63.2).abs() < 2.0, "flow temperature {flow}");
    let missing = common::target(Endpoint::RtuOverTcp(address), 9);
    let silent = links
        .exchange(
            &missing,
            &read(Area::HoldingRegister, 0, 1),
            Duration::from_millis(200),
        )
        .await;
    assert_eq!(silent, Err(LinkError::Timeout));
}

#[tokio::test]
async fn a_device_taking_one_connection_refuses_a_second() {
    let address = common::meter(true).await;
    let first = common::links();
    let second = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    let request = read(Area::InputRegister, 0, 2);
    assert!(first.exchange(&target, &request, WAIT).await.is_ok());
    let refused = second.exchange(&target, &request, WAIT).await;
    assert!(
        matches!(refused, Err(LinkError::Closed) | Err(LinkError::Timeout)),
        "got {refused:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn rtu_on_a_serial_line_reads_and_stays_silent_for_missing_units() {
    use modbusio::endpoint::{Parity, SerialSettings};
    let port = common::trunk();
    let links = common::links();
    let settings = SerialSettings {
        port,
        baud: 19200,
        parity: Parity::Even,
        stop_bits: 1,
    };
    let room = common::target(Endpoint::Serial(settings.clone()), 5);
    let answer = links
        .exchange(&room, &read(Area::InputRegister, 0, 3), WAIT)
        .await
        .expect("an answer");
    assert_eq!(registers(answer).len(), 3);
    let missing = common::target(Endpoint::Serial(settings), 9);
    let silent = links
        .exchange(
            &missing,
            &read(Area::InputRegister, 0, 1),
            Duration::from_millis(200),
        )
        .await;
    assert_eq!(silent, Err(LinkError::Timeout));
}

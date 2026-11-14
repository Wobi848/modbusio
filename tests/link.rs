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

fn single_writes() -> [Request; 2] {
    [
        Request::WriteCoil {
            address: 0,
            value: true,
        },
        Request::WriteRegister {
            address: 1,
            value: 3000,
        },
    ]
}

#[tokio::test]
async fn a_single_write_over_rtu_is_confirmed_by_the_request_coming_back() {
    // the answer to 05 and 06 repeats the request byte for byte; it is not an adapter's echo
    let address = common::rtu_gateway().await;
    let links = common::links();
    let drive = common::target(Endpoint::RtuOverTcp(address), 2);
    let [coil, register] = single_writes();
    assert_eq!(
        links.exchange(&drive, &coil, WAIT).await,
        Ok(Response::Written)
    );
    // a read shows that the line gives nothing back; from then on the confirmation is taken at once
    links
        .exchange(&drive, &read(Area::HoldingRegister, 0, 2), WAIT)
        .await
        .expect("an answer");
    let started = std::time::Instant::now();
    assert_eq!(
        links.exchange(&drive, &register, WAIT).await,
        Ok(Response::Written)
    );
    assert!(started.elapsed() < WAIT / 2, "{:?}", started.elapsed());
    let written = links
        .exchange(&drive, &read(Area::HoldingRegister, 1, 1), WAIT)
        .await
        .expect("an answer");
    assert_eq!(registers(written), [3000]);
}

/// A two-wire adapter without direction control in front of `line`: it gives every request back,
/// in two pieces, before the line's answer.
async fn echoing_adapter(line: std::net::SocketAddr) -> std::net::SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (listener, address) = common::listener().await;
    tokio::spawn(async move {
        let (mut master, _) = listener.accept().await.expect("the master");
        let mut line = tokio::net::TcpStream::connect(line)
            .await
            .expect("the line");
        let (mut request, mut answer) = ([0u8; 256], [0u8; 256]);
        loop {
            tokio::select! {
                read = master.read(&mut request) => {
                    let length = read.expect("a request");
                    if length == 0 {
                        return;
                    }
                    let (first, rest) = request[..length].split_at(5);
                    master.write_all(first).await.expect("the echo's start");
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    master.write_all(rest).await.expect("the echo's end");
                    line.write_all(&request[..length]).await.expect("the line takes it");
                }
                read = line.read(&mut answer) => {
                    let length = read.expect("an answer");
                    master.write_all(&answer[..length]).await.expect("the master takes it");
                }
            }
        }
    });
    address
}

#[tokio::test]
async fn an_adapter_giving_back_what_it_sends_does_not_hide_the_answer() {
    let address = echoing_adapter(common::rtu_gateway().await).await;
    let links = common::links();
    let drive = common::target(Endpoint::RtuOverTcp(address), 2);
    let answer = links
        .exchange(&drive, &read(Area::HoldingRegister, 0, 2), WAIT)
        .await
        .expect("an answer after the echo");
    assert_eq!(registers(answer).len(), 2);
    for write in single_writes() {
        assert_eq!(
            links.exchange(&drive, &write, WAIT).await,
            Ok(Response::Written)
        );
    }
    let written = links
        .exchange(&drive, &read(Area::HoldingRegister, 1, 1), WAIT)
        .await
        .expect("an answer");
    assert_eq!(registers(written), [3000]);
    let several = Request::WriteRegisters {
        address: 1,
        values: vec![2900, 0],
    };
    assert_eq!(
        links.exchange(&drive, &several, WAIT).await,
        Ok(Response::Written)
    );

    // a unit that is not there: the echo alone is no confirmation
    let missing = common::target(Endpoint::RtuOverTcp(address), 9);
    let wait = Duration::from_millis(200);
    for write in single_writes() {
        assert_eq!(
            links.exchange(&missing, &write, wait).await,
            Err(LinkError::Timeout)
        );
    }
    assert_eq!(
        links
            .exchange(&missing, &read(Area::HoldingRegister, 0, 2), wait)
            .await,
        Err(LinkError::Timeout)
    );
}

/// A TCP port answering every request with what `answer` sends for it.
async fn raw_tcp_device<Answering>(
    answer: impl Fn(tokio::net::TcpStream, Vec<u8>) -> Answering + Send + Sync + 'static,
) -> std::net::SocketAddr
where
    Answering: Future<Output = tokio::net::TcpStream> + Send,
{
    use tokio::io::AsyncReadExt;
    let (listener, address) = common::listener().await;
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.expect("a master");
            let mut request = [0u8; 260];
            while let Ok(length @ 1..) = stream.read(&mut request).await {
                stream = answer(stream, request[..length].to_vec()).await;
            }
        }
    });
    address
}

#[tokio::test]
async fn an_answer_cut_off_by_the_wait_does_not_spoil_the_next_one() {
    use tokio::io::AsyncWriteExt;
    // the answer's first bytes come in time, the rest after the wait has run out
    let address = raw_tcp_device(|mut stream, request| async move {
        let mut answer = request[..4].to_vec();
        answer.extend([0, 5, request[6], 0x03, 2, 0, request[9]]);
        stream.write_all(&answer[..5]).await.expect("the start");
        tokio::time::sleep(Duration::from_millis(150)).await;
        stream.write_all(&answer[5..]).await.expect("the rest");
        stream
    })
    .await;
    let links = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    let cut_off = links
        .exchange(
            &target,
            &read(Area::HoldingRegister, 7, 1),
            Duration::from_millis(50),
        )
        .await;
    assert_eq!(cut_off, Err(LinkError::Timeout));
    // the simulated device answers a read at address n with the value n
    let next = links
        .exchange(&target, &read(Area::HoldingRegister, 9, 1), WAIT)
        .await
        .expect("the answer to the second request");
    assert_eq!(registers(next), [9]);
}

#[tokio::test]
async fn a_port_speaking_something_else_is_no_modbus_device() {
    use tokio::io::AsyncWriteExt;
    let address = raw_tcp_device(|mut stream, _| async move {
        stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n")
            .await
            .expect("sent");
        stream
    })
    .await;
    let links = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    // the second request starts on a new connection, not in the middle of the first one's answer
    for _ in 0..2 {
        let answer = links
            .exchange(&target, &read(Area::HoldingRegister, 0, 1), WAIT)
            .await;
        let Err(error @ LinkError::Io(_)) = answer else {
            panic!("an answer that is no Modbus expected, got {answer:?}");
        };
        assert!(!error.device_answered());
        assert!(error.to_string().contains("not Modbus TCP"), "{error}");
    }
}

#[tokio::test]
async fn an_idle_connection_is_closed_so_the_plants_master_gets_in_again() {
    let address = common::meter(true).await;
    let idle = Duration::from_millis(300);
    let ours = common::links().closing_idle_after(idle);
    let theirs = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    let request = read(Area::InputRegister, 0, 2);
    // requests in a row keep the one connection the meter takes
    for _ in 0..4 {
        assert!(ours.exchange(&target, &request, WAIT).await.is_ok());
        tokio::time::sleep(idle / 3).await;
    }
    assert!(theirs.exchange(&target, &request, WAIT).await.is_err());
    tokio::time::sleep(idle * 2).await;
    assert!(theirs.exchange(&target, &request, WAIT).await.is_ok());
}

#[tokio::test]
async fn a_connection_the_device_closed_is_opened_anew_for_a_read_only() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::AsyncWriteExt;
    // a device that answers and then closes, as some do with a connection left idle
    let requests = Arc::new(AtomicUsize::new(0));
    let seen = requests.clone();
    let address = raw_tcp_device(move |mut stream, request| {
        seen.fetch_add(1, Ordering::SeqCst);
        async move {
            let mut answer = request[..4].to_vec();
            answer.extend([0, 5, request[6], 0x03, 2, 0, request[9]]);
            // the write after the first answer goes to a closed connection
            let _ = stream.write_all(&answer).await;
            let _ = stream.shutdown().await;
            stream
        }
    })
    .await;
    let links = common::links();
    let target = common::target(Endpoint::Tcp(address), 1);
    let closed_by_now = Duration::from_millis(50);
    for address in [3, 4] {
        let answer = links
            .exchange(&target, &read(Area::HoldingRegister, address, 1), WAIT)
            .await
            .expect("an answer, on a new connection where the old one was closed");
        assert_eq!(registers(answer), [address]);
        tokio::time::sleep(closed_by_now).await;
    }
    // a write is not sent a second time: the device may have taken the first one
    let before = requests.load(Ordering::SeqCst);
    let write = Request::WriteRegisters {
        address: 1,
        values: vec![5],
    };
    assert_eq!(
        links.exchange(&target, &write, WAIT).await,
        Err(LinkError::Closed)
    );
    tokio::time::sleep(closed_by_now).await;
    assert_eq!(requests.load(Ordering::SeqCst), before + 1);
}

#[cfg(unix)]
#[tokio::test]
async fn a_line_the_analyser_holds_gets_no_request_at_any_speed_until_released() {
    use modbusio::endpoint::{Parity, SerialSettings};
    let port = common::trunk();
    let links = common::links();
    let at = |baud| {
        Endpoint::Serial(SerialSettings {
            port: port.clone(),
            baud,
            parity: Parity::Even,
            stop_bits: 1,
        })
    };
    links.hold(&at(19200)).await;
    for baud in [9600, 19200] {
        let refused = links
            .exchange(
                &common::target(at(baud), 5),
                &read(Area::InputRegister, 0, 1),
                WAIT,
            )
            .await;
        assert!(matches!(refused, Err(LinkError::NotSent(_))), "{refused:?}");
    }
    links.release(&at(19200));
    let answer = links
        .exchange(
            &common::target(at(19200), 5),
            &read(Area::InputRegister, 0, 1),
            WAIT,
        )
        .await;
    assert!(answer.is_ok(), "{answer:?}");
}

//! Reading points: blocks planned, made smaller where the device refuses, what was learnt kept
//! (concept section 6). And the explorer at the end of the address range (section 5.3).

mod common;

use std::time::{Duration, Instant};

use common::sim;
use modbusio::access::Limits;
use modbusio::endpoint::Endpoint;
use modbusio::explore::explore;
use modbusio::profile::Point;
use modbusio::protocol::Area;
use modbusio::read::read_points;
use modbusio::values::{DataType, WordOrder};

const WAIT: Duration = Duration::from_millis(500);

async fn device(device: sim::Device) -> std::net::SocketAddr {
    let (listener, address) = common::listener().await;
    tokio::spawn(sim::serve_tcp(
        listener,
        sim::Units::new(vec![(1, device)]),
        false,
    ));
    address
}

fn register(name: &str, address: u16) -> Point {
    Point::new(name, Area::HoldingRegister, address, DataType::U16)
}

#[tokio::test]
async fn a_device_refusing_gaps_is_read_point_by_point_and_remembered() {
    let mut map = sim::Device {
        most_registers: 125,
        ..sim::Device::default()
    };
    for address in [0, 1, 5] {
        map.holding.insert(address, address * 10);
    }
    let target = common::target(Endpoint::Tcp(device(map).await), 1);
    let links = common::links();
    let mut limits = Limits::default();
    let points = [register("a", 0), register("b", 1), register("c", 5)];
    let readings = read_points(&links, &target, WordOrder::Abcd, &points, &mut limits, WAIT).await;
    let texts: Vec<&str> = readings
        .iter()
        .map(|reading| reading.text.as_str())
        .collect();
    assert_eq!(texts, ["0", "10", "50"]);
    assert!(limits.no_holes, "the refused block across 2..4 is learnt");
}

#[tokio::test]
async fn a_device_taking_few_registers_gets_smaller_blocks() {
    let mut map = sim::Device {
        most_registers: 4,
        ..sim::Device::default()
    };
    for address in 0..10 {
        map.holding.insert(address, address);
    }
    let target = common::target(Endpoint::Tcp(device(map).await), 1);
    let links = common::links();
    let mut limits = Limits::default();
    let points: Vec<Point> = (0..10)
        .map(|address| register(&format!("p{address}"), address))
        .collect();
    let readings = read_points(&links, &target, WordOrder::Abcd, &points, &mut limits, WAIT).await;
    assert!(
        readings.iter().all(|reading| reading.error.is_none()),
        "{readings:?}"
    );
    assert!(
        limits.most_registers <= 4,
        "learnt {}",
        limits.most_registers
    );
}

#[tokio::test]
async fn a_silent_device_fails_fast_instead_of_point_by_point() {
    let address = common::gateway_without_answer().await;
    let target = common::target(Endpoint::RtuOverTcp(address), 9);
    let links = common::links();
    let mut limits = Limits::default();
    let points: Vec<Point> = (0..20)
        .map(|address| register(&format!("p{address}"), address * 10))
        .collect();
    let started = Instant::now();
    let readings = read_points(
        &links,
        &target,
        WordOrder::Abcd,
        &points,
        &mut limits,
        Duration::from_millis(200),
    )
    .await;
    assert!(
        readings
            .iter()
            .all(|reading| reading.error.as_deref() == Some("timeout"))
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(limits, Limits::default(), "silence teaches nothing");
}

#[tokio::test]
async fn scale_registers_and_swapped_floats_read_as_values() {
    let address = common::gateway().await;
    let target = common::target(Endpoint::Tcp(address), 1);
    let links = common::links();
    let mut limits = Limits::default();
    let mut volume = register("Volume", 8);
    volume.scale_register = Some(9);
    volume.unit = "m³".into();
    let mut energy = Point::new("Energy", Area::HoldingRegister, 0, DataType::F32);
    energy.unit = "kWh".into();
    let mut serial = Point::new("Serial", Area::HoldingRegister, 20, DataType::Text(6));
    serial.order = Some(WordOrder::Abcd);
    let readings = read_points(
        &links,
        &target,
        WordOrder::Cdab,
        &[volume, energy, serial],
        &mut limits,
        WAIT,
    )
    .await;
    let texts: Vec<&str> = readings
        .iter()
        .map(|reading| reading.text.as_str())
        .collect();
    assert_eq!(texts, ["88.21 m³", "1234.5 kWh", "HM5012345678"]);
}

#[tokio::test]
async fn the_explorer_reads_up_to_the_last_address() {
    let mut map = sim::Device {
        most_registers: 125,
        ..sim::Device::default()
    };
    map.holding.insert(65534, 7);
    map.holding.insert(65535, 8);
    map.input.insert(65533, 9);
    let target = common::target(Endpoint::Tcp(device(map).await), 1);
    let links = common::links();
    let mut limits = Limits::default();
    let cells = |explored: modbusio::explore::Explored| -> Vec<(u16, Option<u16>)> {
        assert_eq!(explored.stopped, None);
        explored
            .cells
            .iter()
            .map(|cell| (cell.address, cell.value))
            .collect()
    };

    let holding = explore(
        &links,
        &target,
        Area::HoldingRegister,
        65532,
        100,
        &mut limits,
        WAIT,
    )
    .await;
    assert_eq!(
        cells(holding),
        [
            (65532, None),
            (65533, None),
            (65534, Some(7)),
            (65535, Some(8))
        ]
    );

    // nothing at the last address: it is listed as missing, not left out
    let input = explore(
        &links,
        &target,
        Area::InputRegister,
        65533,
        3,
        &mut limits,
        WAIT,
    )
    .await;
    assert_eq!(
        cells(input),
        [(65533, Some(9)), (65534, None), (65535, None)]
    );
}

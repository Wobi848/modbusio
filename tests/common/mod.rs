//! What the integration tests share: the simulated devices of `examples/`, started in the test on
//! a free loopback port.

#![allow(dead_code)]

#[path = "../../examples/sim/mod.rs"]
pub mod sim;

use std::net::SocketAddr;
use std::sync::Arc;

use modbusio::capture::Capture;
use modbusio::endpoint::{Endpoint, Target};
use modbusio::links::Links;

pub async fn listener() -> (tokio::net::TcpListener, SocketAddr) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a free port");
    let address = listener.local_addr().expect("bound");
    (listener, address)
}

/// The energy meter on TCP, answering every unit.
pub async fn meter(one_connection: bool) -> SocketAddr {
    let (listener, address) = listener().await;
    let units = sim::Units::new(vec![(1, sim::energy_meter())]).answering_any_unit();
    tokio::spawn(sim::serve_tcp(listener, units, one_connection));
    address
}

pub fn gateway_units() -> sim::Units {
    sim::Units::new(vec![
        (1, sim::heat_meter()),
        (2, sim::drive()),
        (5, sim::room_controller()),
    ])
}

/// The gateway on TCP: units 1, 2, 5; others exception 0B after 50 ms.
pub async fn gateway() -> SocketAddr {
    let (listener, address) = listener().await;
    let units = gateway_units().behind_gateway(std::time::Duration::from_millis(50));
    tokio::spawn(sim::serve_tcp(listener, units, false));
    address
}

/// The gateway passing RTU over TCP: units 1, 2, 5; others silent.
pub async fn rtu_gateway() -> SocketAddr {
    let (listener, address) = listener().await;
    tokio::spawn(sim::serve_rtu_over_tcp(listener, gateway_units()));
    address
}

/// A serial gateway (RTU over TCP) with nothing behind it: every request stays unanswered.
pub async fn gateway_without_answer() -> SocketAddr {
    let (listener, address) = listener().await;
    tokio::spawn(sim::serve_rtu_over_tcp(
        listener,
        sim::Units::new(Vec::new()),
    ));
    address
}

/// The trunk of `simulated_rtu` on a pseudo-terminal; returns the terminal's name.
#[cfg(unix)]
pub fn trunk() -> String {
    use tokio_serial::SerialPort;
    let (ours, theirs) = tokio_serial::SerialStream::pair().expect("a pseudo-terminal");
    let name = theirs.name().expect("a name");
    std::mem::forget(theirs);
    tokio::spawn(tokio_modbus::server::rtu::Server::new(ours).serve_forever(gateway_units()));
    name
}

/// The trunk of `simulated_rtu --settings`: its devices answer only at settings of their own.
#[cfg(unix)]
pub fn trunk_with_own_settings() -> String {
    use tokio_serial::SerialPort;
    let (ours, theirs) = tokio_serial::SerialStream::pair().expect("a pseudo-terminal");
    let name = theirs.name().expect("a name");
    std::mem::forget(theirs);
    let units = gateway_units().with_own_settings(sim::line_settings(&ours), &sim::OWN_SETTINGS);
    tokio::spawn(tokio_modbus::server::rtu::Server::new(ours).serve_forever(units));
    name
}

/// The simulated trunk with its master (`examples/sim/trunk.rs`) on a pseudo-terminal; returns
/// the terminal's name.
#[cfg(unix)]
pub fn played_trunk(options: sim::trunk::Options) -> String {
    use tokio_serial::SerialPort;
    let (ours, theirs) = tokio_serial::SerialStream::pair().expect("a pseudo-terminal");
    let name = theirs.name().expect("a name");
    std::mem::forget(theirs);
    tokio::spawn(sim::trunk::play(ours, options));
    name
}

pub fn links() -> Links {
    Links::new(Arc::new(Capture::default()))
}

pub fn target(endpoint: Endpoint, unit: u8) -> Target {
    Target { endpoint, unit }
}

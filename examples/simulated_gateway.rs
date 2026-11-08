//! A simulated serial gateway: `cargo run --example simulated_gateway -- <IP> [port] [--rtu]`.
//! Behind it unit 1 a heat meter (floats CDAB, a scale register), unit 2 a drive (gaps, at most
//! 16 registers per request, refuses 16 for one register), unit 5 a room controller. Other units
//! answer exception 0B after 200 ms, as a gateway does when nothing answers on its line. With
//! `--rtu` it passes RTU frames over TCP (transparent mode) and stays silent for missing units.

mod sim;

use std::time::Duration;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ip = args.first().map_or("127.0.0.3", String::as_str);
    let port = args
        .get(1)
        .and_then(|port| port.parse().ok())
        .unwrap_or(502u16);
    let rtu = args.iter().any(|arg| arg == "--rtu");
    let address: std::net::SocketAddr = format!("{ip}:{port}").parse().expect("an IP address");
    let listener = tokio::net::TcpListener::bind(address).await?;
    let units = sim::Units::new(vec![
        (1, sim::heat_meter()),
        (2, sim::drive()),
        (5, sim::room_controller()),
    ]);
    if rtu {
        println!("gateway (RTU over TCP) on {address}: units 1, 2, 5");
        sim::serve_rtu_over_tcp(listener, units).await
    } else {
        println!("gateway on {address}: units 1, 2, 5");
        sim::serve_tcp(
            listener,
            units.behind_gateway(Duration::from_millis(200)),
            false,
        )
        .await
    }
}

//! A simulated electricity meter on Modbus TCP: `cargo run --example simulated_meter -- <IP> [port]
//! [--one-connection]`. Answers every unit ID, as direct TCP devices do; floats in ABCD, energy as
//! u32, identification (43/14). With `--one-connection` it takes one connection at a time.

mod sim;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ip = args.first().map_or("127.0.0.2", String::as_str);
    let port = args
        .get(1)
        .and_then(|port| port.parse().ok())
        .unwrap_or(502u16);
    let one_connection = args.iter().any(|arg| arg == "--one-connection");
    let address: std::net::SocketAddr = format!("{ip}:{port}").parse().expect("an IP address");
    let listener = tokio::net::TcpListener::bind(address).await?;
    let units = sim::Units::new(vec![(1, sim::energy_meter())]).answering_any_unit();
    println!(
        "energy meter on {address}{}",
        if one_connection {
            ", one connection"
        } else {
            ""
        }
    );
    sim::serve_tcp(listener, units, one_connection).await
}

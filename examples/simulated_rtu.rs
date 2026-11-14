//! A simulated RS-485 trunk on a pseudo-terminal (Linux): `cargo run --example simulated_rtu`.
//! It prints the terminal's name (`/dev/pts/7`), which modbusio opens like a USB RS-485 adapter.
//! On it: unit 1 a heat meter, unit 2 a drive, unit 5 a room controller; other units are silent.
//! A pseudo-terminal has no speed or parity, so every setting reaches the devices. With
//! `--settings` the devices have settings of their own, for finding devices at other settings
//! (docs/concept-other-settings.md): the trunk runs 19200 with one stop bit, the drive (unit 2) was
//! left at 9600, the room controller (unit 5) at two stop bits. A pseudo-terminal carries speed and
//! stop bits to this side, not parity.

mod sim;

#[cfg(unix)]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    use tokio_serial::SerialPort;
    let (ours, theirs) = tokio_serial::SerialStream::pair().map_err(std::io::Error::other)?;
    let name = theirs.name().unwrap_or_default();
    // the other side is opened by modbusio; keep it open here so the terminal lives
    std::mem::forget(theirs);
    println!("{name}");
    let units = sim::Units::new(vec![
        (1, sim::heat_meter()),
        (2, sim::drive()),
        (5, sim::room_controller()),
    ]);
    #[cfg(target_os = "linux")]
    let units = if std::env::args().any(|arg| arg == "--settings") {
        units.with_own_settings(sim::line_settings(&ours), &sim::OWN_SETTINGS)
    } else {
        units
    };
    tokio_modbus::server::rtu::Server::new(ours)
        .serve_forever(units)
        .await
}

#[cfg(not(unix))]
fn main() {
    eprintln!("simulated_rtu needs a pseudo-terminal (Linux)");
}

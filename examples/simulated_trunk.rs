//! A whole RS-485 trunk played on a pseudo-terminal (Linux), for the bus analyser
//! (docs/concept-analyser.md): `cargo run --example simulated_trunk`. It prints the terminal's
//! name (`/dev/pts/7`), which modbusio opens like a USB adapter hung on the trunk in parallel, and
//! writes what a master and its units would send. It reads nothing: the analyser never sends.
//!
//! The master polls every cycle: unit 1 a heat meter (HR 0, 8 registers), unit 5 a room controller
//! (IR 0, 4 registers), unit 12 a drive (HR 100, 2 registers) that answers slowly, unit 7 a device
//! (HR 0, 10 registers, and HR 100, 21 registers, which it refuses with exception 02), and unit 9
//! that never answers. Faults, unless `--clean`: unit 12 misses every third answer after unit 5;
//! every eighth answer of unit 5 arrives damaged. Every tenth cycle the master writes the drive's
//! set frequency (HR 1), every twentieth it writes to all units at once (unit 0).
//! `--second-master` lets another master ask unit 1 while unit 5's answer is due. `--cycle-ms 2000` sets the cycle.

mod sim;

#[cfg(unix)]
fn options() -> sim::trunk::Options {
    let mut options = sim::trunk::Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--clean" => options.clean = true,
            "--second-master" => options.second_master = true,
            "--cycle-ms" => {
                let millis = args
                    .next()
                    .and_then(|text| text.parse().ok())
                    .unwrap_or(2000);
                options.cycle = std::time::Duration::from_millis(millis);
            }
            other => eprintln!("unknown option {other}"),
        }
    }
    options
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    use tokio_serial::SerialPort;

    let options = options();
    let (ours, theirs) = tokio_serial::SerialStream::pair().map_err(std::io::Error::other)?;
    let name = theirs.name().unwrap_or_default();
    // the other side is opened by modbusio; keep it open here so the terminal lives
    std::mem::forget(theirs);
    println!("{name}");
    sim::trunk::play(ours, options).await
}

#[cfg(not(unix))]
fn main() {
    eprintln!("simulated_trunk needs a pseudo-terminal (Linux)");
}

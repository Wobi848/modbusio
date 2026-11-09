//! The command line (concept section 12, M1): the same logic as the web interface, as text.

mod table;

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::{Context, anyhow, bail};
use clap::Subcommand;
use modbusio::access::Limits;
use modbusio::capture::Capture;
use modbusio::endpoint::{Endpoint, SerialSettings, Target};
use modbusio::interfaces::{self, Subnet};
use modbusio::links::{ANSWER_TIMEOUT, Links};
use modbusio::profile::{self, Point};
use modbusio::protocol::Area;
use modbusio::protocol::tcp::MODBUS_PORT;
use modbusio::scan::{self, Search};
use modbusio::values::{DataType, WordOrder};
use modbusio::write::{self, Journal, Planned};
use modbusio::{explore, listen, serial, storage};

#[derive(Subcommand)]
pub enum Command {
    /// List network adapters (* = used by default)
    Interfaces,
    /// List serial ports, USB adapters first
    Ports,
    /// Find devices: a TCP subnet, the units behind a gateway, or a serial line
    Scan {
        /// Ask every address of this subnet, e.g. 192.168.10.0/24 (default: the adapter's)
        #[arg(long, value_name = "NET/BITS", value_parser = Subnet::parse)]
        subnet: Option<Subnet>,
        /// IP address of the network adapter whose subnet to ask
        #[arg(long)]
        interface: Option<Ipv4Addr>,
        /// TCP port
        #[arg(long, default_value_t = MODBUS_PORT)]
        port: u16,
        /// Ask the units behind this gateway (IP or IP:PORT) instead of a subnet
        #[arg(long, value_name = "ADDRESS")]
        gateway: Option<String>,
        /// The gateway passes RTU frames over TCP (transparent mode)
        #[arg(long)]
        rtu: bool,
        /// Ask the units on this serial port (COM3, /dev/ttyUSB0)
        #[arg(long, value_name = "PORT")]
        serial: Option<String>,
        /// Speed of the serial line; leave out with --find
        #[arg(long, default_value_t = 9600)]
        baud: u32,
        /// Frame of the serial line: 8E1, 8N1, 8N2, 8O1
        #[arg(long, default_value = "8E1")]
        frame: String,
        /// Find the speed and frame first, trying units 1 to 10
        #[arg(long)]
        find: bool,
        /// First unit ID to ask
        #[arg(long, default_value_t = 1)]
        from: u8,
        /// Last unit ID to ask
        #[arg(long, default_value_t = 247)]
        to: u8,
        /// Milliseconds to wait for each answer
        #[arg(long, default_value_t = 300)]
        timeout: u64,
        /// Tries more for a silent unit
        #[arg(long, default_value_t = 0)]
        retries: u8,
    },
    /// Listen to a serial line without sending: another master, its settings, the units talking
    Listen {
        /// Serial port
        port: String,
        /// Seconds per setting
        #[arg(long, default_value_t = 1.5)]
        seconds: f64,
    },
    /// Read values, e.g.: read 10.0.0.5/1 ir 0 2 --type f32
    Read {
        #[command(flatten)]
        place: Place,
        /// How many values of the type
        #[arg(default_value_t = 1)]
        count: u16,
        /// Save the values as CSV (for Excel)
        #[arg(long, value_name = "FILE")]
        csv: Option<PathBuf>,
    },
    /// Show which addresses exist and what each register could mean, e.g.: explore 10.0.0.5/1 hr 0 100
    Explore {
        /// Device: 10.0.0.5/1, rtu-tcp:10.0.0.9:4001/2, serial:COM3:9600:8E1/5
        #[arg(value_parser = parse_target)]
        device: Target,
        /// coil, di, ir, hr
        #[arg(value_parser = parse_area)]
        area: Area,
        start: u16,
        #[arg(default_value_t = 100)]
        count: u32,
    },
    /// Write a value, e.g.: write 10.0.0.5/2 hr 1 3000; reads the old value first and logs it
    Write {
        #[command(flatten)]
        place: Place,
        /// New value: a number, or on/off for coils
        value: String,
    },
    /// List the values written and not put back yet
    Changed,
    /// Put back the old value of a write, by its number from `changed`, or all of them
    Restore {
        /// Number of the write, or `all`
        which: String,
    },
}

/// One place on a device: device, area, address, type.
#[derive(clap::Args)]
pub struct Place {
    /// Device: 10.0.0.5/1, rtu-tcp:10.0.0.9:4001/2, serial:COM3:9600:8E1/5
    #[arg(value_parser = parse_target)]
    device: Target,
    /// coil, di, ir, hr
    #[arg(value_parser = parse_area)]
    area: Area,
    /// Address on the wire, from 0
    address: u16,
    /// u16 s16 u32 s32 u64 s64 f32 f64 text20 bit3
    #[arg(long = "type", default_value = "u16", value_parser = parse_type)]
    kind: DataType,
    /// Word order of multi-register values: ABCD CDAB BADC DCBA
    #[arg(long, default_value = "ABCD", value_parser = parse_order)]
    order: WordOrder,
    /// Multiply the raw value by this
    #[arg(long, default_value_t = 1.0)]
    factor: f64,
}

impl Place {
    fn point(&self, offset: u16) -> anyhow::Result<Point> {
        let address =
            u32::from(self.address) + u32::from(offset) * u32::from(self.kind.registers());
        let address = u16::try_from(address).map_err(|_| anyhow!("runs past address 65535"))?;
        let mut point = Point::new(
            &format!("{} {address}", self.area.short()),
            self.area,
            address,
            self.kind,
        );
        point.order = Some(self.order);
        point.scale.factor = self.factor;
        point.check().map_err(|error| anyhow!(error))?;
        Ok(point)
    }
}

fn parse_target(text: &str) -> Result<Target, String> {
    Target::parse_key(text)
}

fn parse_area(text: &str) -> Result<Area, String> {
    Area::parse(text).ok_or_else(|| format!("{text} is not coil, di, ir or hr"))
}

fn parse_type(text: &str) -> Result<DataType, String> {
    DataType::parse(text).ok_or_else(|| format!("{text} is not a type like u16, s32 or f32"))
}

fn parse_order(text: &str) -> Result<WordOrder, String> {
    WordOrder::parse(text).ok_or_else(|| format!("{text} is not ABCD, CDAB, BADC or DCBA"))
}

pub async fn run(command: Command, data_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let links = Arc::new(Links::new(Arc::new(Capture::default())));
    match command {
        Command::Interfaces => interfaces_list(),
        Command::Ports => ports(),
        Command::Scan {
            subnet,
            interface,
            port,
            gateway,
            rtu,
            serial,
            baud,
            frame,
            find,
            from,
            to,
            timeout,
            retries,
        } => {
            let timeout = Duration::from_millis(timeout);
            if from > to {
                bail!("--from {from} is above --to {to}");
            }
            if let Some(port) = serial {
                scan_serial(
                    links,
                    port,
                    baud,
                    &frame,
                    find,
                    (from, to),
                    timeout,
                    retries,
                )
                .await
            } else if let Some(gateway) = gateway {
                let endpoint = if rtu {
                    format!("rtu-tcp:{gateway}")
                } else {
                    format!("tcp:{gateway}")
                };
                let endpoint: Endpoint =
                    endpoint.parse().map_err(|error: String| anyhow!(error))?;
                let search = Arc::new(Search::default());
                scan::search_units(links, endpoint, from, to, timeout, retries, search.clone())
                    .await;
                print_report(&search);
                Ok(())
            } else {
                scan_tcp(links, subnet, interface, port, timeout).await
            }
        }
        Command::Listen { port, seconds } => listen_to(&port, seconds).await,
        Command::Read { place, count, csv } => read(&links, &place, count, csv).await,
        Command::Explore {
            device,
            area,
            start,
            count,
        } => explore_device(&links, &device, area, start, count).await,
        Command::Write { place, value } => write_value(&links, &place, &value, data_dir).await,
        Command::Changed => changed(data_dir),
        Command::Restore { which } => restore(&links, &which, data_dir).await,
    }
}

fn interfaces_list() -> anyhow::Result<()> {
    let adapters = interfaces::list(false).context("cannot list network adapters")?;
    let default = interfaces::pick_default(&adapters).map(|adapter| adapter.ip);
    let rows = adapters
        .iter()
        .map(|adapter| {
            vec![
                if Some(adapter.ip) == default {
                    "*".into()
                } else {
                    String::new()
                },
                adapter.name.clone(),
                format!("{}/{}", adapter.ip, adapter.prefix_len),
                adapter.description.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    table::print(&["", "Adapter", "Address", "Description"], &rows);
    Ok(())
}

fn ports() -> anyhow::Result<()> {
    let ports = serial::ports().map_err(|error| anyhow!(error))?;
    if ports.is_empty() {
        println!("No serial port found. Is the USB adapter plugged in?");
        return Ok(());
    }
    let rows = ports
        .iter()
        .map(|port| {
            vec![
                port.name.clone(),
                if port.usb {
                    "USB".into()
                } else {
                    String::new()
                },
                port.description.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    table::print(&["Port", "", "Adapter"], &rows);
    Ok(())
}

async fn scan_tcp(
    links: Arc<Links>,
    subnet: Option<Subnet>,
    interface: Option<Ipv4Addr>,
    port: u16,
    timeout: Duration,
) -> anyhow::Result<()> {
    let (subnet, own) = match subnet {
        Some(subnet) => (subnet, None),
        None => {
            let adapter = interfaces::choose(interface, false)?;
            println!(
                "Adapter {} {}/{}",
                adapter.name, adapter.ip, adapter.prefix_len
            );
            (adapter.subnet(), Some(adapter.ip))
        }
    };
    let hosts = subnet.hosts(own).ok_or_else(|| {
        anyhow!(
            "{}/{} is too large to ask address by address (up to /22)",
            subnet.address,
            subnet.prefix_len
        )
    })?;
    println!("Asking {} addresses on port {port} …", hosts.len());
    let search = Arc::new(Search::default());
    let connect = timeout.max(Duration::from_millis(300));
    scan::scan_tcp(
        links,
        hosts,
        port,
        connect,
        timeout.max(ANSWER_TIMEOUT / 2),
        search.clone(),
    )
    .await;
    let report = search.report.lock().expect("report lock");
    if report.hosts.is_empty() {
        println!(
            "{}",
            modbusio::diagnosis::Note::NoHostOpen {
                subnet: format!("{}/{}", subnet.address, subnet.prefix_len),
                port
            }
        );
    }
    drop(report);
    print_report(&search);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn scan_serial(
    links: Arc<Links>,
    port: String,
    baud: u32,
    frame: &str,
    find: bool,
    (from, to): (u8, u8),
    timeout: Duration,
    retries: u8,
) -> anyhow::Result<()> {
    let settings = if find {
        println!("Trying the usual speeds and frames with units 1 to 10 …");
        let search = Arc::new(Search::default());
        let found = scan::find_settings(
            links.clone(),
            port.clone(),
            scan::SETTINGS_UNITS.to_vec(),
            timeout,
            search.clone(),
        )
        .await;
        print_notes(&search);
        let Some(found) = found else {
            return Ok(());
        };
        found
    } else {
        let (parity, stop_bits) = SerialSettings::parse_frame(frame)
            .ok_or_else(|| anyhow!("{frame} is not a frame like 8E1"))?;
        SerialSettings {
            port,
            baud,
            parity,
            stop_bits,
        }
    };
    let endpoint = Endpoint::Serial(settings);
    let search = Arc::new(Search::default());
    let most = timeout * u32::from(retries + 1) * (u32::from(to - from) + 1);
    println!(
        "Asking units {from} to {to} on {} (at most {} s) …",
        scan::line_text(&endpoint),
        most.as_secs()
    );
    scan::search_units(links, endpoint, from, to, timeout, retries, search.clone()).await;
    print_report(&search);
    Ok(())
}

fn print_notes(search: &Search) {
    for note in &search.report.lock().expect("report lock").notes {
        println!("{}", note.text);
    }
}

fn print_report(search: &Search) {
    let report = search.report.lock().expect("report lock");
    let rows = report
        .devices
        .iter()
        .map(|device| {
            vec![
                device.target.to_string(),
                device.vendor.clone().unwrap_or_default(),
                device.product.clone().unwrap_or_default(),
                device.revision.clone().unwrap_or_default(),
                device.key.clone(),
            ]
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        println!("No device found.");
    } else {
        table::print(
            &[
                "Device",
                "Vendor",
                "Product",
                "Version",
                "Key for read/write",
            ],
            &rows,
        );
    }
    drop(report);
    print_notes(search);
}

async fn listen_to(port: &str, seconds: f64) -> anyhow::Result<()> {
    let each = Duration::from_secs_f64(seconds.clamp(0.2, 30.0));
    println!("Listening to {port} without sending …");
    let listened = listen::listen(port, each).await?;
    let rows = listened
        .trials
        .iter()
        .map(|trial| {
            vec![
                trial.settings.clone(),
                trial.heard.frames.to_string(),
                trial.heard.stray_bytes.to_string(),
                trial
                    .heard
                    .units
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            ]
        })
        .collect::<Vec<_>>();
    table::print(&["Setting", "Frames", "Stray bytes", "Units"], &rows);
    let note = match &listened.active {
        Some(settings) => modbusio::diagnosis::Note::OtherMaster {
            port: port.into(),
            settings: settings.clone(),
        },
        None if listened.quiet => modbusio::diagnosis::Note::Quiet { port: port.into() },
        None => modbusio::diagnosis::Note::Damaged {
            line: port.into(),
            count: listened
                .trials
                .iter()
                .map(|trial| trial.heard.stray_bytes)
                .sum(),
        },
    };
    println!("{note}");
    Ok(())
}

async fn read(
    links: &Links,
    place: &Place,
    count: u16,
    csv: Option<PathBuf>,
) -> anyhow::Result<()> {
    let points = (0..count.max(1))
        .map(|offset| place.point(offset))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut limits = Limits::default();
    let readings = modbusio::read::read_points(
        links,
        &place.device,
        place.order,
        &points,
        &mut limits,
        ANSWER_TIMEOUT,
    )
    .await;
    let rows = points
        .iter()
        .zip(&readings)
        .map(|(point, reading)| {
            vec![
                format!("{} {}", point.area.short(), point.address),
                reading.text.clone(),
                reading.raw.join(" "),
                reading.error.as_deref().map(error_text).unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    table::print(&["Address", "Value", "Raw", "Error"], &rows);
    if let Some(path) = csv {
        let mut lines = vec![vec![
            "Address".to_string(),
            "Value".into(),
            "Raw".into(),
            "Error".into(),
        ]];
        lines.extend(rows);
        std::fs::write(&path, modbusio::csv::file(&lines))
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    Ok(())
}

/// An error code from the reading in words.
fn error_text(code: &str) -> String {
    match code
        .strip_prefix("exception_")
        .and_then(|hex| u8::from_str_radix(hex, 16).ok())
    {
        Some(exception) => modbusio::protocol::Exception(exception).to_string(),
        None => match code {
            "timeout" => "no answer in time".into(),
            "damaged" => "damaged answer".into(),
            "crc" => "CRC error".into(),
            "malformed" => "answer does not fit".into(),
            "other_unit" => "another unit answered".into(),
            "not_sent" => "not sent".into(),
            "closed" => "connection closed".into(),
            "open" => "cannot connect".into(),
            "io" => "connection failed".into(),
            other => other.replace('_', " "),
        },
    }
}

async fn explore_device(
    links: &Links,
    device: &Target,
    area: Area,
    start: u16,
    count: u32,
) -> anyhow::Result<()> {
    let mut limits = Limits::default();
    let explored = explore::explore(
        links,
        device,
        area,
        start,
        count,
        &mut limits,
        ANSWER_TIMEOUT,
    )
    .await;
    let (rows, order) = explore::interpret(&explored.cells);
    let lines = rows
        .iter()
        .filter(|row| row.value.is_some())
        .map(|row| {
            let pair = |order: WordOrder| {
                row.pairs
                    .iter()
                    .find(|pair| pair.order == order)
                    .map(|pair| pair.float.clone())
                    .unwrap_or_default()
            };
            vec![
                format!("{} {}", area.short(), row.address),
                row.value.map(|value| value.to_string()).unwrap_or_default(),
                row.signed
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                row.hex.clone().unwrap_or_default(),
                row.text.clone().unwrap_or_default(),
                pair(WordOrder::Abcd),
                pair(WordOrder::Cdab),
            ]
        })
        .collect::<Vec<_>>();
    table::print(
        &[
            "Address", "u16", "s16", "Hex", "Text", "f32 ABCD", "f32 CDAB",
        ],
        &lines,
    );
    let missing = rows.iter().filter(|row| row.value.is_none()).count();
    println!("{} addresses with data, {missing} without.", lines.len());
    if let Some(order) = order {
        println!(
            "{}",
            modbusio::diagnosis::Note::WordOrder {
                order: order.name().into()
            }
        );
    }
    if limits.most_registers < modbusio::protocol::pdu::MOST_REGISTERS && !area.is_bits() {
        println!(
            "{}",
            modbusio::diagnosis::Note::BlockLimit {
                count: limits.most_registers
            }
        );
    }
    if let Some(error) = explored.stopped {
        println!("Stopped: {error}");
    }
    Ok(())
}

fn journal_of(data_dir: &Option<PathBuf>) -> anyhow::Result<(Journal, Option<PathBuf>)> {
    let path = data_dir.as_ref().map(|dir| dir.join(storage::JOURNAL));
    let mut journal = Journal::default();
    if let Some(path) = &path {
        let (entries, skipped) =
            storage::load_entries(path).context("cannot read the write log")?;
        if skipped > 0 {
            eprintln!("{skipped} lines of the write log could not be read.");
        }
        journal.entries = entries;
    }
    Ok((journal, path))
}

async fn write_value(
    links: &Links,
    place: &Place,
    value: &str,
    data_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    if !place.area.writable() {
        bail!("{} cannot be written", place.area);
    }
    let point = place.point(0)?;
    let registers =
        profile::registers_for(&point, place.order, value, 0).map_err(|error| anyhow!(error))?;
    let (journal, path) = journal_of(&data_dir)?;
    let journal = Mutex::new(journal);
    let planned = Planned {
        target: place.device.clone(),
        point: None,
        area: place.area,
        address: place.address,
        values: registers,
        text: value.to_string(),
        restores: None,
    };
    let describe = |registers: &[u16]| profile::evaluate(&point, place.order, registers, 0).text;
    let mut limits = Limits::default();
    let entry = write::journaled(
        links,
        planned,
        &mut limits,
        ANSWER_TIMEOUT,
        &journal,
        path.as_deref(),
        describe,
        SystemTime::now(),
    )
    .await
    .map_err(|error| anyhow!(error))?;
    report_entry(&entry)
}

fn report_entry(entry: &write::Entry) -> anyhow::Result<()> {
    if let Some(error) = &entry.error {
        bail!("not written: {error}");
    }
    println!(
        "Written: {} (was {}). Number {} in the log; put back with: restore {}",
        entry.text,
        entry.before_text.as_deref().unwrap_or("?"),
        entry.id,
        entry.id
    );
    if entry.kept_other() {
        println!(
            "The device confirmed but holds another value: {:?}",
            entry.after
        );
    }
    Ok(())
}

fn changed(data_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let (journal, _) = journal_of(&data_dir)?;
    let standing = journal.changed();
    if standing.is_empty() {
        println!("Nothing written that is not put back.");
        return Ok(());
    }
    let rows = standing
        .iter()
        .map(|entry| {
            vec![
                entry.id.to_string(),
                entry.device.clone(),
                format!("{} {}", entry.area.short(), entry.address),
                entry.text.clone(),
                entry.before_text.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    table::print(&["Number", "Device", "Address", "Written", "Before"], &rows);
    Ok(())
}

async fn restore(links: &Links, which: &str, data_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let (journal, path) = journal_of(&data_dir)?;
    let chosen: Vec<write::Entry> = if which == "all" {
        journal.changed().into_iter().cloned().collect()
    } else {
        let id: u64 = which
            .parse()
            .map_err(|_| anyhow!("{which} is not a number from `changed` or all"))?;
        let standing = journal.changed();
        let entry = standing
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| anyhow!("number {id} is not among the values still written"))?;
        vec![(*entry).clone()]
    };
    let plans: Vec<(write::Entry, Vec<u16>)> = chosen
        .into_iter()
        .map(|entry| {
            let original = journal.original(&entry);
            (entry, original.unwrap_or_default())
        })
        .collect();
    let journal = Mutex::new(journal);
    for (entry, original) in plans {
        if original.is_empty() {
            println!("{}: the old value is not known, nothing put back", entry.id);
            continue;
        }
        let target = Target::parse_key(&entry.device).map_err(|error| anyhow!(error))?;
        let planned = Planned {
            target,
            point: entry.point.clone(),
            area: entry.area,
            address: entry.address,
            text: entry
                .before_text
                .clone()
                .unwrap_or_else(|| format!("{original:?}")),
            values: original,
            restores: Some(entry.id),
        };
        let describe = |registers: &[u16]| format!("{registers:?}");
        let mut limits = Limits::default();
        let restored = write::journaled(
            links,
            planned,
            &mut limits,
            ANSWER_TIMEOUT,
            &journal,
            path.as_deref(),
            describe,
            SystemTime::now(),
        )
        .await
        .map_err(|error| anyhow!(error))?;
        match restored.error {
            Some(error) => println!("{}: not put back: {error}", entry.id),
            None => println!("{}: put back {}", entry.id, restored.text),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::error_text;

    #[test]
    fn reading_errors_are_words() {
        assert_eq!(error_text("open"), "cannot connect");
        assert_eq!(error_text("timeout"), "no answer in time");
        assert_eq!(
            error_text("exception_02"),
            "exception 02: the device has no data at this address"
        );
    }
}

//! Finding devices (concept section 5): hosts with the Modbus port open in a subnet, the units of a
//! host, behind a gateway or on a serial line, the line's settings, and who each device is.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tokio::task::JoinSet;

use crate::diagnosis::{Note, NoteView};
use crate::endpoint::{BAUD_RATES, Endpoint, SerialSettings, Target};
use crate::link::LinkError;
use crate::links::Links;
use crate::protocol::{Area, Request, Response};

/// The unit IDs a direct TCP device usually answers on.
pub const DIRECT_UNITS: [u8; 3] = [1, 255, 0];
/// A unit no one uses: a device answering it answers every unit.
const UNLIKELY_UNIT: u8 = 247;
/// Hosts asked at once.
const PARALLEL_HOSTS: usize = 64;
/// Units a settings search tries at each speed: most devices sit low.
pub const SETTINGS_UNITS: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

#[derive(Clone, Debug, Default, Serialize)]
pub struct Report {
    /// `hosts`, `units`, `settings`, `identify`.
    pub phase: String,
    pub done: usize,
    pub total: usize,
    pub devices: Vec<Found>,
    /// TCP hosts with the port open.
    pub hosts: Vec<Host>,
    pub notes: Vec<NoteView>,
    pub finished: bool,
    /// The settings a settings search found.
    pub settings: Option<String>,
    /// Seconds the search may take at most, for a serial line.
    pub estimate_seconds: Option<u64>,
    /// The search for devices at other settings: where each unit answered.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other_settings: Vec<crate::other_settings::UnitSettings>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Found {
    pub key: String,
    #[serde(skip)]
    pub target: Target,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub revision: Option<String>,
}

impl Found {
    pub fn new(target: Target) -> Found {
        Found {
            key: target.key(),
            target,
            vendor: None,
            product: None,
            revision: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Host {
    pub address: String,
    pub gateway: bool,
    pub every_unit: bool,
    pub answered: bool,
}

/// The shared state of a running search: the report others read, and the request to stop.
#[derive(Default)]
pub struct Search {
    pub report: Mutex<Report>,
    pub stop: AtomicBool,
}

impl Search {
    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub(crate) fn update(&self, change: impl FnOnce(&mut Report)) {
        change(&mut self.report.lock().expect("report lock"));
    }

    pub(crate) fn note(&self, note: Note) {
        self.update(|report| {
            let view = NoteView::from(note);
            if !report.notes.contains(&view) {
                report.notes.push(view);
            }
        });
    }

    pub(crate) fn found(&self, found: Found) {
        self.update(|report| {
            if !report.devices.iter().any(|device| device.key == found.key) {
                report.devices.push(found);
            }
        });
    }

    pub fn finish(&self) {
        self.update(|report| report.finished = true);
    }
}

fn probe() -> Request {
    Request::Read {
        area: Area::HoldingRegister,
        address: 0,
        count: 1,
    }
}

/// What one probe of a unit showed.
#[derive(Debug, PartialEq)]
pub(crate) enum Probed {
    Answered,
    /// A gateway answered that the unit is not reached.
    Gateway,
    Silent,
    Damaged,
    OtherUnit(u8),
    /// The line failed: stop asking it.
    Failed(LinkError),
}

/// Whether one unit answers, asked once as the search asks: `None` when it does (with data or an
/// exception of its own), else the code of what went wrong (docs/api.md).
pub async fn unanswered(links: &Links, target: &Target, timeout: Duration) -> Option<String> {
    match links.exchange(target, &probe(), timeout).await {
        Ok(_) => None,
        Err(error) if error.device_answered() => None,
        Err(error) => Some(error.code()),
    }
}

pub(crate) async fn probe_unit(
    links: &Links,
    target: &Target,
    timeout: Duration,
    retries: u8,
) -> Probed {
    let mut last = Probed::Silent;
    for _ in 0..=retries {
        last = match links.exchange(target, &probe(), timeout).await {
            Ok(_) => return Probed::Answered,
            Err(error) if error.device_answered() => return Probed::Answered,
            Err(LinkError::Exception(_)) => return Probed::Gateway,
            Err(LinkError::OtherUnit(unit)) => return Probed::OtherUnit(unit),
            Err(LinkError::Crc | LinkError::Damaged { .. }) => Probed::Damaged,
            Err(LinkError::Timeout) => Probed::Silent,
            Err(error) => return Probed::Failed(error),
        };
    }
    last
}

/// Asks every address in `hosts` for port `port`, then the usual units on each open one.
pub async fn scan_tcp(
    links: Arc<Links>,
    hosts: Vec<Ipv4Addr>,
    port: u16,
    connect_timeout: Duration,
    answer_timeout: Duration,
    search: Arc<Search>,
) {
    search.update(|report| {
        report.phase = "hosts".into();
        report.total = hosts.len();
        report.done = 0;
    });
    let open = open_hosts(hosts, port, connect_timeout, &search).await;
    search.update(|report| {
        report.phase = "units".into();
        report.total = open.len();
        report.done = 0;
    });
    let mut tasks = JoinSet::new();
    for address in open {
        let links = links.clone();
        let search = search.clone();
        tasks.spawn(async move {
            if !search.stopped() {
                units_of_host(&links, address, answer_timeout, &search).await;
            }
            search.update(|report| report.done += 1);
        });
    }
    while tasks.join_next().await.is_some() {}
    identify_all(&links, answer_timeout, &search).await;
}

async fn open_hosts(
    hosts: Vec<Ipv4Addr>,
    port: u16,
    connect_timeout: Duration,
    search: &Arc<Search>,
) -> Vec<SocketAddr> {
    let limit = Arc::new(tokio::sync::Semaphore::new(PARALLEL_HOSTS));
    let mut tasks = JoinSet::new();
    for ip in hosts {
        let limit = limit.clone();
        let search = search.clone();
        tasks.spawn(async move {
            let _permit = limit.acquire().await.ok()?;
            if search.stopped() {
                return None;
            }
            let address = SocketAddr::from((ip, port));
            let connected =
                tokio::time::timeout(connect_timeout, tokio::net::TcpStream::connect(address))
                    .await;
            search.update(|report| report.done += 1);
            matches!(connected, Ok(Ok(_))).then_some(address)
        });
    }
    let mut open = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Ok(Some(address)) = result {
            open.push(address);
        }
    }
    open.sort();
    open
}

async fn units_of_host(links: &Links, address: SocketAddr, timeout: Duration, search: &Search) {
    let endpoint = Endpoint::Tcp(address);
    let mut answering = Vec::new();
    let mut gateway = false;
    for unit in DIRECT_UNITS {
        let target = Target {
            endpoint: endpoint.clone(),
            unit,
        };
        match probe_unit(links, &target, timeout, 0).await {
            Probed::Answered => answering.push(unit),
            Probed::Gateway => gateway = true,
            Probed::Failed(_) => break,
            _ => {}
        }
    }
    let every_unit = answering.len() == DIRECT_UNITS.len() && {
        let unlikely = Target {
            endpoint: endpoint.clone(),
            unit: UNLIKELY_UNIT,
        };
        probe_unit(links, &unlikely, timeout, 0).await == Probed::Answered
    };
    let text = address.to_string();
    if every_unit {
        search.note(Note::AnswersEveryUnit {
            address: text.clone(),
        });
        answering = vec![1];
    }
    if gateway {
        search.note(Note::Gateway {
            address: text.clone(),
        });
    }
    if answering.is_empty() && !gateway {
        search.note(Note::PortOpenNoAnswer {
            address: text.clone(),
        });
    }
    for unit in &answering {
        search.found(Found::new(Target {
            endpoint: endpoint.clone(),
            unit: *unit,
        }));
    }
    search.update(|report| {
        report.hosts.push(Host {
            address: text,
            gateway,
            every_unit,
            answered: !answering.is_empty(),
        });
        report.hosts.sort_by(|a, b| a.address.cmp(&b.address));
    });
}

/// Asks units `from..=to` on one line: behind a TCP gateway, over RTU over TCP, or on a serial
/// port. Each unit gets `retries` more tries when it is silent.
pub async fn search_units(
    links: Arc<Links>,
    endpoint: Endpoint,
    from: u8,
    to: u8,
    timeout: Duration,
    retries: u8,
    search: Arc<Search>,
) {
    let units: Vec<u8> = (from..=to).collect();
    let line = line_text(&endpoint);
    search.update(|report| {
        report.phase = "units".into();
        report.total = units.len();
        report.done = 0;
        report.estimate_seconds =
            Some((timeout * u32::from(retries + 1) * units.len() as u32).as_secs());
    });
    let mut damaged = 0;
    let mut failed = false;
    for unit in units {
        if search.stopped() {
            break;
        }
        let target = Target {
            endpoint: endpoint.clone(),
            unit,
        };
        match probe_unit(&links, &target, timeout, retries).await {
            Probed::Answered => search.found(Found::new(target)),
            Probed::Damaged => damaged += 1,
            Probed::OtherUnit(answered) => search.note(Note::OtherUnit {
                line: line.clone(),
                asked: unit,
                answered,
            }),
            Probed::Failed(error) => {
                // the line itself failed: that is the one thing to say, not "nothing answered"
                search.note(Note::LineFailed {
                    line: line.clone(),
                    reason: error.to_string(),
                });
                failed = true;
                break;
            }
            Probed::Gateway | Probed::Silent => {}
        }
        search.update(|report| report.done += 1);
    }
    if damaged > 0 {
        search.note(Note::Damaged {
            line: line.clone(),
            count: damaged,
        });
    }
    if !failed
        && search
            .report
            .lock()
            .expect("report lock")
            .devices
            .is_empty()
    {
        search.note(Note::NothingAnswered { line });
    }
    identify_all(&links, timeout, &search).await;
}

/// Tries the usual speeds and frames on a serial port with the units in `units` until one
/// answers; returns those settings.
pub async fn find_settings(
    links: Arc<Links>,
    port: String,
    units: Vec<u8>,
    timeout: Duration,
    search: Arc<Search>,
) -> Option<SerialSettings> {
    let candidates: Vec<SerialSettings> = BAUD_RATES
        .iter()
        .flat_map(|&baud| {
            SerialSettings::frames().map(|(parity, stop_bits)| SerialSettings {
                port: port.clone(),
                baud,
                parity,
                stop_bits,
            })
        })
        .collect();
    search.update(|report| {
        report.phase = "settings".into();
        report.total = candidates.len();
        report.done = 0;
        report.estimate_seconds =
            Some((timeout * (candidates.len() * units.len()) as u32).as_secs());
    });
    for settings in candidates {
        if search.stopped() {
            return None;
        }
        let endpoint = Endpoint::Serial(settings.clone());
        let mut damaged = 0;
        for &unit in &units {
            let target = Target {
                endpoint: endpoint.clone(),
                unit,
            };
            match probe_unit(&links, &target, timeout, 0).await {
                Probed::Answered => {
                    let text = format!("{} {}", settings.baud, settings.frame_text());
                    search.note(Note::SettingsFound {
                        settings: text.clone(),
                    });
                    search.update(|report| report.settings = Some(text));
                    return Some(settings);
                }
                Probed::Damaged => damaged += 1,
                Probed::Failed(error) => {
                    search.note(Note::LineFailed {
                        line: port.clone(),
                        reason: error.to_string(),
                    });
                    return None;
                }
                _ => {}
            }
        }
        if damaged > 0 {
            search.note(Note::Damaged {
                line: line_text(&endpoint),
                count: damaged,
            });
        }
        search.update(|report| report.done += 1);
    }
    search.note(Note::NothingAnswered { line: port });
    None
}

/// Who a device is (43/14, basic objects), where it says.
pub async fn identify(
    links: &Links,
    target: &Target,
    timeout: Duration,
) -> Option<crate::protocol::Identity> {
    let request = Request::Identify { code: 1, object: 0 };
    match links.exchange(target, &request, timeout).await {
        Ok(Response::Identity(identity)) => Some(identity),
        _ => None,
    }
}

async fn identify_all(links: &Links, timeout: Duration, search: &Search) {
    let targets: Vec<Target> = search
        .report
        .lock()
        .expect("report lock")
        .devices
        .iter()
        .map(|device| device.target.clone())
        .collect();
    search.update(|report| {
        report.phase = "identify".into();
        report.total = targets.len();
        report.done = 0;
    });
    for target in targets {
        if search.stopped() {
            break;
        }
        if let Some(identity) = identify(links, &target, timeout).await {
            let key = target.key();
            search.update(|report| {
                if let Some(device) = report.devices.iter_mut().find(|device| device.key == key) {
                    device.vendor = identity.get(0).map(str::to_string);
                    device.product = identity.get(1).map(str::to_string);
                    device.revision = identity.get(2).map(str::to_string);
                }
            });
        }
        search.update(|report| report.done += 1);
    }
}

/// A line for people: `COM3 9600 8E1`, `10.0.0.9:502`.
pub fn line_text(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Tcp(address) => address.to_string(),
        Endpoint::RtuOverTcp(address) => format!("{address} (RTU)"),
        Endpoint::Serial(settings) => format!(
            "{} {} {}",
            settings.port,
            settings.baud,
            settings.frame_text()
        ),
    }
}

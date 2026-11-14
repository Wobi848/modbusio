//! Searching: network adapters and serial ports to choose from, the searches themselves, and
//! listening to a serial line. A search runs in the background; the page asks for its report.

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{ApiError, Shared};
use crate::diagnosis::{Note, NoteView};
use crate::endpoint::Endpoint;
use crate::interfaces::{self, Subnet};
use crate::scan::{self, Report, Search};
use crate::site::SiteDevice;

/// Timeouts the page may ask for, in milliseconds.
const TIMEOUTS: std::ops::RangeInclusive<u64> = 50..=10_000;

pub async fn adapters(State(state): State<Shared>) -> Result<Json<Value>, ApiError> {
    let adapters = interfaces::list(state.loopback)
        .map_err(|error| ApiError::internal(format!("cannot list network adapters: {error}")))?;
    let default = interfaces::pick_default(&adapters).map(|adapter| adapter.ip);
    let list: Vec<Value> = adapters
        .iter()
        .map(|adapter| {
            let subnet = adapter.subnet();
            json!({
                "name": adapter.name,
                "ip": adapter.ip.to_string(),
                "prefix_len": adapter.prefix_len,
                "subnet": format!("{}/{}", Ipv4Addr::from(u32::from(subnet.address) & mask(subnet.prefix_len)), subnet.prefix_len),
                "description": adapter.description,
                "kind": match adapter.kind() {
                    interfaces::AdapterKind::Cable => "cable",
                    interfaces::AdapterKind::Wireless => "wireless",
                    interfaces::AdapterKind::Virtual => "virtual",
                },
                "default": Some(adapter.ip) == default,
            })
        })
        .collect();
    Ok(Json(json!(list)))
}

fn mask(prefix_len: u8) -> u32 {
    u32::MAX
        .checked_shl(32 - u32::from(prefix_len))
        .unwrap_or(0)
}

pub async fn serial_ports(State(state): State<Shared>) -> Result<Json<Value>, ApiError> {
    let mut ports = crate::serial::ports().map_err(ApiError::internal)?;
    // the browser tests' simulated lines are pseudo-terminals, which no system lists as ports;
    // several are named with commas between them
    if state.loopback
        && let Some(names) = std::env::var_os("MODBUSIO_TEST_SERIAL")
    {
        for name in names.to_string_lossy().split(',') {
            ports.push(crate::serial::PortChoice {
                name: name.to_string(),
                description: Some("simulated line".into()),
                usb: false,
            });
        }
    }
    Ok(Json(json!(ports)))
}

pub async fn state(State(state): State<Shared>) -> Json<Value> {
    let search = state.search.lock().expect("search lock").clone();
    match search {
        Some(search) => Json(json!(*search.report.lock().expect("report lock"))),
        None => Json(json!(Report {
            finished: true,
            ..Report::default()
        })),
    }
}

pub async fn stop(State(state): State<Shared>) -> Json<Value> {
    if let Some(search) = state.search.lock().expect("search lock").as_ref() {
        search.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    Json(json!({}))
}

const RUNNING: &str = "a search is running; stop it first";

fn running(search: &Option<Arc<Search>>) -> bool {
    search
        .as_ref()
        .is_some_and(|search| !search.report.lock().expect("report lock").finished)
}

/// A new search, unless one runs; refused while one does.
pub(super) fn begin(state: &Shared) -> Result<Arc<Search>, ApiError> {
    let mut current = state.search.lock().expect("search lock");
    if running(&current) {
        return Err(ApiError::conflict(RUNNING));
    }
    let search = Arc::new(Search::default());
    *current = Some(search.clone());
    Ok(search)
}

/// Refuses to open another site while a search runs: what it finds joins the open site, and
/// that has to be the one searched on.
pub(super) fn refuse_while_searching(state: &Shared) -> Result<(), ApiError> {
    if running(&state.search.lock().expect("search lock")) {
        return Err(ApiError::conflict(RUNNING));
    }
    Ok(())
}

/// Refuses to send on a line the bus analyser listens to: modbusio would be a second master there.
pub(super) fn refuse_held(state: &Shared, endpoint: &Endpoint) -> Result<(), ApiError> {
    if state.links.is_held(endpoint) {
        return Err(ApiError::conflict(crate::links::held_text(endpoint)));
    }
    Ok(())
}

pub(super) fn serial_line(port: &str) -> Endpoint {
    Endpoint::Serial(crate::endpoint::SerialSettings {
        port: port.to_string(),
        baud: 9600,
        parity: crate::endpoint::Parity::Even,
        stop_bits: 1,
    })
}

/// When a search ends, its devices join the open site with what was learnt about them.
fn keep_found(state: &Shared, search: &Search) {
    let found = search.report.lock().expect("report lock").devices.clone();
    {
        let mut site = state.site.lock().expect("site lock");
        for device in found {
            let mut kept = SiteDevice::new(device.key.clone());
            kept.vendor = device.vendor.clone();
            kept.product = device.product.clone();
            kept.revision = device.revision.clone();
            if !site.add(kept)
                && let Some(known) = site.device_mut(&device.key)
            {
                known.vendor = device.vendor.or(known.vendor.take());
                known.product = device.product.or(known.product.take());
                known.revision = device.revision.or(known.revision.take());
            }
        }
    }
    if let Err(error) = state.save_site() {
        // the devices stay on the site in memory; the console window says why the file did not
        eprintln!("{}", error.message());
    }
}

pub(super) fn run(
    state: &Shared,
    search: Arc<Search>,
    work: impl Future<Output = ()> + Send + 'static,
) {
    let state = state.clone();
    tokio::spawn(async move {
        work.await;
        keep_found(&state, &search);
        search.finish();
    });
}

pub(super) fn timeout(milliseconds: Option<u64>, default: u64) -> Result<Duration, ApiError> {
    let milliseconds = milliseconds.unwrap_or(default);
    if !TIMEOUTS.contains(&milliseconds) {
        return Err(ApiError::bad_request(format!(
            "the timeout must be {} to {} ms",
            TIMEOUTS.start(),
            TIMEOUTS.end()
        )));
    }
    Ok(Duration::from_millis(milliseconds))
}

#[derive(Deserialize)]
pub struct TcpScan {
    /// The plant's subnet; the adapter's when left out.
    subnet: Option<String>,
    /// The adapter whose subnet to ask.
    interface: Option<Ipv4Addr>,
    port: Option<u16>,
    timeout_ms: Option<u64>,
}

pub async fn scan_tcp(
    State(state): State<Shared>,
    Json(request): Json<TcpScan>,
) -> Result<Json<Value>, ApiError> {
    let port = request.port.unwrap_or(crate::protocol::tcp::MODBUS_PORT);
    if port == 0 {
        return Err(ApiError::bad_request("the port must be 1 to 65535"));
    }
    let timeout = timeout(request.timeout_ms, 500)?;
    let (subnet, own) = match request
        .subnet
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        Some(text) => (Subnet::parse(text).map_err(ApiError::bad_request)?, None),
        None => {
            let adapter = interfaces::choose(request.interface, state.loopback)
                .map_err(|error| ApiError::conflict(format!("{error:#}")))?;
            (adapter.subnet(), Some(adapter.ip))
        }
    };
    let subnet_text = format!("{}/{}", subnet.address, subnet.prefix_len);
    let hosts = subnet.hosts(own).ok_or_else(|| {
        ApiError::bad_request(
            Note::SubnetTooLarge {
                subnet: subnet_text.clone(),
            }
            .to_string(),
        )
    })?;
    let search = begin(&state)?;
    let links = state.links.clone();
    let task_search = search.clone();
    run(&state, search.clone(), async move {
        scan::scan_tcp(
            links,
            hosts,
            port,
            timeout,
            timeout.max(Duration::from_millis(800)),
            task_search.clone(),
        )
        .await;
        let report = &mut task_search.report.lock().expect("report lock");
        if report.hosts.is_empty() {
            report.notes.push(
                Note::NoHostOpen {
                    subnet: subnet_text,
                    port,
                }
                .into(),
            );
        }
    });
    Ok(Json(json!({ "started": true })))
}

#[derive(Deserialize)]
pub struct UnitScan {
    /// The line: `tcp:10.0.0.9:502`, `rtu-tcp:10.0.0.9:4001`, `serial:COM3:9600:8E1`.
    endpoint: String,
    from: Option<u8>,
    to: Option<u8>,
    timeout_ms: Option<u64>,
    retries: Option<u8>,
}

pub async fn scan_units(
    State(state): State<Shared>,
    Json(request): Json<UnitScan>,
) -> Result<Json<Value>, ApiError> {
    let endpoint: Endpoint = request.endpoint.parse().map_err(ApiError::bad_request)?;
    let (from, to) = (request.from.unwrap_or(1), request.to.unwrap_or(247));
    if from == 0 || from > to || to > 247 {
        return Err(ApiError::bad_request(
            "units must run from 1 to 247, the first not above the last",
        ));
    }
    let retries = request.retries.unwrap_or(0);
    if retries > 5 {
        return Err(ApiError::bad_request("at most 5 retries"));
    }
    let timeout = timeout(
        request.timeout_ms,
        if endpoint.is_serial_line() { 300 } else { 500 },
    )?;
    refuse_held(&state, &endpoint)?;
    let search = begin(&state)?;
    let links = state.links.clone();
    run(
        &state,
        search.clone(),
        scan::search_units(links, endpoint, from, to, timeout, retries, search),
    );
    Ok(Json(json!({ "started": true })))
}

#[derive(Deserialize)]
pub struct SettingsSearch {
    port: String,
    /// Units to try at each setting; 1 to 10 when left out.
    units: Option<Vec<u8>>,
    timeout_ms: Option<u64>,
}

pub async fn find_settings(
    State(state): State<Shared>,
    Json(request): Json<SettingsSearch>,
) -> Result<Json<Value>, ApiError> {
    if request.port.trim().is_empty() {
        return Err(ApiError::bad_request("choose a serial port"));
    }
    let units = request
        .units
        .unwrap_or_else(|| scan::SETTINGS_UNITS.to_vec());
    if units.is_empty() || units.len() > 20 || units.iter().any(|unit| !(1..=247).contains(unit)) {
        return Err(ApiError::bad_request("give 1 to 20 units from 1 to 247"));
    }
    let timeout = timeout(request.timeout_ms, 200)?;
    refuse_held(&state, &serial_line(&request.port))?;
    let search = begin(&state)?;
    let links = state.links.clone();
    let task_search = search.clone();
    run(&state, search.clone(), async move {
        scan::find_settings(links, request.port, units, timeout, task_search).await;
    });
    Ok(Json(json!({ "started": true })))
}

#[derive(Deserialize)]
pub struct Listen {
    port: String,
    seconds: Option<f64>,
}

pub async fn listen(
    State(state): State<Shared>,
    Json(request): Json<Listen>,
) -> Result<Json<Value>, ApiError> {
    let seconds = request.seconds.unwrap_or(1.5);
    if !(0.2..=30.0).contains(&seconds) {
        return Err(ApiError::bad_request(
            "listen 0.2 to 30 seconds per setting",
        ));
    }
    if state
        .search
        .lock()
        .expect("search lock")
        .as_ref()
        .is_some_and(|search| !search.report.lock().expect("report lock").finished)
    {
        return Err(ApiError::conflict("a search is running; stop it first"));
    }
    let line = serial_line(&request.port);
    refuse_held(&state, &line)?;
    // modbusio's own connection to the port would hold it
    state.links.close(&line).await;
    let listened = crate::listen::listen(&request.port, Duration::from_secs_f64(seconds))
        .await
        .map_err(|error| ApiError::device(error.to_string()))?;
    let note: NoteView = match &listened.active {
        Some(settings) => Note::OtherMaster {
            port: request.port.clone(),
            settings: settings.clone(),
        },
        None if listened.quiet => Note::Quiet {
            port: request.port.clone(),
        },
        None => Note::Damaged {
            line: request.port.clone(),
            count: listened
                .trials
                .iter()
                .map(|trial| trial.heard.stray_bytes)
                .sum(),
        },
    }
    .into();
    state
        .listened
        .lock()
        .expect("listened lock")
        .insert(request.port.clone(), listened.clone());
    Ok(Json(json!({ "listened": listened, "note": note })))
}

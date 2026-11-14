//! The bus analyser (docs/concept-analyser.md): listening to a trunk in the background, what it
//! heard for the page, and the capture file. Nothing here sends: the port is opened read-only
//! (`analyser::port`), and modbusio's own requests to it are refused while it listens.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{ApiError, Shared, download};
use crate::analyser::exchanges::Exchange;
use crate::analyser::port::{self, ReadOnly};
use crate::analyser::units::{TimesSummary, UnitStats};
use crate::analyser::{Analysis, Numbered, Seen};
use crate::capture::Frame;
use crate::endpoint::{Endpoint, SerialSettings};
use crate::protocol::rtu;

/// Items of a list the page gets at most per request; it asks again for the rest.
const MOST_PER_ANSWER: usize = 500;

/// An analysis, running or ended; kept until the next one starts.
pub struct Listening {
    pub settings: SerialSettings,
    pub started: SystemTime,
    pub analysis: Arc<Mutex<Analysis>>,
    /// Why listening ended by itself (adapter unplugged), or `stopped` when the user ended it.
    pub ended: Arc<Mutex<Option<String>>>,
    task: tokio::task::AbortHandle,
}

impl Listening {
    pub fn running(&self) -> bool {
        self.ended.lock().expect("ended lock").is_none()
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::Serial(self.settings.clone())
    }
}

#[derive(Deserialize)]
pub struct Start {
    port: String,
    /// Speed and frame (`19200`, `8E1`); found by listening when left out.
    baud: Option<u32>,
    frame: Option<String>,
}

pub async fn start(
    State(state): State<Shared>,
    Json(request): Json<Start>,
) -> Result<Json<Value>, ApiError> {
    let port = request.port.trim().to_string();
    if port.is_empty() {
        return Err(ApiError::bad_request("choose a serial port"));
    }
    super::scanning::refuse_while_searching(&state)?;
    if state
        .analyser
        .lock()
        .expect("analyser lock")
        .as_ref()
        .is_some_and(Listening::running)
    {
        return Err(ApiError::conflict(
            "the bus analyser is listening; stop it first",
        ));
    }
    let given = match (request.baud, request.frame.as_deref()) {
        (Some(baud), Some(frame)) => Some((baud, frame)),
        (None, None) => None,
        _ => {
            return Err(ApiError::bad_request(
                "give speed and frame both, or neither",
            ));
        }
    };
    // modbusio's own connection would hold the port, and must not send while the analyser listens
    let line = super::scanning::serial_line(&port);
    state.links.hold(&line).await;
    let opened = match port::settings(&port, given).await {
        Ok(settings) => ReadOnly::open(&settings)
            .map(|read_only| (settings, read_only))
            .map_err(|error| error.to_string()),
        Err(error) => Err(error),
    };
    let (settings, read_only) = match opened {
        Ok(opened) => opened,
        Err(error) => {
            state.links.release(&line);
            return Err(ApiError::device(error));
        }
    };
    let analysis = Arc::new(Mutex::new(Analysis::default()));
    let ended = Arc::new(Mutex::new(None));
    let quiet = port::quiet_after(settings.baud);
    let task = {
        let (analysis, ended, links) = (analysis.clone(), ended.clone(), state.links.clone());
        tokio::spawn(async move {
            let why = port::hear(read_only, analysis, tokio::time::Instant::now(), quiet).await;
            *ended.lock().expect("ended lock") = Some(why);
            links.release(&line);
        })
        .abort_handle()
    };
    *state.analyser.lock().expect("analyser lock") = Some(Listening {
        settings,
        started: SystemTime::now(),
        analysis,
        ended,
        task,
    });
    Ok(view(&state, &After::default()))
}

pub async fn stop(State(state): State<Shared>) -> Json<Value> {
    if let Some(listening) = state.analyser.lock().expect("analyser lock").as_ref()
        && listening.running()
    {
        listening.task.abort();
        *listening.ended.lock().expect("ended lock") = Some("stopped".into());
        state.links.release(&listening.endpoint());
    }
    view(&state, &After::default())
}

/// Counts and lists start again; listening goes on.
pub async fn clear(State(state): State<Shared>) -> Json<Value> {
    if let Some(listening) = state.analyser.lock().expect("analyser lock").as_ref() {
        listening.analysis.lock().expect("analysis lock").clear();
    }
    view(&state, &After::default())
}

/// What the page already has: the numbers of the last frame and exchange.
#[derive(Default, Deserialize)]
pub struct After {
    frames: Option<u64>,
    exchanges: Option<u64>,
}

pub async fn state(State(state): State<Shared>, Query(after): Query<After>) -> Json<Value> {
    view(&state, &after)
}

fn view(state: &Shared, after: &After) -> Json<Value> {
    let analyser = state.analyser.lock().expect("analyser lock");
    let Some(listening) = analyser.as_ref() else {
        return Json(json!({ "running": false }));
    };
    let analysis = listening.analysis.lock().expect("analysis lock");
    let elapsed = listening.started.elapsed().unwrap_or_default();
    Json(json!({
        "running": listening.running(),
        "ended": *listening.ended.lock().expect("ended lock"),
        "port": listening.settings.port,
        "settings": format!("{} {}", listening.settings.baud, listening.settings.frame_text()),
        "started": seconds_since_epoch(listening.started),
        "elapsed_ms": elapsed.as_millis() as u64,
        "bus": analysis.bus,
        "load": load(analysis.bus.bytes, listening.settings.baud, elapsed),
        "units": analysis.units.iter().map(|(unit, stats)| unit_view(*unit, stats)).collect::<Vec<_>>(),
        "exchanges": newest(analysis.exchanges(), after.exchanges).map(exchange_view).collect::<Vec<_>>(),
        "frames": newest(analysis.frames(), after.frames).map(frame_view).collect::<Vec<_>>(),
    }))
}

/// The share of time the trunk carried bytes, 0 to 1.
fn load(bytes: u64, baud: u32, elapsed: Duration) -> f64 {
    let busy = bytes as f64 * rtu::character_micros(baud) as f64;
    (busy / elapsed.as_micros().max(1) as f64).min(1.0)
}

/// The items after `after`, at most [`MOST_PER_ANSWER`] of the newest.
fn newest<T>(
    list: &std::collections::VecDeque<Numbered<T>>,
    after: Option<u64>,
) -> impl Iterator<Item = &Numbered<T>> {
    let new = list
        .iter()
        .rev()
        .take_while(move |item| after.is_none_or(|after| item.number > after))
        .count()
        .min(MOST_PER_ANSWER);
    list.iter().skip(list.len() - new)
}

fn millis(summary: Option<TimesSummary>) -> Value {
    summary.map_or(Value::Null, |times| {
        let ms = |micros: u64| micros as f64 / 1000.0;
        json!({
            "min": ms(times.min),
            "median": ms(times.median),
            "p95": ms(times.p95),
            "max": ms(times.max),
        })
    })
}

fn unit_view(unit: u8, stats: &UnitStats) -> Value {
    json!({
        "unit": unit,
        "requests": stats.requests,
        "answers": stats.answers,
        "no_answer": stats.no_answer,
        "damaged": stats.damaged,
        "exceptions": stats.exceptions,
        "writes": stats.writes,
        "answer_only": stats.answer_only,
        "answer_ms": millis(stats.answer_times.summary()),
        "cycle_ms": millis(stats.cycle.summary()),
        "last_seen_ms": stats.last_seen / 1000,
    })
}

#[derive(Serialize)]
struct ExchangeView<'a> {
    number: u64,
    #[serde(flatten)]
    exchange: &'a Exchange,
    request: String,
    answer: String,
}

fn exchange_view(numbered: &Numbered<Exchange>) -> Value {
    let exchange = &numbered.item;
    json!(ExchangeView {
        number: numbered.number,
        exchange,
        request: hex(&exchange.request_bytes),
        answer: hex(&exchange.answer_bytes),
    })
}

fn frame_view(numbered: &Numbered<Seen>) -> Value {
    json!({
        "number": numbered.number,
        "at": numbered.item.at,
        "role": numbered.item.role,
        "bytes": hex(&numbered.item.bytes),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn seconds_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The frames kept, for Wireshark: requests as sent, answers and damaged bytes as received.
pub async fn file(State(state): State<Shared>) -> Result<Response, ApiError> {
    let analyser = state.analyser.lock().expect("analyser lock");
    let listening = analyser
        .as_ref()
        .ok_or_else(|| ApiError::conflict("nothing heard yet: start the bus analyser"))?;
    let frames: Vec<Frame> = listening
        .analysis
        .lock()
        .expect("analysis lock")
        .frames()
        .iter()
        .map(|seen| seen.item.capture(listening.started))
        .collect();
    Ok(download(
        "modbusio-trunk.pcapng",
        "application/octet-stream",
        crate::pcap::write(&frames),
    ))
}

//! Finding devices at other serial settings (docs/concept-other-settings.md): the route that starts
//! it. The search listens first and sends nothing while another master talks; that check is in
//! `other_settings::search`, not in the page.

use std::collections::BTreeSet;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use serde_json::{Value, json};

use super::scanning::{begin, refuse_held, run, serial_line, timeout};
use super::{ApiError, Shared};
use crate::endpoint::SerialSettings;
use crate::other_settings::{self, Asking, LISTEN_EACH};

#[derive(Deserialize)]
pub struct OtherSettings {
    port: String,
    /// The trunk's setting, the one the master uses: `19200` and `8E1`.
    baud: u32,
    frame: String,
    from: Option<u8>,
    to: Option<u8>,
    /// Empty: long enough for each speed (`other_settings::wait_at`).
    timeout_ms: Option<u64>,
    /// 1200, 2400, 4800 and 57600 as well.
    #[serde(default)]
    all_speeds: bool,
    /// Only say how long it would take, for the question in the page; nothing is sent.
    #[serde(default)]
    check: bool,
}

pub async fn start(
    State(state): State<Shared>,
    Json(request): Json<OtherSettings>,
) -> Result<Json<Value>, ApiError> {
    let port = request.port.trim().to_string();
    if port.is_empty() {
        return Err(ApiError::bad_request("choose a serial port"));
    }
    let (parity, stop_bits) = SerialSettings::parse_frame(&request.frame).ok_or_else(|| {
        ApiError::bad_request(format!("frame {} must be like 8E1 or 8N2", request.frame))
    })?;
    if !crate::endpoint::BAUD_RATES.contains(&request.baud) {
        return Err(ApiError::bad_request(format!(
            "{} is not a usual speed",
            request.baud
        )));
    }
    let (from, to) = (request.from.unwrap_or(1), request.to.unwrap_or(247));
    if from == 0 || from > to || to > 247 {
        return Err(ApiError::bad_request(
            "units must run from 1 to 247, the first not above the last",
        ));
    }
    let wait = match request.timeout_ms {
        Some(milliseconds) => Some(timeout(Some(milliseconds), milliseconds)?),
        None => None,
    };
    refuse_held(&state, &serial_line(&port))?;
    let (master_asks, master_silent) = master_on(&state, &port);
    let asking = Asking {
        trunk: SerialSettings {
            port,
            baud: request.baud,
            parity,
            stop_bits,
        },
        units: (from..=to).collect(),
        all_speeds: request.all_speeds,
        wait,
        master_asks,
        master_silent,
        listen_each: LISTEN_EACH,
    };
    let estimate = asking.estimate().as_secs();
    if request.check {
        let settings = crate::other_settings::candidates(&asking.trunk, asking.all_speeds).len();
        return Ok(Json(
            json!({ "estimate_seconds": estimate, "settings": settings }),
        ));
    }
    let search = begin(&state)?;
    let links = state.links.clone();
    let task_search = search.clone();
    run(&state, search, async move {
        other_settings::search(links, asking, task_search).await;
    });
    Ok(Json(
        json!({ "started": true, "estimate_seconds": estimate }),
    ))
}

/// The units the master asks on `port`, and those of them that never answered, from the bus
/// analyser's last analysis of this port; empty when there is none.
fn master_on(state: &Shared, port: &str) -> (BTreeSet<u8>, BTreeSet<u8>) {
    let analyser = state.analyser.lock().expect("analyser lock");
    let Some(listening) = analyser.as_ref().filter(|listening| {
        listening
            .port()
            .is_some_and(|settings| settings.port == port)
    }) else {
        return Default::default();
    };
    let analysis = listening.analysis.lock().expect("analysis lock");
    let asks = analysis
        .units
        .iter()
        .filter(|(_, stats)| stats.requests > 0)
        .map(|(unit, _)| *unit)
        .collect();
    let silent = analysis
        .units
        .iter()
        .filter(|(_, stats)| stats.answers == 0 && stats.answer_only == 0 && stats.no_answer > 0)
        .map(|(unit, _)| *unit)
        .collect();
    (asks, silent)
}

//! The capture for Wireshark: started and stopped from *Settings*, the last one as a file.

use axum::Json;
use axum::extract::State;
use axum::response::Response;
use serde_json::{Value, json};

use super::{ApiError, Shared, download};

pub async fn state(State(state): State<Shared>) -> Json<Value> {
    let kept = state
        .captured
        .lock()
        .expect("captured lock")
        .as_ref()
        .map(Vec::len);
    Json(json!({
        "running": state.capture.count().is_some(),
        "frames": state.capture.count(),
        "kept": kept,
    }))
}

pub async fn start(State(state): State<Shared>) -> Json<Value> {
    state.capture.start();
    self::state(State(state)).await
}

pub async fn stop(State(state): State<Shared>) -> Json<Value> {
    if let Some(frames) = state.capture.stop() {
        *state.captured.lock().expect("captured lock") = Some(frames);
    }
    self::state(State(state)).await
}

pub async fn file(State(state): State<Shared>) -> Result<Response, ApiError> {
    let captured = state.captured.lock().expect("captured lock");
    let frames = captured
        .as_ref()
        .ok_or_else(|| ApiError::conflict("no capture yet: start one and stop it"))?;
    Ok(download(
        "modbusio-capture.pcapng",
        "application/octet-stream",
        crate::pcap::write(frames),
    ))
}

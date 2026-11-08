//! The local web server: the interface from `web/` and the JSON API it talks to (docs/api.md).
//! Listens on 127.0.0.1 only, because the API can write to the plant.

mod capturing;
mod devices;
mod exports;
mod guard;
mod profiles;
mod recording;
mod running;
mod scanning;
mod sites;
mod state;
mod views;
mod writing;

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::extract::DefaultBodyLimit;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rust_embed::RustEmbed;
use serde_json::json;
use tokio::net::TcpListener;

pub use state::{AppState, Shared};

pub const DEFAULT_PORT: u16 = 8082;

/// A register list or site file brought in.
const MOST_UPLOAD_BYTES: usize = 16 * 1024 * 1024;

#[derive(RustEmbed)]
#[folder = "web/"]
struct Assets;

pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/api/info", get(info))
        .route("/api/adapters", get(scanning::adapters))
        .route("/api/serial-ports", get(scanning::serial_ports))
        .route("/api/scan", get(scanning::state))
        .route("/api/scan/tcp", post(scanning::scan_tcp))
        .route("/api/scan/units", post(scanning::scan_units))
        .route("/api/scan/settings", post(scanning::find_settings))
        .route("/api/scan/stop", post(scanning::stop))
        .route("/api/listen", post(scanning::listen))
        .route("/api/site", get(sites::site))
        .route("/api/site/rename", post(sites::rename))
        .route("/api/sites", get(sites::list))
        .route("/api/sites/new", post(sites::create))
        .route("/api/sites/open", post(sites::open))
        .route("/api/sites/delete", post(sites::delete))
        .route(
            "/api/site.json",
            get(sites::export)
                .post(sites::import)
                .layer(DefaultBodyLimit::max(MOST_UPLOAD_BYTES)),
        )
        .route("/api/devices/add", post(sites::add_device))
        .route("/api/devices/update", post(sites::update_device))
        .route("/api/devices/remove", post(sites::remove_device))
        .route("/api/devices/identify", post(devices::identify))
        .route("/api/devices/read", post(devices::read))
        .route("/api/values", post(devices::values))
        .route("/api/explore", post(devices::explore))
        .route("/api/profiles", get(profiles::list).post(profiles::save))
        .route("/api/profiles/get", get(profiles::get))
        .route("/api/profiles/delete", post(profiles::delete))
        .route(
            "/api/profiles/import",
            post(profiles::import).layer(DefaultBodyLimit::max(MOST_UPLOAD_BYTES)),
        )
        .route("/api/profile.csv", get(profiles::csv))
        .route("/api/profile.xlsx", get(profiles::xlsx))
        .route(
            "/api/session",
            get(writing::session).post(writing::set_session),
        )
        .route("/api/write", post(writing::write))
        .route("/api/journal", get(writing::journal))
        .route("/api/restore", post(writing::restore))
        .route("/api/journal.csv", get(writing::journal_csv))
        .route("/api/journal.xlsx", get(writing::journal_xlsx))
        .route("/api/recording", get(recording::state))
        .route("/api/recording/add", post(recording::add))
        .route("/api/recording/remove", post(recording::remove))
        .route("/api/recording/start", post(recording::start))
        .route("/api/recording/stop", post(recording::stop))
        .route("/api/recording/clear", post(recording::clear))
        .route("/api/recording.csv", get(recording::csv))
        .route("/api/recording.xlsx", get(recording::xlsx))
        .route("/api/capture", get(capturing::state))
        .route("/api/capture/start", post(capturing::start))
        .route("/api/capture/stop", post(capturing::stop))
        .route("/api/capture.pcapng", get(capturing::file))
        .route("/api/devices.csv", get(exports::devices_csv))
        .route("/api/points.csv", get(exports::points_csv))
        .route("/api/points.xlsx", get(exports::points_xlsx))
        .route("/api/site.xlsx", get(exports::site_xlsx))
        .fallback(static_file)
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            guard::same_origin_only,
        ))
        .with_state(state)
}

/// Serves until the process ends. Takes the next free port if `port` is taken. Started to open
/// the browser while a modbusio already serves `data_dir`, it opens that one and ends.
pub async fn serve(
    port: u16,
    open_in_browser: bool,
    data_dir: Option<PathBuf>,
    loopback: bool,
) -> anyhow::Result<()> {
    if open_in_browser
        && let Some(dir) = &data_dir
        && let Some(running) = running::already_running(dir).await
    {
        let url = local_url(running);
        println!("modbusio is already running at {url}; opening it there.");
        open_browser(&url);
        return Ok(());
    }
    let mut state = AppState::new(data_dir.clone())?;
    state.loopback = loopback;
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
        Ok(listener) => listener,
        Err(_) => TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .context("cannot open a local web server port")?,
    };
    let address = listener.local_addr()?;
    if let Some(dir) = &data_dir {
        running::remember(dir, address)
            .with_context(|| format!("cannot write to {}", dir.display()))?;
    }
    state.web_address = Some(address);
    let url = local_url(address);
    println!("modbusio is running at {url}");
    println!("Close this window to quit.");
    if open_in_browser {
        open_browser(&url);
    }
    axum::serve(listener, router(Arc::new(state)))
        .await
        .context("the web server stopped")
}

fn local_url(address: SocketAddr) -> String {
    format!("http://127.0.0.1:{}/", address.port())
}

/// Copied from bacnetio.
fn open_browser(url: &str) {
    let started = if cfg!(windows) {
        // "start" is a cmd built-in; the empty title keeps the URL from being taken as title
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
    {
        std::process::Command::new("xdg-open").arg(url).spawn()
    } else {
        return;
    };
    if started.is_err() {
        println!("Open {url} in your browser.");
    }
}

/// The version, what this system can do, and problems with files at the start.
async fn info(
    axum::extract::State(state): axum::extract::State<Shared>,
) -> Json<serde_json::Value> {
    Json(json!({
        "program": "modbusio",
        "version": env!("CARGO_PKG_VERSION"),
        "serial": cfg!(not(target_os = "android")),
        "data_dir": state.data_dir.as_ref().map(|dir| dir.display().to_string()),
        "problems": *state.load_problems.lock().expect("problems lock"),
    }))
}

async fn static_file(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

/// A file to download, with its name.
fn download(name: &str, mime: &str, body: impl Into<axum::body::Body>) -> Response {
    (
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        body.into(),
    )
        .into_response()
}

const CSV: &str = "text/csv; charset=utf-8";
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

fn csv_file(name: &str, table: &crate::export::Table) -> Response {
    download(name, CSV, table.csv())
}

fn xlsx_file(name: &str, tables: &[crate::export::Table]) -> Result<Response, ApiError> {
    let bytes = crate::workbook::write(tables)
        .map_err(|error| ApiError::internal(format!("cannot write the Excel file: {error}")))?;
    Ok(download(name, XLSX, bytes))
}

/// The viewer's time zone, as JavaScript gives it negated (`-getTimezoneOffset()`).
#[derive(serde::Deserialize)]
struct Zone {
    #[serde(default)]
    offset: i32,
}

/// An error the API answers with: `{"error": "..."}` in plain words. Copied from bacnetio.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl ToString) -> Self {
        ApiError {
            status,
            message: message.to_string(),
        }
    }

    /// The request itself is wrong.
    pub fn bad_request(message: impl ToString) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    /// Not possible right now: writing still locked, a search running.
    pub fn conflict(message: impl ToString) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    /// Nothing of that name is there.
    pub fn not_found(message: impl ToString) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    /// The line or a device did not do what was asked.
    pub fn device(message: impl ToString) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, message)
    }

    pub fn internal(message: impl ToString) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

//! The open connections, one per line, shared by everything that talks to devices. A TCP device
//! often takes only one connection, and a serial line only one request at a time, so all requests
//! to a line wait for each other here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::Instant;

use crate::capture::Capture;
use crate::endpoint::{Endpoint, Target};
use crate::link::{Link, LinkError};
use crate::protocol::{Request, Response};

/// How long a connection may take to open.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// How long an answer may take, by default: TCP and RTU both, slow devices included.
pub const ANSWER_TIMEOUT: Duration = Duration::from_millis(1500);

/// How long a TCP connection stays open without a request. Many devices take only one or a few
/// connections: after a search or a look at the values, the plant's own master must get in again.
pub const IDLE_CLOSE: Duration = Duration::from_secs(10);

type Slot = Arc<tokio::sync::Mutex<Option<Link>>>;

pub struct Links {
    slots: Mutex<HashMap<String, Slot>>,
    pub capture: Arc<Capture>,
    idle_close: Duration,
}

impl Default for Links {
    fn default() -> Links {
        Links::new(Arc::default())
    }
}

impl Links {
    pub fn new(capture: Arc<Capture>) -> Links {
        Links {
            slots: Mutex::default(),
            capture,
            idle_close: IDLE_CLOSE,
        }
    }

    /// Closes idle TCP connections after `idle` instead of [`IDLE_CLOSE`].
    pub fn closing_idle_after(mut self, idle: Duration) -> Links {
        self.idle_close = idle;
        self
    }

    fn slot(&self, endpoint: &Endpoint) -> Slot {
        self.slots
            .lock()
            .expect("links lock")
            .entry(endpoint.line_key())
            .or_default()
            .clone()
    }

    /// Sends `request` to `target` and waits up to `timeout` for the answer. Opens the line first
    /// when needed, or again at other settings (a serial port at another speed).
    pub async fn exchange(
        &self,
        target: &Target,
        request: &Request,
        timeout: Duration,
    ) -> Result<Response, LinkError> {
        let slot = self.slot(&target.endpoint);
        let mut open = slot.lock().await;
        let reused = open
            .as_ref()
            .is_some_and(|link| link.endpoint == target.endpoint);
        if !reused {
            self.open(&slot, &mut open, &target.endpoint).await?;
        }
        let link = open.as_mut().expect("opened above");
        let mut result = link.exchange(target.unit, request, timeout).await;
        // the device closed the connection while it was idle: once more on a new one, where
        // asking twice changes nothing
        if reused && result == Err(LinkError::Closed) && request.only_reads() {
            let link = self.open(&slot, &mut open, &target.endpoint).await?;
            result = link.exchange(target.unit, request, timeout).await;
        }
        if result.as_ref().is_err_and(LinkError::drops_connection) {
            *open = None;
        }
        result
    }

    /// Opens the line into `open`, closing what was open there.
    async fn open<'slot>(
        &self,
        slot: &Slot,
        open: &'slot mut Option<Link>,
        endpoint: &Endpoint,
    ) -> Result<&'slot mut Link, LinkError> {
        // close first: a serial port cannot be opened twice
        *open = None;
        let link = open.insert(Link::open(endpoint, CONNECT_TIMEOUT, self.capture.clone()).await?);
        // a serial port stays open: it is modbusio's alone, and opening it can reset an adapter
        if !matches!(endpoint, Endpoint::Serial(_)) {
            tokio::spawn(close_when_idle(
                slot.clone(),
                link.opened(),
                self.idle_close,
            ));
        }
        Ok(link)
    }

    /// Closes the connection to a line, so another program can use it.
    pub async fn close(&self, endpoint: &Endpoint) {
        let slot = self.slot(endpoint);
        *slot.lock().await = None;
    }
}

/// Closes the connection opened at `opened` once no request has used it for `idle`.
async fn close_when_idle(slot: Slot, opened: Instant, idle: Duration) {
    loop {
        let mut open = slot.lock().await;
        let Some(link) = open.as_ref().filter(|link| link.opened() == opened) else {
            // closed or opened anew in the meantime
            return;
        };
        let close_at = link.last_activity() + idle;
        if Instant::now() >= close_at {
            *open = None;
            return;
        }
        drop(open);
        tokio::time::sleep_until(close_at).await;
    }
}

//! The open connections, one per line, shared by everything that talks to devices. A TCP device
//! often takes only one connection, and a serial line only one request at a time, so all requests
//! to a line wait for each other here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::capture::Capture;
use crate::endpoint::{Endpoint, Target};
use crate::link::{Link, LinkError};
use crate::protocol::{Request, Response};

/// How long a connection may take to open.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// How long an answer may take, by default: TCP and RTU both, slow devices included.
pub const ANSWER_TIMEOUT: Duration = Duration::from_millis(1500);

type Slot = Arc<tokio::sync::Mutex<Option<Link>>>;

#[derive(Default)]
pub struct Links {
    slots: Mutex<HashMap<String, Slot>>,
    pub capture: Arc<Capture>,
}

impl Links {
    pub fn new(capture: Arc<Capture>) -> Links {
        Links {
            slots: Mutex::default(),
            capture,
        }
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
        if open
            .as_ref()
            .is_none_or(|link| link.endpoint != target.endpoint)
        {
            // close first: a serial port cannot be opened twice
            *open = None;
            *open =
                Some(Link::open(&target.endpoint, CONNECT_TIMEOUT, self.capture.clone()).await?);
        }
        let link = open.as_mut().expect("opened above");
        let result = link.exchange(target.unit, request, timeout).await;
        if result.as_ref().is_err_and(LinkError::drops_connection) {
            *open = None;
        }
        result
    }

    /// Closes the connection to a line, so another program can use it.
    pub async fn close(&self, endpoint: &Endpoint) {
        let slot = self.slot(endpoint);
        *slot.lock().await = None;
    }
}

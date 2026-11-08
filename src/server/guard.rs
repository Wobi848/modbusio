//! Copied from bacnetio, the program's name changed (concept section 14, answer 6).
//!
//! Only pages modbusio served itself may use its API. The server listens on loopback only, but a
//! web page elsewhere can still reach it: directly (the browser sends that page's `Origin`) or by
//! DNS rebinding, where a foreign name points to 127.0.0.1 and the browser sends that name as
//! `Host`. Both would let a foreign page write to the plant.

use std::net::SocketAddr;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::state::Shared;

pub async fn same_origin_only(
    State(state): State<Shared>,
    request: Request,
    next: Next,
) -> Response {
    match state.web_address {
        Some(address) if !allowed(request.headers(), address) => (
            StatusCode::FORBIDDEN,
            axum::Json(
                serde_json::json!({ "error": "only pages served by this modbusio may use it" }),
            ),
        )
            .into_response(),
        _ => next.run(request).await,
    }
}

/// `Host` names this server, and `Origin`, when sent, is this server.
fn allowed(headers: &HeaderMap, address: SocketAddr) -> bool {
    let port = address.port();
    let own = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    let text = |name| headers.get(name).and_then(|value| value.to_str().ok());
    let host_ok = text(header::HOST).is_some_and(|host| own.iter().any(|name| name == host));
    let origin_ok = text(header::ORIGIN)
        .is_none_or(|origin| own.iter().any(|name| origin == format!("http://{name}")));
    host_ok && origin_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::net::Ipv4Addr;

    fn headers(host: Option<&'static str>, origin: Option<&'static str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(host) = host {
            headers.insert(header::HOST, HeaderValue::from_static(host));
        }
        if let Some(origin) = origin {
            headers.insert(header::ORIGIN, HeaderValue::from_static(origin));
        }
        headers
    }

    #[test]
    fn only_the_own_host_and_origin_pass() {
        let own = SocketAddr::from((Ipv4Addr::LOCALHOST, 18099));
        assert!(allowed(&headers(Some("127.0.0.1:18099"), None), own));
        assert!(allowed(
            &headers(Some("localhost:18099"), Some("http://localhost:18099")),
            own
        ));
        assert!(!allowed(&headers(Some("evil.example:18099"), None), own));
        assert!(!allowed(
            &headers(Some("127.0.0.1:18099"), Some("http://evil.example:18099")),
            own
        ));
        assert!(!allowed(&headers(Some("127.0.0.1:9999"), None), own));
        assert!(!allowed(&headers(None, None), own));
    }
}

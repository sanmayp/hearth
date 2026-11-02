//! Reqlog. It logs one line per request.
//! It never logs keys, bodies, or text.

use axum::{
    body::Body,
    http::{header, HeaderMap, Request},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Read the Origin value. Missing means "none".
pub fn origin_value(headers: &HeaderMap) -> String {
    headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none")
        .to_string()
}

/// Check the key header. It returns present and matched flags.
/// Both render as yes or no. Key bytes never leave this fn.
pub fn key_flags(headers: &HeaderMap, token: &str) -> (&'static str, &'static str) {
    if headers.get(header::AUTHORIZATION).is_none() {
        return ("no", "no");
    }
    if crate::auth::authorized(headers, token) {
        ("yes", "yes")
    } else {
        ("yes", "no")
    }
}

/// Request log layer. It sits outermost, so it sees preflights too.
/// It logs path only. Query may hold draft text, so it stays out.
pub async fn layer(
    state: Arc<crate::routes::AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let origin = origin_value(req.headers());
    let (present, matched) = key_flags(req.headers(), &state.config.token);
    let resp = next.run(req).await;
    let status = resp.status().as_u16();
    tracing::info!(
        method = %method,
        path = %path,
        origin = %origin,
        status,
        key_present = present,
        key_match = matched,
        "request"
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    /// Headers with the given auth value.
    fn with_auth(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn origin_reads_value_or_none() {
        let mut headers = HeaderMap::new();
        assert_eq!(origin_value(&headers), "none");
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://app.example"),
        );
        assert_eq!(origin_value(&headers), "https://app.example");
    }

    #[test]
    fn key_flags_cover_missing_wrong_and_right() {
        assert_eq!(key_flags(&HeaderMap::new(), "secret"), ("no", "no"));
        assert_eq!(
            key_flags(&with_auth("Bearer wrong"), "secret"),
            ("yes", "no")
        );
        assert_eq!(
            key_flags(&with_auth("Bearer secret"), "secret"),
            ("yes", "yes")
        );
    }
}

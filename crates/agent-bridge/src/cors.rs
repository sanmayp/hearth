//! CORS. It lets the phone webview call the bridge.
//! Preflight skips auth. Real calls keep every check.

use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, Method, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Allowed request headers. Browsers send these names.
const ALLOW_HEADERS: &str = "authorization, content-type";
/// Allowed methods. The bridge serves these only.
const ALLOW_METHODS: &str = "GET, POST, OPTIONS";
/// Preflight cache life in seconds.
const MAX_AGE: &str = "600";

/// Match the request origin. An exact allowlist hit returns it.
pub fn allowed_origin(origins: &[String], headers: &HeaderMap) -> Option<String> {
    let origin = headers.get(header::ORIGIN)?.to_str().ok()?;
    origins.iter().find(|o| o.as_str() == origin).cloned()
}

/// Stamp CORS headers on a response. Vary always lands.
/// Allow headers land only for allowlisted origins. Never star.
pub fn stamp(headers: &mut HeaderMap, origin: Option<&str>) {
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    let Some(origin) = origin else {
        return;
    };
    let Ok(value) = HeaderValue::from_str(origin) else {
        return;
    };
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static(ALLOW_HEADERS),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static(ALLOW_METHODS),
    );
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static(MAX_AGE),
    );
}

/// CORS layer. It sits outside auth and wraps every response.
/// OPTIONS answers 204 directly. All other calls pass through first.
pub async fn layer(
    state: Arc<crate::routes::AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let origin = allowed_origin(&state.config.origins, req.headers());
    if req.method() == Method::OPTIONS {
        let mut resp = axum::http::StatusCode::NO_CONTENT.into_response();
        stamp(resp.headers_mut(), origin.as_deref());
        return resp;
    }
    let mut resp = next.run(req).await;
    stamp(resp.headers_mut(), origin.as_deref());
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One allowlisted app origin.
    fn origins() -> Vec<String> {
        vec!["https://app.example".to_string()]
    }

    /// Headers with the given origin.
    fn with_origin(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn exact_origin_matches_and_others_miss() {
        assert_eq!(
            allowed_origin(&origins(), &with_origin("https://app.example")),
            Some("https://app.example".to_string())
        );
        assert_eq!(
            allowed_origin(&origins(), &with_origin("https://evil.example")),
            None
        );
        assert_eq!(allowed_origin(&origins(), &HeaderMap::new()), None);
        assert_eq!(
            allowed_origin(&[], &with_origin("https://app.example")),
            None
        );
    }

    #[test]
    fn stamp_sets_full_set_for_allowed_origin() {
        let mut headers = HeaderMap::new();
        stamp(&mut headers, Some("https://app.example"));
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "https://app.example"
        );
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_HEADERS],
            "authorization, content-type"
        );
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_METHODS],
            "GET, POST, OPTIONS"
        );
        assert_eq!(headers[header::ACCESS_CONTROL_MAX_AGE], "600");
        assert_eq!(headers[header::VARY], "Origin");
    }

    #[test]
    fn stamp_sets_vary_only_without_origin() {
        let mut headers = HeaderMap::new();
        stamp(&mut headers, None);
        assert_eq!(headers[header::VARY], "Origin");
        assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    }
}

//! CORS. It lets the phone app call the API.
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
/// Allowed methods. The API serves these only.
const ALLOW_METHODS: &str = "GET, POST, OPTIONS";

/// Default CORS origins. Tauri shells first, then local shells.
pub fn default_origins() -> Vec<String> {
    [
        "http://tauri.localhost",
        "https://tauri.localhost",
        "tauri://localhost",
        "https://example.ts.net:8447",
        "http://localhost",
        "https://localhost",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Parse the origin allowlist. Star refuses start. Never echo star.
pub fn parse_origins(raw: Option<String>) -> Result<Vec<String>, String> {
    let Some(raw) = raw else {
        return Ok(default_origins());
    };
    let mut out = vec![];
    for entry in raw.split(',') {
        let clean = entry.trim();
        if clean.is_empty() {
            continue;
        }
        if clean == "*" {
            return Err("HEARTH_CORS_ORIGINS must not hold star".to_string());
        }
        out.push(clean.to_string());
    }
    Ok(out)
}

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
}

/// CORS layer. It sits outside auth and wraps every response.
/// OPTIONS answers 204 directly. All other calls pass through first.
pub async fn layer(origins: Arc<Vec<String>>, req: Request<Body>, next: Next) -> Response {
    let origin = allowed_origin(&origins, req.headers());
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
        vec!["http://tauri.localhost".to_string()]
    }

    /// Headers with the given origin.
    fn with_origin(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn defaults_hold_tauri_app_and_local() {
        let defaults = default_origins();
        assert_eq!(defaults.len(), 6);
        assert!(defaults.contains(&"http://tauri.localhost".to_string()));
        assert!(defaults.contains(&"tauri://localhost".to_string()));
    }

    #[test]
    fn parse_sets_custom_list_and_refuses_star() {
        let out = parse_origins(Some("https://app.example, http://localhost".into())).unwrap();
        assert_eq!(out, vec!["https://app.example", "http://localhost"]);
        assert_eq!(parse_origins(None).unwrap(), default_origins());
        let err = parse_origins(Some("https://app.example, *".into())).unwrap_err();
        assert!(err.contains("star"));
    }

    #[test]
    fn exact_origin_matches_and_others_miss() {
        assert_eq!(
            allowed_origin(&origins(), &with_origin("http://tauri.localhost")),
            Some("http://tauri.localhost".to_string())
        );
        assert_eq!(
            allowed_origin(&origins(), &with_origin("https://evil.example")),
            None
        );
        assert_eq!(allowed_origin(&origins(), &HeaderMap::new()), None);
        assert_eq!(
            allowed_origin(&[], &with_origin("http://tauri.localhost")),
            None
        );
    }

    #[test]
    fn stamp_sets_full_set_for_allowed_origin() {
        let mut headers = HeaderMap::new();
        stamp(&mut headers, Some("http://tauri.localhost"));
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "http://tauri.localhost"
        );
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_HEADERS],
            "authorization, content-type"
        );
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_METHODS],
            "GET, POST, OPTIONS"
        );
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

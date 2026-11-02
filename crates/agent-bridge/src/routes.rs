//! Routes. It serves panes, output, preview, and send.
//! It never logs tokens or message text.

use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::Config;
use crate::{audit::Audit, codes::CodeErr, codes::CodeStore, limit::Limiter};

/// Max output lines per call.
const MAX_LINES: usize = 200;
/// Default output lines.
const DEFAULT_LINES: usize = 50;
/// Max message chars per send.
const MAX_TEXT: usize = 2000;
/// Audit sender. The bridge holds one token for the owner.
const SENDER: &str = "owner";

/// Shared app state. It holds config, codes, and limits.
pub struct AppState {
    /// Env config. Its Debug hides the token.
    pub config: Config,
    /// One-time confirm codes.
    pub codes: CodeStore,
    /// Send rate limiter.
    pub limiter: Limiter,
}

impl AppState {
    /// Build state from config. Limits use prod values.
    pub fn new(config: Config) -> Self {
        let ttl = config.code_ttl_secs;
        Self {
            config,
            codes: CodeStore::new(ttl),
            limiter: Limiter::new(10, 60),
        }
    }
}

/// Build the full router. Health stays open. All /v1 needs Bearer auth.
pub fn build_router(state: Arc<AppState>) -> Router<()> {
    let v1 = Router::new()
        .route("/v1/panes", get(list_panes))
        .route("/v1/panes/:id/output", get(pane_output))
        .route("/v1/panes/:id/message/preview", get(preview))
        .route("/v1/panes/:id/message", post(send_message))
        .layer(middleware::from_fn_with_state(state.clone(), require_token));
    // CORS sits outside auth. Preflight answers before any key check.
    // Errors pass through too, so they carry CORS headers as well.
    let cors_state = state.clone();
    let log_state = state.clone();
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .merge(v1)
        .with_state(state)
        .layer(middleware::from_fn(move |req, next| {
            crate::cors::layer(cors_state.clone(), req, next)
        }))
        .layer(middleware::from_fn(move |req, next| {
            crate::reqlog::layer(log_state.clone(), req, next)
        }))
}

/// Reject /v1 calls with bad auth.
async fn require_token(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if crate::auth::authorized(req.headers(), &state.config.token) {
        next.run(req).await
    } else {
        plain(
            StatusCode::UNAUTHORIZED,
            "Key is missing or wrong. Open Settings and paste it.",
        )
    }
}

/// Liveness. The process answers. No deps checked.
async fn live() -> impl IntoResponse {
    Json(json!({"ok": true}))
}

/// Readiness. It checks the herdr CLI.
async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let ok = crate::herdr::list_panes(&state.config.herdr).await.is_ok();
    Json(json!({"ok": ok}))
}

/// Plain JSON message with a status. All errors use this shape.
fn plain(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"message": message}))).into_response()
}

/// List allowlisted panes. Empty allowlist gives an empty list.
async fn list_panes(State(state): State<Arc<AppState>>) -> Response {
    let allow = crate::allow::load_allowlist(state.config.allowlist.as_deref());
    let panes = match crate::herdr::list_panes(&state.config.herdr).await {
        Ok(p) => p,
        Err(_) => {
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            )
        }
    };
    let rows: Vec<_> = panes
        .iter()
        .filter(|p| allow.contains(&p.id))
        .map(|p| {
            json!({
                "id": p.id,
                "label": p.label,
                "tab": p.tab,
                "kind": p.kind,
                "status": p.status,
            })
        })
        .collect();
    Json(json!({"panes": rows})).into_response()
}

/// Pane output params. Lines cap at 200.
async fn pane_output(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if !crate::herdr::valid_pane_id(&id) {
        return plain(StatusCode::BAD_REQUEST, "Pane id is bad.");
    }
    let lines: usize = match query.get("lines") {
        None => DEFAULT_LINES,
        Some(v) => match v.parse() {
            Ok(n) if (1..=MAX_LINES).contains(&n) => n,
            _ => return plain(StatusCode::BAD_REQUEST, "Lines must sit from 1 to 200."),
        },
    };
    let panes = match crate::herdr::list_panes(&state.config.herdr).await {
        Ok(p) => p,
        Err(_) => {
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            )
        }
    };
    let Some(pane) = panes.iter().find(|p| p.id == id) else {
        return plain(StatusCode::NOT_FOUND, "Pane is unknown. Refresh and retry.");
    };
    let allow = crate::allow::load_allowlist(state.config.allowlist.as_deref());
    if !allow.contains(&id) {
        return plain(
            StatusCode::FORBIDDEN,
            "Pane is not allowed. Pick a listed pane.",
        );
    }
    let text = match crate::herdr::read_pane(&state.config.herdr, &id).await {
        Ok(t) => t,
        Err(_) => {
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            )
        }
    };
    let raw: Vec<&str> = text.lines().collect();
    let kind = crate::kinds::normalize_kind(&pane.kind);
    let approval = crate::kinds::detect_approval(&kind, &raw);
    let start = raw.len().saturating_sub(lines);
    let out: Vec<String> = raw[start..].iter().map(|l| crate::redact(l)).collect();
    let count = out.len();
    let (waiting, prompt) = match approval {
        Some(p) => (true, Some(crate::redact(&p))),
        None => (false, None),
    };
    Json(json!({
        "pane": id,
        "lines": out,
        "count": count,
        "redacted": true,
        "approval_waiting": waiting,
        "approval_prompt": prompt,
    }))
    .into_response()
}

/// Preview a draft. It returns a one-time code plus exact text.
async fn preview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if !crate::herdr::valid_pane_id(&id) {
        return plain(StatusCode::BAD_REQUEST, "Pane id is bad.");
    }
    let Some(text) = query.get("text") else {
        return plain(StatusCode::BAD_REQUEST, "Text is required.");
    };
    if text.chars().count() > MAX_TEXT {
        return plain(
            StatusCode::BAD_REQUEST,
            "Text is too long. Keep 2000 chars or less.",
        );
    }
    let panes = match crate::herdr::list_panes(&state.config.herdr).await {
        Ok(p) => p,
        Err(_) => {
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            )
        }
    };
    let Some(pane) = panes.iter().find(|p| p.id == id) else {
        return plain(StatusCode::NOT_FOUND, "Pane is unknown. Refresh and retry.");
    };
    let allow = crate::allow::load_allowlist(state.config.allowlist.as_deref());
    if !allow.contains(&id) {
        return plain(
            StatusCode::FORBIDDEN,
            "Pane is not allowed. Pick a listed pane.",
        );
    }
    let kind = crate::kinds::normalize_kind(&pane.kind);
    if !crate::kinds::is_allowed_kind(&kind, &state.config.extra_kinds) {
        return plain(
            StatusCode::FORBIDDEN,
            "Agent kind is not allowed. Pick a listed pane.",
        );
    }
    if pane.status == "unknown" || pane.status.is_empty() {
        return plain(
            StatusCode::FORBIDDEN,
            "Pane status is unknown. Refresh and retry.",
        );
    }
    let output = match crate::herdr::read_pane(&state.config.herdr, &id).await {
        Ok(t) => t,
        Err(_) => {
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            )
        }
    };
    let raw: Vec<&str> = output.lines().collect();
    let approval = crate::kinds::detect_approval(&kind, &raw);
    if crate::kinds::is_short_answer(text) && approval.is_none() {
        return plain(
            StatusCode::FORBIDDEN,
            "Short answers need a visible approval prompt. Retry when it shows.",
        );
    }
    let (code, ttl) = state.codes.issue(&id, text);
    let (waiting, prompt) = match approval {
        Some(p) => (true, Some(crate::redact(&p))),
        None => (false, None),
    };
    Json(json!({
        "code": code,
        "text": text,
        "pane": id,
        "expires_in": ttl,
        "approval_waiting": waiting,
        "approval_prompt": prompt,
    }))
    .into_response()
}

/// Send a message. It needs the live code from preview.
/// It audits every attempt once. Text never reaches logs.
async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    // Parse first. Bad bodies fail before any herdr call.
    let parsed: Option<(String, String)> = String::from_utf8(body.to_vec())
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            let text = v.get("text")?.as_str()?.to_string();
            let code = v.get("code")?.as_str()?.to_string();
            Some((text, code))
        });
    let (text, code) = match parsed {
        Some(pair) => pair,
        None => {
            audit_now(&state, &id, 0, "refused:bad_body").await;
            return plain(StatusCode::BAD_REQUEST, "Body must hold text and code.");
        }
    };
    let text_len = text.chars().count();
    // Each check audits once, then returns. No path skips audit.
    if !crate::herdr::valid_pane_id(&id) {
        audit_now(&state, &id, text_len, "refused:shape").await;
        return plain(StatusCode::BAD_REQUEST, "Pane id is bad.");
    }
    let panes = match crate::herdr::list_panes(&state.config.herdr).await {
        Ok(p) => p,
        Err(_) => {
            audit_now(&state, &id, text_len, "error:herdr").await;
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            );
        }
    };
    let Some(pane) = panes.iter().find(|p| p.id == id) else {
        audit_now(&state, &id, text_len, "refused:unknown_pane").await;
        return plain(StatusCode::NOT_FOUND, "Pane is unknown. Refresh and retry.");
    };
    let allow = crate::allow::load_allowlist(state.config.allowlist.as_deref());
    if !allow.contains(&id) {
        audit_now(&state, &id, text_len, "refused:allowlist").await;
        return plain(
            StatusCode::FORBIDDEN,
            "Pane is not allowed. Pick a listed pane.",
        );
    }
    // Labels drift. Both id and label must sit in the allowlist.
    if !allow.contains(&pane.label) {
        audit_now(&state, &id, text_len, "refused:label").await;
        return plain(
            StatusCode::FORBIDDEN,
            "Pane label drifted. Pick the pane again.",
        );
    }
    let kind = crate::kinds::normalize_kind(&pane.kind);
    if !crate::kinds::is_allowed_kind(&kind, &state.config.extra_kinds) {
        audit_now(&state, &id, text_len, "refused:kind").await;
        return plain(
            StatusCode::FORBIDDEN,
            "Agent kind is not allowed. Pick a listed pane.",
        );
    }
    if pane.status == "unknown" || pane.status.is_empty() {
        audit_now(&state, &id, text_len, "refused:status").await;
        return plain(
            StatusCode::FORBIDDEN,
            "Pane status is unknown. Refresh and retry.",
        );
    }
    if text_len > MAX_TEXT {
        audit_now(&state, &id, text_len, "refused:text_too_long").await;
        return plain(
            StatusCode::BAD_REQUEST,
            "Text is too long. Keep 2000 chars or less.",
        );
    }
    match state.codes.check(&id, &code, &text) {
        Ok(()) => {}
        Err(CodeErr::Expired) => {
            audit_now(&state, &id, text_len, "refused:expired_code").await;
            return code_refused(&id, "Code expired. Fetch a fresh preview.");
        }
        Err(CodeErr::Used) => {
            audit_now(&state, &id, text_len, "refused:used_code").await;
            return code_refused(&id, "Code is used. Fetch a fresh preview.");
        }
        Err(CodeErr::Missing) => {
            audit_now(&state, &id, text_len, "refused:bad_code").await;
            return code_refused(&id, "Code is bad. Fetch a fresh preview.");
        }
    }
    if let Err(wait) = state.limiter.try_send() {
        audit_now(&state, &id, text_len, "refused:rate").await;
        let mut resp = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "message": format!("Rate limit hit. Retry in {wait} s."),
                "retry_after": wait,
            })),
        )
            .into_response();
        resp.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from_str(&wait.to_string()).unwrap_or(HeaderValue::from_static("60")),
        );
        return resp;
    }
    // Short answers ride only on a live approval prompt.
    if crate::kinds::is_short_answer(&text) {
        let output = match crate::herdr::read_pane(&state.config.herdr, &id).await {
            Ok(t) => t,
            Err(_) => {
                audit_now(&state, &id, text_len, "error:herdr").await;
                return plain(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "herdr is down. Retry soon.",
                );
            }
        };
        let raw: Vec<&str> = output.lines().collect();
        if crate::kinds::detect_approval(&kind, &raw).is_none() {
            audit_now(&state, &id, text_len, "refused:approval").await;
            return plain(
                StatusCode::FORBIDDEN,
                "Short answers need a visible approval prompt. Retry when it shows.",
            );
        }
    }
    // One code works once. Consume it at send time.
    state.codes.consume(&code);
    // Escape sends alone. It never pastes text and never adds Enter.
    if crate::kinds::is_escape_key(&text) {
        if crate::herdr::send_escape(&state.config.herdr, &id)
            .await
            .is_err()
        {
            audit_now(&state, &id, text_len, "error:herdr").await;
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            );
        }
    } else {
        // Empty text means bare Enter. It skips send-text.
        if !text.is_empty()
            && crate::herdr::send_text(&state.config.herdr, &id, &text)
                .await
                .is_err()
        {
            audit_now(&state, &id, text_len, "error:herdr").await;
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            );
        }
        // Enter submits the text.
        if crate::herdr::send_enter(&state.config.herdr, &id)
            .await
            .is_err()
        {
            audit_now(&state, &id, text_len, "error:herdr").await;
            return plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "herdr is down. Retry soon.",
            );
        }
    }
    tracing::info!(pane = %id, text_len, "agent-bridge sent");
    audit_now(&state, &id, text_len, "sent").await;
    Json(json!({"sent": true, "pane": id})).into_response()
}

/// 403 for bad codes. It points at a fresh preview.
fn code_refused(id: &str, message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "message": message,
            "preview": format!("/v1/panes/{id}/message/preview"),
        })),
    )
        .into_response()
}

/// Audit one attempt. Failures log to stderr without text.
async fn audit_now(state: &Arc<AppState>, pane: &str, text_len: usize, result: &str) {
    let audit = Audit::new(state.config.audit_log.clone());
    if let Err(e) = audit.append(pane, SENDER, text_len, result).await {
        tracing::error!(pane = %pane, result, "audit append failed: {e}");
    }
}

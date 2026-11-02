//! API tests. They use the fake herdr only.
//! They never call real herdr or touch real panes.

use agent_bridge::{build_router, default_origins, AppState, Config};
use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, Request, StatusCode},
    Router,
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

/// Serializes fake-herdr env use. One API test runs at a time.
static LOCK: Mutex<()> = Mutex::new(());

/// Captured tracing output. Reqlog tests read this.
/// API tests run serially, so snapshots stay exact.
static LOG_BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);

/// tracing writer. It appends every line to the capture buffer.
struct BufWriter;

impl std::io::Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        LOG_BUF.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Install the capture subscriber once. Global fits all test threads.
fn init_log_capture() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let sub = tracing_subscriber::fmt()
            .json()
            .with_writer(|| BufWriter)
            .finish();
        let _ = tracing::subscriber::set_global_default(sub);
    });
}

/// Parse captured lines since a byte mark. Each line is one JSON log.
fn new_log_lines(since: usize) -> Vec<Value> {
    let buf = LOG_BUF.lock().unwrap();
    String::from_utf8_lossy(&buf[since..])
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Current capture size. Tests snapshot this before requests.
fn log_mark() -> usize {
    LOG_BUF.lock().unwrap().len()
}

/// Path to the fake herdr script in fixtures.
fn fake_herdr() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-herdr.sh")
}

/// One test setup. It holds the lock for the whole test.
struct Fixture {
    /// Lock guard. It serializes env access.
    _guard: std::sync::MutexGuard<'static, ()>,
    /// Scratch dir for this test.
    dir: PathBuf,
    /// App router under test.
    app: Router<()>,
    /// Auth token for this test.
    token: String,
}

/// Setup options. Each test names what it needs.
#[derive(Default)]
struct Opts {
    /// Allowlist file content. None means missing file.
    allow: Option<String>,
    /// Pane list stdout for the fake. None means script default.
    panes: Option<String>,
    /// Pane read stdout for the fake. None means script default.
    read: Option<String>,
    /// Extra allowed kinds.
    extra_kinds: Vec<String>,
    /// CORS origins. None means the prod defaults.
    origins: Option<Vec<String>>,
    /// Code TTL in seconds.
    ttl: Option<u64>,
    /// Fake herdr fails all calls.
    fail: bool,
}

/// Build a fixture. It resets all fake env vars first.
fn setup(name: &str, opts: Opts) -> Fixture {
    let guard = LOCK.lock().unwrap();
    let dir = std::env::temp_dir().join(format!("agent-bridge-api-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    for var in [
        "FAKE_HERDR_PANES_FILE",
        "FAKE_HERDR_READ_FILE",
        "FAKE_HERDR_SENDLOG",
        "FAKE_HERDR_FAIL",
    ] {
        std::env::remove_var(var);
    }
    let allow_path = dir.join("allow.txt");
    if let Some(content) = opts.allow {
        std::fs::write(&allow_path, content).unwrap();
    }
    let audit = dir.join("audit.jsonl");
    let sendlog = dir.join("send.log");
    std::fs::write(&sendlog, "").unwrap();
    if let Some(panes) = opts.panes {
        let path = dir.join("panes.json");
        std::fs::write(&path, panes).unwrap();
        std::env::set_var("FAKE_HERDR_PANES_FILE", &path);
    }
    if let Some(read) = opts.read {
        let path = dir.join("read.json");
        std::fs::write(&path, read).unwrap();
        std::env::set_var("FAKE_HERDR_READ_FILE", &path);
    }
    std::env::set_var("FAKE_HERDR_SENDLOG", &sendlog);
    if opts.fail {
        std::env::set_var("FAKE_HERDR_FAIL", "1");
    }
    let token = "fixture-token-1".to_string();
    let config = Config {
        token: token.clone(),
        allowlist: Some(allow_path),
        audit_log: audit,
        herdr: fake_herdr(),
        extra_kinds: opts.extra_kinds,
        code_ttl_secs: opts.ttl.unwrap_or(60),
        port: 0,
        origins: opts.origins.unwrap_or_else(default_origins),
    };
    let app = build_router(Arc::new(AppState::new(config)));
    Fixture {
        _guard: guard,
        dir,
        app,
        token,
    }
}

/// Default panes stdout. Two panes with known kinds.
fn panes_two() -> String {
    r#"{"result":{"type":"pane_list","panes":[
{"pane_id":"w1:p1","label":"lifeos","tab_id":"w1:t1","agent":"muse","agent_status":"idle"},
{"pane_id":"w1:p2","label":"site","tab_id":"w1:t1","agent":"muse","agent_status":"working"}
]}}"#
        .to_string()
}

/// Read stdout that wraps plain text.
fn read_text(text: &str) -> String {
    serde_json::json!({"result": {"type": "pane_read", "text": text}}).to_string()
}

/// Percent-encode a query value. It keeps unreserved chars plain.
fn enc(text: &str) -> String {
    let mut out = String::new();
    for b in text.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Send a request. It returns status, headers, and parsed JSON.
async fn call(
    fx: &Fixture,
    method: &str,
    path: &str,
    auth: bool,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    call_origin(fx, method, path, auth, body, None).await
}

/// Send a request with an Origin header. CORS tests use this.
async fn call_origin(
    fx: &Fixture,
    method: &str,
    path: &str,
    auth: bool,
    body: Option<Value>,
    origin: Option<&str>,
) -> (StatusCode, HeaderMap, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if auth {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", fx.token));
    }
    if let Some(origin) = origin {
        builder = builder.header(header::ORIGIN, origin);
    }
    let req = if let Some(json) = body {
        builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_string(&json).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let resp = fx.app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, json)
}

/// Read the send log lines.
fn sendlog(fx: &Fixture) -> Vec<String> {
    let text = std::fs::read_to_string(fx.dir.join("send.log")).unwrap();
    text.lines().map(str::to_string).collect()
}

/// Read the audit lines as JSON.
fn audit(fx: &Fixture) -> Vec<Value> {
    let path = fx.dir.join("audit.jsonl");
    if !path.exists() {
        return vec![];
    }
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Preview then send in one step. It returns the send response.
async fn preview_and_send(fx: &Fixture, id: &str, text: &str) -> (StatusCode, HeaderMap, Value) {
    let (status, _, preview) = call(
        fx,
        "GET",
        &format!("/v1/panes/{id}/message/preview?text={}", enc(text)),
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "preview: {preview}");
    assert_eq!(preview["text"], text);
    let code = preview["code"].as_str().unwrap().to_string();
    call(
        fx,
        "POST",
        &format!("/v1/panes/{id}/message"),
        true,
        Some(serde_json::json!({"text": text, "code": code})),
    )
    .await
}

#[tokio::test]
async fn health_stays_open() {
    let fx = setup("health", Opts::default());
    let (status, _, body) = call(&fx, "GET", "/health/live", false, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn auth_blocks_bad_tokens() {
    let fx = setup("auth", Opts::default());
    let (status, _, _) = call(&fx, "GET", "/v1/panes", false, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let req = Request::builder()
        .uri("/v1/panes")
        .header(header::AUTHORIZATION, "Bearer wrong")
        .body(Body::empty())
        .unwrap();
    let resp = fx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn empty_allowlist_refuses_everything() {
    let fx = setup(
        "empty-allow",
        Opts {
            allow: Some("".to_string()),
            panes: Some(panes_two()),
            ..Default::default()
        },
    );
    let (status, _, body) = call(&fx, "GET", "/v1/panes", true, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["panes"], serde_json::json!([]));
    let (status, _, _) = call(&fx, "GET", "/v1/panes/w1:p1/output", true, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=hi",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(sendlog(&fx).is_empty());
    // Only the POST audits. Reads never write audit lines.
    assert_eq!(audit(&fx).len(), 1);
}

#[tokio::test]
async fn list_shows_only_allowlisted_panes() {
    let fx = setup(
        "list",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            ..Default::default()
        },
    );
    let (status, _, body) = call(&fx, "GET", "/v1/panes", true, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["panes"],
        serde_json::json!([{
            "id": "w1:p1",
            "label": "lifeos",
            "tab": "w1:t1",
            "kind": "muse",
            "status": "idle",
        }])
    );
}

#[tokio::test]
async fn valid_code_sends_exact_bytes() {
    let fx = setup(
        "send",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    let text = "Hello, brij — deploy at 6? caf\u{e9} ok";
    let (status, _, body) = preview_and_send(&fx, "w1:p1", text).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sent"], true);
    let log = sendlog(&fx);
    assert_eq!(log.len(), 2);
    assert_eq!(log[0], format!("send-text w1:p1 {text}"));
    assert_eq!(log[1], "send-keys w1:p1 Enter");
    let lines = audit(&fx);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["result"], "sent");
    assert_eq!(lines[0]["text_len"], text.chars().count() as u64);
    assert_eq!(lines[0]["pane"], "w1:p1");
    assert_eq!(lines[0]["sender"], "owner");
    assert!(lines[0].get("time").is_some());
}

#[tokio::test]
async fn wrong_reused_and_expired_codes_fail() {
    let fx = setup(
        "codes",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    // Wrong code fails before any herdr send.
    let (status, _, body) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": "nope"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body["preview"].as_str().unwrap().contains("preview"));
    assert!(sendlog(&fx).is_empty());
    // First use sends. Second use fails as used.
    let (status, _, preview) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=hi",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let code = preview["code"].as_str().unwrap().to_string();
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": code})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": code})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(sendlog(&fx).len(), 2);
    drop(fx);
    // Old code expires after its TTL.
    let fx = setup(
        "codes-expire",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ttl: Some(1),
            ..Default::default()
        },
    );
    let (status, _, preview) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=hi",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["expires_in"], 1);
    let code = preview["code"].as_str().unwrap().to_string();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": code})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(sendlog(&fx).is_empty());
}

#[tokio::test]
async fn preview_text_matches_sent_bytes() {
    let fx = setup(
        "match",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    let (status, _, preview) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=exact",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let code = preview["code"].as_str().unwrap().to_string();
    // Altered text fails even with the live code.
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "exact!", "code": code})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(sendlog(&fx).is_empty());
}

#[tokio::test]
async fn pane_id_injection_is_rejected() {
    let fx = setup(
        "inject",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    for bad in [
        "evil",
        "w1",
        "w1:p-1",
        "w1:p1;id",
        "w1:p1%20x",
        "w1:p1%0Aid",
        "w1:p1%7Cid",
    ] {
        let (status, _, _) = call(&fx, "GET", &format!("/v1/panes/{bad}/output"), true, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        let (status, _, _) = call(
            &fx,
            "POST",
            &format!("/v1/panes/{bad}/message"),
            true,
            Some(serde_json::json!({"text": "hi", "code": "x"})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }
    assert!(sendlog(&fx).is_empty());
}

#[tokio::test]
async fn rate_limit_caps_sends() {
    let fx = setup(
        "rate",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    for i in 0..10 {
        let (status, _, _) = preview_and_send(&fx, "w1:p1", &format!("msg {i}")).await;
        assert_eq!(status, StatusCode::OK, "send {i}");
    }
    let (status, headers, body) = preview_and_send(&fx, "w1:p1", "msg 10").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(body["retry_after"].as_u64().unwrap() >= 1);
    assert!(headers.contains_key(header::RETRY_AFTER));
    assert_eq!(sendlog(&fx).len(), 20);
}

#[tokio::test]
async fn output_redacts_tokens_and_slices_lines() {
    // The run builds at runtime. No long literal sits in source.
    let run = "ef".repeat(14);
    let fx = setup(
        "redact",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text(&format!("one\ntwo\nkey {run} here\nfour\n"))),
            ..Default::default()
        },
    );
    let (status, _, body) = call(&fx, "GET", "/v1/panes/w1:p1/output?lines=2", true, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["count"], 2);
    assert_eq!(body["redacted"], true);
    let lines = body["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "key [redacted] here");
    assert_eq!(lines[1], "four");
    let dumped = serde_json::to_string(&body).unwrap();
    assert!(!dumped.contains(&run));
}

#[tokio::test]
async fn audit_holds_shape_without_text() {
    let fx = setup(
        "audit",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    let secret = "supersecret-plan-99";
    let (status, _, _) = preview_and_send(&fx, "w1:p1", secret).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": secret, "code": "bad"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let raw = std::fs::read_to_string(fx.dir.join("audit.jsonl")).unwrap();
    assert!(!raw.contains(secret));
    let lines = audit(&fx);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["result"], "sent");
    assert_eq!(lines[1]["result"], "refused:bad_code");
    for line in &lines {
        assert!(line.get("time").is_some());
        assert_eq!(line["pane"], "w1:p1");
        assert_eq!(line["sender"], "owner");
        assert_eq!(line["text_len"], secret.chars().count() as u64);
        assert!(line.get("text").is_none());
        assert!(line.get("code").is_none());
    }
}

#[tokio::test]
async fn unknown_kind_refuses_until_owner_allows() {
    let panes = r#"{"result":{"type":"pane_list","panes":[
{"pane_id":"w1:p9","label":"lab","tab_id":"w1:t1","agent":"mysterybot","agent_status":"idle"}
]}}"#
        .to_string();
    let fx = setup(
        "kind",
        Opts {
            allow: Some("w1:p9\nlab\n".to_string()),
            panes: Some(panes.clone()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    let (status, _, _) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p9/message/preview?text=hi",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    drop(fx);
    // Owner allow in config opens the kind with generic rules.
    let fx = setup(
        "kind-extra",
        Opts {
            allow: Some("w1:p9\nlab\n".to_string()),
            panes: Some(panes),
            read: Some(read_text("working\n")),
            extra_kinds: vec!["mysterybot".to_string()],
            ..Default::default()
        },
    );
    let (status, _, body) = preview_and_send(&fx, "w1:p9", "hi").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sent"], true);
}

#[tokio::test]
async fn unknown_status_refuses_sends() {
    let panes = r#"{"result":{"type":"pane_list","panes":[
{"pane_id":"w1:p9","label":"lab","tab_id":"w1:t1","agent":"muse","agent_status":"unknown"}
]}}"#
        .to_string();
    let fx = setup(
        "status",
        Opts {
            allow: Some("w1:p9\nlab\n".to_string()),
            panes: Some(panes),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    let (status, _, _) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p9/message/preview?text=hi",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Reads still work. Status gates sends only.
    let (status, _, _) = call(&fx, "GET", "/v1/panes/w1:p9/output", true, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn label_drift_refuses_sends() {
    let fx = setup(
        "drift",
        Opts {
            allow: Some("w1:p1\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("working\n")),
            ..Default::default()
        },
    );
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(sendlog(&fx).is_empty());
    let lines = audit(&fx);
    assert_eq!(lines[0]["result"], "refused:label");
}

#[tokio::test]
async fn short_answers_need_a_live_prompt() {
    // Plain output refuses y at preview time.
    let fx = setup(
        "prompt-no",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("compiling still\n")),
            ..Default::default()
        },
    );
    let (status, _, _) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=y",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    drop(fx);
    // Live prompt accepts y plus Enter.
    let fx = setup(
        "prompt-yes",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("run this? Approve (y/n)\n")),
            ..Default::default()
        },
    );
    let (status, _, body) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=y",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["approval_waiting"], true);
    let code = body["code"].as_str().unwrap().to_string();
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "y", "code": code})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let log = sendlog(&fx);
    assert_eq!(log, vec!["send-text w1:p1 y", "send-keys w1:p1 Enter"]);
    // Bare Enter sends the key alone.
    let (status, _, body) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let code = body["code"].as_str().unwrap().to_string();
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "", "code": code})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let log = sendlog(&fx);
    assert_eq!(log.len(), 3);
    assert_eq!(log[2], "send-keys w1:p1 Enter");
}

#[tokio::test]
async fn caps_reject_big_lines_and_text() {
    let fx = setup(
        "caps",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("hi\n")),
            ..Default::default()
        },
    );
    for bad in ["0", "201", "abc"] {
        let (status, _, _) = call(
            &fx,
            "GET",
            &format!("/v1/panes/w1:p1/output?lines={bad}"),
            true,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }
    let big = "x".repeat(2001);
    let (status, _, _) = call(
        &fx,
        "GET",
        &format!("/v1/panes/w1:p1/message/preview?text={big}"),
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": big, "code": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(sendlog(&fx).is_empty());
}

#[tokio::test]
async fn unknown_pane_gives_404() {
    let fx = setup(
        "unknown",
        Opts {
            allow: Some("w9:p9\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("hi\n")),
            ..Default::default()
        },
    );
    let (status, _, _) = call(&fx, "GET", "/v1/panes/w9:p9/output", true, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w9:p9/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(sendlog(&fx).is_empty());
}

#[tokio::test]
async fn herdr_down_gives_503() {
    let fx = setup(
        "down",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            fail: true,
            ..Default::default()
        },
    );
    let (status, _, _) = call(&fx, "GET", "/v1/panes", true, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let (status, _, _) = call(&fx, "GET", "/v1/panes/w1:p1/output", true, None).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let (status, _, _) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=hi",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let (status, _, _) = call(
        &fx,
        "POST",
        "/v1/panes/w1:p1/message",
        true,
        Some(serde_json::json!({"text": "hi", "code": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let lines = audit(&fx);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["result"], "error:herdr");
}

/// Phone app origin. It sits in the default allowlist.
const APP_ORIGIN: &str = "https://example.ts.net:8447";

/// CORS setup. One allowlisted pane plus default origins.
fn cors_opts() -> Opts {
    Opts {
        allow: Some("w1:p1\nlifeos\n".to_string()),
        panes: Some(panes_two()),
        read: Some(read_text("working\n")),
        ..Default::default()
    }
}

#[tokio::test]
async fn preflight_answers_204_without_key() {
    let fx = setup("cors-preflight", cors_opts());
    for path in ["/v1/panes", "/v1/panes/w1:p1/message"] {
        let (status, headers, _) =
            call_origin(&fx, "OPTIONS", path, false, None, Some(APP_ORIGIN)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{path}");
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            APP_ORIGIN,
            "{path}"
        );
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_HEADERS],
            "authorization, content-type",
            "{path}"
        );
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_METHODS],
            "GET, POST, OPTIONS",
            "{path}"
        );
        assert_eq!(headers[header::ACCESS_CONTROL_MAX_AGE], "600", "{path}");
        assert_eq!(headers[header::VARY], "Origin", "{path}");
    }
}

#[tokio::test]
async fn allowed_origin_is_echoed_on_get() {
    let fx = setup("cors-echo", cors_opts());
    let (status, headers, _) =
        call_origin(&fx, "GET", "/v1/panes", true, None, Some(APP_ORIGIN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], APP_ORIGIN);
    assert_eq!(headers[header::VARY], "Origin");
}

#[tokio::test]
async fn unknown_origin_gets_no_allow_header() {
    let fx = setup("cors-unknown", cors_opts());
    let (status, headers, _) = call_origin(
        &fx,
        "GET",
        "/v1/panes",
        true,
        None,
        Some("https://evil.example"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    assert_eq!(headers[header::VARY], "Origin");
    // Preflight from unknown origins stays 204 but grants nothing.
    let (status, headers, _) = call_origin(
        &fx,
        "OPTIONS",
        "/v1/panes",
        false,
        None,
        Some("https://evil.example"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[tokio::test]
async fn get_without_key_still_401_with_cors_headers() {
    let fx = setup("cors-401", cors_opts());
    let (status, headers, _) =
        call_origin(&fx, "GET", "/v1/panes", false, None, Some(APP_ORIGIN)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], APP_ORIGIN);
}

#[tokio::test]
async fn reqlog_with_key_holds_fields_without_key() {
    init_log_capture();
    let fx = setup("reqlog-key", cors_opts());
    let mark = log_mark();
    let (status, _, _) = call_origin(&fx, "GET", "/v1/panes", true, None, Some(APP_ORIGIN)).await;
    assert_eq!(status, StatusCode::OK);
    let lines = new_log_lines(mark);
    assert_eq!(lines.len(), 1);
    let fields = &lines[0]["fields"];
    assert_eq!(fields["method"], "GET");
    assert_eq!(fields["path"], "/v1/panes");
    assert_eq!(fields["origin"], APP_ORIGIN);
    assert_eq!(fields["status"], 200);
    assert_eq!(fields["key_present"], "yes");
    assert_eq!(fields["key_match"], "yes");
    let raw = serde_json::to_string(&lines[0]).unwrap();
    assert!(!raw.contains(&fx.token));
}

#[tokio::test]
async fn reqlog_without_key_marks_absent() {
    init_log_capture();
    let fx = setup("reqlog-nokey", cors_opts());
    let mark = log_mark();
    let (status, _, _) = call(&fx, "GET", "/v1/panes", false, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let lines = new_log_lines(mark);
    assert_eq!(lines.len(), 1);
    let fields = &lines[0]["fields"];
    assert_eq!(fields["method"], "GET");
    assert_eq!(fields["path"], "/v1/panes");
    assert_eq!(fields["origin"], "none");
    assert_eq!(fields["status"], 401);
    assert_eq!(fields["key_present"], "no");
    assert_eq!(fields["key_match"], "no");
    let raw = serde_json::to_string(&lines[0]).unwrap();
    assert!(!raw.contains(&fx.token));
}

#[tokio::test]
async fn reqlog_includes_preflight_and_wrong_key() {
    init_log_capture();
    let fx = setup("reqlog-preflight", cors_opts());
    let mark = log_mark();
    let (status, _, _) =
        call_origin(&fx, "OPTIONS", "/v1/panes", false, None, Some(APP_ORIGIN)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let req = Request::builder()
        .uri("/v1/panes")
        .header(header::AUTHORIZATION, "Bearer wrong")
        .body(Body::empty())
        .unwrap();
    let resp = fx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let lines = new_log_lines(mark);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["fields"]["method"], "OPTIONS");
    assert_eq!(lines[0]["fields"]["status"], 204);
    assert_eq!(lines[0]["fields"]["key_present"], "no");
    assert_eq!(lines[1]["fields"]["method"], "GET");
    assert_eq!(lines[1]["fields"]["status"], 401);
    assert_eq!(lines[1]["fields"]["key_present"], "yes");
    assert_eq!(lines[1]["fields"]["key_match"], "no");
    for line in &lines {
        let raw = serde_json::to_string(line).unwrap();
        assert!(!raw.contains(&fx.token));
        assert!(!raw.contains("wrong"));
    }
}

/// Panes stdout with a supervisor session plus one builder.
fn panes_with_supervisor() -> String {
    r#"{"result":{"type":"pane_list","panes":[
{"pane_id":"w1:p9","label":"supervisor","tab_id":"w1:t1","agent":"supervisor","agent_status":"idle"},
{"pane_id":"w1:p1","label":"lifeos","tab_id":"w1:t1","agent":"muse","agent_status":"idle"}
]}}"#
        .to_string()
}

#[tokio::test]
async fn digit_quick_key_needs_a_live_prompt() {
    // Plain output refuses 1 at preview time.
    let fx = setup(
        "quick-no",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("compiling still\n")),
            ..Default::default()
        },
    );
    let (status, _, _) = call(
        &fx,
        "GET",
        "/v1/panes/w1:p1/message/preview?text=1",
        true,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    drop(fx);
    // Live prompt accepts 1 plus Enter.
    let fx = setup(
        "quick-yes",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("Pick one:\n1) ship 2) hold\npress enter\n")),
            ..Default::default()
        },
    );
    let (status, _, body) = preview_and_send(&fx, "w1:p1", "1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sent"], true);
    let log = sendlog(&fx);
    assert_eq!(log, vec!["send-text w1:p1 1", "send-keys w1:p1 Enter"]);
    let lines = audit(&fx);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["result"], "sent");
    assert_eq!(lines[0]["text_len"], 1);
    assert!(lines[0].get("text").is_none());
    assert!(lines[0].get("code").is_none());
}

#[tokio::test]
async fn escape_key_sends_alone_without_enter() {
    // Escape needs no prompt. It is the way out of stuck screens.
    let fx = setup(
        "esc",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("compiling still\n")),
            ..Default::default()
        },
    );
    let (status, _, body) = preview_and_send(&fx, "w1:p1", "\u{1b}").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sent"], true);
    let log = sendlog(&fx);
    assert_eq!(log, vec!["send-keys w1:p1 Escape"]);
    let lines = audit(&fx);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["result"], "sent");
    assert_eq!(lines[0]["text_len"], 1);
    assert!(lines[0].get("text").is_none());
    assert!(lines[0].get("code").is_none());
}

#[tokio::test]
async fn continue_word_sends_as_plain_text() {
    let fx = setup(
        "continue",
        Opts {
            allow: Some("w1:p1\nlifeos\n".to_string()),
            panes: Some(panes_two()),
            read: Some(read_text("waiting\n")),
            ..Default::default()
        },
    );
    let (status, _, body) = preview_and_send(&fx, "w1:p1", "continue").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sent"], true);
    let log = sendlog(&fx);
    assert_eq!(
        log,
        vec!["send-text w1:p1 continue", "send-keys w1:p1 Enter"]
    );
    let lines = audit(&fx);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["text_len"], 8);
    assert!(lines[0].get("text").is_none());
}

#[tokio::test]
async fn supervisor_kind_accepts_sends() {
    let fx = setup(
        "supervisor",
        Opts {
            allow: Some("w1:p9\nsupervisor\n".to_string()),
            panes: Some(panes_with_supervisor()),
            read: Some(read_text("watching\n")),
            ..Default::default()
        },
    );
    let (status, _, body) = preview_and_send(&fx, "w1:p9", "status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["sent"], true);
    let lines = audit(&fx);
    assert_eq!(lines[0]["pane"], "w1:p9");
    assert!(lines[0].get("text").is_none());
}

#[tokio::test]
async fn tauri_origin_preflight_echoes() {
    let fx = setup("cors-tauri", cors_opts());
    for origin in [
        "http://tauri.localhost",
        "https://tauri.localhost",
        "tauri://localhost",
    ] {
        let (status, headers, _) =
            call_origin(&fx, "OPTIONS", "/v1/panes", false, None, Some(origin)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{origin}");
        assert_eq!(
            headers[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            origin,
            "{origin}"
        );
    }
    let (status, headers, _) = call_origin(
        &fx,
        "GET",
        "/v1/panes",
        true,
        None,
        Some("http://tauri.localhost"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "http://tauri.localhost"
    );
}

//! Hearth server. Mounts all modules under `/v1`.
//! Probes: `/health/live` and `/health/ready`.
//! Contract manifest: `/v1/routes`.

use aiusage::AiUsage;
use app::App;
use axum::{
    extract::{Extension, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Json, Router,
};
use business::Business;
use factory::Factory;
use health::HealthMod;
use hearth_core::{run_migrations, AppState, Module};
use hello::Hello;
use inventory::Inventory;
use reminders::Reminders;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::net::SocketAddr;
use tasks::Tasks;
use today::Today;
use tracing::info;
use watch::Watch;

/// CORS preflight plus allowlist headers. Lives outside auth.
mod cors;

/// Sorted API contract manifest. G1 diffs this file.
const ROUTES: &[&str] = &[
    "/health/live",
    "/health/ready",
    "/v1/ai/results",
    "/v1/ai/usage",
    "/v1/app/apk",
    "/v1/app/update",
    "/v1/business/invoices",
    "/v1/business/overview",
    "/v1/business/schedule",
    "/v1/factory/status",
    "/v1/features/:id/mark",
    "/v1/health/checkin",
    "/v1/health/checkins",
    "/v1/health/status",
    "/v1/hello",
    "/v1/inventory",
    "/v1/inventory/by-barcode/:code",
    "/v1/inventory/rooms",
    "/v1/reminders",
    "/v1/reminders/:id/done",
    "/v1/reminders/:id/snooze",
    "/v1/routes",
    "/v1/tasks",
    "/v1/tasks/:id/done",
    "/v1/tasks/:id/move",
    "/v1/tasks/:id/split",
    "/v1/tasks/:id/steps/:n/done",
    "/v1/tasks/:id/undo",
    "/v1/today",
    "/v1/watch/ask-queue",
    "/v1/watch/features",
    "/v1/watch/muse",
    "/v1/watch/needs-you",
    "/v1/watch/threads",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--print-routes") {
        let mut routes = ROUTES.to_vec();
        routes.sort_unstable();
        println!("{}", json!({"routes": routes}));
        return Ok(());
    }
    let migrate_only = args.iter().any(|a| a == "--migrate-only");
    tracing_subscriber::fmt()
        .with_env_filter("hearth=debug,tower_http=info")
        .json()
        .init();

    let db = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL not set");
    let token = std::env::var("HEARTH_TOKEN").expect("HEARTH_TOKEN not set (refuse to serve open)");
    let port: u16 = std::env::var("PORT")
        .unwrap_or("8080".into())
        .parse()
        .expect("bad PORT");
    let origins = cors::parse_origins(std::env::var("HEARTH_CORS_ORIGINS").ok())
        .expect("bad HEARTH_CORS_ORIGINS");

    let pool = PgPool::connect(&db).await?;
    let redis = redis::Client::open(redis_url)?;
    let state = AppState {
        pool: pool.clone(),
        redis,
    };

    let aiusage = AiUsage;
    let app = App;
    let business = Business;
    let health = HealthMod;
    let hello = Hello;
    let today = Today;
    let reminders = Reminders;
    let tasks = Tasks;
    let inventory = Inventory;
    let watch = Watch;
    let factory = Factory;
    let all: [&dyn Module; 11] = [
        &business, &health, &hello, &today, &reminders, &tasks, &inventory, &watch, &factory, &app,
        &aiusage,
    ];
    let migs: Vec<(&'static str, Vec<hearth_core::Migration>)> =
        all.iter().map(|m| (m.id(), m.migrations())).collect();
    run_migrations(&pool, &migs).await?;
    info!("migrations done");
    if migrate_only {
        info!("migrate-only: done");
        return Ok(());
    }

    let app = build_router(state, &all, token, origins);
    let bind_raw = std::env::var("BIND_ADDR").unwrap_or("127.0.0.1".into());
    let addr = parse_bind(&bind_raw, port).expect("bad BIND_ADDR");
    info!(%addr, "Hearth serves");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

/// Build the full router. Pure: safe to test.
/// Takes a module slice, so new lanes never churn this shape.
/// Health stays open; all /v1 routes need the bearer token.
fn build_router(
    state: AppState,
    modules: &[&dyn Module],
    token: String,
    origins: Vec<String>,
) -> Router<()> {
    let mut v1 = Router::new().route("/v1/routes", get(routes_manifest));
    for m in modules {
        v1 = v1.merge(m.routes(&state));
    }
    let v1 = v1.layer(middleware::from_fn_with_state(token, require_token));
    // CORS sits outside auth. Preflight answers before any key check.
    // Errors pass through too, so they carry CORS headers as well.
    let origins = std::sync::Arc::new(origins);
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .merge(v1)
        .layer(Extension(state))
        .layer(middleware::from_fn(move |req, next| {
            cors::layer(origins.clone(), req, next)
        }))
}

/// Reject /v1 calls without the bearer token.
async fn require_token(
    State(token): State<String>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if authorized(req.headers(), &token) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// True when the Authorization header carries the token. Pure, so tests cover it.
fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let want = format!("Bearer {token}");
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        == Some(want.as_str())
}

/// Parse bind address plus port. Pure, so tests cover it.
fn parse_bind(addr: &str, port: u16) -> Result<SocketAddr, String> {
    addr.parse::<std::net::IpAddr>()
        .map(|ip| SocketAddr::new(ip, port))
        .map_err(|e| e.to_string())
}

/// Liveness: the process answers. No deps checked.
async fn live() -> Json<Value> {
    Json(json!({"ok": true}))
}

/// Readiness: PG and Redis answer.
async fn ready(Extension(state): Extension<AppState>) -> Json<Value> {
    let db_ok = sqlx::query("SELECT 1").fetch_one(&state.pool).await.is_ok();
    let cache_ok = ping_cache(&state).await;
    Json(json!({"ok": db_ok && cache_ok, "db": db_ok, "cache": cache_ok}))
}

/// Ping Redis. False on any failure.
async fn ping_cache(state: &AppState) -> bool {
    let mut conn = match state.redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return false,
    };
    redis::cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .is_ok()
}

/// Sorted route manifest for the G1 contract diff.
async fn routes_manifest() -> Json<Value> {
    let mut routes = ROUTES.to_vec();
    routes.sort_unstable();
    Json(json!({"routes": routes}))
}

/// SIGTERM/SIGINT drain: stop accept, finish in flight.
async fn shutdown() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("SIGTERM hook");
    let mut int = signal(SignalKind::interrupt()).expect("SIGINT hook");
    tokio::select! {
        _ = term.recv() => info!("SIGTERM: drain"),
        _ = int.recv() => info!("SIGINT: drain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_defaults_to_loopback_shape() {
        let addr = super::parse_bind("127.0.0.1", 8187).unwrap();
        assert!(addr.ip().is_loopback());
        assert_eq!(addr.port(), 8187);
        assert!(super::parse_bind("zzz", 8080).is_err());
    }

    #[test]
    fn token_gate_accepts_only_bearer_match() {
        let mut good = HeaderMap::new();
        good.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer abc123".parse().unwrap(),
        );
        assert!(super::authorized(&good, "abc123"));
        let mut bad = HeaderMap::new();
        bad.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer nope".parse().unwrap(),
        );
        assert!(!super::authorized(&bad, "abc123"));
        assert!(!super::authorized(&HeaderMap::new(), "abc123"));
    }

    #[test]
    fn manifest_is_sorted_and_has_hello() {
        let mut routes = ROUTES.to_vec();
        routes.sort_unstable();
        assert_eq!(routes, ROUTES);
        assert!(ROUTES.contains(&"/v1/hello"));
    }

    /// Lazy app state. It never connects. Router tests use it.
    fn test_state() -> AppState {
        AppState {
            pool: PgPool::connect_lazy("postgres://127.0.0.1:1/none").unwrap(),
            redis: redis::Client::open("redis://127.0.0.1:1/").unwrap(),
        }
    }

    /// Send one request at the test router. No modules mount.
    /// /v1/routes answers without a database.
    async fn call(
        method: &str,
        path: &str,
        auth: bool,
        origin: Option<&str>,
    ) -> (StatusCode, HeaderMap) {
        use axum::body::Body;
        use tower::ServiceExt;
        let mut builder = axum::http::Request::builder().method(method).uri(path);
        if auth {
            builder = builder.header(axum::http::header::AUTHORIZATION, "Bearer test-key");
        }
        if let Some(origin) = origin {
            builder = builder.header(axum::http::header::ORIGIN, origin);
        }
        let req = builder.body(Body::empty()).unwrap();
        let app = super::build_router(
            test_state(),
            &[],
            "test-key".into(),
            super::cors::default_origins(),
        );
        let resp = app.oneshot(req).await.unwrap();
        (resp.status(), resp.headers().clone())
    }

    #[tokio::test]
    async fn preflight_answers_204_without_key() {
        let (status, headers) = call(
            "OPTIONS",
            "/v1/routes",
            false,
            Some("http://tauri.localhost"),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            headers[axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "http://tauri.localhost"
        );
        assert_eq!(
            headers[axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS],
            "authorization, content-type"
        );
        assert_eq!(
            headers[axum::http::header::ACCESS_CONTROL_ALLOW_METHODS],
            "GET, POST, OPTIONS"
        );
        assert_eq!(headers[axum::http::header::VARY], "Origin");
    }

    #[tokio::test]
    async fn allowed_origin_is_echoed_on_authed_get() {
        let (status, headers) =
            call("GET", "/v1/routes", true, Some("http://tauri.localhost")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers[axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "http://tauri.localhost"
        );
    }

    #[tokio::test]
    async fn unknown_origin_gets_no_allow_header() {
        let (status, headers) = call("GET", "/v1/routes", true, Some("https://evil.example")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN));
        assert_eq!(headers[axum::http::header::VARY], "Origin");
    }

    #[tokio::test]
    async fn get_without_key_is_401_with_cors_headers() {
        let (status, headers) =
            call("GET", "/v1/routes", false, Some("http://tauri.localhost")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            headers[axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "http://tauri.localhost"
        );
    }
}

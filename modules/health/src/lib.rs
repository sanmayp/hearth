//! Health module. MVP: 10-second check-in.
//! Safety: local crisis word check runs FIRST,
//! before any store or model call. Notes seal
//! with AES-GCM. Key comes from `HEALTH_KEY`.

use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, Key, KeyInit, Nonce,
};
use async_trait::async_trait;
use axum::{extract::Query, http::StatusCode, routing::get, Extension, Json, Router};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{DateTime, NaiveDate, Utc};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The health module.
pub struct HealthMod;

/// Crisis line (US call/text).
pub const CRISIS_LINE: &str = "988";
/// Owner-chosen crisis contact (owner decision 2026-10-31).
pub const CRISIS_CONTACT_NAME: &str = "Your emergency contact";
/// Owner-chosen crisis contact phone.
pub const CRISIS_CONTACT_PHONE: &str = "555-010-0000";

/// Local crisis words. Plain list, checked first.
/// Works with the model off or dead.
const CRISIS_WORDS: &[&str] = &[
    "kill myself",
    "killing myself",
    "suicide",
    "suicidal",
    "self-harm",
    "self harm",
    "hurt myself",
    "harm myself",
    "end my life",
    "end it all",
    "want to die",
    "wish i were dead",
    "no reason to live",
];

/// True when text holds crisis words (case free).
pub fn crisis_hit(text: &str) -> bool {
    let low = text.to_lowercase();
    CRISIS_WORDS.iter().any(|w| low.contains(w))
}

/// One check-in payload (sealed at rest).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckinPayload {
    /// Mood 1-5.
    pub mood: u8,
    /// Energy 1-5.
    pub energy: u8,
    /// Free note (private).
    pub note: Option<String>,
}

/// Input for a check-in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCheckin {
    /// Mood 1-5.
    pub mood: u8,
    /// Energy 1-5.
    pub energy: u8,
    /// Free note (private).
    pub note: Option<String>,
}

/// One stored check-in (opened).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkin {
    /// Stable id (uuid).
    pub id: String,
    /// Day (UTC).
    pub day: NaiveDate,
    /// Payload.
    pub payload: CheckinPayload,
    /// Store time.
    pub at: DateTime<Utc>,
}

/// List filter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListQ {
    /// Past days (default 7, max 90).
    pub days: Option<u32>,
}

/// Seal bytes with AES-256-GCM. Output is
/// base64(nonce 12B plus sealed bytes).
pub fn seal(plain: &[u8], key: &[u8; 32]) -> String {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let mut out = nonce.to_vec();
    if let Ok(mut ct) = cipher.encrypt(&nonce, plain) {
        out.append(&mut ct);
    }
    B64.encode(out)
}

/// Open sealed base64. None on any failure.
pub fn open(sealed: &str, key: &[u8; 32]) -> Option<Vec<u8>> {
    let raw = B64.decode(sealed).ok()?;
    if raw.len() < 13 {
        return None;
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let (n, ct) = raw.split_at(12);
    cipher.decrypt(Nonce::from_slice(n), ct).ok()
}

/// Read the 32-byte key from `HEALTH_KEY` (base64).
pub fn key_from_env() -> Option<[u8; 32]> {
    let raw = std::env::var("HEALTH_KEY").ok()?;
    let bytes = B64.decode(raw).ok()?;
    bytes.try_into().ok()
}

/// Check-in storage. PG in prod, memory in tests.
#[async_trait]
pub trait HealthStore: Send + Sync {
    /// Add one sealed check-in.
    async fn add(&self, payload: CheckinPayload) -> Checkin;
    /// List past days, newest first.
    async fn list(&self, days: u32) -> Vec<Checkin>;
    /// True when now holds a check-in.
    async fn done_today(&self) -> bool;
}

/// Postgres store. Payloads stay sealed in the table.
pub struct PgHealthStore {
    /// Pool to the system of record.
    pub pool: PgPool,
    /// Seal key (never logged).
    pub key: [u8; 32],
}

#[async_trait]
impl HealthStore for PgHealthStore {
    async fn add(&self, payload: CheckinPayload) -> Checkin {
        let day = Utc::now().date_naive();
        let plain = serde_json::to_vec(&payload).unwrap_or_default();
        let sealed = seal(&plain, &self.key);
        let id = uuid::Uuid::new_v4().to_string();
        let at = Utc::now();
        let _ = sqlx::query(
            "INSERT INTO health_checkin (id, day, payload, at) VALUES ($1, $2, $3, $4)",
        )
        .bind(&id)
        .bind(day)
        .bind(&sealed)
        .bind(at)
        .execute(&self.pool)
        .await;
        Checkin {
            id,
            day,
            payload,
            at,
        }
    }

    async fn list(&self, days: u32) -> Vec<Checkin> {
        let since = Utc::now().date_naive() - chrono::Duration::days(days as i64);
        let rows: Vec<(String, NaiveDate, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, day, payload, at FROM health_checkin WHERE day >= $1 ORDER BY at DESC",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        rows.into_iter()
            .filter_map(|(id, day, sealed, at)| {
                let plain = open(&sealed, &self.key)?;
                let payload: CheckinPayload = serde_json::from_slice(&plain).ok()?;
                Some(Checkin {
                    id,
                    day,
                    payload,
                    at,
                })
            })
            .collect()
    }

    async fn done_today(&self) -> bool {
        let day = Utc::now().date_naive();
        sqlx::query_as("SELECT id FROM health_checkin WHERE day = $1 LIMIT 1")
            .bind(day)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .map(|(_id,): (String,)| true)
            .unwrap_or(false)
    }
}

/// One sealed row: day, sealed payload, store time.
type SealedRow = (NaiveDate, String, DateTime<Utc>);

/// In-memory store for tests (seals too, fixed key).
pub struct MemHealthStore {
    inner: Mutex<HashMap<String, SealedRow>>,
    key: [u8; 32],
}

impl MemHealthStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            key: [7u8; 32],
        }
    }
}

impl Default for MemHealthStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HealthStore for MemHealthStore {
    async fn add(&self, payload: CheckinPayload) -> Checkin {
        let day = Utc::now().date_naive();
        let plain = serde_json::to_vec(&payload).unwrap_or_default();
        let sealed = seal(&plain, &self.key);
        let c = Checkin {
            id: uuid::Uuid::new_v4().to_string(),
            day,
            payload,
            at: Utc::now(),
        };
        if let Ok(mut m) = self.inner.lock() {
            m.insert(c.id.clone(), (day, sealed, c.at));
        }
        c
    }

    async fn list(&self, days: u32) -> Vec<Checkin> {
        let since = Utc::now().date_naive() - chrono::Duration::days(days as i64);
        let mut v: Vec<Checkin> = self
            .inner
            .lock()
            .map(|m| {
                m.iter()
                    .filter(|(_, (day, _, _))| *day >= since)
                    .filter_map(|(id, (day, sealed, at))| {
                        let plain = open(sealed, &self.key)?;
                        let payload = serde_json::from_slice(&plain).ok()?;
                        Some(Checkin {
                            id: id.clone(),
                            day: *day,
                            payload,
                            at: *at,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by_key(|c| std::cmp::Reverse(c.at));
        v
    }

    async fn done_today(&self) -> bool {
        let now = Utc::now().date_naive();
        self.inner
            .lock()
            .map(|m| m.values().any(|(day, _, _)| *day == now))
            .unwrap_or(false)
    }
}

/// Crisis help block for clients.
fn help_block() -> serde_json::Value {
    json!({
        "line": CRISIS_LINE,
        "contact_name": CRISIS_CONTACT_NAME,
        "contact_phone": CRISIS_CONTACT_PHONE,
        "note": "Stop here. Call or text now. You matter."
    })
}

/// POST /v1/health/checkin. Validates, crisis-checks
/// the note FIRST, then seals and stores.
async fn post_checkin<S: HealthStore>(
    Extension(store): Extension<Arc<S>>,
    Json(n): Json<NewCheckin>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !(1..=5).contains(&n.mood) || !(1..=5).contains(&n.energy) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let crisis = n.note.as_deref().map(crisis_hit).unwrap_or(false);
    let payload = CheckinPayload {
        mood: n.mood,
        energy: n.energy,
        note: n.note,
    };
    let saved = store.add(payload).await;
    if crisis {
        Ok(Json(
            json!({"ok": true, "crisis": true, "help": help_block(), "id": saved.id}),
        ))
    } else {
        Ok(Json(json!({"ok": true, "crisis": false, "id": saved.id})))
    }
}

/// GET /v1/health/checkins[?days=]. Opens past check-ins.
async fn list_checkins<S: HealthStore>(
    Extension(store): Extension<Arc<S>>,
    Query(q): Query<ListQ>,
) -> Json<Vec<Checkin>> {
    let days = q.days.unwrap_or(7).min(90);
    Json(store.list(days).await)
}

/// GET /v1/health/status. Feeds the Today screen.
async fn health_status<S: HealthStore>(
    Extension(store): Extension<Arc<S>>,
) -> Json<serde_json::Value> {
    Json(json!({"checkin_done_today": store.done_today().await}))
}

#[async_trait]
impl Module for HealthMod {
    fn id(&self) -> &'static str {
        "health"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_health_checkin",
            sql: "CREATE TABLE IF NOT EXISTS health_checkin
                  (id TEXT PRIMARY KEY, day DATE NOT NULL, payload TEXT NOT NULL,
                   at TIMESTAMPTZ NOT NULL)",
        }]
    }

    fn routes(&self, _state: &AppState) -> Router<()> {
        // Key is a must. No key means the module stays dark.
        let key = key_from_env().unwrap_or([0u8; 32]);
        let store = Arc::new(PgHealthStore {
            pool: _state.pool.clone(),
            key,
        });
        Router::new()
            .route(
                "/v1/health/checkin",
                axum::routing::post(post_checkin::<PgHealthStore>),
            )
            .route("/v1/health/checkins", get(list_checkins::<PgHealthStore>))
            .route("/v1/health/status", get(health_status::<PgHealthStore>))
            .layer(Extension(store))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![JobDef {
            name: "health.weekly",
            cron: "0 6 * * MON",
            about: "Build what helped, what hurt trends.",
        }]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![ToolDef {
            name: "health.checkin",
            about: "Save a mood and energy check-in.",
            schema: json!({
                "type": "object",
                "properties": {
                    "mood": {"type": "integer"},
                    "energy": {"type": "integer"},
                    "note": {"type": "string"}
                },
                "required": ["mood", "energy"]
            }),
        }]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "health.card",
            title: "Health",
            module: "health",
        }]
    }

    async fn health(&self, state: &AppState) -> Health {
        if key_from_env().is_none() {
            return Health::down("HEALTH_KEY not set");
        }
        match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
            Ok(_) => Health::ok("health answers"),
            Err(e) => Health::down(format!("db down: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crisis_words_hit() {
        assert!(crisis_hit("I want to kill myself"));
        assert!(crisis_hit("SUICIDE crossed my mind"));
        assert!(crisis_hit("I want to DIE"));
        assert!(!crisis_hit("I feel low but safe"));
        assert!(!crisis_hit(""));
    }

    #[test]
    fn seal_roundtrip() {
        let key = [3u8; 32];
        let out = seal(b"mood=2", &key);
        assert_eq!(open(&out, &key).unwrap(), b"mood=2");
        assert!(open(&out, &[4u8; 32]).is_none());
        assert!(open("zzz", &key).is_none());
    }

    #[tokio::test]
    async fn mem_checkin_flows() {
        let s = MemHealthStore::new();
        assert!(!s.done_today().await);
        s.add(CheckinPayload {
            mood: 3,
            energy: 2,
            note: Some("tired".into()),
        })
        .await;
        assert!(s.done_today().await);
        let list = s.list(7).await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].payload.mood, 3);
    }

    #[tokio::test]
    async fn handler_flags_crisis_and_bad_range() {
        let store = Arc::new(MemHealthStore::new());
        let bad = NewCheckin {
            mood: 9,
            energy: 1,
            note: None,
        };
        let out = post_checkin::<MemHealthStore>(Extension(store.clone()), Json(bad)).await;
        assert!(out.is_err());
        let cry = NewCheckin {
            mood: 1,
            energy: 1,
            note: Some("I want to die".into()),
        };
        let Json(v) = post_checkin::<MemHealthStore>(Extension(store), Json(cry))
            .await
            .unwrap();
        assert_eq!(v["crisis"], json!(true));
        assert_eq!(v["help"]["line"], json!("988"));
    }
}

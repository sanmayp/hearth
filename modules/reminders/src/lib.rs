//! Reminders module. One-shot and recurring reminders,
//! snooze, due list. Push and calendar sync follow
//! (they need live servers).

use async_trait::async_trait;
use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The reminders module.
pub struct Reminders;

/// One reminder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reminder {
    /// Stable id (uuid).
    pub id: String,
    /// Short title.
    pub title: String,
    /// First fire time, if timed.
    pub at: Option<DateTime<Utc>>,
    /// Recurrence rule, e.g. "RRULE:FREQ=DAILY".
    pub rrule: Option<String>,
    /// True when done (one-shot) or skipped (occasion).
    pub done: bool,
    /// Hidden until this time (snooze).
    pub snoozed_until: Option<DateTime<Utc>>,
}

/// Input for a new reminder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReminder {
    /// Short title.
    pub title: String,
    /// First fire time, if timed.
    pub at: Option<DateTime<Utc>>,
    /// Recurrence rule, if repeat.
    pub rrule: Option<String>,
}

/// Snooze input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnoozeIn {
    /// Hide until this time.
    pub until: DateTime<Utc>,
}

/// List filter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListQ {
    /// When true, keep due items only.
    pub due_only: Option<bool>,
}

/// Next fire time after `now`. MVP rule subset:
/// FREQ=DAILY or WEEKLY, plus INTERVAL and UNTIL.
/// Unknown rules fall back to the plain time.
pub fn next_due(rem: &Reminder, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if rem.done {
        return None;
    }
    if let Some(until) = rem.snoozed_until {
        if until > now {
            return Some(until);
        }
    }
    let start = rem.at?;
    let stored = rem.rrule.as_deref().unwrap_or("");
    let rule = stored.strip_prefix("RRULE:").unwrap_or(stored);
    if rule.is_empty() {
        return (start > now).then_some(start);
    }
    let freq = rule_value(rule, "FREQ")?;
    let step_days: i64 = match freq.as_str() {
        "DAILY" => rule_value(rule, "INTERVAL")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        "WEEKLY" => {
            rule_value(rule, "INTERVAL")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1)
                * 7
        }
        _ => return (start > now).then_some(start),
    };
    let until = rule_value(rule, "UNTIL").and_then(|v| parse_until(&v));
    let mut next = start;
    let mut guard = 0;
    while next <= now && guard < 4000 {
        next += Duration::days(step_days);
        guard += 1;
    }
    if next <= now {
        return None;
    }
    if let Some(u) = until {
        if next > u {
            return None;
        }
    }
    Some(next)
}

/// Read one RRULE value by key.
fn rule_value(rule: &str, key: &str) -> Option<String> {
    rule.split(';').find_map(|part| {
        let (k, v) = part.split_once('=')?;
        (k.trim() == key).then(|| v.trim().to_string())
    })
}

/// Parse UNTIL in RFC3339 or basic date form.
fn parse_until(v: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(v) {
        return Some(dt.with_timezone(&Utc));
    }
    chrono::NaiveDate::parse_from_str(v, "%Y%m%d")
        .ok()
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|n| n.and_utc())
}

/// True when the reminder needs the owner now.
pub fn is_due(rem: &Reminder, now: DateTime<Utc>) -> bool {
    if rem.done {
        return false;
    }
    if let Some(until) = rem.snoozed_until {
        if until > now {
            return false;
        }
    }
    match rem.at {
        None => true,
        Some(at) => at <= now,
    }
}

/// Reminder storage. PG in prod, memory in tests.
#[async_trait]
pub trait ReminderStore: Send + Sync {
    /// List all, oldest first.
    async fn list(&self) -> Vec<Reminder>;
    /// Add one. Assigns the id.
    async fn add(&self, n: NewReminder) -> Reminder;
    /// Mark done. False when missing.
    async fn complete(&self, id: &str) -> bool;
    /// Snooze until a time. False when missing.
    async fn snooze(&self, id: &str, until: DateTime<Utc>) -> bool;
}

/// Postgres store.
pub struct PgReminderStore {
    /// Pool to the system of record.
    pub pool: PgPool,
}

#[async_trait]
impl ReminderStore for PgReminderStore {
    async fn list(&self) -> Vec<Reminder> {
        sqlx::query_as(
            "SELECT id, title, at, rrule, done, snoozed_until FROM reminders ORDER BY at NULLS LAST, id",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(
            |(id, title, at, rrule, done, snoozed_until): (
                String,
                String,
                Option<DateTime<Utc>>,
                Option<String>,
                bool,
                Option<DateTime<Utc>>,
            )| Reminder { id, title, at, rrule, done, snoozed_until },
        )
        .collect()
    }

    async fn add(&self, n: NewReminder) -> Reminder {
        let rem = Reminder {
            id: uuid::Uuid::new_v4().to_string(),
            title: n.title,
            at: n.at,
            rrule: n.rrule,
            done: false,
            snoozed_until: None,
        };
        let _ = sqlx::query(
            "INSERT INTO reminders (id, title, at, rrule, done) VALUES ($1, $2, $3, $4, FALSE)",
        )
        .bind(&rem.id)
        .bind(&rem.title)
        .bind(rem.at)
        .bind(&rem.rrule)
        .execute(&self.pool)
        .await;
        rem
    }

    async fn complete(&self, id: &str) -> bool {
        sqlx::query("UPDATE reminders SET done = TRUE WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }

    async fn snooze(&self, id: &str, until: DateTime<Utc>) -> bool {
        sqlx::query("UPDATE reminders SET snoozed_until = $1 WHERE id = $2")
            .bind(until)
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }
}

/// In-memory store for tests.
pub struct MemReminderStore {
    inner: Mutex<HashMap<String, Reminder>>,
}

impl MemReminderStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemReminderStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReminderStore for MemReminderStore {
    async fn list(&self) -> Vec<Reminder> {
        let mut v: Vec<Reminder> = self
            .inner
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        v.sort_by_key(|r| r.at.unwrap_or(DateTime::<Utc>::MAX_UTC));
        v
    }

    async fn add(&self, n: NewReminder) -> Reminder {
        let rem = Reminder {
            id: uuid::Uuid::new_v4().to_string(),
            title: n.title,
            at: n.at,
            rrule: n.rrule,
            done: false,
            snoozed_until: None,
        };
        if let Ok(mut m) = self.inner.lock() {
            m.insert(rem.id.clone(), rem.clone());
        }
        rem
    }

    async fn complete(&self, id: &str) -> bool {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(r) = m.get_mut(id) {
                r.done = true;
                return true;
            }
        }
        false
    }

    async fn snooze(&self, id: &str, until: DateTime<Utc>) -> bool {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(r) = m.get_mut(id) {
                r.snoozed_until = Some(until);
                return true;
            }
        }
        false
    }
}

/// GET /v1/reminders. Lists open items, due first with due_only.
async fn list_reminders<S: ReminderStore>(
    Extension(store): Extension<Arc<S>>,
    Query(q): Query<ListQ>,
) -> Json<Vec<Reminder>> {
    let now = Utc::now();
    let mut items: Vec<Reminder> = store.list().await.into_iter().filter(|r| !r.done).collect();
    if q.due_only.unwrap_or(false) {
        items.retain(|r| is_due(r, now));
    }
    Json(items)
}

/// POST /v1/reminders. Adds one.
async fn add_reminder<S: ReminderStore>(
    Extension(store): Extension<Arc<S>>,
    Json(n): Json<NewReminder>,
) -> Json<Reminder> {
    Json(store.add(n).await)
}

/// POST /v1/reminders/:id/done. Marks done.
async fn complete_reminder<S: ReminderStore>(
    Extension(store): Extension<Arc<S>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({"ok": store.complete(&id).await}))
}

/// POST /v1/reminders/:id/snooze. Hides until a time.
async fn snooze_reminder<S: ReminderStore>(
    Extension(store): Extension<Arc<S>>,
    Path(id): Path<String>,
    Json(body): Json<SnoozeIn>,
) -> Json<serde_json::Value> {
    Json(json!({"ok": store.snooze(&id, body.until).await}))
}

#[async_trait]
impl Module for Reminders {
    fn id(&self) -> &'static str {
        "reminders"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_reminders",
            sql: "CREATE TABLE IF NOT EXISTS reminders
                  (id TEXT PRIMARY KEY, title TEXT NOT NULL, at TIMESTAMPTZ,
                   rrule TEXT, done BOOLEAN NOT NULL DEFAULT FALSE,
                   snoozed_until TIMESTAMPTZ)",
        }]
    }

    fn routes(&self, state: &AppState) -> Router<()> {
        let store = Arc::new(PgReminderStore {
            pool: state.pool.clone(),
        });
        Router::new()
            .route(
                "/v1/reminders",
                get(list_reminders::<PgReminderStore>).post(add_reminder::<PgReminderStore>),
            )
            .route(
                "/v1/reminders/:id/done",
                axum::routing::post(complete_reminder::<PgReminderStore>),
            )
            .route(
                "/v1/reminders/:id/snooze",
                axum::routing::post(snooze_reminder::<PgReminderStore>),
            )
            .layer(Extension(store))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![
            JobDef {
                name: "reminders.dispatch",
                cron: "*/1 * * * *",
                about: "Push due reminders (ntfy plus Web Push).",
            },
            JobDef {
                name: "reminders.cal_sync",
                cron: "*/15 * * * *",
                about: "Sync Google and CalDAV calendars.",
            },
        ]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "reminders.add",
                about: "Add a reminder with time and repeat.",
                schema: json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "at": {"type": "string"},
                        "rrule": {"type": "string"}
                    },
                    "required": ["title"]
                }),
            },
            ToolDef {
                name: "reminders.due",
                about: "List reminders due now.",
                schema: json!({"type": "object", "properties": {}, "required": []}),
            },
        ]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "reminders.due",
            title: "Due",
            module: "reminders",
        }]
    }

    async fn health(&self, state: &AppState) -> Health {
        match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
            Ok(_) => Health::ok("reminders answer"),
            Err(e) => Health::down(format!("db down: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn rem(at: Option<DateTime<Utc>>, rrule: Option<&str>) -> Reminder {
        Reminder {
            id: "x".into(),
            title: "t".into(),
            at,
            rrule: rrule.map(str::to_string),
            done: false,
            snoozed_until: None,
        }
    }

    #[test]
    fn daily_rolls_forward() {
        let r = rem(Some(at("2026-10-30T09:00:00Z")), Some("RRULE:FREQ=DAILY"));
        let next = next_due(&r, at("2026-10-31T10:00:00Z")).unwrap();
        assert_eq!(next, at("2026-11-01T09:00:00Z"));
    }

    #[test]
    fn weekly_with_interval() {
        let r = rem(
            Some(at("2026-10-20T09:00:00Z")),
            Some("RRULE:FREQ=WEEKLY;INTERVAL=2"),
        );
        let next = next_due(&r, at("2026-10-31T10:00:00Z")).unwrap();
        assert_eq!(next, at("2026-11-03T09:00:00Z"));
    }

    #[test]
    fn until_stops() {
        let r = rem(
            Some(at("2026-10-30T09:00:00Z")),
            Some("RRULE:FREQ=DAILY;UNTIL=20261030"),
        );
        assert!(next_due(&r, at("2026-10-31T10:00:00Z")).is_none());
    }

    #[test]
    fn plain_past_has_no_next() {
        let r = rem(Some(at("2026-10-30T09:00:00Z")), None);
        assert!(next_due(&r, at("2026-10-31T10:00:00Z")).is_none());
    }

    #[test]
    fn snooze_hides_due() {
        let mut r = rem(Some(at("2026-10-30T09:00:00Z")), None);
        let now = at("2026-10-31T10:00:00Z");
        assert!(is_due(&r, now));
        r.snoozed_until = Some(at("2026-11-01T09:00:00Z"));
        assert!(!is_due(&r, now));
    }

    #[tokio::test]
    async fn mem_crud_flows() {
        let s = MemReminderStore::new();
        let a = s
            .add(NewReminder {
                title: "a".into(),
                at: None,
                rrule: None,
            })
            .await;
        assert_eq!(s.list().await.len(), 1);
        assert!(s.snooze(&a.id, at("2026-11-01T09:00:00Z")).await);
        assert!(s.complete(&a.id).await);
        assert!(!s.complete("missing").await);
    }
}

//! Today module. Home dashboard data: agenda, due items,
//! alerts, check-in state, and one next action.
//! Rule picks the action. No model call here.

use async_trait::async_trait;
use axum::{routing::get, Extension, Json, Router};
use chrono::{DateTime, Utc};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The today module.
pub struct Today;

/// One listed item (task, reminder, event, alert).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    /// Stable id from the source module.
    pub id: String,
    /// Short title.
    pub title: String,
    /// Due time when known.
    pub at: Option<DateTime<Utc>>,
}

/// Stored snapshot. Siblings feed it through events.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    /// Agenda items for now.
    pub agenda: Vec<Item>,
    /// Reminders due now.
    pub reminders_due: Vec<Item>,
    /// Tasks due now.
    pub tasks_due: Vec<Item>,
    /// Business alerts now.
    pub business_alerts: Vec<Item>,
    /// True after the health check-in.
    pub health_checkin_done: bool,
    /// Last feed time.
    pub updated_at: Option<DateTime<Utc>>,
}

/// View served to clients: snapshot plus one action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayView {
    /// Agenda items for now.
    pub agenda: Vec<Item>,
    /// Reminders due now.
    pub reminders_due: Vec<Item>,
    /// Tasks due now.
    pub tasks_due: Vec<Item>,
    /// Business alerts now.
    pub business_alerts: Vec<Item>,
    /// True after the health check-in.
    pub health_checkin_done: bool,
    /// The one suggested next action, if any.
    pub next_action: Option<Item>,
    /// Last feed time.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Pick one next action. Oldest due task wins,
/// then oldest due reminder, then first agenda item.
pub fn next_action(snap: &Snapshot) -> Option<Item> {
    oldest(&snap.tasks_due)
        .or_else(|| oldest(&snap.reminders_due))
        .or_else(|| snap.agenda.first().cloned())
}

/// Oldest item by `at`. Items without time sort last.
fn oldest(items: &[Item]) -> Option<Item> {
    items
        .iter()
        .min_by_key(|i| i.at.unwrap_or(DateTime::<Utc>::MAX_UTC))
        .cloned()
}

/// Snapshot storage. PG in prod, memory in tests.
#[async_trait]
pub trait TodayStore: Send + Sync {
    /// Read the snapshot. Empty when never fed.
    async fn get(&self) -> Snapshot;
    /// Feed the snapshot. Sets time to now.
    async fn put(&self, snap: Snapshot);
}

/// Postgres snapshot store (one row, id 1).
pub struct PgTodayStore {
    /// Pool to the system of record.
    pub pool: PgPool,
}

#[async_trait]
impl TodayStore for PgTodayStore {
    async fn get(&self) -> Snapshot {
        let row: Option<(serde_json::Value, DateTime<Utc>)> =
            sqlx::query_as("SELECT data, updated_at FROM today_snapshot WHERE id = 1")
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
        match row {
            Some((data, at)) => {
                let mut snap: Snapshot = serde_json::from_value(data).unwrap_or_default();
                snap.updated_at = Some(at);
                snap
            }
            None => Snapshot::default(),
        }
    }

    async fn put(&self, snap: Snapshot) {
        let data = serde_json::to_value(&snap).unwrap_or(json!({}));
        let _ = sqlx::query(
            "INSERT INTO today_snapshot (id, data, updated_at) VALUES (1, $1, now())
             ON CONFLICT (id) DO UPDATE SET data = $1, updated_at = now()",
        )
        .bind(data)
        .execute(&self.pool)
        .await;
    }
}

/// In-memory snapshot store for tests.
pub struct MemTodayStore {
    inner: Mutex<HashMap<String, Snapshot>>,
}

impl MemTodayStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemTodayStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TodayStore for MemTodayStore {
    async fn get(&self) -> Snapshot {
        self.inner
            .lock()
            .ok()
            .and_then(|m| m.get("snap").cloned())
            .unwrap_or_default()
    }

    async fn put(&self, mut snap: Snapshot) {
        snap.updated_at = Some(Utc::now());
        if let Ok(mut m) = self.inner.lock() {
            m.insert("snap".into(), snap);
        }
    }
}

/// GET /v1/today. Snapshot plus one next action.
async fn get_today<S: TodayStore>(Extension(store): Extension<Arc<S>>) -> Json<TodayView> {
    let snap = store.get().await;
    Json(TodayView {
        agenda: snap.agenda.clone(),
        reminders_due: snap.reminders_due.clone(),
        tasks_due: snap.tasks_due.clone(),
        business_alerts: snap.business_alerts.clone(),
        health_checkin_done: snap.health_checkin_done,
        next_action: next_action(&snap),
        updated_at: snap.updated_at,
    })
}

/// PUT /v1/today. Feed from the event worker (tests use it too).
async fn put_today<S: TodayStore>(
    Extension(store): Extension<Arc<S>>,
    Json(snap): Json<Snapshot>,
) -> Json<serde_json::Value> {
    store.put(snap).await;
    Json(json!({"ok": true}))
}

#[async_trait]
impl Module for Today {
    fn id(&self) -> &'static str {
        "today"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_today_snapshot",
            sql: "CREATE TABLE IF NOT EXISTS today_snapshot
                  (id INT PRIMARY KEY, data JSONB NOT NULL DEFAULT '{}',
                   updated_at TIMESTAMPTZ NOT NULL DEFAULT now())",
        }]
    }

    fn routes(&self, state: &AppState) -> Router<()> {
        let store = Arc::new(PgTodayStore {
            pool: state.pool.clone(),
        });
        Router::new()
            .route(
                "/v1/today",
                get(get_today::<PgTodayStore>).put(put_today::<PgTodayStore>),
            )
            .layer(Extension(store))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![JobDef {
            name: "today.rollup",
            cron: "0 5 * * *",
            about: "Roll up sibling feeds into the snapshot.",
        }]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![ToolDef {
            name: "today.brief",
            about: "Read the now snapshot plus next action.",
            schema: json!({"type": "object", "properties": {}, "required": []}),
        }]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "today.card",
            title: "Now",
            module: "today",
        }]
    }

    async fn health(&self, state: &AppState) -> Health {
        match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
            Ok(_) => Health::ok("today answers"),
            Err(e) => Health::down(format!("db down: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, at: Option<DateTime<Utc>>) -> Item {
        Item {
            id: id.into(),
            title: id.into(),
            at,
        }
    }

    #[test]
    fn task_beats_reminder_beats_agenda() {
        let snap = Snapshot {
            agenda: vec![item("a", None)],
            reminders_due: vec![item("r", None)],
            tasks_due: vec![item("t", None)],
            ..Default::default()
        };
        assert_eq!(next_action(&snap).unwrap().id, "t");
    }

    #[test]
    fn oldest_due_first() {
        let early = DateTime::parse_from_rfc3339("2026-10-30T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let late = DateTime::parse_from_rfc3339("2026-10-31T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let snap = Snapshot {
            tasks_due: vec![item("late", Some(late)), item("early", Some(early))],
            ..Default::default()
        };
        assert_eq!(next_action(&snap).unwrap().id, "early");
    }

    #[test]
    fn empty_means_no_action() {
        assert!(next_action(&Snapshot::default()).is_none());
    }

    #[tokio::test]
    async fn mem_store_roundtrip_sets_time() {
        let s = MemTodayStore::new();
        assert!(s.get().await.updated_at.is_none());
        s.put(Snapshot::default()).await;
        assert!(s.get().await.updated_at.is_some());
    }

    #[tokio::test]
    async fn handler_serves_view() {
        let store = Arc::new(MemTodayStore::new());
        store
            .put(Snapshot {
                tasks_due: vec![item("t1", None)],
                ..Default::default()
            })
            .await;
        let Json(view) = get_today::<MemTodayStore>(Extension(store)).await;
        assert_eq!(view.tasks_due.len(), 1);
        assert_eq!(view.next_action.unwrap().id, "t1");
    }
}

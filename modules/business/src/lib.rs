//! Business module. Read-only view of shop data.
//! Sources bring snapshots. The page shows them.
//! v1 has no write path. No POST, no PUT, no DELETE.

use async_trait::async_trait;
use axum::{routing::get, Extension, Json, Router};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

/// The business module.
pub struct Business;

/// One key number, e.g. sales today.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Kpi {
    /// Short name.
    pub name: String,
    /// Plain value text.
    pub value: String,
}

/// One bill from a vendor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Invoice {
    /// Stable id.
    pub id: String,
    /// Who sent the bill.
    pub vendor: String,
    /// Amount in cents.
    pub amount_cents: i64,
    /// Currency code, e.g. USD.
    pub currency: String,
    /// Due date text, if any.
    pub due: Option<String>,
    /// Status: open, paid, or late.
    pub status: String,
}

/// One vendor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vendor {
    /// Vendor name.
    pub name: String,
    /// Contact text, if any.
    pub contact: Option<String>,
}

/// One staff shift.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Shift {
    /// Staff name.
    pub staff: String,
    /// Role text, if any.
    pub role: Option<String>,
    /// Start time text.
    pub start: String,
    /// End time text.
    pub end: String,
}

/// One notice for the owner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Alert {
    /// Short title.
    pub title: String,
    /// More detail, if any.
    pub detail: Option<String>,
    /// Level: info or warn.
    pub level: String,
}

/// Top view: numbers, vendors, notices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Overview {
    /// Key numbers.
    pub kpis: Vec<Kpi>,
    /// Known vendors.
    pub vendors: Vec<Vendor>,
    /// Open notices.
    pub alerts: Vec<Alert>,
    /// Last fetch time, if known.
    pub updated_at: Option<String>,
}

/// Full pull from one source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Snapshot {
    /// Key numbers.
    pub kpis: Vec<Kpi>,
    /// Open and past bills.
    pub invoices: Vec<Invoice>,
    /// Known vendors.
    pub vendors: Vec<Vendor>,
    /// Staff shifts.
    pub schedule: Vec<Shift>,
    /// Open notices.
    pub alerts: Vec<Alert>,
}

impl Snapshot {
    /// Merge another pull into this one.
    fn merge(&mut self, other: Snapshot) {
        self.kpis.extend(other.kpis);
        self.invoices.extend(other.invoices);
        self.vendors.extend(other.vendors);
        self.schedule.extend(other.schedule);
        self.alerts.extend(other.alerts);
    }
}

/// One read-only link to a shop system.
/// Each source pulls by API or DB view.
#[async_trait]
pub trait Source: Send + Sync {
    /// Stable id, e.g. "shop-pos".
    fn id(&self) -> &'static str;
    /// Pull one read-only snapshot.
    async fn fetch(&self) -> Snapshot;
}

/// Test source. Holds one fixed snapshot.
pub struct MemSource {
    /// Fixed id.
    pub id: &'static str,
    /// Fixed pull.
    pub data: Snapshot,
}

#[async_trait]
impl Source for MemSource {
    fn id(&self) -> &'static str {
        self.id
    }

    async fn fetch(&self) -> Snapshot {
        self.data.clone()
    }
}

/// Holds all sources. Merges pulls into one view.
pub struct SourceRegistry {
    items: Vec<Arc<dyn Source>>,
}

impl SourceRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    /// Add one source. Consumes self, so builds chain.
    pub fn with(mut self, s: Arc<dyn Source>) -> Self {
        self.items.push(s);
        self
    }

    /// Source ids in join order.
    pub fn ids(&self) -> Vec<&'static str> {
        self.items.iter().map(|s| s.id()).collect()
    }

    /// Pull each source once. Merge all pulls.
    pub async fn snapshot(&self) -> Snapshot {
        let mut out = Snapshot::default();
        for s in &self.items {
            out.merge(s.fetch().await);
        }
        out
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Business storage. PG in prod, memory in tests.
/// Read-only: no method writes shop data.
#[async_trait]
pub trait BusinessStore: Send + Sync {
    /// Top view: numbers, vendors, notices.
    async fn overview(&self) -> Overview;
    /// All known bills.
    async fn invoices(&self) -> Vec<Invoice>;
    /// All known shifts.
    async fn schedule(&self) -> Vec<Shift>;
}

/// Read one cache row. Empty vec when missing.
async fn read_key(pool: &PgPool, key: &str) -> (serde_json::Value, Option<String>) {
    let row: Option<(serde_json::Value, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT data, updated_at FROM business_cache WHERE key = $1")
            .bind(key)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);
    match row {
        Some((data, at)) => (data, Some(at.to_rfc3339())),
        None => (json!([]), None),
    }
}

/// Postgres store. Reads the cache table only.
pub struct PgBusinessStore {
    /// Pool to the system of record.
    pub pool: PgPool,
}

#[async_trait]
impl BusinessStore for PgBusinessStore {
    async fn overview(&self) -> Overview {
        let (data, at) = read_key(&self.pool, "overview").await;
        let mut o: Overview = serde_json::from_value(data).unwrap_or_default();
        o.updated_at = at;
        o
    }

    async fn invoices(&self) -> Vec<Invoice> {
        let (data, _) = read_key(&self.pool, "invoices").await;
        serde_json::from_value(data).unwrap_or_default()
    }

    async fn schedule(&self) -> Vec<Shift> {
        let (data, _) = read_key(&self.pool, "schedule").await;
        serde_json::from_value(data).unwrap_or_default()
    }
}

/// In-memory store for tests.
pub struct MemBusinessStore {
    inner: Mutex<Snapshot>,
}

impl MemBusinessStore {
    /// New store with one fixed snapshot.
    pub fn new(data: Snapshot) -> Self {
        Self {
            inner: Mutex::new(data),
        }
    }
}

impl Default for MemBusinessStore {
    fn default() -> Self {
        Self::new(Snapshot::default())
    }
}

#[async_trait]
impl BusinessStore for MemBusinessStore {
    async fn overview(&self) -> Overview {
        let s = self.inner.lock().map(|g| g.clone()).unwrap_or_default();
        Overview {
            kpis: s.kpis,
            vendors: s.vendors,
            alerts: s.alerts,
            updated_at: None,
        }
    }

    async fn invoices(&self) -> Vec<Invoice> {
        self.inner
            .lock()
            .map(|g| g.invoices.clone())
            .unwrap_or_default()
    }

    async fn schedule(&self) -> Vec<Shift> {
        self.inner
            .lock()
            .map(|g| g.schedule.clone())
            .unwrap_or_default()
    }
}

/// GET /v1/business/overview. Shows the top view.
async fn get_overview<S: BusinessStore>(Extension(store): Extension<Arc<S>>) -> Json<Overview> {
    Json(store.overview().await)
}

/// GET /v1/business/invoices. Lists bills.
async fn get_invoices<S: BusinessStore>(Extension(store): Extension<Arc<S>>) -> Json<Vec<Invoice>> {
    Json(store.invoices().await)
}

/// GET /v1/business/schedule. Lists shifts.
async fn get_schedule<S: BusinessStore>(Extension(store): Extension<Arc<S>>) -> Json<Vec<Shift>> {
    Json(store.schedule().await)
}

#[async_trait]
impl Module for Business {
    fn id(&self) -> &'static str {
        "business"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_business_cache",
            sql: "CREATE TABLE IF NOT EXISTS business_cache
                  (key TEXT PRIMARY KEY, data JSONB NOT NULL,
                   updated_at TIMESTAMPTZ NOT NULL DEFAULT now())",
        }]
    }

    fn routes(&self, state: &AppState) -> Router<()> {
        let store = Arc::new(PgBusinessStore {
            pool: state.pool.clone(),
        });
        Router::new()
            .route(
                "/v1/business/overview",
                get(get_overview::<PgBusinessStore>),
            )
            .route(
                "/v1/business/invoices",
                get(get_invoices::<PgBusinessStore>),
            )
            .route(
                "/v1/business/schedule",
                get(get_schedule::<PgBusinessStore>),
            )
            .layer(Extension(store))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![JobDef {
            name: "business.daily_brief",
            cron: "0 6 * * *",
            about: "Build the daily business brief.",
        }]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "business.overview",
                about: "Read the business top view.",
                schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            ToolDef {
                name: "business.invoices",
                about: "Read bills, with status filter.",
                schema: json!({
                    "type": "object",
                    "properties": {
                        "status": {"type": "string"}
                    },
                    "required": []
                }),
            },
            ToolDef {
                name: "business.schedule",
                about: "Read staff shifts.",
                schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        ]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "business.today",
            title: "Business",
            module: "business",
        }]
    }

    async fn health(&self, state: &AppState) -> Health {
        match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
            Ok(_) => Health::ok("business answers"),
            Err(e) => Health::down(format!("db down: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Snapshot {
        Snapshot {
            kpis: vec![Kpi {
                name: "Sales today".into(),
                value: "$1,240".into(),
            }],
            invoices: vec![Invoice {
                id: "inv-1".into(),
                vendor: "Bean Co".into(),
                amount_cents: 45000,
                currency: "USD".into(),
                due: Some("2026-11-05".into()),
                status: "open".into(),
            }],
            vendors: vec![Vendor {
                name: "Bean Co".into(),
                contact: None,
            }],
            schedule: vec![Shift {
                staff: "Sam".into(),
                role: Some("Barista".into()),
                start: "08:00".into(),
                end: "14:00".into(),
            }],
            alerts: vec![Alert {
                title: "Low milk".into(),
                detail: None,
                level: "warn".into(),
            }],
        }
    }

    #[tokio::test]
    async fn mem_store_serves_snapshot() {
        let s = MemBusinessStore::new(sample());
        let o = s.overview().await;
        assert_eq!(o.kpis.len(), 1);
        assert_eq!(o.vendors.len(), 1);
        assert_eq!(o.alerts.len(), 1);
        assert_eq!(s.invoices().await.len(), 1);
        assert_eq!(s.schedule().await.len(), 1);
    }

    #[tokio::test]
    async fn mem_store_empty_by_default() {
        let s = MemBusinessStore::default();
        assert!(s.overview().await.kpis.is_empty());
        assert!(s.invoices().await.is_empty());
        assert!(s.schedule().await.is_empty());
    }

    #[tokio::test]
    async fn registry_merges_sources() {
        let a = Arc::new(MemSource {
            id: "a",
            data: sample(),
        });
        let b = Arc::new(MemSource {
            id: "b",
            data: sample(),
        });
        let reg = SourceRegistry::new().with(a).with(b);
        assert_eq!(reg.ids(), vec!["a", "b"]);
        let snap = reg.snapshot().await;
        assert_eq!(snap.kpis.len(), 2);
        assert_eq!(snap.invoices.len(), 2);
        assert_eq!(snap.schedule.len(), 2);
    }

    #[tokio::test]
    async fn registry_empty_gives_blank() {
        let reg = SourceRegistry::new();
        let snap = reg.snapshot().await;
        assert!(snap.kpis.is_empty());
        assert!(reg.ids().is_empty());
    }

    #[test]
    fn module_facts() {
        let m = Business;
        assert_eq!(m.id(), "business");
        assert_eq!(m.migrations().len(), 1);
        assert_eq!(m.jobs().len(), 1);
        assert_eq!(m.jobs()[0].name, "business.daily_brief");
        assert_eq!(m.tools().len(), 3);
        assert_eq!(m.widgets().len(), 1);
    }
}

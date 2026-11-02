//! Inventory module. MVP scope: manual entry plus scan.
//! Rooms seed from `lidar_inventory` when it lands.
//! Receipts, reorder, and photo upload follow.

use async_trait::async_trait;
use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The inventory module.
pub struct Inventory;

/// One owned item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    /// Stable id (uuid).
    pub id: String,
    /// Plain name, e.g. "passport".
    pub name: String,
    /// Room name, e.g. "study".
    pub room: String,
    /// Barcode or QR text, if scanned.
    pub barcode: Option<String>,
    /// Free note.
    pub note: Option<String>,
}

/// Input for a new item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewItem {
    /// Plain name.
    pub name: String,
    /// Room name.
    pub room: String,
    /// Barcode or QR text, if scanned.
    pub barcode: Option<String>,
    /// Free note.
    pub note: Option<String>,
}

/// Search filter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchQ {
    /// Plain words, e.g. "passport".
    pub q: Option<String>,
}

/// Item storage. PG in prod, memory in tests.
#[async_trait]
pub trait InventoryStore: Send + Sync {
    /// List all items.
    async fn list(&self) -> Vec<Item>;
    /// Add one. Assigns the id.
    async fn add(&self, n: NewItem) -> Item;
    /// Find by barcode. None when missing.
    async fn by_barcode(&self, code: &str) -> Option<Item>;
    /// Search names. Empty query lists all.
    async fn search(&self, q: &str) -> Vec<Item>;
    /// List rooms.
    async fn rooms(&self) -> Vec<String>;
    /// Add a room. False when it exists.
    async fn add_room(&self, room: &str) -> bool;
}

/// True when the item name holds all query words.
fn matches(item: &Item, q: &str) -> bool {
    let name = item.name.to_lowercase();
    q.split_whitespace()
        .all(|w| name.contains(&w.to_lowercase()))
}

/// Postgres store.
pub struct PgInventoryStore {
    /// Pool to the system of record.
    pub pool: PgPool,
}

#[async_trait]
impl InventoryStore for PgInventoryStore {
    async fn list(&self) -> Vec<Item> {
        sqlx::query_as("SELECT id, name, room, barcode, note FROM items ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(
                |(id, name, room, barcode, note): (
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                )| Item {
                    id,
                    name,
                    room,
                    barcode,
                    note,
                },
            )
            .collect()
    }

    async fn add(&self, n: NewItem) -> Item {
        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: n.name,
            room: n.room,
            barcode: n.barcode,
            note: n.note,
        };
        let _ = sqlx::query(
            "INSERT INTO items (id, name, room, barcode, note) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&item.id)
        .bind(&item.name)
        .bind(&item.room)
        .bind(&item.barcode)
        .bind(&item.note)
        .execute(&self.pool)
        .await;
        let _ = sqlx::query("INSERT INTO rooms (name) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(&item.room)
            .execute(&self.pool)
            .await;
        item
    }

    async fn by_barcode(&self, code: &str) -> Option<Item> {
        sqlx::query_as("SELECT id, name, room, barcode, note FROM items WHERE barcode = $1")
            .bind(code)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None)
            .map(
                |(id, name, room, barcode, note): (
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                )| Item {
                    id,
                    name,
                    room,
                    barcode,
                    note,
                },
            )
    }

    async fn search(&self, q: &str) -> Vec<Item> {
        let words: Vec<String> = q.split_whitespace().map(|w| format!("%{w}%")).collect();
        if words.is_empty() {
            return self.list().await;
        }
        let mut sql = "SELECT id, name, room, barcode, note FROM items WHERE ".to_string();
        sql.push_str(
            &(1..=words.len())
                .map(|i| format!("name ILIKE ${i}"))
                .collect::<Vec<_>>()
                .join(" AND "),
        );
        sql.push_str(" ORDER BY name");
        let mut query = sqlx::query_as(&sql);
        for w in &words {
            query = query.bind(w);
        }
        query
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(
                |(id, name, room, barcode, note): (
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                )| Item {
                    id,
                    name,
                    room,
                    barcode,
                    note,
                },
            )
            .collect()
    }

    async fn rooms(&self) -> Vec<String> {
        sqlx::query_as("SELECT name FROM rooms ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(n,): (String,)| n)
            .collect()
    }

    async fn add_room(&self, room: &str) -> bool {
        sqlx::query("INSERT INTO rooms (name) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(room)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }
}

/// In-memory store for tests.
pub struct MemInventoryStore {
    items: Mutex<HashMap<String, Item>>,
    rooms: Mutex<Vec<String>>,
}

impl MemInventoryStore {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(HashMap::new()),
            rooms: Mutex::new(vec![]),
        }
    }
}

impl Default for MemInventoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InventoryStore for MemInventoryStore {
    async fn list(&self) -> Vec<Item> {
        let mut v: Vec<Item> = self
            .items
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        v.sort_by_key(|i| i.name.clone());
        v
    }

    async fn add(&self, n: NewItem) -> Item {
        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: n.name.clone(),
            room: n.room.clone(),
            barcode: n.barcode,
            note: n.note,
        };
        if let Ok(mut m) = self.items.lock() {
            m.insert(item.id.clone(), item.clone());
        }
        self.add_room(&n.room).await;
        item
    }

    async fn by_barcode(&self, code: &str) -> Option<Item> {
        self.items.lock().ok().and_then(|m| {
            m.values()
                .find(|i| i.barcode.as_deref() == Some(code))
                .cloned()
        })
    }

    async fn search(&self, q: &str) -> Vec<Item> {
        if q.trim().is_empty() {
            return self.list().await;
        }
        let mut v: Vec<Item> = self
            .items
            .lock()
            .map(|m| m.values().filter(|i| matches(i, q)).cloned().collect())
            .unwrap_or_default();
        v.sort_by_key(|i| i.name.clone());
        v
    }

    async fn rooms(&self) -> Vec<String> {
        let mut v = self.rooms.lock().map(|r| r.clone()).unwrap_or_default();
        v.sort();
        v
    }

    async fn add_room(&self, room: &str) -> bool {
        if let Ok(mut r) = self.rooms.lock() {
            if r.iter().any(|x| x == room) {
                return false;
            }
            r.push(room.to_string());
            return true;
        }
        false
    }
}

/// GET /v1/inventory[?q=]. Lists or searches items.
async fn list_items<S: InventoryStore>(
    Extension(store): Extension<Arc<S>>,
    Query(q): Query<SearchQ>,
) -> Json<Vec<Item>> {
    match q.q {
        Some(q) => Json(store.search(&q).await),
        None => Json(store.list().await),
    }
}

/// POST /v1/inventory. Adds one item.
async fn add_item<S: InventoryStore>(
    Extension(store): Extension<Arc<S>>,
    Json(n): Json<NewItem>,
) -> Json<Item> {
    Json(store.add(n).await)
}

/// GET /v1/inventory/by-barcode/:code. Finds a scan.
async fn find_barcode<S: InventoryStore>(
    Extension(store): Extension<Arc<S>>,
    Path(code): Path<String>,
) -> Json<serde_json::Value> {
    match store.by_barcode(&code).await {
        Some(item) => Json(json!({"found": true, "item": item})),
        None => Json(json!({"found": false})),
    }
}

/// GET /v1/inventory/rooms. Lists rooms.
async fn list_rooms<S: InventoryStore>(Extension(store): Extension<Arc<S>>) -> Json<Vec<String>> {
    Json(store.rooms().await)
}

#[async_trait]
impl Module for Inventory {
    fn id(&self) -> &'static str {
        "inventory"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_inventory",
            sql: "CREATE TABLE IF NOT EXISTS rooms (name TEXT PRIMARY KEY);
                  CREATE TABLE IF NOT EXISTS items
                  (id TEXT PRIMARY KEY, name TEXT NOT NULL, room TEXT NOT NULL,
                   barcode TEXT, note TEXT)",
        }]
    }

    fn routes(&self, state: &AppState) -> Router<()> {
        let store = Arc::new(PgInventoryStore {
            pool: state.pool.clone(),
        });
        Router::new()
            .route(
                "/v1/inventory",
                get(list_items::<PgInventoryStore>).post(add_item::<PgInventoryStore>),
            )
            .route(
                "/v1/inventory/by-barcode/:code",
                get(find_barcode::<PgInventoryStore>),
            )
            .route("/v1/inventory/rooms", get(list_rooms::<PgInventoryStore>))
            .layer(Extension(store))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![JobDef {
            name: "inventory.receipts",
            cron: "*/30 * * * *",
            about: "Link emailed PDF receipts to items.",
        }]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "inventory.where",
                about: "Find where an item lives.",
                schema: json!({
                    "type": "object",
                    "properties": {"q": {"type": "string"}},
                    "required": ["q"]
                }),
            },
            ToolDef {
                name: "inventory.add",
                about: "Add an item with a room.",
                schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "room": {"type": "string"},
                        "barcode": {"type": "string"},
                        "note": {"type": "string"}
                    },
                    "required": ["name", "room"]
                }),
            },
        ]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "inventory.count",
            title: "Items",
            module: "inventory",
        }]
    }

    async fn health(&self, state: &AppState) -> Health {
        match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
            Ok(_) => Health::ok("inventory answers"),
            Err(e) => Health::down(format!("db down: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_item(name: &str, room: &str) -> NewItem {
        NewItem {
            name: name.into(),
            room: room.into(),
            barcode: None,
            note: None,
        }
    }

    #[test]
    fn match_needs_all_words() {
        let i = Item {
            id: "1".into(),
            name: "Blue passport case".into(),
            room: "study".into(),
            barcode: None,
            note: None,
        };
        assert!(matches(&i, "passport case"));
        assert!(!matches(&i, "passport red"));
        assert!(matches(&i, "BLUE"));
    }

    #[tokio::test]
    async fn mem_item_flows() {
        let s = MemInventoryStore::new();
        s.add(new_item("Passport", "study")).await;
        let mut coded = new_item("Milk", "kitchen");
        coded.barcode = Some("123".into());
        s.add(coded).await;
        assert_eq!(s.list().await.len(), 2);
        assert_eq!(s.search("pass").await.len(), 1);
        assert!(s.search("").await.len() == 2);
        let found = s.by_barcode("123").await.unwrap();
        assert_eq!(found.room, "kitchen");
        assert!(s.by_barcode("zzz").await.is_none());
        assert_eq!(
            s.rooms().await,
            vec!["kitchen".to_string(), "study".to_string()]
        );
        assert!(!s.add_room("study").await);
    }
}

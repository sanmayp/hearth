//! Tasks module. Inbox, today, someday.
//! Rule splitter cuts big tasks into steps.
//! The agent refines steps when AI turns on.

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

/// The tasks module.
pub struct Tasks;

/// One task list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskList {
    /// New tasks land here.
    #[default]
    Inbox,
    /// Tasks for now.
    Today,
    /// Tasks for later.
    Someday,
}

impl TaskList {
    fn from_str(s: &str) -> Self {
        match s {
            "today" => Self::Today,
            "someday" => Self::Someday,
            _ => Self::Inbox,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Today => "today",
            Self::Someday => "someday",
        }
    }
}

/// One step inside a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Step {
    /// Step title.
    pub title: String,
    /// True when done.
    pub done: bool,
}

/// One task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    /// Stable id (uuid).
    pub id: String,
    /// Short title.
    pub title: String,
    /// Home list.
    pub list: TaskList,
    /// Project name, if any.
    pub project: Option<String>,
    /// True when done.
    pub done: bool,
    /// Steps (empty for small tasks).
    pub steps: Vec<Step>,
}

/// Input for a new task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    /// Short title.
    pub title: String,
    /// Home list (default inbox).
    pub list: Option<String>,
    /// Project name, if any.
    pub project: Option<String>,
}

/// Move input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveIn {
    /// Target list name.
    pub list: String,
}

/// List filter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListQ {
    /// Keep one list only.
    pub list: Option<String>,
}

/// Rule splitter. Cuts a title on " then ", " and ",
/// or "; " into steps. Two parts or more win.
/// One part means the task stays whole.
pub fn split_rule(title: &str) -> Vec<String> {
    for sep in [" then ", " and ", "; "] {
        let parts: Vec<String> = title
            .split(sep)
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        if parts.len() >= 2 {
            return parts;
        }
    }
    vec![]
}

/// Task storage. PG in prod, memory in tests.
#[async_trait]
pub trait TaskStore: Send + Sync {
    /// List open tasks, oldest first.
    async fn list(&self, list: Option<TaskList>) -> Vec<Task>;
    /// Add one. Assigns the id.
    async fn add(&self, n: NewTask) -> Task;
    /// Mark done. False when missing.
    async fn complete(&self, id: &str) -> bool;
    /// Bring back (undo). False when missing.
    async fn uncomplete(&self, id: &str) -> bool;
    /// Move lists. False when missing.
    async fn move_to(&self, id: &str, list: TaskList) -> bool;
    /// Split into steps. False when missing or unsplittable.
    async fn split(&self, id: &str) -> bool;
    /// Mark one step done. False when missing.
    async fn step_done(&self, id: &str, idx: usize) -> bool;
}

fn row_to_task(
    row: (
        String,
        String,
        String,
        Option<String>,
        bool,
        serde_json::Value,
    ),
) -> Task {
    let (id, title, list, project, done, steps) = row;
    Task {
        id,
        title,
        list: TaskList::from_str(&list),
        project,
        done,
        steps: serde_json::from_value(steps).unwrap_or_default(),
    }
}

/// Postgres store.
pub struct PgTaskStore {
    /// Pool to the system of record.
    pub pool: PgPool,
}

#[async_trait]
impl TaskStore for PgTaskStore {
    async fn list(&self, list: Option<TaskList>) -> Vec<Task> {
        let rows: Vec<(
            String,
            String,
            String,
            Option<String>,
            bool,
            serde_json::Value,
        )> = match list {
            Some(l) => sqlx::query_as(
                "SELECT id, title, list, project, done, steps FROM tasks
                     WHERE done = FALSE AND list = $1 ORDER BY id",
            )
            .bind(l.as_str())
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default(),
            None => sqlx::query_as(
                "SELECT id, title, list, project, done, steps FROM tasks
                     WHERE done = FALSE ORDER BY id",
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default(),
        };
        rows.into_iter().map(row_to_task).collect()
    }

    async fn add(&self, n: NewTask) -> Task {
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: n.title,
            list: n.list.map(|l| TaskList::from_str(&l)).unwrap_or_default(),
            project: n.project,
            done: false,
            steps: vec![],
        };
        let _ = sqlx::query(
            "INSERT INTO tasks (id, title, list, project, done, steps)
             VALUES ($1, $2, $3, $4, FALSE, '[]')",
        )
        .bind(&task.id)
        .bind(&task.title)
        .bind(task.list.as_str())
        .bind(&task.project)
        .execute(&self.pool)
        .await;
        task
    }

    async fn complete(&self, id: &str) -> bool {
        sqlx::query("UPDATE tasks SET done = TRUE WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }

    async fn uncomplete(&self, id: &str) -> bool {
        sqlx::query("UPDATE tasks SET done = FALSE WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }

    async fn move_to(&self, id: &str, list: TaskList) -> bool {
        sqlx::query("UPDATE tasks SET list = $1 WHERE id = $2")
            .bind(list.as_str())
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }

    async fn split(&self, id: &str) -> bool {
        let row: Option<(String, serde_json::Value)> =
            sqlx::query_as("SELECT title, steps FROM tasks WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
        let Some((title, steps)) = row else {
            return false;
        };
        let have: Vec<Step> = serde_json::from_value(steps).unwrap_or_default();
        if !have.is_empty() {
            return false;
        }
        let parts = split_rule(&title);
        if parts.is_empty() {
            return false;
        }
        let steps: Vec<Step> = parts
            .into_iter()
            .map(|title| Step { title, done: false })
            .collect();
        let data = serde_json::to_value(&steps).unwrap_or(json!([]));
        sqlx::query("UPDATE tasks SET steps = $1 WHERE id = $2")
            .bind(data)
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }

    async fn step_done(&self, id: &str, idx: usize) -> bool {
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT steps FROM tasks WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
        let Some((data,)) = row else {
            return false;
        };
        let mut steps: Vec<Step> = serde_json::from_value(data).unwrap_or_default();
        let Some(step) = steps.get_mut(idx) else {
            return false;
        };
        step.done = true;
        let data = serde_json::to_value(&steps).unwrap_or(json!([]));
        sqlx::query("UPDATE tasks SET steps = $1 WHERE id = $2")
            .bind(data)
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected() > 0)
            .unwrap_or(false)
    }
}

/// In-memory store for tests.
pub struct MemTaskStore {
    inner: Mutex<HashMap<String, Task>>,
}

impl MemTaskStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskStore for MemTaskStore {
    async fn list(&self, list: Option<TaskList>) -> Vec<Task> {
        let mut v: Vec<Task> = self
            .inner
            .lock()
            .map(|m| {
                m.values()
                    .filter(|t| !t.done && list.as_ref().map(|l| *l == t.list).unwrap_or(true))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by_key(|t| t.id.clone());
        v
    }

    async fn add(&self, n: NewTask) -> Task {
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title: n.title,
            list: n.list.map(|l| TaskList::from_str(&l)).unwrap_or_default(),
            project: n.project,
            done: false,
            steps: vec![],
        };
        if let Ok(mut m) = self.inner.lock() {
            m.insert(task.id.clone(), task.clone());
        }
        task
    }

    async fn complete(&self, id: &str) -> bool {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(t) = m.get_mut(id) {
                t.done = true;
                return true;
            }
        }
        false
    }

    async fn uncomplete(&self, id: &str) -> bool {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(t) = m.get_mut(id) {
                t.done = false;
                return true;
            }
        }
        false
    }

    async fn move_to(&self, id: &str, list: TaskList) -> bool {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(t) = m.get_mut(id) {
                t.list = list;
                return true;
            }
        }
        false
    }

    async fn split(&self, id: &str) -> bool {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(t) = m.get_mut(id) {
                if !t.steps.is_empty() {
                    return false;
                }
                let parts = split_rule(&t.title);
                if parts.is_empty() {
                    return false;
                }
                t.steps = parts
                    .into_iter()
                    .map(|title| Step { title, done: false })
                    .collect();
                return true;
            }
        }
        false
    }

    async fn step_done(&self, id: &str, idx: usize) -> bool {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(t) = m.get_mut(id) {
                if let Some(s) = t.steps.get_mut(idx) {
                    s.done = true;
                    return true;
                }
            }
        }
        false
    }
}

/// GET /v1/tasks[?list=]. Lists open tasks.
async fn list_tasks<S: TaskStore>(
    Extension(store): Extension<Arc<S>>,
    Query(q): Query<ListQ>,
) -> Json<Vec<Task>> {
    let list = q.list.map(|l| TaskList::from_str(&l));
    Json(store.list(list).await)
}

/// POST /v1/tasks. Adds one.
async fn add_task<S: TaskStore>(
    Extension(store): Extension<Arc<S>>,
    Json(n): Json<NewTask>,
) -> Json<Task> {
    Json(store.add(n).await)
}

/// POST /v1/tasks/:id/done. Marks done.
async fn complete_task<S: TaskStore>(
    Extension(store): Extension<Arc<S>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({"ok": store.complete(&id).await}))
}

/// POST /v1/tasks/:id/undo. Brings back.
async fn uncomplete_task<S: TaskStore>(
    Extension(store): Extension<Arc<S>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({"ok": store.uncomplete(&id).await}))
}

/// POST /v1/tasks/:id/move. Moves lists.
async fn move_task<S: TaskStore>(
    Extension(store): Extension<Arc<S>>,
    Path(id): Path<String>,
    Json(body): Json<MoveIn>,
) -> Json<serde_json::Value> {
    let list = TaskList::from_str(&body.list);
    Json(json!({"ok": store.move_to(&id, list).await}))
}

/// POST /v1/tasks/:id/split. Cuts into steps.
async fn split_task<S: TaskStore>(
    Extension(store): Extension<Arc<S>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({"ok": store.split(&id).await}))
}

/// POST /v1/tasks/:id/steps/:n/done. Marks one step done.
async fn step_task_done<S: TaskStore>(
    Extension(store): Extension<Arc<S>>,
    Path((id, n)): Path<(String, usize)>,
) -> Json<serde_json::Value> {
    Json(json!({"ok": store.step_done(&id, n).await}))
}

#[async_trait]
impl Module for Tasks {
    fn id(&self) -> &'static str {
        "tasks"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_tasks",
            sql: "CREATE TABLE IF NOT EXISTS tasks
                  (id TEXT PRIMARY KEY, title TEXT NOT NULL, list TEXT NOT NULL DEFAULT 'inbox',
                   project TEXT, done BOOLEAN NOT NULL DEFAULT FALSE,
                   steps JSONB NOT NULL DEFAULT '[]')",
        }]
    }

    fn routes(&self, state: &AppState) -> Router<()> {
        let store = Arc::new(PgTaskStore {
            pool: state.pool.clone(),
        });
        Router::new()
            .route(
                "/v1/tasks",
                get(list_tasks::<PgTaskStore>).post(add_task::<PgTaskStore>),
            )
            .route(
                "/v1/tasks/:id/done",
                axum::routing::post(complete_task::<PgTaskStore>),
            )
            .route(
                "/v1/tasks/:id/undo",
                axum::routing::post(uncomplete_task::<PgTaskStore>),
            )
            .route(
                "/v1/tasks/:id/move",
                axum::routing::post(move_task::<PgTaskStore>),
            )
            .route(
                "/v1/tasks/:id/split",
                axum::routing::post(split_task::<PgTaskStore>),
            )
            .route(
                "/v1/tasks/:id/steps/:n/done",
                axum::routing::post(step_task_done::<PgTaskStore>),
            )
            .layer(Extension(store))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![JobDef {
            name: "tasks.rollover",
            cron: "0 5 * * *",
            about: "Move past-due someday tasks to today.",
        }]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "tasks.add",
                about: "Add a task to a list.",
                schema: json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "list": {"type": "string"},
                        "project": {"type": "string"}
                    },
                    "required": ["title"]
                }),
            },
            ToolDef {
                name: "tasks.split",
                about: "Cut a big task into steps.",
                schema: json!({
                    "type": "object",
                    "properties": {"id": {"type": "string"}},
                    "required": ["id"]
                }),
            },
        ]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "tasks.due",
            title: "Tasks",
            module: "tasks",
        }]
    }

    async fn health(&self, state: &AppState) -> Health {
        match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
            Ok(_) => Health::ok("tasks answer"),
            Err(e) => Health::down(format!("db down: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitter_cuts_then() {
        assert_eq!(
            split_rule("Buy milk then call Sam"),
            vec!["Buy milk", "call Sam"]
        );
    }

    #[test]
    fn splitter_needs_two_parts() {
        assert!(split_rule("Buy milk").is_empty());
    }

    #[test]
    fn list_parse_defaults_inbox() {
        assert_eq!(TaskList::from_str("zzz"), TaskList::Inbox);
    }

    #[tokio::test]
    async fn mem_task_flows() {
        let s = MemTaskStore::new();
        let t = s
            .add(NewTask {
                title: "A then B".into(),
                list: None,
                project: None,
            })
            .await;
        assert_eq!(t.list, TaskList::Inbox);
        assert!(s.split(&t.id).await);
        assert!(!s.split(&t.id).await);
        assert!(s.step_done(&t.id, 0).await);
        assert!(!s.step_done(&t.id, 9).await);
        assert!(s.move_to(&t.id, TaskList::Today).await);
        assert_eq!(s.list(Some(TaskList::Today)).await.len(), 1);
        assert!(s.complete(&t.id).await);
        assert!(s.list(None).await.is_empty());
        assert!(s.uncomplete(&t.id).await);
        assert_eq!(s.list(None).await.len(), 1);
    }
}

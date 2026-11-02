//! The Module trait. Each domain crate owns one.
//! Modules never read each other's tables.

use async_trait::async_trait;
use axum::Router;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use crate::{AppState, Result};

/// One domain module (Today, Tasks, Health, ...).
#[async_trait]
pub trait Module: Send + Sync {
    /// Stable id, e.g. "today". Used in flags, events, routes.
    fn id(&self) -> &'static str;
    /// Ordered SQL migrations owned by this module.
    fn migrations(&self) -> Vec<Migration>;
    /// HTTP routes, mounted under `/v1/<id>`.
    /// Handlers take stores via `Extension`, so tests use memory stores.
    fn routes(&self, state: &AppState) -> Router<()>;
    /// Background jobs this module offers.
    fn jobs(&self) -> Vec<JobDef>;
    /// Tools the agent may call (JSON schema each).
    fn tools(&self) -> Vec<ToolDef>;
    /// Dashboard cards this module offers to Today.
    fn widgets(&self) -> Vec<WidgetDef>;
    /// Liveness of this module and its data source.
    async fn health(&self, state: &AppState) -> Health;
}

/// One SQL migration owned by a module.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Monotonic number, unique per module.
    pub version: i64,
    /// Short name, e.g. "create_tasks".
    pub name: &'static str,
    /// The SQL to run once.
    pub sql: &'static str,
}

/// One background job offered by a module.
#[derive(Debug, Clone, Serialize)]
pub struct JobDef {
    /// Stable name, e.g. "reminders.dispatch".
    pub name: &'static str,
    /// Cron expression (UTC).
    pub cron: &'static str,
    /// One line: what the job does.
    pub about: &'static str,
}

/// One agent-callable tool with JSON schema.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    /// Stable name, e.g. "tasks.add".
    pub name: &'static str,
    /// One line: what the tool does.
    pub about: &'static str,
    /// JSON schema for the tool input.
    pub schema: Value,
}

/// One dashboard card offered to the Today screen.
#[derive(Debug, Clone, Serialize)]
pub struct WidgetDef {
    /// Stable id, e.g. "today.agenda".
    pub id: &'static str,
    /// Short title for the card.
    pub title: &'static str,
    /// Owning module id.
    pub module: &'static str,
}

/// Health of one module.
#[derive(Debug, Clone, Serialize)]
pub struct Health {
    /// True when the module and its source answer.
    pub ok: bool,
    /// One line of detail (or "stale since ..." label).
    pub detail: String,
}

impl Health {
    pub fn ok(detail: impl Into<String>) -> Self {
        Self {
            ok: true,
            detail: detail.into(),
        }
    }

    pub fn down(detail: impl Into<String>) -> Self {
        Self {
            ok: false,
            detail: detail.into(),
        }
    }
}

/// Run all migrations for the given modules, in order.
/// Tracks applied versions in `_hearth_migrations`.
pub async fn run_migrations(
    pool: &PgPool,
    modules: &[(&'static str, Vec<Migration>)],
) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _hearth_migrations
         (module TEXT NOT NULL, version BIGINT NOT NULL, applied_at TIMESTAMPTZ NOT NULL DEFAULT now(),
          PRIMARY KEY (module, version))",
    )
    .execute(pool)
    .await?;

    for (module, migrations) in modules {
        let mut ordered = migrations.clone();
        ordered.sort_by_key(|m| m.version);
        for m in ordered {
            let done: Option<(i64,)> = sqlx::query_as(
                "SELECT version FROM _hearth_migrations WHERE module = $1 AND version = $2",
            )
            .bind(module)
            .bind(m.version)
            .fetch_optional(pool)
            .await?;
            if done.is_none() {
                // Simple protocol: one migration may hold many statements.
                sqlx::raw_sql(m.sql).execute(pool).await?;
                sqlx::query("INSERT INTO _hearth_migrations (module, version) VALUES ($1, $2)")
                    .bind(module)
                    .bind(m.version)
                    .execute(pool)
                    .await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_sort_by_version() {
        let mut ms = [
            Migration {
                version: 2,
                name: "b",
                sql: "SELECT 2",
            },
            Migration {
                version: 1,
                name: "a",
                sql: "SELECT 1",
            },
        ];
        ms.sort_by_key(|m| m.version);
        assert_eq!(ms[0].version, 1);
        assert_eq!(ms[1].version, 2);
    }
}

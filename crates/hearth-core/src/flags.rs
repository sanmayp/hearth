//! Feature flags. Postgres table, cached in Redis.
//! Switching a module off is the rollback.

use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::{Error, Result};

/// Flag storage. PG in prod, memory in tests.
#[async_trait]
pub trait FlagStore: Send + Sync {
    /// Read one flag. Missing flag means off.
    async fn is_on(&self, module: &str) -> Result<bool>;
    /// Set one flag.
    async fn set(&self, module: &str, on: bool) -> Result<()>;
}

/// Postgres flags with a Redis cache layer.
pub struct PgFlags {
    /// System of record pool.
    pub pool: PgPool,
    /// Cache client.
    pub redis: redis::Client,
}

impl PgFlags {
    /// Create the flags table when missing.
    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS feature_flags
             (module TEXT PRIMARY KEY, enabled BOOLEAN NOT NULL DEFAULT FALSE)",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn key(module: &str) -> String {
        format!("flag:{module}")
    }
}

#[async_trait]
impl FlagStore for PgFlags {
    async fn is_on(&self, module: &str) -> Result<bool> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let key = Self::key(module);
        let hit: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut conn).await?;
        if let Some(v) = hit {
            return Ok(v == "1");
        }
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM feature_flags WHERE module = $1")
                .bind(module)
                .fetch_optional(&self.pool)
                .await?;
        let on = row.map(|r| r.0).unwrap_or(false);
        let val = if on { "1" } else { "0" };
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg(val)
            .arg("EX")
            .arg(Duration::from_secs(60).as_secs())
            .query_async(&mut conn)
            .await?;
        Ok(on)
    }

    async fn set(&self, module: &str, on: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO feature_flags (module, enabled) VALUES ($1, $2)
             ON CONFLICT (module) DO UPDATE SET enabled = $2",
        )
        .bind(module)
        .bind(on)
        .execute(&self.pool)
        .await?;
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let _: () = redis::cmd("DEL")
            .arg(Self::key(module))
            .query_async(&mut conn)
            .await?;
        Ok(())
    }
}

/// In-memory flags for tests. No IO.
pub struct InMemoryFlags {
    inner: Mutex<HashMap<String, bool>>,
}

impl InMemoryFlags {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryFlags {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FlagStore for InMemoryFlags {
    async fn is_on(&self, module: &str) -> Result<bool> {
        let map = self
            .inner
            .lock()
            .map_err(|e| Error::Config(e.to_string()))?;
        Ok(map.get(module).copied().unwrap_or(false))
    }

    async fn set(&self, module: &str, on: bool) -> Result<()> {
        let mut map = self
            .inner
            .lock()
            .map_err(|e| Error::Config(e.to_string()))?;
        map.insert(module.to_string(), on);
        Ok(())
    }
}

/// Flags handle used by the server.
pub struct Flags {
    store: Box<dyn FlagStore>,
}

impl Flags {
    /// Wrap any store.
    pub fn new(store: Box<dyn FlagStore>) -> Self {
        Self { store }
    }

    /// Read one module flag. Missing means off.
    pub async fn is_on(&self, module: &str) -> Result<bool> {
        self.store.is_on(module).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_flag_means_off() {
        let f = InMemoryFlags::new();
        assert!(!f.is_on("today").await.unwrap());
    }

    #[tokio::test]
    async fn set_then_read() {
        let f = InMemoryFlags::new();
        f.set("today", true).await.unwrap();
        assert!(f.is_on("today").await.unwrap());
        f.set("today", false).await.unwrap();
        assert!(!f.is_on("today").await.unwrap());
    }
}

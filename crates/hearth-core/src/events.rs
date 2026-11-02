//! Typed cross-module events on Redis Streams.
//! Modules never touch each other's tables.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{Error, Result};

/// One typed event between modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Owning module id.
    pub module: String,
    /// Event kind, e.g. "task.done".
    pub kind: String,
    /// Small payload. Never whole records.
    pub payload: Value,
}

impl Event {
    pub fn new(module: impl Into<String>, kind: impl Into<String>, payload: Value) -> Self {
        Self {
            module: module.into(),
            kind: kind.into(),
            payload,
        }
    }

    /// Stream key for this module.
    pub fn stream(&self) -> String {
        format!("events:{}", self.module)
    }
}

/// Event bus. Streams in prod, memory in tests.
#[async_trait]
pub trait Bus: Send + Sync {
    /// Publish one event.
    async fn publish(&self, event: &Event) -> Result<String>;
}

/// Redis Streams bus.
pub struct StreamBus {
    /// Cache client (streams live here, facts in PG).
    pub redis: redis::Client,
}

#[async_trait]
impl Bus for StreamBus {
    async fn publish(&self, event: &Event) -> Result<String> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let payload =
            serde_json::to_string(&event.payload).map_err(|e| Error::Config(e.to_string()))?;
        let id: String = redis::cmd("XADD")
            .arg(event.stream())
            .arg("*")
            .arg("module")
            .arg(&event.module)
            .arg("kind")
            .arg(&event.kind)
            .arg("payload")
            .arg(payload)
            .query_async(&mut conn)
            .await?;
        Ok(id)
    }
}

/// In-memory bus for tests. Keeps events per stream.
pub struct InMemoryBus {
    inner: Mutex<HashMap<String, Vec<Event>>>,
}

impl InMemoryBus {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Read events kept for one stream (tests only).
    pub fn kept(&self, stream: &str) -> Vec<Event> {
        self.inner
            .lock()
            .map(|m| m.get(stream).cloned().unwrap_or_default())
            .unwrap_or_default()
    }
}

impl Default for InMemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Bus for InMemoryBus {
    async fn publish(&self, event: &Event) -> Result<String> {
        let mut map = self
            .inner
            .lock()
            .map_err(|e| Error::Config(e.to_string()))?;
        let list = map.entry(event.stream()).or_default();
        list.push(event.clone());
        Ok(format!("mem-{}", list.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn memory_bus_keeps_per_stream() {
        let bus = InMemoryBus::new();
        let e = Event::new("tasks", "task.done", json!({"id": 1}));
        let id = bus.publish(&e).await.unwrap();
        assert_eq!(id, "mem-1");
        let kept = bus.kept("events:tasks");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].kind, "task.done");
        assert!(bus.kept("events:today").is_empty());
    }
}

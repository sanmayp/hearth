//! Shared app state. Cloned into each handler.

use sqlx::PgPool;

/// State shared by the server and all modules.
#[derive(Clone)]
pub struct AppState {
    /// Postgres pool. System of record.
    pub pool: PgPool,
    /// Redis client. Cache only, never the sole copy.
    pub redis: redis::Client,
}

//! One error type for the core.

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("cache: {0}")]
    Cache(#[from] redis::RedisError),
    #[error("model: {0}")]
    Model(String),
    #[error("guard: {0}")]
    Guard(String),
    #[error("config: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;

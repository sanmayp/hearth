//! Hearth core: shared types for all modules.
//! Each module is one crate. It talks to others
//! through typed events, never through their tables.

pub mod error;
pub mod events;
pub mod flags;
pub mod guard;
pub mod llm;
pub mod module;
pub mod state;

pub use error::{Error, Result};
pub use events::{Bus, Event, InMemoryBus, StreamBus};
pub use flags::{FlagStore, Flags, InMemoryFlags, PgFlags};
pub use guard::{DataClass, DataGuard, Field};
pub use llm::{ChatReq, ChatResp, Llm, MetaMuse, NoopLlm};
pub use module::{run_migrations, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
pub use state::AppState;

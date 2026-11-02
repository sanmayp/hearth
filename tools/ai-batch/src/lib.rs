//! ai-batch. It fans public text out to model endpoints.
//! It meters every call. Mock stays labelled. Keys never log.

pub mod batch;
pub mod chunk;
pub mod config;
pub mod provider;
pub mod report;

pub use batch::{plan_calls, run_batch, BatchSummary, PlannedCall};
pub use chunk::{chunk_text, estimate_tokens, read_input_dir, Chunk};
pub use config::{parse_args, validate_url, BothMode, Command, Config, Provider, Task};
pub use provider::{CallResult, CallStatus, ProviderClient};
pub use report::generate_report;

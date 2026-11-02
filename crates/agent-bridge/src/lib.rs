//! Agent bridge. It exposes herdr panes over local HTTP.
//! It binds loopback only. It sends text after two taps.

pub mod allow;
pub mod audit;
pub mod auth;
pub mod codes;
pub mod config;
pub mod cors;
pub mod herdr;
pub mod kinds;
pub mod limit;
pub mod redact;
pub mod reqlog;
pub mod routes;

pub use allow::load_allowlist;
pub use audit::Audit;
pub use auth::{authorized, ct_eq};
pub use codes::{CodeErr, CodeStore};
pub use config::{default_origins, Config};
pub use herdr::{parse_pane_list, parse_pane_read, valid_pane_id, PaneInfo};
pub use kinds::{detect_approval, is_allowed_kind, is_short_answer, normalize_kind};
pub use limit::Limiter;
pub use redact::redact;
pub use routes::{build_router, AppState};

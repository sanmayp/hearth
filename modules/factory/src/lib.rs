//! Factory module. Read-only status from the factory files:
//! STATE.md sections, builds.json, and the last ledger lines.

use async_trait::async_trait;
use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

/// The factory module.
pub struct Factory;

/// How many ledger lines the board shows.
const LEDGER_TAIL: usize = 10;

/// One STATE.md section.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateSection {
    /// Section title, without the hashes.
    pub title: String,
    /// Section body, trimmed.
    pub body: String,
}

/// Parsed STATE.md: sections, or raw text when parsing fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateView {
    /// Sections in file order. Empty when parsing fails.
    pub sections: Vec<StateSection>,
    /// Full text. Set only when no section parsed.
    pub raw: Option<String>,
}

/// Whole status: state, builds, and ledger tail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactoryStatus {
    /// STATE.md, parsed or raw. Missing when the file is gone.
    pub state: Option<StateView>,
    /// builds.json as stored. Missing when the file is gone.
    pub builds: Option<Value>,
    /// Last ledger entries, oldest first. Empty when the file is gone.
    pub ledger: Vec<Value>,
}

#[async_trait]
impl Module for Factory {
    fn id(&self) -> &'static str {
        "factory"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![]
    }

    fn routes(&self, _state: &AppState) -> Router<()> {
        Router::new().route("/v1/factory/status", get(get_status))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "factory.card",
            title: "Factory",
            module: "factory",
        }]
    }

    async fn health(&self, _state: &AppState) -> Health {
        match std::fs::metadata(factory_dir()) {
            Ok(_) => Health::ok("factory files read"),
            Err(e) => Health::down(format!("factory files down: {e}")),
        }
    }
}

/// Serve the status. Each file stands alone; all missing is a 503.
async fn get_status() -> impl IntoResponse {
    let dir = factory_dir();
    let state = std::fs::read_to_string(dir.join("STATE.md"))
        .ok()
        .map(|t| parse_state(&t));
    let builds = std::fs::read_to_string(dir.join("builds.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());
    let ledger = std::fs::read_to_string(dir.join("ledger.jsonl"))
        .map(|t| tail_ledger(&t))
        .unwrap_or_default();
    if state.is_none() && builds.is_none() && ledger.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message": "Factory files are missing. Retry soon."})),
        )
            .into_response();
    }
    Json(json!(FactoryStatus {
        state,
        builds,
        ledger
    }))
    .into_response()
}

/// Factory dir. Env wins, else the checkout default.
fn factory_dir() -> PathBuf {
    if let Ok(p) = std::env::var("FACTORY_DIR") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("hearth/factory")
}

/// Parse STATE.md into sections. Raw fallback when none parse.
fn parse_state(text: &str) -> StateView {
    let mut sections: Vec<StateSection> = vec![];
    let mut title: Option<String> = None;
    let mut body: Vec<String> = vec![];
    for line in text.lines() {
        if let Some(t) = line.strip_prefix("## ") {
            if let Some(old) = title.take() {
                sections.push(StateSection {
                    title: old,
                    body: body.join("\n").trim().to_string(),
                });
                body.clear();
            }
            title = Some(t.trim().to_string());
        } else if title.is_some() {
            body.push(line.to_string());
        }
    }
    if let Some(old) = title {
        sections.push(StateSection {
            title: old,
            body: body.join("\n").trim().to_string(),
        });
    }
    if sections.is_empty() {
        StateView {
            sections,
            raw: Some(text.to_string()),
        }
    } else {
        StateView {
            sections,
            raw: None,
        }
    }
}

/// Last JSON lines of the ledger, oldest first. Bad lines drop out.
fn tail_ledger(text: &str) -> Vec<Value> {
    text.lines()
        .rev()
        .take(LEDGER_TAIL)
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect::<Vec<Value>>()
        .into_iter()
        .rev()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE: &str =
        "# factory/STATE.md\n\n## In flight\n\n- Lane A ships.\n\n## Merged\n\n- Lane B done.\n";

    #[test]
    fn parses_state_sections_in_order() {
        let v = parse_state(STATE);
        assert!(v.raw.is_none());
        assert_eq!(v.sections.len(), 2);
        assert_eq!(v.sections[0].title, "In flight");
        assert_eq!(v.sections[0].body, "- Lane A ships.");
        assert_eq!(v.sections[1].title, "Merged");
    }

    #[test]
    fn state_without_sections_falls_back_to_raw() {
        let v = parse_state("just words\nno headers\n");
        assert!(v.sections.is_empty());
        assert_eq!(v.raw.as_deref(), Some("just words\nno headers\n"));
    }

    #[test]
    fn ledger_tail_keeps_last_ten_oldest_first() {
        let text = (1..=12)
            .map(|i| format!("{{\"ts\":\"t{i}\"}}"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = tail_ledger(&text);
        assert_eq!(out.len(), 10);
        assert_eq!(out[0]["ts"], "t3");
        assert_eq!(out[9]["ts"], "t12");
    }

    #[test]
    fn ledger_drops_bad_lines() {
        let out = tail_ledger("{\"a\":1}\nbroken\n{\"b\":2}\n");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn live_files_parse() {
        let dir = super::factory_dir();
        let Ok(state) = std::fs::read_to_string(dir.join("STATE.md")) else {
            return;
        };
        let v = parse_state(&state);
        assert!(!v.sections.is_empty() || v.raw.is_some());
        if let Ok(text) = std::fs::read_to_string(dir.join("ledger.jsonl")) {
            assert!(!tail_ledger(&text).is_empty());
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("builds.json")) {
            assert!(serde_json::from_str::<Value>(&text).is_ok());
        }
    }
}

//! AI usage module. It reads batch meter files. Read-only.
//! Missing files mean zero, never an error.

use async_trait::async_trait;
use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::collections::HashMap;

/// The usage module.
pub struct AiUsage;

/// Usage file path. Env wins, then the batch default.
fn usage_path() -> String {
    std::env::var("HEARTH_AI_USAGE_FILE").unwrap_or_else(|_| "out/ai-batch/usage.jsonl".into())
}

/// Results file path. Env wins, then the batch default.
fn results_path() -> String {
    std::env::var("HEARTH_AI_RESULTS_FILE").unwrap_or_else(|_| "out/ai-batch/results.jsonl".into())
}

/// One usage row, as the batch wrote it.
#[derive(Debug, Deserialize)]
struct UsageRow {
    /// Provider label.
    #[serde(default)]
    provider: String,
    /// Input tokens.
    #[serde(default)]
    tokens_in: u64,
    /// Output tokens.
    #[serde(default)]
    tokens_out: u64,
    /// Wall ms.
    #[serde(default)]
    latency_ms: u64,
    /// Outcome word.
    #[serde(default)]
    status: String,
    /// True for mock rows.
    #[serde(default)]
    mock: bool,
}

/// Totals for one provider.
#[derive(Debug, Default, Serialize)]
struct ProviderTotals {
    /// Calls counted.
    calls: u64,
    /// Live calls.
    live: u64,
    /// Mock calls.
    mock: u64,
    /// Input tokens.
    tokens_in: u64,
    /// Output tokens.
    tokens_out: u64,
    /// p50 latency over ok calls.
    p50_ms: u64,
    /// p95 latency over ok calls.
    p95_ms: u64,
    /// Errors.
    errors: u64,
}

/// Percentile by nearest rank. Empty gives zero.
fn percentile(sorted: &[u64], pct: u64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((pct as usize * sorted.len()).div_ceil(100)).saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

/// Sum rows into per-provider totals. Pure, so tests cover it.
fn summarize(rows: &[UsageRow]) -> BTreeMap<String, ProviderTotals> {
    let mut latencies: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut totals: BTreeMap<String, ProviderTotals> = BTreeMap::new();
    for row in rows {
        let name = if row.provider.is_empty() {
            "unknown".to_string()
        } else {
            row.provider.clone()
        };
        let entry = totals.entry(name.clone()).or_default();
        entry.calls += 1;
        if row.mock {
            entry.mock += 1;
        } else {
            entry.live += 1;
        }
        if row.status == "ok" {
            latencies.entry(name).or_default().push(row.latency_ms);
        } else {
            entry.errors += 1;
        }
        entry.tokens_in += row.tokens_in;
        entry.tokens_out += row.tokens_out;
    }
    for (name, mut lat) in latencies {
        lat.sort_unstable();
        if let Some(entry) = totals.get_mut(&name) {
            entry.p50_ms = percentile(&lat, 50);
            entry.p95_ms = percentile(&lat, 95);
        }
    }
    totals
}

/// Read usage rows. Bad lines drop out. Missing file means empty.
fn read_usage_rows() -> Vec<UsageRow> {
    let Ok(text) = std::fs::read_to_string(usage_path()) else {
        return vec![];
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Serve totals per provider. Zero when nothing ran.
async fn get_usage() -> impl IntoResponse {
    let rows = read_usage_rows();
    let totals = summarize(&rows);
    let (calls, live, mock, tokens_in, tokens_out, errors): (u64, u64, u64, u64, u64, u64) =
        totals.values().fold((0, 0, 0, 0, 0, 0), |acc, t| {
            (
                acc.0 + t.calls,
                acc.1 + t.live,
                acc.2 + t.mock,
                acc.3 + t.tokens_in,
                acc.4 + t.tokens_out,
                acc.5 + t.errors,
            )
        });
    Json(json!({
        "providers": totals,
        "totals": {
            "calls": calls,
            "live": live,
            "mock": mock,
            "tokens_in": tokens_in,
            "tokens_out": tokens_out,
            "errors": errors,
        },
    }))
}

/// Valid result tasks. Anything else answers 400.
fn valid_task(task: &str) -> bool {
    matches!(task, "summary" | "explain" | "suppliers" | "digest")
}

/// Serve batch result rows. Newest first. Task filter is exact.
async fn get_results(Query(query): Query<HashMap<String, String>>) -> impl IntoResponse {
    if let Some(task) = query.get("task") {
        if !valid_task(task) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"message": "Task is unknown."})),
            )
                .into_response();
        }
    }
    let limit: usize = match query.get("limit") {
        None => 20,
        Some(v) => match v.parse() {
            Ok(n) if (1..=100).contains(&n) => n,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"message": "Limit must sit from 1 to 100."})),
                )
                    .into_response()
            }
        },
    };
    let Ok(text) = std::fs::read_to_string(results_path()) else {
        return Json(json!({"rows": []})).into_response();
    };
    let mut rows: Vec<Value> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .filter(|row: &Value| match query.get("task") {
            None => true,
            Some(task) => row.get("task").and_then(|t| t.as_str()) == Some(task.as_str()),
        })
        .collect();
    rows.reverse();
    rows.truncate(limit);
    Json(json!({"rows": rows})).into_response()
}

#[async_trait]
impl Module for AiUsage {
    fn id(&self) -> &'static str {
        "aiusage"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![]
    }

    fn routes(&self, _state: &AppState) -> Router<()> {
        Router::new()
            .route("/v1/ai/usage", get(get_usage))
            .route("/v1/ai/results", get(get_results))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "aiusage.card",
            title: "AI usage",
            module: "aiusage",
        }]
    }

    async fn health(&self, _state: &AppState) -> Health {
        match std::fs::read_to_string(usage_path()) {
            Ok(_) => Health::ok("usage reads"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Health::ok("usage reads, no runs yet")
            }
            Err(e) => Health::down(format!("usage down: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample rows. Mixed providers, live plus mock.
    fn sample() -> Vec<UsageRow> {
        vec![
            UsageRow {
                provider: "baseten".into(),
                tokens_in: 10,
                tokens_out: 5,
                latency_ms: 100,
                status: "ok".into(),
                mock: false,
            },
            UsageRow {
                provider: "baseten".into(),
                tokens_in: 10,
                tokens_out: 5,
                latency_ms: 300,
                status: "ok".into(),
                mock: false,
            },
            UsageRow {
                provider: "mock".into(),
                tokens_in: 4,
                tokens_out: 0,
                latency_ms: 1,
                status: "ok".into(),
                mock: true,
            },
            UsageRow {
                provider: "modal".into(),
                tokens_in: 0,
                tokens_out: 0,
                latency_ms: 50,
                status: "error".into(),
                mock: false,
            },
        ]
    }

    #[test]
    fn totals_split_live_mock_and_percentiles() {
        let totals = summarize(&sample());
        let baseten = &totals["baseten"];
        assert_eq!(baseten.calls, 2);
        assert_eq!(baseten.live, 2);
        assert_eq!(baseten.mock, 0);
        assert_eq!(baseten.tokens_in, 20);
        assert_eq!(baseten.p50_ms, 100);
        assert_eq!(baseten.p95_ms, 300);
        let mock = &totals["mock"];
        assert_eq!((mock.live, mock.mock), (0, 1));
        let modal = &totals["modal"];
        assert_eq!(modal.errors, 1);
        assert_eq!(modal.p50_ms, 0);
    }

    #[test]
    fn empty_rows_give_empty_totals() {
        assert!(summarize(&[]).is_empty());
    }
}

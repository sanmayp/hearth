//! Batch. It fans chunks out with caps and meters all.
//! It writes results plus usage. Text never reaches usage rows.

use crate::chunk::Chunk;
use crate::config::{BothMode, Config, Provider, Task};
use crate::provider::{CallResult, CallStatus, ProviderClient};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// One planned call. Dry-run prints these.
#[derive(Debug, Clone)]
pub struct PlannedCall {
    /// Chunk id.
    pub id: String,
    /// Provider label.
    pub provider: &'static str,
}

/// Run summary. Counts are real, never estimates.
#[derive(Debug, Default)]
pub struct BatchSummary {
    /// Chunks planned.
    pub planned: usize,
    /// Requests sent.
    pub ran: u64,
    /// Replies ok.
    pub ok: u64,
    /// Errors.
    pub errors: u64,
    /// Skipped by cap stop.
    pub skipped_cap: u64,
    /// Metered input tokens.
    pub tokens_in: u64,
    /// Metered output tokens.
    pub tokens_out: u64,
    /// True when a cap stopped the run early.
    pub capped: bool,
}

/// One results row. Output text lives here only.
#[derive(Debug, Serialize)]
struct ResultRow<'a> {
    /// Chunk id.
    id: &'a str,
    /// Task name.
    task: &'a str,
    /// Input file.
    file: &'a str,
    /// Provider label.
    provider: &'static str,
    /// Model name.
    model: &'a str,
    /// Reply text.
    output: &'a str,
    /// Input tokens.
    tokens_in: u64,
    /// Output tokens.
    tokens_out: u64,
    /// Wall ms.
    latency_ms: u64,
    /// Outcome.
    status: CallStatus,
    /// True for mock rows.
    mock: bool,
    /// Unix ms.
    at_ms: u64,
}

/// One usage row. No text, no keys, ever.
#[derive(Debug, Serialize)]
struct UsageRow<'a> {
    /// Chunk id.
    id: &'a str,
    /// Task name.
    task: &'a str,
    /// Input file.
    file: &'a str,
    /// Provider label.
    provider: &'static str,
    /// Model name.
    model: &'a str,
    /// Input tokens.
    tokens_in: u64,
    /// Output tokens.
    tokens_out: u64,
    /// Wall ms.
    latency_ms: u64,
    /// Outcome.
    status: CallStatus,
    /// True for mock rows.
    mock: bool,
    /// Unix ms.
    at_ms: u64,
}

/// Task label for rows.
fn task_label(task: Task) -> &'static str {
    match task {
        Task::Summary => "summary",
        Task::Explain => "explain",
        Task::Suppliers => "suppliers",
        Task::Digest => "digest",
    }
}

/// Unix time in ms.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Build the call plan. Split alternates, race doubles.
pub fn plan_calls(chunks: &[Chunk], cfg: &Config) -> Vec<PlannedCall> {
    let mut plan = vec![];
    for (i, chunk) in chunks.iter().enumerate() {
        match cfg.provider {
            Provider::Baseten => plan.push(one(chunk, "baseten")),
            Provider::Modal => plan.push(one(chunk, "modal")),
            Provider::Mock => plan.push(one(chunk, "mock")),
            Provider::Both => match cfg.both {
                BothMode::Split => {
                    let label = if i.is_multiple_of(2) {
                        "baseten"
                    } else {
                        "modal"
                    };
                    plan.push(one(chunk, label));
                }
                BothMode::Race => {
                    plan.push(one(chunk, "baseten"));
                    plan.push(one(chunk, "modal"));
                }
            },
        }
    }
    plan
}

/// One planned call.
fn one(chunk: &Chunk, provider: &'static str) -> PlannedCall {
    PlannedCall {
        id: chunk.id.clone(),
        provider,
    }
}

/// Run the batch. It enforces caps before each dispatch.
pub async fn run_batch(
    cfg: &Config,
    chunks: Vec<Chunk>,
    baseten: Option<ProviderClient>,
    modal: Option<ProviderClient>,
) -> Result<BatchSummary, String> {
    std::fs::create_dir_all(&cfg.out).map_err(|e| format!("Out dir is bad: {e}"))?;
    let results_path = cfg.out.join("results.jsonl");
    let usage_path = cfg.out.join("usage.jsonl");
    let mut summary = BatchSummary {
        planned: chunks.len(),
        ..Default::default()
    };
    let sem = Arc::new(Semaphore::new(cfg.concurrency));
    let budget = Arc::new(std::sync::Mutex::new(Budget::default()));
    let mut handles = vec![];
    for (i, chunk) in chunks.into_iter().enumerate() {
        let requests = requests_per_chunk(cfg);
        let est = crate::chunk::estimate_tokens(&chunk.text) * requests as u64;
        if !admit(cfg, &budget, requests as u64, est) {
            summary.capped = true;
            summary.skipped_cap += 1;
            continue;
        }
        let permit = sem.clone().acquire_owned().await.unwrap();
        let job = dispatch_job(cfg, i, chunk, &baseten, &modal)?;
        let budget = budget.clone();
        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let out = job.run().await;
            settle(&budget, est, &out);
            out
        }));
    }
    let mut result_lines = vec![];
    let mut usage_lines = vec![];
    for handle in handles {
        let rows = handle.await.unwrap();
        for (chunk, result) in rows {
            summary.ran += 1;
            match result.status {
                CallStatus::Ok => summary.ok += 1,
                _ => summary.errors += 1,
            }
            summary.tokens_in += result.tokens_in;
            summary.tokens_out += result.tokens_out;
            let at_ms = now_ms();
            result_lines.push(
                serde_json::to_string(&ResultRow {
                    id: &chunk.id,
                    task: task_label(cfg.task),
                    file: &chunk.file,
                    provider: result.provider,
                    model: &result.model,
                    output: &result.output,
                    tokens_in: result.tokens_in,
                    tokens_out: result.tokens_out,
                    latency_ms: result.latency_ms,
                    status: result.status,
                    mock: result.mock,
                    at_ms,
                })
                .unwrap(),
            );
            usage_lines.push(
                serde_json::to_string(&UsageRow {
                    id: &chunk.id,
                    task: task_label(cfg.task),
                    file: &chunk.file,
                    provider: result.provider,
                    model: &result.model,
                    tokens_in: result.tokens_in,
                    tokens_out: result.tokens_out,
                    latency_ms: result.latency_ms,
                    status: result.status,
                    mock: result.mock,
                    at_ms,
                })
                .unwrap(),
            );
            // Lengths only. Text never logs.
            println!(
                "call id={} provider={} status={:?} tokens={}+{} ms={} mock={}",
                chunk.id,
                result.provider,
                result.status,
                result.tokens_in,
                result.tokens_out,
                result.latency_ms,
                result.mock
            );
        }
    }
    std::fs::write(&results_path, result_lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    std::fs::write(&usage_path, usage_lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(summary)
}

/// Spend budget. Calls reserve exactly. Tokens reserve an
/// estimate, then settle to metered totals on completion.
#[derive(Debug, Default)]
struct Budget {
    /// Reserved requests.
    calls: u64,
    /// Reserved tokens.
    tokens: u64,
}

/// Requests one chunk spends. Race spends two.
fn requests_per_chunk(cfg: &Config) -> usize {
    if cfg.provider == Provider::Both && cfg.both == BothMode::Race {
        2
    } else {
        1
    }
}

/// Admit one chunk. Reserves budget. False stops the run.
/// Zero caps mean uncapped.
fn admit(cfg: &Config, budget: &Arc<std::sync::Mutex<Budget>>, requests: u64, est: u64) -> bool {
    let mut budget = budget.lock().unwrap();
    if cfg.max_calls > 0 && budget.calls + requests > cfg.max_calls {
        return false;
    }
    if cfg.max_tokens > 0 && budget.tokens + est > cfg.max_tokens {
        return false;
    }
    budget.calls += requests;
    budget.tokens += est;
    true
}

/// Settle reserved estimate to metered totals.
fn settle(budget: &Arc<std::sync::Mutex<Budget>>, est: u64, rows: &[(Chunk, CallResult)]) {
    let actual: u64 = rows.iter().map(|(_, r)| r.tokens_in + r.tokens_out).sum();
    let mut budget = budget.lock().unwrap();
    budget.tokens = budget.tokens.saturating_sub(est).saturating_add(actual);
}

/// One dispatch unit. Race holds two clients.
struct Job {
    /// Chunk to send.
    chunk: Chunk,
    /// Task prompt.
    task: Task,
    /// Clients to call. Race calls both.
    clients: Vec<ProviderClient>,
    /// True for race mode.
    race: bool,
}

impl Job {
    /// Run the job. Race keeps the first win, records both.
    async fn run(self) -> Vec<(Chunk, CallResult)> {
        if !self.race {
            let result = self.clients[0].call(self.task, &self.chunk.text).await;
            return vec![(self.chunk, result)];
        }
        let (a, b) = tokio::join!(
            self.clients[0].call(self.task, &self.chunk.text),
            self.clients[1].call(self.task, &self.chunk.text)
        );
        let mut rows = vec![];
        let a_wins = a.status == CallStatus::Ok;
        let (winner, mut loser) = if a_wins { (a, b) } else { (b, a) };
        if winner.status == CallStatus::Ok {
            loser.status = CallStatus::Lost;
        }
        rows.push((self.chunk.clone(), winner));
        rows.push((self.chunk, loser));
        rows
    }
}

/// Build one job from the plan. Fails closed on missing clients.
fn dispatch_job(
    cfg: &Config,
    i: usize,
    chunk: Chunk,
    baseten: &Option<ProviderClient>,
    modal: &Option<ProviderClient>,
) -> Result<Job, String> {
    let mock = ProviderClient::mock();
    let pick = |label: &str| -> Result<ProviderClient, String> {
        match label {
            "baseten" => baseten
                .clone()
                .ok_or("Baseten client is missing".to_string()),
            "modal" => modal.clone().ok_or("Modal client is missing".to_string()),
            _ => Ok(mock.clone()),
        }
    };
    match cfg.provider {
        Provider::Baseten => Ok(Job {
            chunk,
            task: cfg.task,
            clients: vec![pick("baseten")?],
            race: false,
        }),
        Provider::Modal => Ok(Job {
            chunk,
            task: cfg.task,
            clients: vec![pick("modal")?],
            race: false,
        }),
        Provider::Mock => Ok(Job {
            chunk,
            task: cfg.task,
            clients: vec![mock],
            race: false,
        }),
        Provider::Both => match cfg.both {
            BothMode::Split => {
                let label = if i.is_multiple_of(2) {
                    "baseten"
                } else {
                    "modal"
                };
                Ok(Job {
                    chunk,
                    task: cfg.task,
                    clients: vec![pick(label)?],
                    race: false,
                })
            }
            BothMode::Race => Ok(Job {
                chunk,
                task: cfg.task,
                clients: vec![pick("baseten")?, pick("modal")?],
                race: true,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test config. Mock provider, tiny caps.
    fn test_config() -> Config {
        Config {
            input: "fixtures/ai-input".into(),
            provider: Provider::Mock,
            both: BothMode::Split,
            task: Task::Summary,
            concurrency: 4,
            max_calls: 0,
            max_tokens: 0,
            out: std::env::temp_dir(),
            baseten_key: None,
            baseten_model: None,
            baseten_base: "https://inference.baseten.co/v1".to_string(),
            modal_id: None,
            modal_url: None,
            modal_model: None,
        }
    }

    /// Test chunks. Plain public words.
    fn test_chunks(n: usize) -> Vec<Chunk> {
        (0..n)
            .map(|i| Chunk {
                id: format!("f.txt#{i}"),
                file: "f.txt".to_string(),
                index: i,
                text: "Public words for the batch test run.".to_string(),
            })
            .collect()
    }

    #[test]
    fn plan_splits_and_races() {
        let mut cfg = test_config();
        cfg.provider = Provider::Both;
        cfg.both = BothMode::Split;
        let plan = plan_calls(&test_chunks(4), &cfg);
        assert_eq!(plan.len(), 4);
        assert_eq!(plan[0].provider, "baseten");
        assert_eq!(plan[1].provider, "modal");
        cfg.both = BothMode::Race;
        let plan = plan_calls(&test_chunks(4), &cfg);
        assert_eq!(plan.len(), 8);
    }

    #[tokio::test]
    async fn cap_stop_limits_runs() {
        let mut cfg = test_config();
        cfg.max_calls = 3;
        cfg.out = std::env::temp_dir().join(format!("aib-cap-{}", std::process::id()));
        let summary = run_batch(&cfg, test_chunks(10), None, None).await.unwrap();
        assert_eq!(summary.ran, 3);
        assert!(summary.capped);
        assert_eq!(summary.skipped_cap, 7);
        let _ = std::fs::remove_dir_all(&cfg.out);
    }

    #[tokio::test]
    async fn serial_run_completes_all() {
        let mut cfg = test_config();
        cfg.concurrency = 1;
        cfg.out = std::env::temp_dir().join(format!("aib-ser-{}", std::process::id()));
        let summary = run_batch(&cfg, test_chunks(5), None, None).await.unwrap();
        assert_eq!(summary.ran, 5);
        assert_eq!(summary.ok, 5);
        assert!(!summary.capped);
        let _ = std::fs::remove_dir_all(&cfg.out);
    }

    #[tokio::test]
    async fn usage_schema_holds_no_text_or_keys() {
        let mut cfg = test_config();
        cfg.out = std::env::temp_dir().join(format!("aib-sch-{}", std::process::id()));
        run_batch(&cfg, test_chunks(1), None, None).await.unwrap();
        let usage = std::fs::read_to_string(cfg.out.join("usage.jsonl")).unwrap();
        let mut keys: Vec<String> = vec![];
        for line in usage.lines() {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            keys.extend(row.as_object().unwrap().keys().cloned());
        }
        keys.sort();
        keys.dedup();
        assert_eq!(
            keys,
            vec![
                "at_ms",
                "file",
                "id",
                "latency_ms",
                "mock",
                "model",
                "provider",
                "status",
                "task",
                "tokens_in",
                "tokens_out",
            ]
        );
        let _ = std::fs::remove_dir_all(&cfg.out);
    }

    #[tokio::test]
    async fn mock_rows_stay_labelled() {
        let mut cfg = test_config();
        cfg.out = std::env::temp_dir().join(format!("aib-mock-{}", std::process::id()));
        let summary = run_batch(&cfg, test_chunks(2), None, None).await.unwrap();
        assert_eq!(summary.ok, 2);
        let usage = std::fs::read_to_string(cfg.out.join("usage.jsonl")).unwrap();
        for line in usage.lines() {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(row["mock"], true);
            assert_eq!(row["provider"], "mock");
            assert!(row.get("output").is_none());
            assert!(row.get("text").is_none());
        }
        let _ = std::fs::remove_dir_all(&cfg.out);
    }
}

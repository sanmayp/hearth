//! Batch. It fans chunks out with caps and meters all.
//! It writes results plus usage. Text never reaches usage rows.

use crate::chunk::Chunk;
use crate::config::{Config, Provider, Task};
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

/// Build the call plan. One call per chunk.
pub fn plan_calls(chunks: &[Chunk], cfg: &Config) -> Vec<PlannedCall> {
    let label = match cfg.provider {
        Provider::Remote => "remote",
        Provider::Mock => "mock",
    };
    chunks.iter().map(|chunk| one(chunk, label)).collect()
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
    remote: Option<ProviderClient>,
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
    for chunk in chunks.into_iter() {
        let est = crate::chunk::estimate_tokens(&chunk.text);
        if !admit(cfg, &budget, 1, est) {
            summary.capped = true;
            summary.skipped_cap += 1;
            continue;
        }
        let permit = sem.clone().acquire_owned().await.unwrap();
        let job = dispatch_job(cfg, chunk, &remote)?;
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

/// One dispatch unit. One chunk, one client.
struct Job {
    /// Chunk to send.
    chunk: Chunk,
    /// Task prompt.
    task: Task,
    /// Client to call.
    client: ProviderClient,
}

impl Job {
    /// Run the job. One call, one row.
    async fn run(self) -> Vec<(Chunk, CallResult)> {
        let result = self.client.call(self.task, &self.chunk.text).await;
        vec![(self.chunk, result)]
    }
}

/// Build one job from the plan. Fails closed on missing clients.
fn dispatch_job(
    cfg: &Config,
    chunk: Chunk,
    remote: &Option<ProviderClient>,
) -> Result<Job, String> {
    let client = match cfg.provider {
        Provider::Remote => remote
            .clone()
            .ok_or("Remote client is missing".to_string())?,
        Provider::Mock => ProviderClient::mock(),
    };
    Ok(Job {
        chunk,
        task: cfg.task,
        client,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test config. Mock provider, tiny caps.
    fn test_config() -> Config {
        Config {
            input: "fixtures/ai-input".into(),
            provider: Provider::Mock,
            task: Task::Summary,
            concurrency: 4,
            max_calls: 0,
            max_tokens: 0,
            out: std::env::temp_dir(),
            remote_url: None,
            remote_key: None,
            remote_model: None,
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
    fn plan_labels_one_call_per_chunk() {
        let mut cfg = test_config();
        cfg.provider = Provider::Remote;
        let plan = plan_calls(&test_chunks(4), &cfg);
        assert_eq!(plan.len(), 4);
        assert!(plan.iter().all(|p| p.provider == "remote"));
        cfg.provider = Provider::Mock;
        let plan = plan_calls(&test_chunks(4), &cfg);
        assert_eq!(plan.len(), 4);
        assert!(plan.iter().all(|p| p.provider == "mock"));
    }

    #[tokio::test]
    async fn cap_stop_limits_runs() {
        let mut cfg = test_config();
        cfg.max_calls = 3;
        cfg.out = std::env::temp_dir().join(format!("aib-cap-{}", std::process::id()));
        let summary = run_batch(&cfg, test_chunks(10), None).await.unwrap();
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
        let summary = run_batch(&cfg, test_chunks(5), None).await.unwrap();
        assert_eq!(summary.ran, 5);
        assert_eq!(summary.ok, 5);
        assert!(!summary.capped);
        let _ = std::fs::remove_dir_all(&cfg.out);
    }

    #[tokio::test]
    async fn usage_schema_holds_no_text_or_keys() {
        let mut cfg = test_config();
        cfg.out = std::env::temp_dir().join(format!("aib-sch-{}", std::process::id()));
        run_batch(&cfg, test_chunks(1), None).await.unwrap();
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
        let summary = run_batch(&cfg, test_chunks(2), None).await.unwrap();
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

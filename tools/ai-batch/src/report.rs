//! Report. It renders usage.jsonl as markdown.
//! Numbers come from the file only. Never hand-written.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// One usage row, as the batch wrote it.
#[derive(Debug, Deserialize)]
struct Row {
    /// Provider label.
    provider: String,
    /// Input tokens.
    tokens_in: u64,
    /// Output tokens.
    tokens_out: u64,
    /// Wall ms.
    latency_ms: u64,
    /// Outcome word.
    status: String,
    /// True for mock rows.
    mock: bool,
}

/// Totals for one provider.
#[derive(Debug, Default)]
struct Totals {
    /// Calls counted.
    calls: u64,
    /// Live calls.
    live: u64,
    /// Mock calls.
    mock: u64,
    /// Errors.
    errors: u64,
    /// Input tokens.
    tokens_in: u64,
    /// Output tokens.
    tokens_out: u64,
    /// Ok latencies for percentiles.
    latencies: Vec<u64>,
}

/// Percentile by nearest rank. Empty gives zero.
fn percentile(sorted: &[u64], pct: u64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((pct as usize * sorted.len()).div_ceil(100)).saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

/// Render the report. Missing file means zeros, never an error.
pub fn generate_report(usage_path: &Path, out_path: &Path) -> Result<(), String> {
    let mut by_provider: BTreeMap<String, Totals> = BTreeMap::new();
    let mut skipped = 0u64;
    if let Ok(text) = std::fs::read_to_string(usage_path) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let row: Row = match serde_json::from_str(line) {
                Ok(row) => row,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let totals = by_provider.entry(row.provider).or_default();
            totals.calls += 1;
            if row.mock {
                totals.mock += 1;
            } else {
                totals.live += 1;
            }
            if row.status != "ok" {
                totals.errors += 1;
            } else {
                totals.latencies.push(row.latency_ms);
            }
            totals.tokens_in += row.tokens_in;
            totals.tokens_out += row.tokens_out;
        }
    }
    let mut live_calls = 0u64;
    let mut out = String::from("# AI usage report\n\n");
    out.push_str("Generated from usage.jsonl. Never hand-written.\n\n");
    if by_provider.is_empty() {
        out.push_str("No runs yet. Live calls so far: 0.\n");
    } else {
        out.push_str("| Provider | Calls | Live | Mock | Tokens in | Tokens out | p50 ms | p95 ms | Errors |\n");
        out.push_str("|---|---|---|---|---|---|---|---|---|\n");
        for (provider, totals) in &by_provider {
            let mut latencies = totals.latencies.clone();
            latencies.sort_unstable();
            live_calls += totals.live;
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                provider,
                totals.calls,
                totals.live,
                totals.mock,
                totals.tokens_in,
                totals.tokens_out,
                percentile(&latencies, 50),
                percentile(&latencies, 95),
                totals.errors
            ));
        }
        out.push_str(&format!("\nLive calls in the last run: {live_calls}.\n"));
    }
    if skipped > 0 {
        out.push_str(&format!("\nSkipped {skipped} bad lines.\n"));
    }
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(out_path, out).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_totals_come_from_rows() {
        let dir = std::env::temp_dir().join(format!("aib-rep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let usage = dir.join("usage.jsonl");
        std::fs::write(
            &usage,
            r#"{"provider":"baseten","tokens_in":10,"tokens_out":5,"latency_ms":100,"status":"ok","mock":false}
{"provider":"baseten","tokens_in":10,"tokens_out":5,"latency_ms":300,"status":"ok","mock":false}
{"provider":"mock","tokens_in":4,"tokens_out":0,"latency_ms":1,"status":"ok","mock":true}
not-json
"#,
        )
        .unwrap();
        let out = dir.join("USAGE-REPORT.md");
        generate_report(&usage, &out).unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("| baseten | 2 | 2 | 0 | 20 | 10 | 100 | 300 | 0 |"));
        assert!(text.contains("| mock | 1 | 0 | 1 | 4 | 0 | 1 | 1 | 0 |"));
        assert!(text.contains("Live calls in the last run: 2."));
        assert!(text.contains("Skipped 1 bad lines."));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_gives_zeros() {
        let dir = std::env::temp_dir().join(format!("aib-rep0-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("USAGE-REPORT.md");
        generate_report(&dir.join("usage.jsonl"), &out).unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("Live calls so far: 0."));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! Provider. It calls one endpoint per chunk.
//! Keys ride headers only. URLs pass validation first.

use crate::config::{validate_url, Provider, Task};
use serde::{Deserialize, Serialize};

/// Call outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CallStatus {
    /// Reply parsed.
    Ok,
    /// Transport, HTTP, or parse failure.
    Error,
}

/// One call result. It holds no keys and no prompts.
#[derive(Debug, Clone)]
pub struct CallResult {
    /// Provider name for the row.
    pub provider: &'static str,
    /// Model name for the row.
    pub model: String,
    /// Reply text. Empty on error.
    pub output: String,
    /// Metered or estimated input tokens.
    pub tokens_in: u64,
    /// Metered or estimated output tokens.
    pub tokens_out: u64,
    /// Wall time in ms, retries included.
    pub latency_ms: u64,
    /// Outcome.
    pub status: CallStatus,
    /// Short error code. Never holds secrets.
    pub error: Option<String>,
    /// True for mock rows. Never mixed with live.
    pub mock: bool,
    /// Attempt count. Retry adds up.
    pub attempts: u32,
}

/// Provider client. Built once per run from validated config.
#[derive(Clone)]
pub struct ProviderClient {
    /// Which endpoint answers.
    pub provider: Provider,
    /// Validated base URL. None for mock.
    pub base_url: Option<String>,
    /// Key. Used once per request. Never logged or stored.
    key: Option<String>,
    /// Model name for the body.
    pub model: String,
    /// HTTP client with a timeout.
    http: Option<reqwest::Client>,
}

/// Debug hides the key. Logs stay safe by default.
impl std::fmt::Debug for ProviderClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderClient")
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("key", &self.key.as_ref().map(|_| "[redacted]"))
            .field("model", &self.model)
            .finish()
    }
}

impl ProviderClient {
    /// Build a remote client. Fails closed without key or model.
    pub fn remote(base: &str, key: &str, model: &str) -> Result<Self, String> {
        let base_url = validate_url(base)?;
        if key.is_empty() {
            return Err("Remote key is empty".to_string());
        }
        if model.is_empty() {
            return Err("Remote model is empty".to_string());
        }
        Ok(Self {
            provider: Provider::Remote,
            base_url: Some(base_url),
            key: Some(key.to_string()),
            model: model.to_string(),
            http: Some(client()),
        })
    }

    /// Build the mock client. It needs nothing.
    pub fn mock() -> Self {
        Self {
            provider: Provider::Mock,
            base_url: None,
            key: None,
            model: "mock-extract".to_string(),
            http: None,
        }
    }

    /// Provider label for rows.
    pub fn label(&self) -> &'static str {
        match self.provider {
            Provider::Remote => "remote",
            Provider::Mock => "mock",
        }
    }

    /// Call once with retries. Backoff waits 0.5s, 2s, 8s.
    pub async fn call(&self, task: Task, text: &str) -> CallResult {
        if self.provider == Provider::Mock {
            return mock_call(text);
        }
        let start = std::time::Instant::now();
        let mut attempts = 0;
        loop {
            attempts += 1;
            match self.call_once(task, text).await {
                Ok(mut result) => {
                    result.attempts = attempts;
                    result.latency_ms = start.elapsed().as_millis() as u64;
                    return result;
                }
                Err((retry, code)) => {
                    if !retry || attempts >= 3 {
                        return CallResult {
                            provider: self.label(),
                            model: self.model.clone(),
                            output: String::new(),
                            tokens_in: 0,
                            tokens_out: 0,
                            latency_ms: start.elapsed().as_millis() as u64,
                            status: CallStatus::Error,
                            error: Some(code),
                            mock: false,
                            attempts,
                        };
                    }
                    let wait = backoff_ms(attempts - 1);
                    tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                }
            }
        }
    }

    /// One HTTP attempt. Returns reply or (retry, code).
    async fn call_once(&self, task: Task, text: &str) -> Result<CallResult, (bool, String)> {
        let base = self.base_url.as_ref().unwrap();
        let url = format!("{base}/chat/completions");
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 300,
            "messages": [
                {"role": "system", "content": system_prompt(task)},
                {"role": "user", "content": text},
            ],
        });
        let resp = self
            .http
            .as_ref()
            .unwrap()
            .post(&url)
            .bearer_auth(self.key.as_ref().unwrap())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    (true, "timeout".to_string())
                } else {
                    (true, "transport".to_string())
                }
            })?;
        let status = resp.status();
        if should_retry_status(status.as_u16()) {
            return Err((true, format!("http{}", status.as_u16())));
        }
        if !status.is_success() {
            return Err((false, format!("http{}", status.as_u16())));
        }
        let value: Reply = resp
            .json()
            .await
            .map_err(|_| (false, "badjson".to_string()))?;
        let output = value
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();
        let (tokens_in, tokens_out) = match value.usage {
            Some(u) => (u.prompt_tokens, u.completion_tokens),
            None => (crate::chunk::estimate_tokens(text), 0),
        };
        Ok(CallResult {
            provider: self.label(),
            model: self.model.clone(),
            output,
            tokens_in,
            tokens_out,
            latency_ms: 0,
            status: CallStatus::Ok,
            error: None,
            mock: false,
            attempts: 0,
        })
    }
}

/// True when an HTTP status deserves a retry. 429 plus 5xx only.
pub fn should_retry_status(code: u16) -> bool {
    code == 429 || (500..600).contains(&code)
}

/// Backoff wait in ms. Attempt counts from zero.
pub fn backoff_ms(attempt: u32) -> u64 {
    500 * 4u64.pow(attempt.min(2))
}

/// HTTP client. One timeout for all calls. TLS verified.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .expect("http client builds")
}

/// System prompt per task. Short and plain.
pub fn system_prompt(task: Task) -> &'static str {
    match task {
        Task::Summary => "Summarize the user text in at most 3 short plain sentences.",
        Task::Explain => "Explain this work log in at most 5 short plain sentences.",
        Task::Suppliers => "List supplier and customer sentences from the user text, one per line.",
        Task::Digest => "Digest these agent log lines into at most 5 short plain bullets.",
    }
}

/// Mock call. Deterministic extract. Labelled mock always.
fn mock_call(text: &str) -> CallResult {
    let start = std::time::Instant::now();
    let first: String = text
        .split(['.', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join(". ");
    CallResult {
        provider: "mock",
        model: "mock-extract".to_string(),
        output: first,
        tokens_in: crate::chunk::estimate_tokens(text),
        tokens_out: 0,
        latency_ms: start.elapsed().as_millis() as u64,
        status: CallStatus::Ok,
        error: None,
        mock: true,
        attempts: 1,
    }
}

/// Chat-completions-shaped reply. Usage may be absent.
#[derive(Debug, Deserialize)]
struct Reply {
    /// Choices. First one wins.
    #[serde(default)]
    choices: Vec<Choice>,
    /// Token meter. None means estimate.
    #[serde(default)]
    usage: Option<Usage>,
}

/// One choice.
#[derive(Debug, Deserialize)]
struct Choice {
    /// Message. Content may be absent.
    #[serde(default)]
    message: Message,
}

/// Message content.
#[derive(Debug, Default, Deserialize)]
struct Message {
    /// Reply text.
    #[serde(default)]
    content: Option<String>,
}

/// Token meter.
#[derive(Debug, Deserialize)]
struct Usage {
    /// Input tokens.
    #[serde(default)]
    prompt_tokens: u64,
    /// Output tokens.
    #[serde(default)]
    completion_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_policy_covers_429_and_5xx() {
        assert!(should_retry_status(429));
        assert!(should_retry_status(500));
        assert!(should_retry_status(503));
        assert!(!should_retry_status(400));
        assert!(!should_retry_status(401));
        assert!(!should_retry_status(200));
        assert_eq!(backoff_ms(0), 500);
        assert_eq!(backoff_ms(1), 2000);
        assert_eq!(backoff_ms(2), 8000);
    }

    #[test]
    fn debug_hides_keys() {
        let client = ProviderClient::remote("https://ai.example.com/v1", "livekey9", "m").unwrap();
        let shown = format!("{client:?}");
        assert!(!shown.contains("livekey9"));
        assert!(shown.contains("[redacted]"));
    }

    #[test]
    fn clients_fail_closed_without_creds() {
        assert!(ProviderClient::remote("https://ai.example.com/v1", "", "m").is_err());
        assert!(ProviderClient::remote("https://ai.example.com/v1", "k", "").is_err());
        assert!(ProviderClient::remote("http://ai.example.com/v1", "k", "m").is_err());
        assert!(ProviderClient::remote("https://ai.example.com/v1", "k", "m").is_ok());
    }

    #[tokio::test]
    async fn mock_call_is_labelled_and_deterministic() {
        let client = ProviderClient::mock();
        let a = client.call(Task::Summary, "First line. Second line.").await;
        let b = client.call(Task::Summary, "First line. Second line.").await;
        assert!(a.mock);
        assert_eq!(a.output, b.output);
        assert_eq!(a.status, CallStatus::Ok);
        assert!(a.tokens_in > 0);
    }
}

//! Config. It parses args plus env. No keys print here.

use std::path::PathBuf;

/// Which endpoint answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// A generic OpenAI-compatible endpoint from env.
    Remote,
    /// Local fake. Labelled mock, never live.
    Mock,
}

/// Prompt task. It picks the system prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    /// Short summary of the chunk.
    Summary,
    /// Explain one lane in plain words.
    Explain,
    /// Supplier and customer sentences as evidence.
    Suppliers,
    /// Digest of agent log lines.
    Digest,
}

/// Batch config. Built from args, then env, then defaults.
#[derive(Clone)]
pub struct Config {
    /// Input folder of public text files.
    pub input: PathBuf,
    /// Provider to call.
    pub provider: Provider,
    /// Prompt task.
    pub task: Task,
    /// Live parallel calls. Clamped 1 to 32.
    pub concurrency: usize,
    /// Max requests per run. 0 means uncapped.
    pub max_calls: u64,
    /// Max metered tokens per run. 0 means uncapped.
    pub max_tokens: u64,
    /// Output folder for results.jsonl plus usage.jsonl.
    pub out: PathBuf,
    /// Remote base URL from HEARTH_AI_URL.
    pub remote_url: Option<String>,
    /// Remote key. Present only, never printed.
    pub remote_key: Option<String>,
    /// Remote model name from HEARTH_AI_MODEL.
    pub remote_model: Option<String>,
}

/// Debug hides keys. Logs stay safe by default.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("input", &self.input)
            .field("provider", &self.provider)
            .field("task", &self.task)
            .field("concurrency", &self.concurrency)
            .field("max_calls", &self.max_calls)
            .field("max_tokens", &self.max_tokens)
            .field("out", &self.out)
            .field("remote_url", &self.remote_url)
            .field(
                "remote_key",
                &self.remote_key.as_ref().map(|_| "[redacted]"),
            )
            .field("remote_model", &self.remote_model)
            .finish()
    }
}

/// Parse CLI args. First word may be `run` or `report`.
pub fn parse_args(argv: &[String]) -> Result<Command, String> {
    let mut args = argv.iter().skip(1);
    let first = args.next().map(|s| s.as_str()).unwrap_or("run");
    if first == "report" {
        let mut usage: Option<PathBuf> = None;
        let mut out: Option<PathBuf> = None;
        for arg in args {
            if let Some(v) = arg.strip_prefix("--usage=") {
                usage = Some(PathBuf::from(v));
            } else if let Some(v) = arg.strip_prefix("--out=") {
                out = Some(PathBuf::from(v));
            } else {
                return Err(format!("Bad report arg: {arg}"));
            }
        }
        return Ok(Command::Report {
            usage: usage.ok_or("report needs --usage=FILE")?,
            out: out.ok_or("report needs --out=FILE")?,
        });
    }
    if first != "run" {
        return Err("First word must be run or report.".to_string());
    }
    let mut cfg = Config {
        input: PathBuf::from("fixtures/ai-input"),
        provider: Provider::Mock,
        task: Task::Summary,
        concurrency: 8,
        max_calls: 200,
        max_tokens: 500_000,
        out: PathBuf::from("out/ai-batch"),
        remote_url: std::env::var("HEARTH_AI_URL").ok(),
        remote_key: std::env::var("HEARTH_AI_KEY").ok(),
        remote_model: std::env::var("HEARTH_AI_MODEL").ok(),
    };
    if let Ok(v) = std::env::var("HEARTH_AI_MAX_CALLS") {
        cfg.max_calls = v.parse().map_err(|_| "HEARTH_AI_MAX_CALLS is bad")?;
    }
    if let Ok(v) = std::env::var("HEARTH_AI_MAX_TOKENS") {
        cfg.max_tokens = v.parse().map_err(|_| "HEARTH_AI_MAX_TOKENS is bad")?;
    }
    let mut dry_run = false;
    for arg in args {
        if let Some(v) = arg.strip_prefix("--input=") {
            cfg.input = PathBuf::from(v);
        } else if let Some(v) = arg.strip_prefix("--provider=") {
            cfg.provider = match v {
                "remote" => Provider::Remote,
                "mock" => Provider::Mock,
                _ => return Err(format!("Bad provider: {v}")),
            };
        } else if let Some(v) = arg.strip_prefix("--task=") {
            cfg.task = match v {
                "summary" => Task::Summary,
                "explain" => Task::Explain,
                "suppliers" => Task::Suppliers,
                "digest" => Task::Digest,
                _ => return Err(format!("Bad task: {v}")),
            };
        } else if let Some(v) = arg.strip_prefix("--concurrency=") {
            cfg.concurrency = v.parse().map_err(|_| "Bad concurrency")?;
        } else if let Some(v) = arg.strip_prefix("--max-calls=") {
            cfg.max_calls = v.parse().map_err(|_| "Bad max-calls")?;
        } else if let Some(v) = arg.strip_prefix("--max-tokens=") {
            cfg.max_tokens = v.parse().map_err(|_| "Bad max-tokens")?;
        } else if let Some(v) = arg.strip_prefix("--out=") {
            cfg.out = PathBuf::from(v);
        } else if arg == "--dry-run" {
            dry_run = true;
        } else {
            return Err(format!("Bad arg: {arg}"));
        }
    }
    cfg.concurrency = cfg.concurrency.clamp(1, 32);
    Ok(Command::Run {
        config: cfg,
        dry_run,
    })
}

/// Top-level command.
#[derive(Debug)]
pub enum Command {
    /// Run the batch.
    Run {
        /// Batch config.
        config: Config,
        /// Print the plan only.
        dry_run: bool,
    },
    /// Render usage.jsonl as markdown.
    Report {
        /// Input usage file.
        usage: PathBuf,
        /// Output markdown file.
        out: PathBuf,
    },
}

/// Validate a remote URL. Requires https, rejects userinfo.
/// The host is the owner's own endpoint. No vendor list lives here.
pub fn validate_url(url: &str) -> Result<String, String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or("URL must start with https://")?;
    if rest.contains('@') {
        return Err("URL must not hold userinfo".to_string());
    }
    let host_end = rest.find('/').unwrap_or(rest.len());
    let (host_port, _) = rest.split_at(host_end);
    let host = host_port
        .rsplit_once(':')
        .map(|(h, _)| h)
        .unwrap_or(host_port);
    if host.is_empty() {
        return Err("URL host is empty".to_string());
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_keys() {
        let argv: Vec<String> = ["ai-batch", "run"].iter().map(|s| s.to_string()).collect();
        let Command::Run { mut config, .. } = parse_args(&argv).unwrap() else {
            panic!("wanted run");
        };
        config.remote_key = Some("livekey9".to_string());
        let shown = format!("{config:?}");
        assert!(!shown.contains("livekey9"));
        assert!(shown.contains("[redacted]"));
    }

    #[test]
    fn concurrency_clamps_to_range() {
        let argv: Vec<String> = ["ai-batch", "run", "--concurrency=99"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let Command::Run { config, .. } = parse_args(&argv).unwrap() else {
            panic!("wanted run");
        };
        assert_eq!(config.concurrency, 32);
        let argv: Vec<String> = ["ai-batch", "run", "--concurrency=0"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let Command::Run { config, .. } = parse_args(&argv).unwrap() else {
            panic!("wanted run");
        };
        assert_eq!(config.concurrency, 1);
    }

    #[test]
    fn provider_flag_names_remote_or_mock() {
        for (flag, want) in [("remote", Provider::Remote), ("mock", Provider::Mock)] {
            let argv: Vec<String> = ["ai-batch", "run", &format!("--provider={flag}")]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let Command::Run { config, .. } = parse_args(&argv).unwrap() else {
                panic!("wanted run");
            };
            assert_eq!(config.provider, want);
        }
        let argv: Vec<String> = ["ai-batch", "run", "--provider=both"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&argv).is_err());
    }

    #[test]
    fn remote_urls_need_https_without_userinfo() {
        assert!(validate_url("https://ai.example.com/v1").is_ok());
        assert!(validate_url("https://ai.example.com:8443/v1").is_ok());
        assert!(validate_url("http://ai.example.com/v1").is_err());
        let userinfo = ["https", "://", "user", "@", "ai.example.com", "/v1"].concat();
        assert!(validate_url(&userinfo).is_err());
        assert!(validate_url("https:///v1").is_err());
    }
}

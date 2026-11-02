//! Config. It parses args plus env. No keys print here.

use std::path::PathBuf;

/// Which endpoint answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Baseten Model APIs or a Baseten deploy.
    Baseten,
    /// A Modal endpoint URL the owner pastes in.
    Modal,
    /// Local fake. Labelled mock, never live.
    Mock,
    /// Both live providers per call plan.
    Both,
}

/// How Both mode spends calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BothMode {
    /// Alternate providers. One request per chunk.
    Split,
    /// Ask both, keep the first win. Two requests per chunk.
    Race,
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
    /// Both-mode shape.
    pub both: BothMode,
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
    /// Baseten key. Present only, never printed.
    pub baseten_key: Option<String>,
    /// Baseten model slug.
    pub baseten_model: Option<String>,
    /// Baseten base URL. Vendor default, overridable.
    pub baseten_base: String,
    /// Modal proxy id plus secret. Present only, never printed.
    pub modal_id: Option<String>,
    /// Modal endpoint URL. Owner pastes it. No default.
    pub modal_url: Option<String>,
    /// Modal model name for the body.
    pub modal_model: Option<String>,
}

/// Debug hides keys. Logs stay safe by default.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("input", &self.input)
            .field("provider", &self.provider)
            .field("both", &self.both)
            .field("task", &self.task)
            .field("concurrency", &self.concurrency)
            .field("max_calls", &self.max_calls)
            .field("max_tokens", &self.max_tokens)
            .field("out", &self.out)
            .field(
                "baseten_key",
                &self.baseten_key.as_ref().map(|_| "[redacted]"),
            )
            .field("baseten_model", &self.baseten_model)
            .field("baseten_base", &self.baseten_base)
            .field("modal_id", &self.modal_id.as_ref().map(|_| "[redacted]"))
            .field("modal_url", &self.modal_url)
            .field("modal_model", &self.modal_model)
            .finish()
    }
}

/// Vendor default base. Lives in vendor docs, not owner infra.
pub const BASETEN_DEFAULT_BASE: &str = "https://inference.baseten.co/v1";

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
    let mut dry_run = false;
    let mut cfg = Config {
        input: PathBuf::from("fixtures/ai-input"),
        provider: Provider::Mock,
        both: BothMode::Split,
        task: Task::Summary,
        concurrency: 8,
        max_calls: 200,
        max_tokens: 500_000,
        out: PathBuf::from("out/ai-batch"),
        baseten_key: std::env::var("BASETEN_API_KEY").ok(),
        baseten_model: std::env::var("BASETEN_MODEL").ok(),
        baseten_base: std::env::var("BASETEN_BASE_URL")
            .unwrap_or_else(|_| BASETEN_DEFAULT_BASE.to_string()),
        modal_id: None,
        modal_url: std::env::var("MODAL_MODEL_URL").ok(),
        modal_model: std::env::var("MODAL_MODEL").ok(),
    };
    if let (Ok(id), Ok(secret)) = (
        std::env::var("MODAL_PROXY_TOKEN_ID"),
        std::env::var("MODAL_PROXY_TOKEN_SECRET"),
    ) {
        cfg.modal_id = Some(format!("{id}.{secret}"));
    }
    if let Ok(v) = std::env::var("HEARTH_AI_MAX_CALLS") {
        cfg.max_calls = v.parse().map_err(|_| "HEARTH_AI_MAX_CALLS is bad")?;
    }
    if let Ok(v) = std::env::var("HEARTH_AI_MAX_TOKENS") {
        cfg.max_tokens = v.parse().map_err(|_| "HEARTH_AI_MAX_TOKENS is bad")?;
    }
    for arg in args {
        if let Some(v) = arg.strip_prefix("--input=") {
            cfg.input = PathBuf::from(v);
        } else if let Some(v) = arg.strip_prefix("--provider=") {
            cfg.provider = match v {
                "baseten" => Provider::Baseten,
                "modal" => Provider::Modal,
                "mock" => Provider::Mock,
                "both" => Provider::Both,
                _ => return Err(format!("Bad provider: {v}")),
            };
        } else if let Some(v) = arg.strip_prefix("--both=") {
            cfg.both = match v {
                "split" => BothMode::Split,
                "race" => BothMode::Race,
                _ => return Err(format!("Bad both mode: {v}")),
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

/// Validate a provider URL. Only vendor hosts pass.
/// Rejects http, userinfo, odd ports, and lookalike hosts.
pub fn validate_url(url: &str) -> Result<String, String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or("URL must start with https://")?;
    if rest.contains('@') {
        return Err("URL must not hold userinfo".to_string());
    }
    let host_end = rest.find('/').unwrap_or(rest.len());
    let (host_port, _) = rest.split_at(host_end);
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => {
            if p != "443" {
                return Err("URL port must be 443 or absent".to_string());
            }
            (h, true)
        }
        None => (host_port, false),
    };
    let _ = port;
    if host.is_empty() {
        return Err("URL host is empty".to_string());
    }
    let ok = host == "inference.baseten.co"
        || host.ends_with(".api.baseten.co")
        || host.ends_with(".modal.run");
    if !ok {
        return Err("URL host is not an allowed provider".to_string());
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
        config.baseten_key = Some("livekey9".to_string());
        config.modal_id = Some("id.livekey8".to_string());
        let shown = format!("{config:?}");
        assert!(!shown.contains("livekey9"));
        assert!(!shown.contains("livekey8"));
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
    fn provider_urls_validate_strictly() {
        assert!(validate_url("https://inference.baseten.co/v1").is_ok());
        assert!(validate_url("https://model-abc.api.baseten.co/x").is_ok());
        assert!(validate_url("https://foo.modal.run/v1").is_ok());
        assert!(validate_url("https://inference.baseten.co:443/v1").is_ok());
        assert!(validate_url("http://inference.baseten.co/v1").is_err());
        let userinfo = ["https", "://", "user", "@", "inference.baseten.co", "/v1"].concat();
        assert!(validate_url(&userinfo).is_err());
        assert!(validate_url("https://evilmodal.run/v1").is_err());
        assert!(validate_url("https://evil.api.baseten.co.evil.com/v1").is_err());
        assert!(validate_url("https://inference.baseten.co:8443/v1").is_err());
        assert!(validate_url("https://127.0.0.1/v1").is_err());
        assert!(validate_url("https://example.com/v1").is_err());
    }
}

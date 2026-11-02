//! Config. It loads from env at start.
//! It refuses bad values with plain words.

use std::path::PathBuf;

/// Bridge config. It holds paths and the token.
#[derive(Clone)]
pub struct Config {
    /// Bearer token from the token file. It never logs.
    pub token: String,
    /// Allowlist file. Missing means empty. Empty blocks all.
    pub allowlist: Option<PathBuf>,
    /// Audit log file. Every send attempt appends one line.
    pub audit_log: PathBuf,
    /// herdr binary path. Tests point it at the fake.
    pub herdr: PathBuf,
    /// Extra agent kinds the owner allows. They use generic rules.
    pub extra_kinds: Vec<String>,
    /// Confirm code life in seconds. Default is 60.
    pub code_ttl_secs: u64,
    /// Port on 127.0.0.1. Default is 8449.
    pub port: u16,
    /// Allowed web origins for CORS. Exact match only. Never star.
    pub origins: Vec<String>,
}

/// Debug hides the token. Logs stay safe by default.
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("token", &"[redacted]")
            .field("allowlist", &self.allowlist)
            .field("audit_log", &self.audit_log)
            .field("herdr", &self.herdr)
            .field("extra_kinds", &self.extra_kinds)
            .field("code_ttl_secs", &self.code_ttl_secs)
            .field("port", &self.port)
            .field("origins", &self.origins)
            .finish()
    }
}

impl Config {
    /// Load from env. It errors when the token file is bad.
    pub fn load_from_env() -> Result<Self, String> {
        let token_path = std::env::var("AGENT_BRIDGE_TOKEN_FILE")
            .map_err(|_| "AGENT_BRIDGE_TOKEN_FILE is not set".to_string())?;
        let raw = std::fs::read_to_string(&token_path)
            .map_err(|_| format!("Token file is missing: {token_path}"))?;
        let token = raw.trim().to_string();
        if token.is_empty() {
            return Err("Token file is empty".to_string());
        }
        let allowlist = std::env::var("AGENT_BRIDGE_ALLOWLIST")
            .ok()
            .map(PathBuf::from);
        let audit_log = std::env::var("AGENT_BRIDGE_AUDIT_LOG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("agent-bridge-audit.jsonl"));
        let herdr = std::env::var("AGENT_BRIDGE_HERDR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("herdr"));
        let extra_kinds = std::env::var("AGENT_BRIDGE_EXTRA_KINDS")
            .unwrap_or_default()
            .split(',')
            .map(crate::kinds::normalize_kind)
            .filter(|k| !k.is_empty())
            .collect();
        let code_ttl_secs = read_u64("AGENT_BRIDGE_CODE_TTL", 60)?;
        let port = read_u16("AGENT_BRIDGE_PORT", 8449)?;
        let origins = read_origins()?;
        // Fail closed. A dead audit file refuses start.
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&audit_log)
            .map_err(|e| format!("Audit log is not writable: {e}"))?;
        Ok(Self {
            token,
            allowlist,
            audit_log,
            herdr,
            extra_kinds,
            code_ttl_secs,
            port,
            origins,
        })
    }
}

/// Default CORS origins. App origin first, then local shells.
/// Tauri webview origins stay in the default set. The phone app
/// runs there, so the bridge must answer its preflights.
pub fn default_origins() -> Vec<String> {
    [
        "https://example.ts.net:8447",
        "http://localhost",
        "https://localhost",
        "capacitor://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
        "tauri://localhost",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Read the CORS origin allowlist. Star refuses start. Never echo star.
fn read_origins() -> Result<Vec<String>, String> {
    let Ok(raw) = std::env::var("AGENT_BRIDGE_ORIGINS") else {
        return Ok(default_origins());
    };
    let mut out = vec![];
    for entry in raw.split(',') {
        let clean = entry.trim();
        if clean.is_empty() {
            continue;
        }
        if clean == "*" {
            return Err("AGENT_BRIDGE_ORIGINS must not hold star".to_string());
        }
        out.push(clean.to_string());
    }
    Ok(out)
}

/// Read a u64 env value. It falls back to the default.
fn read_u64(name: &str, default: u64) -> Result<u64, String> {
    match std::env::var(name) {
        Ok(v) => v.parse().map_err(|_| format!("{name} is not a number")),
        Err(_) => Ok(default),
    }
}

/// Read a u16 env value. It falls back to the default.
fn read_u16(name: &str, default: u16) -> Result<u16, String> {
    match std::env::var(name) {
        Ok(v) => v.parse().map_err(|_| format!("{name} is not a number")),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a closure with a clean bridge env. It restores old values.
    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK.lock().unwrap();
        let names = [
            "AGENT_BRIDGE_TOKEN_FILE",
            "AGENT_BRIDGE_ALLOWLIST",
            "AGENT_BRIDGE_AUDIT_LOG",
            "AGENT_BRIDGE_HERDR",
            "AGENT_BRIDGE_EXTRA_KINDS",
            "AGENT_BRIDGE_CODE_TTL",
            "AGENT_BRIDGE_PORT",
            "AGENT_BRIDGE_ORIGINS",
        ];
        let saved: Vec<(&str, Option<String>)> =
            names.iter().map(|n| (*n, std::env::var(n).ok())).collect();
        for n in &names {
            std::env::remove_var(n);
        }
        for (k, v) in vars {
            if let Some(val) = v {
                std::env::set_var(k, val);
            }
        }
        f();
        for n in &names {
            std::env::remove_var(n);
        }
        for (n, v) in saved {
            if let Some(val) = v {
                std::env::set_var(n, val);
            }
        }
    }

    /// Unique scratch dir. Callers pass their test name.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("agent-bridge-cfg-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn origins_default_to_app_and_local() {
        let dir = scratch("origins-default");
        let token = dir.join("token.txt");
        let audit = dir.join("audit.jsonl");
        std::fs::write(&token, "secret-1\n").unwrap();
        with_env(
            &[
                ("AGENT_BRIDGE_TOKEN_FILE", Some(token.to_str().unwrap())),
                ("AGENT_BRIDGE_AUDIT_LOG", Some(audit.to_str().unwrap())),
            ],
            || {
                let cfg = Config::load_from_env().unwrap();
                assert_eq!(cfg.origins, default_origins());
                assert_eq!(cfg.origins.len(), 7);
                assert!(cfg.origins.contains(&"http://tauri.localhost".to_string()));
                assert!(cfg.origins.contains(&"https://tauri.localhost".to_string()));
                assert!(cfg.origins.contains(&"tauri://localhost".to_string()));
            },
        );
    }

    #[test]
    fn origins_env_sets_custom_list() {
        let dir = scratch("origins-custom");
        let token = dir.join("token.txt");
        let audit = dir.join("audit.jsonl");
        std::fs::write(&token, "secret-1\n").unwrap();
        with_env(
            &[
                ("AGENT_BRIDGE_TOKEN_FILE", Some(token.to_str().unwrap())),
                ("AGENT_BRIDGE_AUDIT_LOG", Some(audit.to_str().unwrap())),
                (
                    "AGENT_BRIDGE_ORIGINS",
                    Some("https://app.example, http://localhost"),
                ),
            ],
            || {
                let cfg = Config::load_from_env().unwrap();
                assert_eq!(cfg.origins, vec!["https://app.example", "http://localhost"]);
            },
        );
    }

    #[test]
    fn origins_star_refuses_start() {
        let dir = scratch("origins-star");
        let token = dir.join("token.txt");
        let audit = dir.join("audit.jsonl");
        std::fs::write(&token, "secret-1\n").unwrap();
        with_env(
            &[
                ("AGENT_BRIDGE_TOKEN_FILE", Some(token.to_str().unwrap())),
                ("AGENT_BRIDGE_AUDIT_LOG", Some(audit.to_str().unwrap())),
                ("AGENT_BRIDGE_ORIGINS", Some("https://app.example, *")),
            ],
            || {
                let err = Config::load_from_env().unwrap_err();
                assert!(err.contains("star"));
            },
        );
    }

    #[test]
    fn missing_token_env_refuses_start() {
        with_env(&[], || {
            let err = Config::load_from_env().unwrap_err();
            assert!(err.contains("AGENT_BRIDGE_TOKEN_FILE"));
        });
    }

    #[test]
    fn missing_token_file_refuses_start() {
        let dir = scratch("missing");
        let absent = dir.join("no-token.txt");
        let audit = dir.join("audit.jsonl");
        with_env(
            &[
                ("AGENT_BRIDGE_TOKEN_FILE", Some(absent.to_str().unwrap())),
                ("AGENT_BRIDGE_AUDIT_LOG", Some(audit.to_str().unwrap())),
            ],
            || {
                let err = Config::load_from_env().unwrap_err();
                assert!(err.contains("missing"));
            },
        );
    }

    #[test]
    fn empty_token_file_refuses_start() {
        let dir = scratch("empty");
        let token = dir.join("token.txt");
        let audit = dir.join("audit.jsonl");
        std::fs::write(&token, "  \n").unwrap();
        with_env(
            &[
                ("AGENT_BRIDGE_TOKEN_FILE", Some(token.to_str().unwrap())),
                ("AGENT_BRIDGE_AUDIT_LOG", Some(audit.to_str().unwrap())),
            ],
            || {
                let err = Config::load_from_env().unwrap_err();
                assert!(err.contains("empty"));
            },
        );
    }

    #[test]
    fn debug_hides_token() {
        let dir = scratch("debug");
        let token = dir.join("token.txt");
        let audit = dir.join("audit.jsonl");
        std::fs::write(&token, "secret-1\n").unwrap();
        with_env(
            &[
                ("AGENT_BRIDGE_TOKEN_FILE", Some(token.to_str().unwrap())),
                ("AGENT_BRIDGE_AUDIT_LOG", Some(audit.to_str().unwrap())),
            ],
            || {
                let cfg = Config::load_from_env().unwrap();
                let shown = format!("{cfg:?}");
                assert!(!shown.contains("secret-1"));
                assert!(shown.contains("[redacted]"));
            },
        );
    }

    #[test]
    fn good_env_loads_and_trims_token() {
        let dir = scratch("good");
        let token = dir.join("token.txt");
        let audit = dir.join("audit.jsonl");
        std::fs::write(&token, "secret-1\n").unwrap();
        with_env(
            &[
                ("AGENT_BRIDGE_TOKEN_FILE", Some(token.to_str().unwrap())),
                ("AGENT_BRIDGE_AUDIT_LOG", Some(audit.to_str().unwrap())),
                ("AGENT_BRIDGE_EXTRA_KINDS", Some("Muse,  ,Scout")),
            ],
            || {
                let cfg = Config::load_from_env().unwrap();
                assert_eq!(cfg.token, "secret-1");
                assert_eq!(cfg.port, 8449);
                assert_eq!(cfg.code_ttl_secs, 60);
                assert_eq!(cfg.extra_kinds, vec!["muse", "scout"]);
            },
        );
    }
}

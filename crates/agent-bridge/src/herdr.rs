//! herdr. It shells out to the herdr CLI only.
//! Args pass as a list. No shell runs. Ever.

use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

/// Seconds a herdr call may run before it counts as down.
const HERDR_TIMEOUT_SECS: u64 = 10;

/// One pane from the list. It feeds list, read, and send checks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PaneInfo {
    /// Pane id, e.g. "w1:p19".
    #[serde(default)]
    pub id: String,
    /// Pane label, e.g. "lifeos".
    #[serde(default)]
    pub label: String,
    /// Tab id, e.g. "w1:t1".
    #[serde(default)]
    pub tab: String,
    /// Agent kind, e.g. "muse". Raw string from herdr.
    #[serde(default)]
    pub kind: String,
    /// Agent status: idle, working, blocked, done, unknown.
    #[serde(default)]
    pub status: String,
}

/// True for pane ids shaped like w1:p19.
/// Pattern: ^w[0-9]+:p[0-9A-Za-z]+$. All else fails fast.
pub fn valid_pane_id(id: &str) -> bool {
    let rest = match id.strip_prefix('w') {
        Some(r) => r,
        None => return false,
    };
    let (digits, rest) = split_while(rest, |c| c.is_ascii_digit());
    if digits.is_empty() {
        return false;
    }
    let rest = match rest.strip_prefix(":p") {
        Some(r) => r,
        None => return false,
    };
    let (tail, rest) = split_while(rest, |c| c.is_ascii_alphanumeric());
    !tail.is_empty() && rest.is_empty()
}

/// Split a prefix run from the rest. It keeps ASCII only.
fn split_while(s: &str, f: impl Fn(char) -> bool) -> (&str, &str) {
    let end = s.find(|c| !f(c)).unwrap_or(s.len());
    s.split_at(end)
}

/// Parse pane list stdout. It accepts several envelopes.
/// Real herdr wraps panes under result. The fake may use plain shapes.
pub fn parse_pane_list(stdout: &str) -> Result<Vec<PaneInfo>, String> {
    let value: serde_json::Value =
        serde_json::from_str(stdout).map_err(|_| "Pane list is not JSON".to_string())?;
    let panes = value
        .get("result")
        .and_then(|r| r.get("panes"))
        .or_else(|| value.get("panes"))
        .or_else(|| value.as_array().map(|_| &value));
    let Some(panes) = panes else {
        return Err("Pane list has no panes".to_string());
    };
    let list = panes.as_array().ok_or("Panes are not a list")?;
    let mut out = vec![];
    for p in list {
        out.push(PaneInfo {
            id: pick_str(p, &["pane_id", "id"]),
            label: pick_str(p, &["label", "title", "terminal_title"]),
            tab: pick_str(p, &["tab_id", "tab"]),
            kind: pick_str(p, &["agent", "display_agent", "kind"]),
            status: pick_str(p, &["agent_status", "status"]),
        });
    }
    Ok(out)
}

/// Pick the first present string field. Missing means empty.
fn pick_str(pane: &serde_json::Value, keys: &[&str]) -> String {
    keys.iter()
        .filter_map(|k| pane.get(k))
        .filter_map(|v| v.as_str())
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Parse pane read stdout. It returns the text.
/// JSON envelopes win. Raw text is the fallback.
pub fn parse_pane_read(stdout: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout) {
        if let Some(text) = value
            .get("result")
            .and_then(|r| r.get("text"))
            .or_else(|| value.get("text"))
            .and_then(|t| t.as_str())
        {
            return text.to_string();
        }
    }
    stdout.to_string()
}

/// Run one herdr call. Args are fixed lists, never shell strings.
/// It errors when herdr fails, times out, or stays silent.
async fn run_herdr(herdr: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new(herdr);
    cmd.args(args);
    cmd.kill_on_drop(true);
    let out = tokio::time::timeout(Duration::from_secs(HERDR_TIMEOUT_SECS), cmd.output())
        .await
        .map_err(|_| "herdr timed out".to_string())?
        .map_err(|e| format!("herdr failed to run: {e}"))?;
    if !out.status.success() {
        return Err("herdr call failed".to_string());
    }
    String::from_utf8(out.stdout).map_err(|_| "herdr gave bad text".to_string())
}

/// List panes through herdr. It parses tolerant JSON.
pub async fn list_panes(herdr: &Path) -> Result<Vec<PaneInfo>, String> {
    let stdout = run_herdr(herdr, &["pane", "list"]).await?;
    parse_pane_list(&stdout)
}

/// Read visible pane output through herdr. Caller checks id shape first.
pub async fn read_pane(herdr: &Path, id: &str) -> Result<String, String> {
    let stdout = run_herdr(herdr, &["pane", "read", id, "--source", "visible"]).await?;
    Ok(parse_pane_read(&stdout))
}

/// Send literal text to a pane. Caller checks id shape first.
/// Text passes as one argv item. No shell sees it.
pub async fn send_text(herdr: &Path, id: &str, text: &str) -> Result<(), String> {
    run_herdr(herdr, &["pane", "send-text", id, text]).await?;
    Ok(())
}

/// Press Enter in a pane. Enter submits every text send.
pub async fn send_enter(herdr: &Path, id: &str) -> Result<(), String> {
    run_herdr(herdr, &["pane", "send-keys", id, "Enter"]).await?;
    Ok(())
}

/// Press Escape in a pane. Only Enter and Escape ever send.
/// No other key name reaches send-keys. Ever.
pub async fn send_escape(herdr: &Path, id: &str) -> Result<(), String> {
    run_herdr(herdr, &["pane", "send-keys", id, "Escape"]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_id_check_matches_spec() {
        for good in ["w1:p1", "w12:pABC99", "w1:p19", "w1:p1X"] {
            assert!(valid_pane_id(good), "{good}");
        }
        for bad in [
            "",
            "w1",
            "p1",
            "w:p1",
            "w1:p",
            "w1:p1;",
            "w1:p1 x",
            "w1:p-1",
            "w1:p1\n",
            "x1:p1",
            "w1:q1",
            "../w1:p1",
            "w1:p1|id",
            "w1:p1`id`",
            "w1:p1$(id)",
        ] {
            assert!(!valid_pane_id(bad), "{bad}");
        }
    }

    #[test]
    fn parses_wrapped_and_plain_lists() {
        let wrapped = r#"{"result":{"type":"pane_list","panes":[
            {"pane_id":"w1:p1","label":"lifeos","tab_id":"w1:t1","agent":"muse","agent_status":"idle"}
        ]}}"#;
        let panes = parse_pane_list(wrapped).unwrap();
        assert_eq!(panes.len(), 1);
        assert_eq!(
            panes[0],
            PaneInfo {
                id: "w1:p1".into(),
                label: "lifeos".into(),
                tab: "w1:t1".into(),
                kind: "muse".into(),
                status: "idle".into(),
            }
        );
        let plain = r#"{"panes":[{"id":"w1:p2","kind":"muse","status":"working"}]}"#;
        let panes = parse_pane_list(plain).unwrap();
        assert_eq!(panes[0].id, "w1:p2");
        assert_eq!(panes[0].label, "");
        let bare = r#"[{"pane_id":"w1:p3"}]"#;
        let panes = parse_pane_list(bare).unwrap();
        assert_eq!(panes[0].id, "w1:p3");
        assert!(parse_pane_list("nope").is_err());
        assert!(parse_pane_list(r#"{"a":1}"#).is_err());
    }

    #[test]
    fn parses_wrapped_and_raw_reads() {
        let wrapped = r#"{"result":{"type":"pane_read","text":"hi\nthere"}}"#;
        assert_eq!(parse_pane_read(wrapped), "hi\nthere");
        let plain = r#"{"text":"abc"}"#;
        assert_eq!(parse_pane_read(plain), "abc");
        assert_eq!(parse_pane_read("raw\ntext"), "raw\ntext");
    }
}

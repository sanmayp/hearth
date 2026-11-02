//! Audit. It appends one JSON line per send attempt.
//! Lines hold length, never text.

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

/// Append-only audit log. Nothing edits or deletes lines.
pub struct Audit {
    /// Log file path.
    path: PathBuf,
}

impl Audit {
    /// Build an audit writer for the path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Append one attempt line. It returns the IO result.
    pub async fn append(
        &self,
        pane: &str,
        sender: &str,
        text_len: usize,
        result: &str,
    ) -> std::io::Result<()> {
        let line = serde_json::json!({
            "time": chrono::Utc::now().to_rfc3339(),
            "pane": pane,
            "sender": sender,
            "text_len": text_len,
            "result": result,
        });
        let mut body = serde_json::to_string(&line).unwrap_or_else(|_| "{}".to_string());
        body.push('\n');
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?
            .write_all(body.as_bytes())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lines_hold_shape_without_text() {
        let dir = std::env::temp_dir().join(format!("agent-bridge-audit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        let audit = Audit::new(path.clone());
        audit.append("w1:p1", "owner", 11, "sent").await.unwrap();
        audit
            .append("w1:p1", "owner", 11, "refused:bad_code")
            .await
            .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 2);
        for line in text.lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(v.get("time").is_some());
            assert_eq!(v["pane"], "w1:p1");
            assert_eq!(v["sender"], "owner");
            assert_eq!(v["text_len"], 11);
            assert!(v.get("text").is_none());
            assert!(v.get("code").is_none());
        }
        assert!(text.contains("refused:bad_code"));
    }
}

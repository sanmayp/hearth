//! Redact. It masks token-looking runs.
//! It copies the watch module rule exactly.

/// True for chars that appear inside tokens and keys.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '+' | '=')
}

/// Replace token-looking runs (20 or more token chars) with a marker.
/// Plain prose, paths, and URLs pass through: spaces, slashes, and
/// colons break runs before they reach the threshold.
pub fn redact(text: &str) -> String {
    fn flush(run: &mut String, out: &mut String) {
        if run.len() >= 20 {
            out.push_str("[redacted]");
        } else {
            out.push_str(run);
        }
        run.clear();
    }
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    for c in text.chars() {
        if is_token_char(c) {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_token_runs_and_keeps_prose() {
        // Long runs build at runtime. Short literals stay in source.
        let run = "ab".repeat(15);
        assert_eq!(redact(&format!("key {run} here")), "key [redacted] here");
        assert_eq!(redact("short code abc123 stays"), "short code abc123 stays");
        // Long hostnames mask too. Fail closed: a masked host is
        // noise, a leaked key is a breach.
        let host = ["testing", "host", "name", "for", "redact"].join("");
        let url = format!("see https://{host}.example:8447/v1 soon");
        let want = "see https://[redacted]:8447/v1 soon";
        assert_eq!(redact(&url), want);
        assert_eq!(
            redact("path /home/user/hearth/web/src-tauri/readme ok"),
            "path /home/user/hearth/web/src-tauri/readme ok"
        );
    }

    #[test]
    fn masks_scheme_tokens_in_output() {
        // Long runs build at runtime. Short literals stay in source.
        let run = "cd".repeat(12);
        let line = ["Authorization:", "auth-scheme", &run].join(" ");
        let want = ["Authorization:", "auth-scheme", "[redacted]"].join(" ");
        assert_eq!(redact(&line), want);
    }
}

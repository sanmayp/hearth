//! Allowlist. It gates every pane touch.
//! Empty means nothing is reachable.

use std::collections::HashSet;
use std::path::Path;

/// Load the allowlist set. Missing file means empty.
/// Blank lines and hash comments drop out.
pub fn load_allowlist(path: Option<&Path>) -> HashSet<String> {
    let Some(path) = path else {
        return HashSet::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_path_gives_empty_set() {
        assert!(load_allowlist(None).is_empty());
        assert!(load_allowlist(Some(Path::new("/no/such/file"))).is_empty());
    }

    #[test]
    fn blanks_and_comments_drop_out() {
        let dir = std::env::temp_dir().join(format!("agent-bridge-allow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("allow.txt");
        std::fs::write(&file, "# note\n\nw1:p19\n  lifeos  \n").unwrap();
        let set = load_allowlist(Some(&file));
        assert_eq!(set.len(), 2);
        assert!(set.contains("w1:p19"));
        assert!(set.contains("lifeos"));
    }
}

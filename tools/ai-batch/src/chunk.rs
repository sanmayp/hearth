//! Chunk. It reads public text and splits it for calls.

use std::path::{Path, PathBuf};

/// One call input. It names its file and slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Stable id like `lanes/a.txt#0`.
    pub id: String,
    /// File path relative to the input root.
    pub file: String,
    /// Slice index inside the file.
    pub index: usize,
    /// Text slice. Never logs in full.
    pub text: String,
}

/// Max chars per chunk.
pub const CHUNK_CHARS: usize = 1500;

/// Rough token estimate. Chars divided by four. Labelled estimate.
pub fn estimate_tokens(text: &str) -> u64 {
    (text.chars().count() as u64).div_ceil(4)
}

/// Read text files under root. Sorted, recursive, symlink-safe.
/// Files outside root never load. Only .txt plus .md load.
pub fn read_input_dir(root: &Path) -> Result<Vec<(String, String)>, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("Input dir is bad: {e}"))?;
    let mut out = vec![];
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| format!("Input read fails: {e}"))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_symlink() {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "txt" && ext != "md" {
                continue;
            }
            let canon = path
                .canonicalize()
                .map_err(|e| format!("Input file is bad: {e}"))?;
            if !canon.starts_with(&root) {
                continue;
            }
            let rel = canon
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            let text =
                std::fs::read_to_string(&canon).map_err(|e| format!("Input file is bad: {e}"))?;
            out.push((rel, text));
        }
    }
    out.sort();
    Ok(out)
}

/// Split text on word bounds. Empty text gives no chunks.
pub fn chunk_text(file: &str, text: &str) -> Vec<Chunk> {
    let mut chunks = vec![];
    let mut current = String::new();
    let mut index = 0;
    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        if current.chars().count() + word.chars().count() > CHUNK_CHARS && !current.is_empty() {
            chunks.push(Chunk {
                id: format!("{file}#{index}"),
                file: file.to_string(),
                index,
                text: current.trim_end().to_string(),
            });
            index += 1;
            current.clear();
        }
        current.push_str(word);
    }
    if !current.trim().is_empty() {
        chunks.push(Chunk {
            id: format!("{file}#{index}"),
            file: file.to_string(),
            index,
            text: current.trim_end().to_string(),
        });
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_words_with_stable_ids() {
        let text = format!("{} tail", "word ".repeat(400));
        let chunks = chunk_text("a.txt", &text);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].id, "a.txt#0");
        assert!(chunks[0].text.len() <= CHUNK_CHARS + 10);
        assert!(chunks[0].text.ends_with("word"));
        assert!(text.starts_with(&chunks[0].text[..100]));
    }

    #[test]
    fn empty_text_gives_no_chunks() {
        assert!(chunk_text("a.txt", "  \n ").is_empty());
    }

    #[test]
    fn symlink_escape_never_loads() {
        let dir = std::env::temp_dir().join(format!("aib-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("in")).unwrap();
        std::fs::write(dir.join("secret.txt"), "shh").unwrap();
        std::fs::write(dir.join("in/ok.txt"), "hello world").unwrap();
        std::os::unix::fs::symlink(dir.join("secret.txt"), dir.join("in/evil.txt")).unwrap();
        let files = read_input_dir(&dir.join("in")).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "ok.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

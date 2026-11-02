//! Kinds. It names known agents and their approval prompts.
//! Unknown kinds refuse sends.
//!
//! Public edition: the table ships Muse and the supervisor
//! role plus generic signs. It covers yes/no style prompts.
//! Owners add more kinds at runtime with AGENT_BRIDGE_EXTRA_KINDS.

/// Supervisor stays reachable with generic rules only.
pub const KNOWN_KINDS: &[&str] = &["muse", "supervisor"];

/// Generic approval signs. They apply to every known kind.
const GENERIC_PATTERNS: &[&str] = &[
    "(y/n)",
    "[y/n]",
    "y/n",
    "yes or no",
    "yes/no",
    "approve",
    "approval",
    "needs approval",
    "waiting for approval",
    "do you want",
    "would you like",
    "allow?",
    "continue?",
    "proceed?",
    "confirm?",
    "press enter",
    "esc to cancel",
];

/// Extra approval signs per kind. Matching is case-insensitive.
fn kind_patterns(kind: &str) -> &'static [&'static str] {
    match kind {
        "muse" => &["approve this", "run it?"],
        _ => &[],
    }
}

/// Normalize a kind string. It lowercases and strips noise.
pub fn normalize_kind(raw: &str) -> String {
    raw.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// True when the kind may receive sends. Unknown kinds fail.
/// Extra owner kinds pass with generic rules only.
pub fn is_allowed_kind(kind: &str, extra: &[String]) -> bool {
    KNOWN_KINDS.contains(&kind) || extra.iter().any(|k| k == kind)
}

/// True for short approval answers. Empty text means Enter.
/// Single y or n in either case counts too. Digits 1 to 4
/// pick menu rows, so they count as short answers as well.
pub fn is_short_answer(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => matches!(c, 'y' | 'Y' | 'n' | 'N' | '1' | '2' | '3' | '4'),
        _ => false,
    }
}

/// Escape key marker. The phone sends this one byte as text.
/// It maps to the Escape key press, never to pasted text.
pub const ESCAPE_TEXT: &str = "\u{1b}";

/// True when the text is the Escape key marker.
pub fn is_escape_key(text: &str) -> bool {
    text == ESCAPE_TEXT
}

/// Find an approval prompt in the last lines. It returns the line.
/// It checks kind patterns plus generic patterns. Case does not matter.
pub fn detect_approval(kind: &str, lines: &[&str]) -> Option<String> {
    let tail: Vec<&str> = lines.iter().rev().take(20).copied().collect();
    for line in tail {
        let low = line.to_lowercase();
        let hit_kind = kind_patterns(kind).iter().any(|p| low.contains(p));
        let hit_generic = GENERIC_PATTERNS.iter().any(|p| low.contains(p));
        if hit_kind || hit_generic {
            let mut prompt = line.trim().to_string();
            if prompt.len() > 300 {
                prompt.truncate(300);
            }
            return Some(prompt);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_names_to_kinds() {
        assert_eq!(normalize_kind("muse"), "muse");
        assert_eq!(normalize_kind("Muse"), "muse");
        assert_eq!(normalize_kind("mystery-bot!"), "mysterybot");
    }

    #[test]
    fn known_kinds_pass_and_unknown_fails() {
        for kind in KNOWN_KINDS {
            assert!(is_allowed_kind(kind, &[]));
        }
        assert!(!is_allowed_kind("mysterybot", &[]));
        assert!(is_allowed_kind("mysterybot", &["mysterybot".to_string()]));
    }

    #[test]
    fn short_answers_are_y_n_digits_and_enter() {
        assert!(is_short_answer(""));
        assert!(is_short_answer("y"));
        assert!(is_short_answer("N"));
        assert!(is_short_answer("1"));
        assert!(is_short_answer("4"));
        assert!(!is_short_answer("5"));
        assert!(!is_short_answer("yes"));
        assert!(!is_short_answer("hi"));
        assert!(!is_short_answer("\u{1b}"));
    }

    #[test]
    fn escape_marker_matches_one_byte_only() {
        assert!(is_escape_key("\u{1b}"));
        assert!(!is_escape_key(""));
        assert!(!is_escape_key("esc"));
        assert!(!is_escape_key("\u{1b}\u{1b}"));
    }

    #[test]
    fn supervisor_kind_passes() {
        assert!(KNOWN_KINDS.contains(&"supervisor"));
        assert!(is_allowed_kind("supervisor", &[]));
    }

    #[test]
    fn detects_generic_prompt_for_every_kind() {
        let lines = ["working hard", "Proceed? (y/n)"];
        assert!(detect_approval("muse", &lines).is_some());
        assert!(detect_approval("supervisor", &lines).is_some());
    }

    #[test]
    fn detects_kind_prompt() {
        let lines = ["task ready", "Approve this run?"];
        let hit = detect_approval("muse", &lines).unwrap();
        assert!(hit.contains("Approve this"));
    }

    #[test]
    fn misses_plain_output() {
        let lines = ["compiling the server", "done in 3 s"];
        assert!(detect_approval("muse", &lines).is_none());
    }
}

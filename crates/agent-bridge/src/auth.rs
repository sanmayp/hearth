//! Auth. It checks the Bearer token in constant time.
//! It never logs the token.

use axum::http::HeaderMap;

/// True when two byte strings match. It runs in constant time.
/// Lengths leak. Contents do not.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// True when the header carries the token. Pure, so tests cover it.
pub fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some(value) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(got) = value.strip_prefix("Bearer ") else {
        return false;
    };
    ct_eq(got.as_bytes(), token.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_compare_matches_exact_bytes() {
        assert!(ct_eq(b"abc123", b"abc123"));
        assert!(!ct_eq(b"abc123", b"abc124"));
        assert!(!ct_eq(b"abc123", b"abc12"));
        assert!(!ct_eq(b"abc12", b"abc123"));
        assert!(!ct_eq(b"", b"abc123"));
    }

    #[test]
    fn gate_accepts_only_bearer_match() {
        let mut good = HeaderMap::new();
        good.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer abc123".parse().unwrap(),
        );
        assert!(authorized(&good, "abc123"));
        let mut bad = HeaderMap::new();
        bad.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer wrong".parse().unwrap(),
        );
        assert!(!authorized(&bad, "abc123"));
        let mut bare = HeaderMap::new();
        bare.insert(axum::http::header::AUTHORIZATION, "abc123".parse().unwrap());
        assert!(!authorized(&bare, "abc123"));
        assert!(!authorized(&HeaderMap::new(), "abc123"));
    }
}

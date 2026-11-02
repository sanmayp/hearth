//! In-app update checks. Pure functions over bytes and text.
//! Fails closed on every bad input. Tested without a device.

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Verified manifest fields. Built only after the signature passes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// Display version, e.g. "0.2.0".
    pub version: String,
    /// Monotonic code. Must beat the installed one.
    #[serde(rename = "versionCode")]
    pub version_code: u32,
    /// APK path. Relative only, same host by construction.
    pub apk_url: String,
    /// Expected APK byte size.
    pub apk_size: u64,
    /// Expected APK SHA-256, lowercase hex.
    pub sha256: String,
    /// Release notes, plain words.
    pub notes: String,
}

/// Check outcome for one manifest.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Newer build ready. Holds the verified manifest.
    Update(Manifest),
    /// Installed code matches the manifest. Nothing to do.
    Current,
}

/// Parse a feed (`{"manifest": {...}, "signature": "..."}`) and verify.
/// Re-emits the manifest in canonical form before checking, so key
/// order and spacing never matter. The publish script must write the
/// manifest bytes in the same canonical form it signs.
pub fn verify_feed(feed_json: &[u8], pubkey_b64: &str) -> Result<Manifest, String> {
    let feed: serde_json::Value =
        serde_json::from_slice(feed_json).map_err(|_| "update feed broken".to_string())?;
    let manifest = feed
        .get("manifest")
        .ok_or("update feed broken".to_string())?;
    let sig = feed
        .get("signature")
        .and_then(|s| s.as_str())
        .ok_or("update feed broken".to_string())?;
    let canonical = serde_json::to_vec(manifest).map_err(|_| "manifest broken".to_string())?;
    verify_manifest(&canonical, sig, pubkey_b64)
}

/// Parse and verify a manifest. Uses nothing before the signature passes.
pub fn verify_manifest(
    manifest_json: &[u8],
    signature_b64: &str,
    pubkey_b64: &str,
) -> Result<Manifest, String> {
    let pubkey = base64::engine::general_purpose::STANDARD
        .decode(pubkey_b64)
        .map_err(|_| "bad pinned key".to_string())?;
    let key = VerifyingKey::from_bytes(
        &pubkey
            .try_into()
            .map_err(|_| "bad pinned key".to_string())?,
    )
    .map_err(|_| "bad pinned key".to_string())?;
    let sig = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|_| "bad signature shape".to_string())?;
    let sig = Signature::from_bytes(
        &sig.try_into()
            .map_err(|_| "bad signature shape".to_string())?,
    );
    key.verify(manifest_json, &sig)
        .map_err(|_| "signature mismatch".to_string())?;
    serde_json::from_slice(manifest_json).map_err(|_| "manifest broken".to_string())
}

/// Compare codes. Older server builds are refused, never installed.
pub fn decide(current_code: u32, manifest: &Manifest) -> Result<Verdict, String> {
    if manifest.version_code > current_code {
        Ok(Verdict::Update(manifest.clone()))
    } else if manifest.version_code == current_code {
        Ok(Verdict::Current)
    } else {
        Err("server offered an older build. Refused.".to_string())
    }
}

/// Resolve the APK path against the API base. Absolute URLs and
/// other hosts are refused. Plain HTTP bases are refused.
pub fn apk_url(api_base: &str, apk_path: &str) -> Result<String, String> {
    let base = url::Url::parse(api_base).map_err(|_| "bad api base".to_string())?;
    if base.scheme() != "https" {
        return Err("update needs https".to_string());
    }
    if apk_path.contains("://") || apk_path.starts_with("//") {
        return Err("apk path must stay on this host".to_string());
    }
    let joined = base
        .join(apk_path.trim_start_matches('/'))
        .map_err(|_| "bad apk path".to_string())?;
    if joined.host_str() != base.host_str() {
        return Err("apk path must stay on this host".to_string());
    }
    Ok(joined.to_string())
}

/// Hash a staged file and compare. Size guards the read first.
pub fn verify_file(path: &Path, sha_hex: &str, size: u64) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|_| "staged file missing".to_string())?;
    if meta.len() != size {
        return Err("size mismatch. Refused.".to_string());
    }
    let bytes = std::fs::read(path).map_err(|_| "staged file unreadable".to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = hex::encode(hasher.finalize());
    if got == sha_hex.to_lowercase() {
        Ok(())
    } else {
        Err("hash mismatch. Refused.".to_string())
    }
}

/// True when both signing certificates match byte for byte.
/// The system installer also enforces this at install time.
pub fn certs_match(installed: &[u8], candidate: &[u8]) -> bool {
    !installed.is_empty() && installed == candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
    use ed25519_dalek::SigningKey;

    fn keypair() -> (SigningKey, String) {
        let secret = [9u8; 32];
        let signing = SigningKey::from_bytes(&secret);
        let pub_b64 =
            base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().to_bytes());
        (signing, pub_b64)
    }

    /// Canonical bytes: sorted keys, compact. Matches the publish script.
    fn manifest_bytes() -> Vec<u8> {
        br#"{"apk_size":9,"apk_url":"/v1/app/apk","notes":"Fixes.","sha256":"abc","version":"0.2.0","versionCode":7}"#.to_vec()
    }

    #[test]
    fn good_signature_verifies() {
        let (signing, pub_b64) = keypair();
        let body = manifest_bytes();
        let sig = signing.sign(&body);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        let m = verify_manifest(&body, &sig_b64, &pub_b64).unwrap();
        assert_eq!(m.version_code, 7);
        assert_eq!(m.apk_url, "/v1/app/apk");
    }

    #[test]
    fn tampered_body_fails() {
        let (signing, pub_b64) = keypair();
        let body = manifest_bytes();
        let sig = signing.sign(&body);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        let mut evil = body.clone();
        evil[20] = b'9';
        assert!(verify_manifest(&evil, &sig_b64, &pub_b64).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let (signing, _) = keypair();
        let other = SigningKey::from_bytes(&[4u8; 32]);
        let other_b64 =
            base64::engine::general_purpose::STANDARD.encode(other.verifying_key().to_bytes());
        let body = manifest_bytes();
        let sig = signing.sign(&body);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        assert!(verify_manifest(&body, &sig_b64, &other_b64).is_err());
    }

    #[test]
    fn feed_with_shuffled_keys_still_verifies() {
        let (signing, pub_b64) = keypair();
        let canonical = manifest_bytes();
        let sig = signing.sign(&canonical);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        // Same fields, scrambled order and spacing.
        let feed = format!(
            r#"{{"signature": "{sig_b64}", "manifest": {{"notes": "Fixes.", "version": "0.2.0", "apk_size": 9, "sha256": "abc", "apk_url": "/v1/app/apk", "versionCode": 7}} }}"#
        );
        let m = verify_feed(feed.as_bytes(), &pub_b64).unwrap();
        assert_eq!(m.version_code, 7);
    }

    #[test]
    fn openssl_made_signature_verifies() {
        // Interop proof: signed with `openssl pkeyutl -sign -rawin`
        // over the canonical bytes. Same call the publish script runs.
        let pub_b64 = "aIctIGUqKrWglOYnSBDB/WPkN2nD8lgH38jLZ9jkO/k="; // pubscan:allow test-only verify key, public by design
        let sig_b64 = "zcdm7EJnLN4ZVBL3CRc8b+r47L9WVO/eJz2HgS9iSh7QWy61CvQTmQs0EUtWjFrs46iS1KT064pHZiwdsg56Aw=="; // pubscan:allow test-only fixture signature
        let m = verify_manifest(&manifest_bytes(), sig_b64, pub_b64).unwrap();
        assert_eq!(m.version, "0.2.0");
    }

    #[test]
    fn feed_without_signature_fails() {
        assert!(verify_feed(br#"{"manifest": {}}"#, "AAAA").is_err());
        assert!(verify_feed(b"not json", "AAAA").is_err());
    }

    #[test]
    fn bad_base64_fails_closed() {
        assert!(verify_manifest(b"{}", "!!!", "!!!").is_err());
        assert!(verify_manifest(b"{}", "AAAA", "AAAA").is_err());
    }

    #[test]
    fn decide_splits_newer_current_older() {
        let m = Manifest {
            version: "0.2.0".into(),
            version_code: 7,
            apk_url: "/v1/app/apk".into(),
            apk_size: 9,
            sha256: "abc".into(),
            notes: "Fixes.".into(),
        };
        assert!(matches!(decide(6, &m), Ok(Verdict::Update(_))));
        assert!(matches!(decide(7, &m), Ok(Verdict::Current)));
        assert!(decide(8, &m).is_err());
    }

    #[test]
    fn apk_url_stays_on_host_and_https() {
        let base = "https://example.ts.net:8447";
        assert_eq!(
            apk_url(base, "/v1/app/apk").unwrap(),
            "https://example.ts.net:8447/v1/app/apk"
        );
        assert!(apk_url(base, "https://evil.example/x.apk").is_err());
        assert!(apk_url(base, "//evil.example/x.apk").is_err());
        assert!(apk_url("http://example.ts.net:8447", "/v1/app/apk").is_err());
        assert!(apk_url("not a url", "/v1/app/apk").is_err());
    }

    #[test]
    fn file_hash_accepts_match_and_refuses_rest() {
        let dir = std::env::temp_dir().join("hearth-update-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("staged.apk");
        std::fs::write(&path, b"ninebytes").unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"ninebytes");
        let hex = hex::encode(hasher.finalize());
        assert!(verify_file(&path, &hex, 9).is_ok());
        assert!(verify_file(&path, &hex, 10).is_err());
        assert!(verify_file(&path, &"00".repeat(32), 9).is_err());
        assert!(verify_file(&dir.join("missing.apk"), &hex, 9).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn certs_match_only_on_equal_nonempty() {
        assert!(certs_match(b"cert-a", b"cert-a"));
        assert!(!certs_match(b"cert-a", b"cert-b"));
        assert!(!certs_match(b"", b""));
    }
}

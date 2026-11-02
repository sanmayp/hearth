//! App module. Read-only update feed for the Android app:
//! a signed manifest plus the newest release APK bytes.

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The app module.
pub struct App;

/// Manifest plus its signature, as the phone receives them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateFeed {
    /// Manifest fields: version, versionCode, apk_url, apk_size, sha256, notes.
    pub manifest: Value,
    /// Base64 Ed25519 signature over the manifest bytes.
    pub signature: String,
}

#[async_trait]
impl Module for App {
    fn id(&self) -> &'static str {
        "app"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![]
    }

    fn routes(&self, _state: &AppState) -> Router<()> {
        Router::new()
            .route("/v1/app/update", get(get_update))
            .route("/v1/app/apk", get(get_apk))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "app.card",
            title: "App update",
            module: "app",
        }]
    }

    async fn health(&self, _state: &AppState) -> Health {
        match load_feed(&update_dir()) {
            Ok(_) => Health::ok("update feed reads"),
            Err(e) => Health::down(format!("update feed down: {e}")),
        }
    }
}

/// Serve the signed manifest. Fails closed when files are bad.
async fn get_update() -> impl IntoResponse {
    match load_feed(&update_dir()) {
        Ok(feed) => Json(json!(feed)).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message": "No update published yet. Retry later."})),
        )
            .into_response(),
    }
}

/// Serve the newest APK bytes. Fixed name, so no path can escape.
async fn get_apk() -> impl IntoResponse {
    let path = update_dir().join("hearth.apk");
    let file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"message": "No update published yet. Retry later."})),
            )
                .into_response();
        }
    };
    match file.metadata().await {
        Ok(m) if m.len() > 0 => Response::builder()
            .header(
                header::CONTENT_TYPE,
                "application/vnd.android.package-archive",
            )
            .header(header::CONTENT_LENGTH, m.len())
            .body(Body::from_stream(tokio_util::io::ReaderStream::new(file)))
            .unwrap()
            .into_response(),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"message": "No update published yet. Retry later."})),
        )
            .into_response(),
    }
}

/// Update dir. Env wins, else the checkout default.
fn update_dir() -> PathBuf {
    if let Ok(p) = std::env::var("UPDATE_DIR") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("hearth/updates")
}

/// Load manifest plus signature from a dir. Pure over the path, so tests cover it.
fn load_feed(dir: &Path) -> Result<UpdateFeed, String> {
    let raw = std::fs::read(dir.join("manifest.json")).map_err(|_| "manifest missing")?;
    let manifest: Value = serde_json::from_slice(&raw).map_err(|_| "manifest broken")?;
    for key in [
        "version",
        "versionCode",
        "apk_url",
        "apk_size",
        "sha256",
        "notes",
    ] {
        if manifest.get(key).is_none() {
            return Err(format!("manifest lacks {key}"));
        }
    }
    let sig = std::fs::read(dir.join("manifest.sig")).map_err(|_| "signature missing")?;
    if sig.len() != 64 {
        return Err("signature wrong size".into());
    }
    Ok(UpdateFeed {
        manifest,
        signature: base64_encode(&sig),
    })
}

/// Tiny base64 (standard alphabet). No new dep for one field.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"; // pubscan:allow base64 alphabet constant
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut n: u32 = 0;
        for (i, b) in chunk.iter().enumerate() {
            n |= (*b as u32) << (16 - 8 * i);
        }
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHA[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hearth-app-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            r#"{"version":"0.2.0","versionCode":7,"apk_url":"/v1/app/apk","apk_size":9,"sha256":"abc","notes":"Fixes."}"#,
        )
        .unwrap();
        std::fs::write(dir.join("manifest.sig"), [7u8; 64]).unwrap();
        dir
    }

    #[test]
    fn feed_combines_manifest_and_signature() {
        let dir = setup("ok");
        let feed = load_feed(&dir).unwrap();
        assert_eq!(feed.manifest["versionCode"], 7);
        assert_eq!(feed.signature.len(), 88);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn feed_refuses_missing_files() {
        let dir = std::env::temp_dir().join("hearth-app-test-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(load_feed(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn feed_refuses_broken_manifest() {
        let dir = setup("broken");
        std::fs::write(dir.join("manifest.json"), "not json").unwrap();
        assert!(load_feed(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn feed_refuses_short_signature() {
        let dir = setup("shortsig");
        std::fs::write(dir.join("manifest.sig"), [1u8; 10]).unwrap();
        assert!(load_feed(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn base64_matches_known_vector() {
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
        assert_eq!(base64_encode(&[7u8; 64]).len(), 88);
    }
}

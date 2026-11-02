//! Update check commands. Thin glue: fetch with the caller's token,
//! verify with the update crate, stage the file, hand it to Android.
//! The token is never logged and never stored here.

use tauri::{AppHandle, Manager};

/// Pinned Ed25519 public key, base64. Baked by the publish script.
const PUBKEY_B64: Option<&str> = option_env!("UPDATE_PUBKEY_B64");
/// Installed version code. Baked by the publish script.
const APP_CODE: Option<&str> = option_env!("APP_VERSION_CODE");
/// Staged APK name in the app cache dir.
const STAGED_NAME: &str = "hearth-update.apk";
/// Extra bytes tolerated past the declared size while streaming.
const SIZE_SLACK: u64 = 1024;

/// Check outcome for the UI.
#[derive(serde::Serialize)]
pub struct CheckOutcome {
    /// True when a newer verified build waits.
    pub available: bool,
    /// Display version of the offered build.
    pub version: String,
    /// Release notes of the offered build.
    pub notes: String,
    /// Version code of the offered build.
    pub code: u32,
}

fn pinned_key() -> Result<&'static str, String> {
    PUBKEY_B64
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "updates are not set up in this build".to_string())
}

fn current_code() -> Result<u32, String> {
    APP_CODE
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "updates are not set up in this build".to_string())
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("net down: {e}"))
}

async fn fetch_manifest(api_base: &str, bearer: &str) -> Result<update::Manifest, String> {
    let url = update::apk_url(api_base, "/v1/app/update")?;
    let res = client()?
        .get(&url)
        .header("Authorization", format!("Bearer {bearer}"))
        .send()
        .await
        .map_err(|_| "no link to Hearth. Check your link and retry.".to_string())?;
    if res.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Hearth needs your key. Open Settings and paste it.".to_string());
    }
    if !res.status().is_success() {
        return Err("Hearth said no. Retry soon.".to_string());
    }
    let body = res
        .bytes()
        .await
        .map_err(|_| "update feed broke off".to_string())?;
    update::verify_feed(&body, pinned_key()?)
}

/// Check for a newer verified build. Reads nothing before the signature passes.
#[tauri::command]
pub async fn check_update(api_base: String, bearer: String) -> Result<CheckOutcome, String> {
    let manifest = fetch_manifest(&api_base, &bearer).await?;
    match update::decide(current_code()?, &manifest)? {
        update::Verdict::Update(m) => Ok(CheckOutcome {
            available: true,
            version: m.version,
            notes: m.notes,
            code: m.version_code,
        }),
        update::Verdict::Current => Ok(CheckOutcome {
            available: false,
            version: manifest.version,
            notes: String::new(),
            code: manifest.version_code,
        }),
    }
}

/// Download the update, verify it, and stage it. Re-checks the
/// manifest first; trusts nothing the caller passes except the token.
#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    api_base: String,
    bearer: String,
) -> Result<String, String> {
    let manifest = fetch_manifest(&api_base, &bearer).await?;
    match update::decide(current_code()?, &manifest)? {
        update::Verdict::Current => return Err("already on the newest build".to_string()),
        update::Verdict::Update(_) => {}
    }
    let url = update::apk_url(&api_base, &manifest.apk_url)?;
    let mut res = client()?
        .get(&url)
        .header("Authorization", format!("Bearer {bearer}"))
        .timeout(std::time::Duration::from_secs(900))
        .send()
        .await
        .map_err(|_| "no link to Hearth. Check your link and retry.".to_string())?;
    if !res.status().is_success() {
        return Err("Hearth said no. Retry soon.".to_string());
    }
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|_| "no cache dir on this device".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|_| "no cache dir on this device".to_string())?;
    let path = dir.join(STAGED_NAME);
    let mut file = std::fs::File::create(&path).map_err(|_| "cannot stage file".to_string())?;
    let mut written: u64 = 0;
    let cap = manifest.apk_size.saturating_add(SIZE_SLACK);
    use std::io::Write;
    loop {
        let chunk = res
            .chunk()
            .await
            .map_err(|_| "download broke off".to_string())?;
        let Some(chunk) = chunk else { break };
        written += chunk.len() as u64;
        if written > cap {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err("file too big. Refused.".to_string());
        }
        file.write_all(&chunk)
            .map_err(|_| "cannot stage file".to_string())?;
    }
    drop(file);
    update::verify_file(&path, &manifest.sha256, manifest.apk_size).map_err(|e| {
        let _ = std::fs::remove_file(&path);
        e
    })?;
    Ok(format!("staged {} bytes", manifest.apk_size))
}

/// Hand the staged APK to the Android installer. Android shows the
/// system dialog and enforces the signing key match itself.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|_| "no cache dir on this device".to_string())?;
    let path = dir.join(STAGED_NAME);
    if !path.is_file() {
        return Err("nothing staged. Download first.".to_string());
    }
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_hearth_installer::InstallerExt;
        app.installer()
            .install(&path.to_string_lossy())
            .map(|_| "opening installer. Tap Install once.".to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err("install needs the Android app".to_string())
    }
}

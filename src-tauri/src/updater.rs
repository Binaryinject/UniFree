use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

const GITHUB_API_URL: &str = "https://api.github.com/repos/Binaryinject/UniFree/releases/latest";

static CANCEL_DOWNLOAD: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub file_name: String,
    pub body: String,
}

#[derive(Serialize, Clone)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check GitHub for latest release, compare with current version
pub async fn check_for_update(current_version: &str) -> Result<Option<UpdateInfo>, String> {
    let client = reqwest::Client::builder()
        .user_agent("UniFree-Updater")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let release: GitHubRelease = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to check update: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v');

    if !is_newer(current_version, latest_version) {
        return Ok(None);
    }

    // Find the NSIS installer asset
    let asset = release
        .assets
        .iter()
        .find(|a| {
            a.name.ends_with("-setup.exe")
                || a.name.ends_with("-installer.exe")
                || a.name.ends_with("installer.exe")
        })
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")));

    let asset = match asset {
        Some(a) => a,
        None => return Err("No installer found in release assets".into()),
    };

    Ok(Some(UpdateInfo {
        version: latest_version.to_string(),
        download_url: asset.browser_download_url.clone(),
        file_name: asset.name.clone(),
        body: release.body.unwrap_or_default(),
    }))
}

/// Cancel ongoing download
pub fn cancel_download() {
    CANCEL_DOWNLOAD.store(true, Ordering::SeqCst);
}

/// Reset cancel flag (call before starting a new download)
fn reset_cancel() {
    CANCEL_DOWNLOAD.store(false, Ordering::SeqCst);
}

/// Download installer and run it silently
pub async fn download_and_install(
    app: AppHandle,
    download_url: &str,
    file_name: &str,
) -> Result<(), String> {
    reset_cancel();

    let client = reqwest::Client::builder()
        .user_agent("UniFree-Updater")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    let total_size = response.content_length().unwrap_or(0);

    let download_dir = std::env::temp_dir();
    let file_path = download_dir.join(file_name);

    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    while let Some(chunk) = stream.next().await {
        // Check for cancellation
        if CANCEL_DOWNLOAD.load(Ordering::SeqCst) {
            drop(file);
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err("Download cancelled".into());
        }

        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        let percent = if total_size > 0 {
            (downloaded as f64 / total_size as f64) * 100.0
        } else {
            0.0
        };

        let _ = app.emit(
            "update-progress",
            UpdateProgress {
                downloaded,
                total: total_size,
                percent,
            },
        );
    }

    file.flush()
        .await
        .map_err(|e| format!("Flush error: {}", e))?;
    drop(file);

    // Run installer silently, wait for it, then relaunch the app.
    //
    // Strategy: spawn a detached cmd that:
    //   1. Waits ~4s for the current app to fully exit and release file locks,
    //   2. Runs the NSIS installer silently (/S) and waits for it to finish,
    //   3. Launches the freshly installed exe via start (don't wait).
    //
    // Using && instead of & ensures each step only runs if the previous succeeded.
    // The installer runs directly (not via start) so cmd properly waits for it.
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to resolve current exe: {}", e))?;
        let installer_str = file_path.display().to_string();
        let exe_str = exe_path.display().to_string();

        let cmd_line = format!(
            "ping -n 5 127.0.0.1 >nul && \"{installer}\" /S && start \"\" \"{exe}\"",
            installer = installer_str,
            exe = exe_str,
        );

        std::process::Command::new("cmd")
            .args(["/C", &cmd_line])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("Failed to start installer: {}", e))?;

        // Release file lock so installer can replace the exe.
        app.exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app; // avoid unused warning on non-Windows.
        return Err("Silent install is only supported on Windows".into());
    }

    Ok(())
}

/// Compare version strings (e.g. "2.3.0" vs "2.3.1")
fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    let current_parts = parse(current);
    let latest_parts = parse(latest);

    for i in 0..std::cmp::max(current_parts.len(), latest_parts.len()) {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let l = latest_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if c > l {
            return false;
        }
    }
    false
}

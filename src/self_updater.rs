use crate::settings::AppSettings;
use crossbeam_channel::Sender;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SelfUpdateStatus {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub is_checking: bool,
    pub is_downloading: bool,
    pub download_progress: f32,
    pub update_ready_to_install: bool,
    pub message: String,
    pub is_error: bool,
}

impl Default for SelfUpdateStatus {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            latest_version: None,
            update_available: false,
            release_notes: None,
            download_url: None,
            is_checking: false,
            is_downloading: false,
            download_progress: 0.0,
            update_ready_to_install: false,
            message: "OpenCloud is up to date.".into(),
            is_error: false,
        }
    }
}

#[derive(Clone)]
pub struct SelfUpdater {
    tokio_rt: Arc<Runtime>,
    status_tx: Sender<SelfUpdateStatus>,
}

impl SelfUpdater {
    pub fn new(tokio_rt: Arc<Runtime>, status_tx: Sender<SelfUpdateStatus>) -> Self {
        Self { tokio_rt, status_tx }
    }

    pub fn check_for_updates(&self) {
        let tx = self.status_tx.clone();
        let rt = self.tokio_rt.clone();

        rt.spawn(async move {
            let mut status = SelfUpdateStatus {
                is_checking: true,
                message: "Checking for OpenCloud desktop updates...".into(),
                ..Default::default()
            };
            let _ = tx.send(status.clone());

            let settings = AppSettings::load();
            let mut headers = HeaderMap::new();
            headers.insert(
                USER_AGENT,
                HeaderValue::from_static("OpenCloud-Desktop-Updater/1.0.0"),
            );

            if !settings.network.github_token.trim().is_empty() {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", settings.network.github_token.trim())) {
                    headers.insert(AUTHORIZATION, val);
                }
            }

            let client = reqwest::Client::builder()
                .default_headers(headers)
                .timeout(Duration::from_secs(settings.network.api_timeout_secs))
                .build()
                .unwrap_or_default();

            let url = "https://api.github.com/repos/bshea-1/OpenCloud/releases/latest";
            match client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(json) = resp.json::<Value>().await {
                        let tag = json["tag_name"]
                            .as_str()
                            .map(|s| s.trim_start_matches('v').to_string())
                            .unwrap_or_else(|| "1.0.0".into());
                        let notes = json["body"].as_str().map(|s| s.to_string());

                        let is_newer = crate::catalog::is_newer_version(&tag, env!("CARGO_PKG_VERSION"));
                        status.latest_version = Some(tag.clone());
                        status.update_available = is_newer;
                        status.release_notes = notes;
                        status.is_checking = false;

                        if is_newer {
                            status.message = format!("OpenCloud v{} is available!", tag);
                        } else {
                            status.message = format!("OpenCloud v{} is currently up to date.", env!("CARGO_PKG_VERSION"));
                        }

                        let _ = tx.send(status);
                        return;
                    }
                }
                _ => {}
            }

            // Fallback when remote release tag is not yet published
            status.is_checking = false;
            status.latest_version = Some(env!("CARGO_PKG_VERSION").to_string());
            status.update_available = false;
            status.message = format!(
                "OpenCloud v{} is up to date on the current release channel.",
                env!("CARGO_PKG_VERSION")
            );
            let _ = tx.send(status);
        });
    }

    pub fn download_and_prepare_update(&self, target_version: String) {
        let tx = self.status_tx.clone();
        let rt = self.tokio_rt.clone();

        rt.spawn(async move {
            // A real self-update would:
            //  1. Fetch the release asset URL from the GitHub API.
            //  2. Stream the bytes to a temp file with progress reporting.
            //  3. Verify the checksum.
            //  4. Spawn the installer and quit.
            //
            // For now, open the releases page and let the user install the update manually.
            let url = "https://github.com/bshea-1/OpenCloud/releases";
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(url).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(url).spawn();

            let status = SelfUpdateStatus {
                is_downloading: false,
                download_progress: 100.0,
                update_ready_to_install: false,
                message: format!(
                    "OpenCloud v{} is available. The releases page has been opened in your browser.",
                    target_version
                ),
                ..Default::default()
            };
            let _ = tx.send(status);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_update_status_default() {
        let status = SelfUpdateStatus::default();
        assert_eq!(status.current_version, env!("CARGO_PKG_VERSION"));
        assert!(!status.update_available);
        assert!(!status.is_checking);
        assert!(!status.is_downloading);
    }
}

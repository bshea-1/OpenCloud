use crate::installer::install_package;
use crate::models::DownloadProgress;
use crossbeam_channel::Sender;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::time::Duration;

pub async fn start_download_and_install(
    app_id: String,
    download_url: String,
    expected_sha256: Option<String>,
    progress_tx: Sender<DownloadProgress>,
) {
    let client = match reqwest::Client::builder()
        .user_agent("OpenCloud-Desktop/1.0.0")
        .timeout(Duration::from_secs(600))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = progress_tx.send(DownloadProgress {
                app_id,
                downloaded_bytes: 0,
                total_bytes: 0,
                percentage: 0.0,
                phase: "failed".into(),
                error: Some(e.to_string()),
                is_done: true,
            });
            return;
        }
    };

    let res = match client.get(&download_url).send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = progress_tx.send(DownloadProgress {
                app_id,
                downloaded_bytes: 0,
                total_bytes: 0,
                percentage: 0.0,
                phase: "failed".into(),
                error: Some(e.to_string()),
                is_done: true,
            });
            return;
        }
    };

    if !res.status().is_success() {
        let _ = progress_tx.send(DownloadProgress {
            app_id,
            downloaded_bytes: 0,
            total_bytes: 0,
            percentage: 0.0,
            phase: "failed".into(),
            error: Some(format!("HTTP error status: {}", res.status())),
            is_done: true,
        });
        return;
    }

    let total_size = res.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let filename = download_url
        .split('/')
        .last()
        .unwrap_or("package.bin")
        .to_string();
    let temp_dir = std::env::temp_dir().join("opencloud_downloads");
    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        let _ = progress_tx.send(DownloadProgress {
            app_id,
            downloaded_bytes: 0,
            total_bytes: 0,
            percentage: 0.0,
            phase: "failed".into(),
            error: Some(e.to_string()),
            is_done: true,
        });
        return;
    }

    let temp_file_path = temp_dir.join(&filename);
    let mut file = match tokio::fs::File::create(&temp_file_path).await {
        Ok(f) => f,
        Err(e) => {
            let _ = progress_tx.send(DownloadProgress {
                app_id,
                downloaded_bytes: 0,
                total_bytes: 0,
                percentage: 0.0,
                phase: "failed".into(),
                error: Some(e.to_string()),
                is_done: true,
            });
            return;
        }
    };

    use tokio::io::AsyncWriteExt;
    let mut stream = res.bytes_stream();
    let mut hasher = Sha256::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = match chunk_result {
            Ok(c) => c,
            Err(e) => {
                let _ = progress_tx.send(DownloadProgress {
                    app_id,
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    percentage: 0.0,
                    phase: "failed".into(),
                    error: Some(e.to_string()),
                    is_done: true,
                });
                return;
            }
        };

        if let Err(e) = file.write_all(&chunk).await {
            let _ = progress_tx.send(DownloadProgress {
                app_id,
                downloaded_bytes: downloaded,
                total_bytes: total_size,
                percentage: 0.0,
                phase: "failed".into(),
                error: Some(e.to_string()),
                is_done: true,
            });
            return;
        }

        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        let percentage = if total_size > 0 {
            (downloaded as f32 / total_size as f32) * 100.0
        } else {
            0.0
        };

        let _ = progress_tx.send(DownloadProgress {
            app_id: app_id.clone(),
            downloaded_bytes: downloaded,
            total_bytes: total_size,
            percentage,
            phase: "downloading".into(),
            error: None,
            is_done: false,
        });
    }

    let _ = file.flush().await;
    drop(file);

    let actual_sha256 = hex::encode(hasher.finalize());
    log::info!("Downloaded {}: computed SHA-256 = {}", app_id, actual_sha256);

    // Phase: Checksum verification
    let settings = crate::settings::AppSettings::load();
    if settings.security.verify_checksums {
        let _ = progress_tx.send(DownloadProgress {
            app_id: app_id.clone(),
            downloaded_bytes: total_size,
            total_bytes: total_size,
            percentage: 100.0,
            phase: "verifying checksum".into(),
            error: None,
            is_done: false,
        });

        if let Some(ref expected) = expected_sha256 {
            let expected_clean = expected.trim().to_lowercase();
            let actual_clean = actual_sha256.to_lowercase();

            if expected_clean != actual_clean {
                let _ = tokio::fs::remove_file(&temp_file_path).await;
                let err_msg = format!(
                    "Security check failed: Checksum mismatch for {}. Expected {}, computed {}",
                    app_id, expected_clean, actual_clean
                );
                log::error!("{}", err_msg);
                let _ = progress_tx.send(DownloadProgress {
                    app_id,
                    downloaded_bytes: total_size,
                    total_bytes: total_size,
                    percentage: 100.0,
                    phase: "failed".into(),
                    error: Some(err_msg),
                    is_done: true,
                });
                return;
            }
            log::info!("SHA-256 checksum verified successfully for {}: {}", app_id, actual_clean);
        } else if settings.security.strict_verification {
            let _ = tokio::fs::remove_file(&temp_file_path).await;
            let err_msg = format!(
                "Security policy violation: Strict verification requires an upstream SHA-256 checksum, but none was provided for {}",
                app_id
            );
            log::error!("{}", err_msg);
            let _ = progress_tx.send(DownloadProgress {
                app_id,
                downloaded_bytes: total_size,
                total_bytes: total_size,
                percentage: 100.0,
                phase: "failed".into(),
                error: Some(err_msg),
                is_done: true,
            });
            return;
        }
    }

    // Phase: Installing
    let _ = progress_tx.send(DownloadProgress {
        app_id: app_id.clone(),
        downloaded_bytes: total_size,
        total_bytes: total_size,
        percentage: 100.0,
        phase: "installing".into(),
        error: None,
        is_done: false,
    });

    let install_res = install_package(&app_id, &temp_file_path);
    let _ = tokio::fs::remove_file(&temp_file_path).await;

    match install_res {
        Ok(_) => {
            let _ = progress_tx.send(DownloadProgress {
                app_id,
                downloaded_bytes: total_size,
                total_bytes: total_size,
                percentage: 100.0,
                phase: "completed".into(),
                error: None,
                is_done: true,
            });
        }
        Err(e) => {
            let _ = progress_tx.send(DownloadProgress {
                app_id,
                downloaded_bytes: total_size,
                total_bytes: total_size,
                percentage: 100.0,
                phase: "failed".into(),
                error: Some(e),
                is_done: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sha256_computation() {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"opencloud test payload");
        let hash = hex::encode(hasher.finalize());
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, "81db95a2eef7b85370bcd2e52eb3e0948d5cd5780a8bb477255290bcdafc1298");
    }
}

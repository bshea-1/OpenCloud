use crate::catalog::refresh_remote_releases;
use crate::downloader::start_download_and_install;
use crate::models::{DownloadProgress, InstallState, SuiteApp};
use crate::settings::AppSettings;
use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct AutoUpdateEvent {
    pub message: String,
    pub is_error: bool,
}

pub struct AutoUpdater {
    tokio_rt: Arc<Runtime>,
    event_tx: Sender<AutoUpdateEvent>,
    catalog_tx: Sender<Vec<SuiteApp>>,
    progress_tx: Sender<DownloadProgress>,
    manual_tx: UnboundedSender<Vec<SuiteApp>>,
    manual_rx: Mutex<Option<UnboundedReceiver<Vec<SuiteApp>>>>,
}

impl AutoUpdater {
    pub fn new(
        tokio_rt: Arc<Runtime>,
        event_tx: Sender<AutoUpdateEvent>,
        catalog_tx: Sender<Vec<SuiteApp>>,
        progress_tx: Sender<DownloadProgress>,
    ) -> Self {
        let (manual_tx, manual_rx) = unbounded_channel();
        Self {
            tokio_rt,
            event_tx,
            catalog_tx,
            progress_tx,
            manual_tx,
            manual_rx: Mutex::new(Some(manual_rx)),
        }
    }

    pub fn start_background_loop(&self, initial_apps: Vec<SuiteApp>) {
        let rt = self.tokio_rt.clone();
        let event_tx = self.event_tx.clone();
        let catalog_tx = self.catalog_tx.clone();
        let progress_tx = self.progress_tx.clone();
        let mut manual_rx = self
            .manual_rx
            .lock()
            .unwrap()
            .take()
            .expect("AutoUpdater manual_rx should be present on start");

        rt.spawn(async move {
            let check_interval = Duration::from_secs(45 * 60);

            loop {
                // Wait for either the 45-minute background timer or a manual check trigger.
                // Receiving a manual check resets the 45m timer cleanly.
                let apps_to_check = tokio::select! {
                    _ = tokio::time::sleep(check_interval) => {
                        initial_apps.clone()
                    }
                    Some(manual_apps) = manual_rx.recv() => {
                        manual_apps
                    }
                };

                let mut apps = apps_to_check;
                refresh_remote_releases(&mut apps).await;
                let _ = catalog_tx.send(apps.clone());

                let settings = AppSettings::load();
                if settings.apps.auto_update_apps {
                    let mut updated_any = false;

                    for app in &apps {
                        if matches!(app.install_state, InstallState::UpdateAvailable) {
                            if settings.apps.is_app_auto_update_enabled(&app.id) {
                                if let Some(ref asset) = app.download_asset {
                                    let _ = event_tx.send(AutoUpdateEvent {
                                        message: format!("Auto-updating {} in background...", app.name),
                                        is_error: false,
                                    });

                                    start_download_and_install(
                                        app.id.clone(),
                                        asset.download_url.clone(),
                                        progress_tx.clone(),
                                    )
                                    .await;

                                    updated_any = true;
                                }
                            }
                        }
                    }

                    if updated_any {
                        let _ = event_tx.send(AutoUpdateEvent {
                            message: "Auto-update cycle complete. Applications updated.".into(),
                            is_error: false,
                        });
                    }
                }
            }
        });
    }

    pub fn trigger_manual_check(&self, current_apps: Vec<SuiteApp>) {
        let _ = self.manual_tx.send(current_apps);
    }
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autoupdate;
mod catalog;
mod downloader;
mod fonts;
mod installer;
mod launcher;
mod models;
mod platform;
mod recents;
mod scanner;
mod self_updater;
mod settings;
mod ui;

use std::sync::Arc;
use ui::app::OpenCloudApp;

fn load_app_icon() -> Option<Arc<egui::IconData>> {
    let icon_bytes = include_bytes!("../assets/icons/app_icon.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(Arc::new(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        }))
    } else {
        None
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    platform::configure_macos_metadata();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([960.0, 640.0])
        .with_title("OpenCloud")
        .with_resizable(true);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "OpenCloud",
        native_options,
        Box::new(|cc| Ok(Box::new(OpenCloudApp::new(cc)))),
    )
}

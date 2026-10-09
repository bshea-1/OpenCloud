use crate::fonts::{activate_font, deactivate_font, is_font_installed, STUDIO_FONTS};
use crate::self_updater::{SelfUpdateStatus, SelfUpdater};
use crate::settings::AppSettings;
use crate::ui::theme::ThemePalette;
use egui::{pos2, Color32, CornerRadius, FontId, Margin, Rect, RichText, ScrollArea, Stroke, Ui};
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SettingsTab {
    General,
    Apps,
    FilesSync,
    Storage,
    Fonts,
    Network,
    About,
}

pub struct SettingsViewState {
    pub current_tab: SettingsTab,
    pub settings: AppSettings,
    pub save_status: Option<(String, bool)>,
    pub cache_size_str: String,
    pub self_update_status: SelfUpdateStatus,
    self_updater: Option<SelfUpdater>,
    pub test_connection_status: Option<(String, bool)>,
}

impl SettingsViewState {
    pub fn new() -> Self {
        let settings = AppSettings::load();
        let cache_size_str = calculate_directory_size(&settings.storage.cache_directory);
        Self {
            current_tab: SettingsTab::General,
            settings,
            save_status: None,
            cache_size_str,
            self_update_status: SelfUpdateStatus::default(),
            self_updater: None,
            test_connection_status: None,
        }
    }

    pub fn set_self_updater(&mut self, updater: SelfUpdater) {
        self.self_updater = Some(updater);
    }

    pub fn save(&mut self) {
        match self.settings.save() {
            Ok(_) => {
                self.save_status = Some(("Preferences saved successfully.".into(), false));
            }
            Err(e) => {
                self.save_status = Some((format!("Failed to save preferences: {}", e), true));
            }
        }
    }

    pub fn purge_cache(&mut self) {
        let p = std::path::Path::new(&self.settings.storage.cache_directory);
        if p.exists() {
            let _ = std::fs::remove_dir_all(p);
            let _ = std::fs::create_dir_all(p);
        }
        self.cache_size_str = calculate_directory_size(&self.settings.storage.cache_directory);
        self.save_status = Some(("Installer and package cache purged.".into(), false));
    }

    pub fn check_self_update(&mut self) {
        if let Some(ref updater) = self.self_updater {
            updater.check_for_updates();
        }
    }

    pub fn apply_self_update(&mut self) {
        if let Some(ref updater) = self.self_updater {
            let target = self
                .self_update_status
                .latest_version
                .clone()
                .unwrap_or_else(|| "0.2.0".into());
            updater.download_and_prepare_update(target);
        }
    }
}

fn calculate_directory_size(path_str: &str) -> String {
    let p = std::path::Path::new(path_str);
    if !p.exists() {
        return "0 MB".into();
    }
    let mut total_bytes: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(p) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total_bytes += meta.len();
            }
        }
    }
    let mb = total_bytes as f64 / (1024.0 * 1024.0);
    format!("{:.1} MB", mb)
}

fn styled_text_edit<'a>(
    text: &'a mut String,
    width: f32,
    palette: &ThemePalette,
) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(text)
        .desired_width(width)
        .min_size(egui::vec2(width, 34.0))
        .margin(Margin::symmetric(12, 8))
        .font(FontId::proportional(13.0))
        .text_color(palette.text_primary)
}

fn render_folder_icon(painter: &egui::Painter, rect: Rect, is_hovered: bool) {
    let cx = rect.center().x;
    let cy = rect.center().y;

    // Tab: top-left (7px wide, 3px high)
    let tab_rect = Rect::from_min_max(
        pos2(cx - 8.0, cy - 7.0),
        pos2(cx - 1.0, cy - 4.0),
    );
    // Folder back body (16px wide, 11px high)
    let back_rect = Rect::from_min_max(
        pos2(cx - 8.0, cy - 5.0),
        pos2(cx + 8.0, cy + 6.0),
    );
    // Folder front cover (16px wide, 8px high)
    let front_rect = Rect::from_min_max(
        pos2(cx - 8.0, cy - 2.0),
        pos2(cx + 8.0, cy + 6.0),
    );

    let (stroke_color, back_fill, front_fill) = if is_hovered {
        (
            Color32::WHITE,
            Color32::from_rgba_unmultiplied(255, 255, 255, 200),
            Color32::from_rgba_unmultiplied(255, 255, 255, 255),
        )
    } else {
        (
            Color32::from_rgba_unmultiplied(255, 255, 255, 240),
            Color32::from_rgba_unmultiplied(255, 255, 255, 140),
            Color32::from_rgba_unmultiplied(255, 255, 255, 220),
        )
    };

    painter.rect_filled(tab_rect, CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }, back_fill);
    painter.rect_filled(back_rect, CornerRadius::same(2), back_fill);
    painter.rect_filled(front_rect, CornerRadius::same(2), front_fill);

    painter.rect_stroke(back_rect, CornerRadius::same(2), Stroke::new(1.0, stroke_color), egui::StrokeKind::Outside);
    painter.rect_stroke(front_rect, CornerRadius::same(2), Stroke::new(1.0, stroke_color), egui::StrokeKind::Outside);
    painter.line_segment([pos2(cx - 7.0, cy - 1.0), pos2(cx + 7.0, cy - 1.0)], Stroke::new(0.8, Color32::from_black_alpha(45)));
}

fn render_browse_button(ui: &mut Ui, path: &mut String, palette: &ThemePalette) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::click());

    let is_hovered = response.hovered();
    let bg_color = if response.is_pointer_button_down_on() {
        palette.sidebar_active
    } else if is_hovered {
        palette.button_secondary_hover
    } else {
        palette.button_secondary_bg
    };

    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        bg_color,
        Stroke::new(1.0, if is_hovered { palette.border_active } else { palette.border }),
        egui::StrokeKind::Inside,
    );

    render_folder_icon(ui.painter(), rect, is_hovered);

    let mut changed = false;
    if response.clicked() {
        let mut dialog = rfd::FileDialog::new();
        let p = std::path::Path::new(path.as_str());
        if p.exists() {
            dialog = dialog.set_directory(p);
        }
        if let Some(selected_folder) = dialog.pick_folder() {
            *path = selected_folder.to_string_lossy().to_string();
            changed = true;
        }
    }

    response.on_hover_text("Select folder...");
    changed
}

pub fn render_settings(
    ui: &mut Ui,
    state: &mut SettingsViewState,
    rt: Arc<Runtime>,
    palette: &ThemePalette,
) {
    ui.horizontal(|ui| {
        ui.heading(
            RichText::new("Preferences")
                .font(FontId::proportional(22.0))
                .color(palette.text_primary)
                .strong(),
        );

        ui.label(
            RichText::new("Studio Configuration & Automation")
                .color(palette.text_muted)
                .font(FontId::proportional(13.0)),
        );
    });

    ui.add_space(14.0);

    // Preferences Tab Bar
    ui.horizontal(|ui| {
        let tabs = [
            (SettingsTab::General, "General"),
            (SettingsTab::Apps, "Apps & Updates"),
            (SettingsTab::FilesSync, "Files & Sync"),
            (SettingsTab::Storage, "Storage & Cache"),
            (SettingsTab::Fonts, "Fonts"),
            (SettingsTab::Network, "Network & Proxy"),
            (SettingsTab::About, "About & Self-Update"),
        ];

        for (tab, label) in tabs {
            if ui
                .selectable_label(state.current_tab == tab, label)
                .clicked()
            {
                state.current_tab = tab;
                state.save_status = None;
            }
        }
    });

    ui.separator();
    ui.add_space(10.0);

    let scroll_h = (ui.available_height() - 56.0).max(100.0);
    ScrollArea::vertical().max_height(scroll_h).show(ui, |ui| {
        match state.current_tab {
            SettingsTab::General => render_general_tab(ui, state, palette),
            SettingsTab::Apps => render_apps_tab(ui, state, palette),
            SettingsTab::FilesSync => render_sync_tab(ui, state, palette),
            SettingsTab::Storage => render_storage_tab(ui, state, palette),
            SettingsTab::Fonts => render_fonts_tab(ui, palette),
            SettingsTab::Network => render_network_tab(ui, state, rt, palette),
            SettingsTab::About => render_about_tab(ui, state, palette),
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui
            .button(
                RichText::new("Save Preferences")
                    .color(Color32::WHITE)
                    .strong(),
            )
            .clicked()
        {
            state.save();
        }

        if ui
            .button(
                RichText::new("Reset All Settings")
                    .color(Color32::from_rgb(239, 68, 68))
                    .strong(),
            )
            .clicked()
        {
            state.settings = AppSettings::default();
            state.save();
            state.save_status = Some(("All preferences have been reset to factory defaults.".into(), false));
        }

        if let Some((ref msg, is_err)) = state.save_status {
            let col = if is_err {
                Color32::from_rgb(239, 68, 68)
            } else {
                palette.success_green
            };
            ui.colored_label(col, msg);
        }
    });
}

fn render_general_tab(ui: &mut Ui, state: &mut SettingsViewState, palette: &ThemePalette) {
    ui.label(
        RichText::new("Startup & System")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    if ui
        .checkbox(
            &mut state.settings.general.launch_at_startup,
            "Launch OpenCloud at login",
        )
        .changed()
    {
        state.save();
    }

    if ui
        .checkbox(
            &mut state.settings.general.keep_running_in_background,
            "Keep OpenCloud running in the background when the main window is closed",
        )
        .changed()
    {
        state.save();
    }

    if ui
        .checkbox(
            &mut state.settings.general.open_in_background,
            "Open minimized in the system tray / menu bar on system login",
        )
        .changed()
    {
        state.save();
    }

    ui.add_space(16.0);
    ui.label(
        RichText::new("OpenCloud Updates")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    if ui
        .checkbox(
            &mut state.settings.general.auto_update_opencloud,
            "Always keep OpenCloud desktop app up to date",
        )
        .changed()
    {
        state.save();
    }

    ui.add_space(16.0);
    ui.label(
        RichText::new("Notifications")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    if ui
        .checkbox(
            &mut state.settings.general.enable_desktop_notifications,
            "Show desktop notifications when new app updates are available",
        )
        .changed()
    {
        state.save();
    }

    if ui
        .checkbox(
            &mut state.settings.general.notify_when_download_completes,
            "Notify when application installations and updates finish",
        )
        .changed()
    {
        state.save();
    }

    if ui
        .checkbox(
            &mut state.settings.general.notify_sync_issues,
            "Notify if creative file syncing encounters conflicts or is paused",
        )
        .changed()
    {
        state.save();
    }

    ui.add_space(16.0);
    ui.label(
        RichText::new("Appearance & Theme")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.label(
        RichText::new("Select your workspace visual style. Changes apply immediately.")
            .font(FontId::proportional(12.0))
            .color(palette.text_muted),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        if ui
            .radio_value(
                &mut state.settings.general.studio_theme,
                "light".into(),
                "Studio Light",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.general.studio_theme,
                "obsidian".into(),
                "Obsidian Dark",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.general.studio_theme,
                "slate".into(),
                "Studio Slate",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.general.studio_theme,
                "midnight".into(),
                "Midnight Navy",
            )
            .changed()
        {
            state.save();
        }
    });

    ui.add_space(16.0);
    ui.label(
        RichText::new("Application Language")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    let languages = [
        "English (North America)",
        "English (International)",
        "Spanish",
        "French",
        "Deutsch",
        "Japanese",
        "Chinese",
    ];

    ui.horizontal(|ui| {
        for lang in languages {
            if ui
                .radio_value(
                    &mut state.settings.general.app_language,
                    lang.into(),
                    lang,
                )
                .changed()
            {
                state.save();
            }
        }
    });

    ui.add_space(20.0);
    ui.label(
        RichText::new("Reset Preferences")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.label(
        RichText::new("Restore all OpenCloud settings, installation paths, and preferences to their factory defaults.")
            .font(FontId::proportional(12.0))
            .color(palette.text_muted),
    );
    ui.add_space(6.0);

    if ui
        .button(
            RichText::new("Reset All Settings to Defaults")
                .font(FontId::proportional(13.0))
                .color(Color32::from_rgb(239, 68, 68)),
        )
        .clicked()
    {
        state.settings = AppSettings::default();
        state.save();
        state.save_status = Some(("All preferences have been reset to factory defaults.".into(), false));
    }
}

fn render_apps_tab(ui: &mut Ui, state: &mut SettingsViewState, palette: &ThemePalette) {
    ui.label(
        RichText::new("Installing")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.label(RichText::new("Default Install Location:").color(palette.text_muted));
    ui.horizontal(|ui| {
        if ui
            .add(styled_text_edit(&mut state.settings.apps.install_directory, 450.0, palette))
            .changed()
        {
            state.save();
        }
        if render_browse_button(ui, &mut state.settings.apps.install_directory, palette) {
            state.save();
        }
        if ui.add_sized([110.0, 34.0], egui::Button::new("Reset to Default")).clicked() {
            #[cfg(target_os = "macos")]
            {
                state.settings.apps.install_directory = "/Applications".into();
            }
            #[cfg(target_os = "windows")]
            {
                state.settings.apps.install_directory = "C:\\Program Files\\OpenCloud".into();
            }
            #[cfg(target_os = "linux")]
            {
                state.settings.apps.install_directory = dirs::home_dir()
                    .map(|p| p.join(".local/bin").to_string_lossy().to_string())
                    .unwrap_or_else(|| "/usr/local/bin".into());
            }
            state.save();
            state.save_status = Some(("Install location reset to default.".into(), false));
        }
    });

    ui.add_space(8.0);
    if ui
        .checkbox(
            &mut state.settings.apps.opt_in_prereleases,
            "Include preview, beta, and release-candidate builds",
        )
        .changed()
    {
        state.save();
    }

    ui.add_space(20.0);
    ui.label(
        RichText::new("Auto-Update")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    if ui
        .checkbox(
            &mut state.settings.apps.auto_update_apps,
            "Auto-update applications",
        )
        .changed()
    {
        state.save();
    }

    if state.settings.apps.auto_update_apps {
        ui.indent("auto_update_suboptions", |ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new("Advanced options:")
                    .font(FontId::proportional(13.0))
                    .color(palette.text_muted)
                    .strong(),
            );

            if ui
                .checkbox(
                    &mut state.settings.apps.import_previous_settings,
                    "Import previous settings and preferences",
                )
                .changed()
            {
                state.save();
            }

            if ui
                .checkbox(
                    &mut state.settings.apps.remove_previous_versions,
                    "Remove older versions",
                )
                .changed()
            {
                state.save();
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Background Check Frequency:").color(palette.text_muted));
                if ui
                    .add(
                        egui::DragValue::new(&mut state.settings.apps.check_interval_hours)
                            .range(1..=72),
                    )
                    .changed()
                {
                    state.save();
                }
                ui.label("hours");
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Max Concurrent Downloads:").color(palette.text_muted));
                if ui
                    .add(
                        egui::DragValue::new(&mut state.settings.apps.max_parallel_downloads)
                            .range(1..=8),
                    )
                    .changed()
                {
                    state.save();
                }
                ui.label("parallel tasks");
            });

            ui.add_space(12.0);
            ui.label(
                RichText::new("Individual App Auto-Update Settings:")
                    .font(FontId::proportional(13.0))
                    .color(palette.text_primary)
                    .strong(),
            );
            ui.label(
                RichText::new("Enable or disable automatic background updates for specific applications.")
                    .font(FontId::proportional(11.0))
                    .color(palette.text_muted),
            );
            ui.add_space(6.0);

            let app_list = [
                ("photocraft", "PhotoCraft"),
                ("vectorcraft", "VectorCraft"),
                ("filmcraft", "FilmCraft"),
                ("lightcraft", "LightCraft"),
                ("pdfcraft", "PdfCraft"),
                ("effectcraft", "EffectCraft"),
                ("designcraft", "DesignCraft"),
                ("soundcraft", "SoundCraft"),
                ("cadcraft", "CadCraft"),
                ("deckcraft", "DeckCraft"),
                ("gridcraft", "GridCraft"),
                ("wordcraft", "WordCraft"),
            ];

            for (id, name) in app_list {
                let mut is_enabled = state.settings.apps.is_app_auto_update_enabled(id);
                if ui.checkbox(&mut is_enabled, name).changed() {
                    state.settings.apps.set_app_auto_update(id, is_enabled);
                    state.save();
                }
            }
        });
    }
}

fn render_sync_tab(ui: &mut Ui, state: &mut SettingsViewState, palette: &ThemePalette) {
    ui.label(
        RichText::new("OpenCloud Sync & Workspaces")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let sync_status_text = if state.settings.sync.sync_enabled {
            RichText::new("Syncing is Active")
                .color(palette.success_green)
                .strong()
        } else {
            RichText::new("Syncing is Paused")
                .color(palette.update_amber)
                .strong()
        };
        ui.label(sync_status_text);

        let toggle_label = if state.settings.sync.sync_enabled {
            "Pause Syncing"
        } else {
            "Resume Syncing"
        };
        if ui.button(toggle_label).clicked() {
            state.settings.sync.sync_enabled = !state.settings.sync.sync_enabled;
            state.save();
        }
    });

    ui.add_space(16.0);
    ui.label(RichText::new("Shared Creative Assets Folder:").color(palette.text_muted));
    ui.horizontal(|ui| {
        if ui
            .add(styled_text_edit(&mut state.settings.sync.shared_assets_path, 450.0, palette))
            .changed()
        {
            state.save();
        }
        if render_browse_button(ui, &mut state.settings.sync.shared_assets_path, palette) {
            state.save();
        }
    });

    ui.add_space(10.0);
    ui.label(RichText::new("Color Swatches & Palettes Folder:").color(palette.text_muted));
    ui.horizontal(|ui| {
        if ui
            .add(styled_text_edit(&mut state.settings.sync.shared_palettes_path, 450.0, palette))
            .changed()
        {
            state.save();
        }
        if render_browse_button(ui, &mut state.settings.sync.shared_palettes_path, palette) {
            state.save();
        }
    });

    ui.add_space(16.0);
    ui.label(
        RichText::new("Transfer Speed & Bandwidth Limit")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        if ui
            .radio_value(
                &mut state.settings.sync.transfer_rate_limit_mb,
                0,
                "Unlimited",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.sync.transfer_rate_limit_mb,
                5,
                "5 MB/s",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.sync.transfer_rate_limit_mb,
                10,
                "10 MB/s",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.sync.transfer_rate_limit_mb,
                25,
                "25 MB/s",
            )
            .changed()
        {
            state.save();
        }
    });
}

fn render_storage_tab(ui: &mut Ui, state: &mut SettingsViewState, palette: &ThemePalette) {
    ui.label(
        RichText::new("Package & Installer Cache")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.label(RichText::new("Download Cache Directory:").color(palette.text_muted));
    ui.horizontal(|ui| {
        if ui
            .add(styled_text_edit(&mut state.settings.storage.cache_directory, 450.0, palette))
            .changed()
        {
            state.save();
            state.cache_size_str = calculate_directory_size(&state.settings.storage.cache_directory);
        }
        if render_browse_button(ui, &mut state.settings.storage.cache_directory, palette) {
            state.save();
            state.cache_size_str = calculate_directory_size(&state.settings.storage.cache_directory);
        }
    });

    ui.add_space(12.0);
    if ui
        .checkbox(
            &mut state.settings.storage.clean_installers_after_install,
            "Automatically delete installer DMGs / ZIPs after installation completes",
        )
        .changed()
    {
        state.save();
    }

    ui.add_space(16.0);
    ui.label(
        RichText::new("Cache Size Limits & Maintenance")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Max Cache Limit:").color(palette.text_muted));
        if ui.radio_value(&mut state.settings.storage.max_cache_size_gb, 5, "5 GB").changed() {
            state.save();
        }
        if ui.radio_value(&mut state.settings.storage.max_cache_size_gb, 10, "10 GB").changed() {
            state.save();
        }
        if ui.radio_value(&mut state.settings.storage.max_cache_size_gb, 25, "25 GB").changed() {
            state.save();
        }
        if ui.radio_value(&mut state.settings.storage.max_cache_size_gb, 0, "Unlimited").changed() {
            state.save();
        }
    });

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Current Cache Size: {}", state.cache_size_str))
                .color(palette.text_primary)
                .strong(),
        );

        if ui.button("Purge Download Cache").clicked() {
            state.purge_cache();
            state.save();
        }
    });
}

fn render_fonts_tab(ui: &mut Ui, palette: &ThemePalette) {
    ui.label(
        RichText::new("Creative Fonts Management")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.label(
        RichText::new("Browse and activate open-source design typefaces directly into your operating system's font library for PhotoCraft, VectorCraft, and FilmCraft.")
            .color(palette.text_muted),
    );
    ui.add_space(14.0);

    for font in STUDIO_FONTS {
        let is_installed = is_font_installed(font.file_name);

        ui.scope(|ui| {
            let card_rect = ui
                .vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(font.name)
                                        .font(FontId::proportional(16.0))
                                        .color(palette.text_primary)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(format!("- {}", font.category))
                                        .font(FontId::proportional(12.0))
                                        .color(palette.text_muted),
                                );
                            });

                            ui.label(
                                RichText::new(font.description)
                                    .font(FontId::proportional(12.0))
                                    .color(palette.text_muted),
                            );

                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(font.sample_text)
                                    .font(FontId::proportional(14.0))
                                    .color(palette.text_primary),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_installed {
                                ui.colored_label(palette.success_green, "Active in System");
                                if ui.small_button("Deactivate").clicked() {
                                    let _ = deactivate_font(font.file_name);
                                }
                            } else {
                                if ui
                                    .button(
                                        RichText::new("Activate Font")
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    let _ = activate_font(font);
                                }
                            }
                        });
                    });
                })
                .response
                .rect;

            ui.painter().rect_stroke(
                card_rect.expand(6.0),
                CornerRadius::same(6),
                Stroke::new(1.0, palette.border),
                egui::StrokeKind::Outside,
            );
        });

        ui.add_space(14.0);
    }
}

fn render_network_tab(
    ui: &mut Ui,
    state: &mut SettingsViewState,
    rt: Arc<Runtime>,
    palette: &ThemePalette,
) {
    ui.label(
        RichText::new("Network & Connectivity")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Network Timeout:").color(palette.text_muted));
        if ui
            .add(egui::DragValue::new(&mut state.settings.network.api_timeout_secs).range(3..=60))
            .changed()
        {
            state.save();
        }
        ui.label("seconds");
    });

    ui.add_space(16.0);
    ui.label(
        RichText::new("Proxy Configuration")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        if ui
            .radio_value(
                &mut state.settings.network.proxy_mode,
                "direct".into(),
                "Direct Connection",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.network.proxy_mode,
                "system".into(),
                "System Proxy",
            )
            .changed()
        {
            state.save();
        }
        if ui
            .radio_value(
                &mut state.settings.network.proxy_mode,
                "custom".into(),
                "Custom Proxy",
            )
            .changed()
        {
            state.save();
        }
    });

    if state.settings.network.proxy_mode == "custom" {
        ui.add_space(6.0);
        ui.label(RichText::new("Proxy Server URL (http://host:port):").color(palette.text_muted));
        if ui
            .add(
                styled_text_edit(&mut state.settings.network.proxy_url, 450.0, palette)
                    .hint_text("http://127.0.0.1:8080"),
            )
            .changed()
        {
            state.save();
        }
    }

    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if ui.button("Test Network Connection").clicked() {
            let token = state.settings.network.github_token.clone();
            state.test_connection_status = Some(("Testing connectivity...".into(), false));

            let (tx, rx) = std::sync::mpsc::channel();
            rt.spawn(async move {
                let mut client_builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5));
                if !token.trim().is_empty() {
                    client_builder = client_builder.default_headers({
                        let mut h = reqwest::header::HeaderMap::new();
                        h.insert(
                            reqwest::header::AUTHORIZATION,
                            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token.trim())).unwrap(),
                        );
                        h
                    });
                }
                let client = client_builder.build().unwrap_or_default();
                let start = std::time::Instant::now();
                match client.get("https://api.github.com/zen").header(reqwest::header::USER_AGENT, "OpenCloud").send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let elapsed = start.elapsed().as_millis();
                        let _ = tx.send((format!("Connection successful! (Latency: {}ms)", elapsed), false));
                    }
                    Ok(resp) => {
                        let _ = tx.send((format!("Remote server responded with HTTP {}", resp.status()), true));
                    }
                    Err(e) => {
                        let _ = tx.send((format!("Connection failed: {}", e), true));
                    }
                }
            });

            if let Ok(res) = rx.recv_timeout(std::time::Duration::from_millis(500)) {
                state.test_connection_status = Some(res);
            }
        }

        if let Some((ref msg, is_err)) = state.test_connection_status {
            let col = if is_err {
                Color32::from_rgb(239, 68, 68)
            } else {
                palette.success_green
            };
            ui.colored_label(col, msg);
        }
    });
}

fn render_about_tab(ui: &mut Ui, state: &mut SettingsViewState, palette: &ThemePalette) {
    ui.label(
        RichText::new("About OpenCloud Desktop")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    ui.label(
        RichText::new("OpenCloud is an open-source, high-performance desktop studio workspace and manager for pure-Rust creative applications.")
            .color(palette.text_muted),
    );

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Installed Version:").color(palette.text_muted));
        ui.label(
            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                .color(palette.text_primary)
                .strong(),
        );
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Host Platform:").color(palette.text_muted));
        ui.label(
            RichText::new(format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH))
                .color(palette.text_primary),
        );
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Rendering Engine:").color(palette.text_muted));
        ui.label(RichText::new("WGPU Native Hardware Acceleration").color(palette.text_primary));
    });

    ui.add_space(20.0);
    ui.label(
        RichText::new("OpenCloud Desktop Updates")
            .font(FontId::proportional(15.0))
            .color(palette.text_primary)
            .strong(),
    );
    ui.add_space(6.0);

    let is_checking = state.self_update_status.is_checking;
    let update_available = state.self_update_status.update_available;
    let is_downloading = state.self_update_status.is_downloading;
    let download_progress = state.self_update_status.download_progress;
    let latest_version = state.self_update_status.latest_version.clone();

    let mut trigger_check = false;
    let mut trigger_update = false;

    ui.horizontal(|ui| {
        if is_checking {
            ui.label(RichText::new("Checking for updates...").color(palette.update_amber));
        } else if update_available {
            ui.colored_label(
                palette.update_amber,
                format!("New version v{} available!", latest_version.as_deref().unwrap_or("")),
            );
        } else {
            ui.colored_label(palette.success_green, "OpenCloud is up to date");
        }

        if ui.button("Check for OpenCloud Updates").clicked() {
            trigger_check = true;
        }
    });

    if update_available {
        ui.add_space(8.0);
        if ui
            .button(
                RichText::new("Download & Apply OpenCloud Update")
                    .color(Color32::BLACK)
                    .strong(),
            )
            .clicked()
        {
            trigger_update = true;
        }
    }

    if is_downloading {
        ui.add_space(8.0);
        ui.add(egui::ProgressBar::new(download_progress / 100.0).desired_width(200.0));
    }

    if trigger_check {
        state.check_self_update();
    }
    if trigger_update {
        state.apply_self_update();
    }
}

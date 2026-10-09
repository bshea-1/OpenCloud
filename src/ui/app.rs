use crate::autoupdate::{AutoUpdateEvent, AutoUpdater};
use crate::catalog::{get_initial_apps, refresh_remote_releases};
use crate::downloader::start_download_and_install;
use crate::installer::uninstall_app;
use crate::launcher::launch_app;
use crate::models::{AppCategory, DownloadProgress, InstallState, RecentProject, SuiteApp};
use crate::recents::get_recent_projects;
use crate::self_updater::{SelfUpdateStatus, SelfUpdater};
use crate::ui::logos::{render_github_icon, render_github_icon_with_id, render_logo};
use crate::ui::settings_view::{render_settings, SettingsTab, SettingsViewState};
use crate::ui::theme::{apply_studio_theme, ThemePalette};
use crossbeam_channel::{unbounded, Receiver, Sender};
use egui::{
    pos2, vec2, Align, CornerRadius, FontId, Layout, Margin, Rect, RichText,
    ScrollArea, Stroke, Ui,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NavTab {
    Apps,
    #[allow(dead_code)]
    YourWork,
    #[allow(dead_code)]
    Fonts,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    InstallSuccess,   // Green
    UninstallSuccess, // Red
    Info,             // Blue
    Error,            // Red
}

#[derive(Clone, Debug)]
pub struct ToastNotification {
    pub title: String,
    pub message: String,
    pub kind: ToastKind,
    pub created_at: std::time::Instant,
    pub duration_secs: f32,
}

pub struct OpenCloudApp {
    apps: Vec<SuiteApp>,
    recent_projects: Vec<RecentProject>,
    current_tab: NavTab,
    selected_category: Option<AppCategory>,
    filter_only_installed: bool,
    filter_only_updates: bool,
    search_query: String,
    toast: Option<ToastNotification>,
    is_refreshing: bool,
    pub is_checking_updates: bool,
    pub manual_update_requested: bool,
    pub selected_app_id: Option<String>,
    pub app_detail_state: crate::ui::app_detail::AppDetailState,

    // Animated view transition tracking
    current_view_key: u32,
    view_transition_start: f64,

    // Settings state
    settings_state: SettingsViewState,

    // Cache the last applied theme name to avoid rebuilding egui Style every frame
    last_applied_theme: String,

    // Async runtime & channels
    tokio_rt: Arc<Runtime>,
    progress_tx: Sender<DownloadProgress>,
    progress_rx: Receiver<DownloadProgress>,
    catalog_rx: Receiver<Vec<SuiteApp>>,
    auto_update_rx: Receiver<AutoUpdateEvent>,
    #[allow(dead_code)]
    auto_updater: Arc<AutoUpdater>,
    self_update_rx: Receiver<SelfUpdateStatus>,
    #[allow(dead_code)]
    self_updater: Arc<SelfUpdater>,
}

impl OpenCloudApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let rt = Arc::new(Runtime::new().expect("Failed to initialize Tokio runtime"));
        let (progress_tx, progress_rx) = unbounded();
        let (catalog_tx, catalog_rx) = unbounded();
        let (auto_update_tx, auto_update_rx) = unbounded();
        let (self_update_tx, self_update_rx) = unbounded();

        let initial_apps = get_initial_apps();
        let initial_recents = get_recent_projects();
        let mut settings_state = SettingsViewState::new();

        // Initialize background AutoUpdater service
        let auto_updater = Arc::new(AutoUpdater::new(
            rt.clone(),
            auto_update_tx,
            catalog_tx.clone(),
            progress_tx.clone(),
        ));
        auto_updater.start_background_loop(initial_apps.clone());

        // Initialize SelfUpdater for OpenCloud desktop app itself
        let self_updater = Arc::new(SelfUpdater::new(rt.clone(), self_update_tx));
        settings_state.set_self_updater((*self_updater).clone());

        // Check for OpenCloud desktop updates on launch
        self_updater.check_for_updates();

        // Initial background query for latest GitHub releases
        let rt_clone = rt.clone();
        let tx_clone = catalog_tx.clone();
        let mut apps_for_refresh = initial_apps.clone();
        rt_clone.spawn(async move {
            refresh_remote_releases(&mut apps_for_refresh).await;
            let _ = tx_clone.send(apps_for_refresh);
        });

        Self {
            apps: initial_apps,
            recent_projects: initial_recents,
            current_tab: NavTab::Apps,
            selected_category: None,
            filter_only_installed: false,
            filter_only_updates: false,
            search_query: String::new(),
            toast: None,
            is_refreshing: true,
            is_checking_updates: false,
            manual_update_requested: false,
            selected_app_id: None,
            app_detail_state: crate::ui::app_detail::AppDetailState::default(),
            current_view_key: 1,
            view_transition_start: 0.0,
            settings_state,
            last_applied_theme: String::new(), // empty triggers apply on first frame
            tokio_rt: rt,
            progress_tx,
            progress_rx,
            catalog_rx,
            auto_update_rx,
            auto_updater,
            self_update_rx,
            self_updater,
        }
    }

    pub fn show_toast(&mut self, title: impl Into<String>, message: impl Into<String>, kind: ToastKind) {
        self.toast = Some(ToastNotification {
            title: title.into(),
            message: message.into(),
            kind,
            created_at: std::time::Instant::now(),
            duration_secs: 4.5,
        });
    }

    pub fn check_for_updates(&mut self) {
        self.is_checking_updates = true;
        self.manual_update_requested = true;
        self.show_toast(
            "Checking for Updates",
            "Checking GitHub releases for the latest versions...",
            ToastKind::Info,
        );
        self.auto_updater.trigger_manual_check(self.apps.clone());
    }

    fn check_background_messages(&mut self) {
        // Check catalog updates
        while let Ok(updated_apps) = self.catalog_rx.try_recv() {
            let updates_count = updated_apps
                .iter()
                .filter(|a| matches!(a.install_state, InstallState::UpdateAvailable))
                .count();

            self.apps = updated_apps;
            self.is_refreshing = false;

            if self.manual_update_requested {
                self.manual_update_requested = false;
                self.is_checking_updates = false;

                if updates_count > 0 {
                    self.show_toast(
                        "Updates Available",
                        format!(
                            "{} application update{} ready to install.",
                            updates_count,
                            if updates_count == 1 { " is" } else { "s are" }
                        ),
                        ToastKind::Info,
                    );
                } else {
                    self.show_toast(
                        "All Apps Up to Date",
                        "All installed applications are running the latest releases.",
                        ToastKind::InstallSuccess,
                    );
                }
            }
        }

        // Check auto-update events - filter out intrusive "up to date" and "checking" messages
        while let Ok(evt) = self.auto_update_rx.try_recv() {
            let msg_lower = evt.message.to_lowercase();
            if !msg_lower.contains("up to date") && !msg_lower.contains("checking") && !msg_lower.contains("scan complete") {
                if evt.is_error {
                    self.show_toast("Update Notice", evt.message, ToastKind::Error);
                }
            }
        }

        // Check OpenCloud self-update events
        while let Ok(self_status) = self.self_update_rx.try_recv() {
            if self_status.update_available {
                self.show_toast(
                    "OpenCloud Update Available",
                    format!(
                        "OpenCloud Desktop v{} is available. Open Preferences to update.",
                        self_status.latest_version.as_deref().unwrap_or("")
                    ),
                    ToastKind::Info,
                );
            }
            self.settings_state.self_update_status = self_status;
        }

        // Check download progress events
        while let Ok(prog) = self.progress_rx.try_recv() {
            let mut toast_info = None;
            if let Some(app) = self.apps.iter_mut().find(|a| a.id == prog.app_id) {
                if prog.is_done {
                    if let Some(err) = prog.error {
                        toast_info = Some(("Installation Failed".to_string(), format!("Failed to install {}: {}", app.name, err), ToastKind::Error));
                        let local_info = crate::scanner::detect_installed_app(&app.id);
                        app.install_state = if local_info.is_installed {
                            InstallState::Installed
                        } else {
                            InstallState::NotInstalled
                        };
                    } else {
                        toast_info = Some(("Installation Complete".to_string(), format!("{} was installed successfully and is ready to use!", app.name), ToastKind::InstallSuccess));
                        let local_info = crate::scanner::detect_installed_app(&app.id);
                        app.install_state = InstallState::Installed;
                        app.installed_version = local_info.version;
                        app.executable_path = local_info.executable_path;
                    }
                } else {
                    app.install_state = InstallState::Downloading {
                        percentage: prog.percentage,
                        phase: prog.phase,
                    };
                }
            }
            if let Some((title, msg, kind)) = toast_info {
                self.show_toast(title, msg, kind);
            }
        }
    }

    fn start_install(&mut self, app_id: String) {
        if let Some(app) = self.apps.iter_mut().find(|a| a.id == app_id) {
            let app_name = app.name.clone();
            if let Some(ref asset) = app.download_asset {
                let download_url = asset.download_url.clone();
                let p_tx = self.progress_tx.clone();
                let app_id_clone = app_id.clone();

                app.install_state = InstallState::Downloading {
                    percentage: 0.0,
                    phase: "connecting".into(),
                };

                self.tokio_rt.spawn(async move {
                    start_download_and_install(app_id_clone, download_url, p_tx).await;
                });
            } else {
                self.show_toast(
                    "Package Pending",
                    format!(
                        "Release package for {} is pending in upstream repo. You can build it from source with `cargo build --release`.",
                        app_name
                    ),
                    ToastKind::Info,
                );
            }
        }
    }

    fn update_all(&mut self) {
        let outdated_ids: Vec<String> = self
            .apps
            .iter()
            .filter(|a| matches!(a.install_state, InstallState::UpdateAvailable))
            .map(|a| a.id.clone())
            .collect();

        for id in outdated_ids {
            self.start_install(id);
        }
    }

    fn start_refresh(&mut self) {
        self.is_refreshing = true;
        self.auto_updater.trigger_manual_check(self.apps.clone());
        self.self_updater.check_for_updates();
    }

    // ══════════════════════════════════════════════════════════════════════
    // TOP GLOBAL HEADER
    // ══════════════════════════════════════════════════════════════════════
    fn render_header(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        ui.horizontal(|ui| {
            // If viewing an app detail screen, show a quick Back to All Apps button on the far left
            if self.selected_app_id.is_some() {
                let (btn_rect, btn_resp) = ui.allocate_exact_size(vec2(96.0, 28.0), egui::Sense::click());
                let is_hovered = btn_resp.hovered();
                let bg = if is_hovered { palette.border_active } else { palette.button_secondary_bg };
                ui.painter().rect(btn_rect, CornerRadius::same(14), bg, Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);

                // Painted vector back chevron (<)
                let chev_x = btn_rect.left() + 14.0;
                let chev_y = btn_rect.center().y;
                let chev_stroke = Stroke::new(1.8, palette.accent_blue);
                ui.painter().line_segment([pos2(chev_x + 3.0, chev_y - 4.5), pos2(chev_x - 2.0, chev_y)], chev_stroke);
                ui.painter().line_segment([pos2(chev_x - 2.0, chev_y), pos2(chev_x + 3.0, chev_y + 4.5)], chev_stroke);

                ui.painter().text(
                    pos2(chev_x + 8.0, chev_y),
                    egui::Align2::LEFT_CENTER,
                    "All Apps",
                    FontId::proportional(12.0),
                    palette.accent_blue,
                );

                if btn_resp.clicked() {
                    self.selected_app_id = None;
                }
                ui.add_space(8.0);
            }

            // Single Unified Gray Search Pill (docked at the far left)
            let search_width = 280.0f32.min(ui.available_width() * 0.4).max(180.0);
            let frame = egui::Frame::new()
                .fill(palette.search_bg)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(14))
                .inner_margin(Margin::symmetric(10, 5));

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    // Crisp painted vector magnifying glass
                    let (icon_rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
                    let center = pos2(icon_rect.left() + 5.0, icon_rect.top() + 5.0);
                    ui.painter().circle_stroke(
                        center,
                        4.0,
                        Stroke::new(1.4, palette.text_muted),
                    );
                    ui.painter().line_segment(
                        [
                            pos2(center.x + 2.8, center.y + 2.8),
                            pos2(icon_rect.right() - 1.0, icon_rect.bottom() - 1.0),
                        ],
                        Stroke::new(1.6, palette.text_muted),
                    );

                    let edit_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(RichText::new("Search").color(palette.text_muted))
                            .text_color(palette.text_primary)
                            .frame(egui::Frame::NONE)
                            .desired_width(search_width - 36.0),
                    );
                    if edit_resp.changed() && !self.search_query.is_empty() {
                        self.selected_app_id = None;
                    }
                });
            });

            // Right-aligned Utility Cluster: GitHub logo + Cogwheel
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(16.0);

                // 1. GitHub Logo Button (Replaces user initial circle, opens dropdown)
                let github_resp = render_github_icon(ui, 24.0, palette.text_primary);
                github_resp.clone().on_hover_text("OpenCloud & StoryTold on GitHub");

                // Dropdown menu below GitHub logo - seamless single-gray surface
                egui::Popup::menu(&github_resp).show(|ui| {
                    ui.set_min_width(290.0);
                    let frame = egui::Frame::new()
                        .fill(palette.popup_bg)
                        .inner_margin(Margin::same(8))
                        .stroke(Stroke::new(1.0, palette.border))
                        .corner_radius(CornerRadius::same(10));

                    frame.show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 4.0);

                            ui.horizontal(|ui| {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("GitHub Repositories")
                                        .font(FontId::proportional(11.0))
                                        .color(palette.text_muted)
                                        .strong(),
                                );
                            });
                            ui.add_space(2.0);

                            // Option 1: OpenCloud Repository (transparent idle, single unified gray)
                            let w = ui.available_width();
                            let (item1_rect, item1_resp) = ui.allocate_exact_size(vec2(w, 42.0), egui::Sense::click());
                            if item1_resp.hovered() {
                                ui.painter().rect_filled(item1_rect, CornerRadius::same(6), palette.button_secondary_bg);
                            }
                            ui.painter().text(
                                pos2(item1_rect.left() + 10.0, item1_rect.top() + 13.0),
                                egui::Align2::LEFT_CENTER,
                                "OpenCloud Repository",
                                FontId::proportional(12.5),
                                palette.text_primary,
                            );
                            ui.painter().text(
                                pos2(item1_rect.left() + 10.0, item1_rect.top() + 29.0),
                                egui::Align2::LEFT_CENTER,
                                "https://github.com/bshea-1/OpenCloud",
                                FontId::proportional(11.0),
                                palette.text_muted,
                            );
                            if item1_resp.clicked() {
                                let url = "https://github.com/bshea-1/OpenCloud";
                                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                #[cfg(target_os = "macos")]
                                let _ = std::process::Command::new("open").arg(url).spawn();
                                egui::Popup::close_all(ui.ctx());
                            }

                            // Option 2: StoryTold Organization (transparent idle, single unified gray)
                            let (item2_rect, item2_resp) = ui.allocate_exact_size(vec2(w, 42.0), egui::Sense::click());
                            if item2_resp.hovered() {
                                ui.painter().rect_filled(item2_rect, CornerRadius::same(6), palette.button_secondary_bg);
                            }
                            ui.painter().text(
                                pos2(item2_rect.left() + 10.0, item2_rect.top() + 13.0),
                                egui::Align2::LEFT_CENTER,
                                "StoryTold Organization",
                                FontId::proportional(12.5),
                                palette.text_primary,
                            );
                            ui.painter().text(
                                pos2(item2_rect.left() + 10.0, item2_rect.top() + 29.0),
                                egui::Align2::LEFT_CENTER,
                                "https://github.com/storytold",
                                FontId::proportional(11.0),
                                palette.text_muted,
                            );
                            if item2_resp.clicked() {
                                let url = "https://github.com/storytold";
                                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                #[cfg(target_os = "macos")]
                                let _ = std::process::Command::new("open").arg(url).spawn();
                                egui::Popup::close_all(ui.ctx());
                            }
                        });
                    });
                });

                ui.add_space(10.0);

                // 2. Settings Gear Icon (smooth 5% hover scale easing, no blue)
                let cog_id = ui.make_persistent_id("opencloud_cogwheel_header_btn");
                let (cog_rect, cog_resp) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::click());
                let cog_hover_t = ui.ctx().animate_bool_with_time(cog_id, cog_resp.hovered(), 0.15);
                let cog_scale = 1.0 + 0.05 * cog_hover_t;

                if cog_hover_t > 0.01 {
                    let alpha = (cog_hover_t * 28.0) as u8;
                    ui.painter().circle_filled(
                        cog_rect.center(),
                        13.0 * cog_scale,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                    );
                }

                let cog_color = if self.current_tab == NavTab::Settings || cog_hover_t > 0.01 {
                    palette.text_primary
                } else {
                    palette.text_muted
                };

                let cog_center = cog_rect.center();
                let base_d = 18.0 * cog_scale;
                let r_outer = base_d * 0.50;
                let r_rim = base_d * 0.35;
                let r_hole = base_d * 0.16;

                // 8 mechanical trapezoidal gear teeth
                for i in 0..8 {
                    let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                    let half_root = 0.22;
                    let half_tip = 0.13;
                    let p1 = cog_center + vec2((angle - half_root).cos(), (angle - half_root).sin()) * (r_rim * 0.90);
                    let p2 = cog_center + vec2((angle - half_tip).cos(), (angle - half_tip).sin()) * r_outer;
                    let p3 = cog_center + vec2((angle + half_tip).cos(), (angle + half_tip).sin()) * r_outer;
                    let p4 = cog_center + vec2((angle + half_root).cos(), (angle + half_root).sin()) * (r_rim * 0.90);
                    ui.painter().add(egui::Shape::convex_polygon(vec![p1, p2, p3, p4], cog_color, Stroke::NONE));
                }

                // Solid gear rim body
                ui.painter().circle_filled(cog_center, r_rim, cog_color);

                // Axle hole in center
                ui.painter().circle_filled(cog_center, r_hole, palette.panel_bg);

                if cog_resp.on_hover_text("Preferences").clicked() {
                    self.current_tab = if self.current_tab == NavTab::Settings {
                        NavTab::Apps
                    } else {
                        NavTab::Settings
                    };
                }
            });
        });
    }

    // ══════════════════════════════════════════════════════════════════════
    // LEFT NAVIGATION SIDEBAR
    // ══════════════════════════════════════════════════════════════════════
    fn render_sidebar(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);

            // SECTION 1: APPS
            ui.add_space(6.0);
            ui.label(
                RichText::new("APPS")
                    .font(FontId::proportional(11.0))
                    .color(palette.text_muted)
                    .strong(),
            );
            ui.add_space(4.0);

            let all_apps_active = self.current_tab == NavTab::Apps
                && self.selected_app_id.is_none()
                && !self.filter_only_installed
                && !self.filter_only_updates
                && self.selected_category.is_none();

            self.render_sidebar_item(ui, "All Apps", all_apps_active, palette, |s| {
                s.current_tab = NavTab::Apps;
                s.filter_only_installed = false;
                s.filter_only_updates = false;
                s.selected_category = None;
                s.selected_app_id = None;
                s.search_query.clear();
            });

            let update_count = self
                .apps
                .iter()
                .filter(|a| matches!(a.install_state, InstallState::UpdateAvailable))
                .count();
            let updates_label = if update_count > 0 {
                format!("Updates ({})", update_count)
            } else {
                "Updates".into()
            };
            let updates_active = self.current_tab == NavTab::Apps && self.selected_app_id.is_none() && self.filter_only_updates;

            self.render_sidebar_item(ui, &updates_label, updates_active, palette, |s| {
                s.current_tab = NavTab::Apps;
                s.filter_only_installed = false;
                s.filter_only_updates = true;
                s.selected_category = None;
                s.selected_app_id = None;
                s.start_refresh();
            });

            // SECTION 2: CATEGORIES (No emojis)
            ui.add_space(18.0);
            ui.label(
                RichText::new("CATEGORIES")
                    .font(FontId::proportional(11.0))
                    .color(palette.text_muted)
                    .strong(),
            );
            ui.add_space(4.0);

            let categories = [
                (AppCategory::Photography, "Photography"),
                (AppCategory::PublishingDocs, "Design and layout"),
                (AppCategory::VideoMotion, "Video and motion"),
                (AppCategory::VectorGraphics, "Illustration"),
                (AppCategory::CadEngineering, "UI and UX"),
                (AppCategory::AudioEngineering, "Audio engineering"),
                (AppCategory::OfficeProductivity, "Office & documents"),
            ];

            for (cat, label) in categories {
                let is_active = self.current_tab == NavTab::Apps
                    && self.selected_app_id.is_none()
                    && self.selected_category == Some(cat)
                    && !self.filter_only_installed
                    && !self.filter_only_updates;

                self.render_sidebar_item(ui, label, is_active, palette, move |s| {
                    s.current_tab = NavTab::Apps;
                    s.selected_category = Some(cat);
                    s.filter_only_installed = false;
                    s.filter_only_updates = false;
                    s.selected_app_id = None;
                    s.search_query.clear();
                });
            }

            // SECTION 3: SETTINGS
            ui.add_space(18.0);
            ui.label(
                RichText::new("SETTINGS")
                    .font(FontId::proportional(11.0))
                    .color(palette.text_muted)
                    .strong(),
            );
            ui.add_space(4.0);

            self.render_sidebar_item(ui, "Preferences", self.current_tab == NavTab::Settings, palette, |s| {
                s.current_tab = NavTab::Settings;
                s.selected_app_id = None;
            });
        });
    }

    fn render_sidebar_item<F: FnOnce(&mut Self)>(
        &mut self,
        ui: &mut Ui,
        label: &str,
        is_active: bool,
        palette: &ThemePalette,
        on_click: F,
    ) {
        let width = ui.available_width();
        let height = 32.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(width, height), egui::Sense::click());

        // Smooth easing animation for tab selection & hover
        let item_id = ui.make_persistent_id(format!("sidebar_nav_item_{}", label));
        let active_t = ui.ctx().animate_bool_with_time(item_id.with("active"), is_active, 0.18);
        let hover_t = ui.ctx().animate_bool_with_time(item_id.with("hover"), resp.hovered() && !is_active, 0.14);

        if (active_t > 0.0 && active_t < 1.0) || (hover_t > 0.0 && hover_t < 1.0) {
            ui.ctx().request_repaint();
        }

        if active_t > 0.01 {
            let active_alpha = (active_t * (palette.sidebar_active.a() as f32)) as u8;
            let bg_color = egui::Color32::from_rgba_unmultiplied(
                palette.sidebar_active.r(),
                palette.sidebar_active.g(),
                palette.sidebar_active.b(),
                active_alpha,
            );
            ui.painter().rect_filled(rect, CornerRadius::same(6), bg_color);

            // Left animated accent indicator pill
            let bar_h = 16.0 * active_t;
            let bar_rect = Rect::from_center_size(
                pos2(rect.left() + 2.0, rect.center().y),
                vec2(3.0, bar_h),
            );
            let bar_alpha = (active_t * 255.0) as u8;
            let bar_color = egui::Color32::from_rgba_unmultiplied(
                palette.accent_blue.r(),
                palette.accent_blue.g(),
                palette.accent_blue.b(),
                bar_alpha,
            );
            ui.painter().rect_filled(bar_rect, CornerRadius::same(1), bar_color);
        } else if hover_t > 0.01 {
            let hover_alpha = (hover_t * (palette.sidebar_hover.a() as f32)) as u8;
            let bg_color = egui::Color32::from_rgba_unmultiplied(
                palette.sidebar_hover.r(),
                palette.sidebar_hover.g(),
                palette.sidebar_hover.b(),
                hover_alpha,
            );
            ui.painter().rect_filled(rect, CornerRadius::same(6), bg_color);
        }

        let font = if is_active {
            FontId::proportional(13.0)
        } else {
            FontId::proportional(13.0)
        };

        let text_color = if is_active {
            palette.text_primary
        } else {
            let base = palette.text_muted;
            let target = palette.text_primary;
            let t = hover_t;
            egui::Color32::from_rgb(
                (base.r() as f32 + (target.r() as f32 - base.r() as f32) * t) as u8,
                (base.g() as f32 + (target.g() as f32 - base.g() as f32) * t) as u8,
                (base.b() as f32 + (target.b() as f32 - base.b() as f32) * t) as u8,
            )
        };

        if label == "All Apps" {
            // Crisp vector 2x2 grid icon (4 rounded squares)
            let icon_x = rect.left() + 10.0;
            let icon_y = rect.center().y - 5.5;
            let sq_size = 4.0;
            let gap = 2.5;

            // Top-left
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(icon_x, icon_y), vec2(sq_size, sq_size)),
                CornerRadius::same(1),
                text_color,
            );
            // Top-right
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(icon_x + sq_size + gap, icon_y), vec2(sq_size, sq_size)),
                CornerRadius::same(1),
                text_color,
            );
            // Bottom-left
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(icon_x, icon_y + sq_size + gap), vec2(sq_size, sq_size)),
                CornerRadius::same(1),
                text_color,
            );
            // Bottom-right
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(icon_x + sq_size + gap, icon_y + sq_size + gap), vec2(sq_size, sq_size)),
                CornerRadius::same(1),
                text_color,
            );

            ui.painter().text(
                pos2(rect.left() + 26.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                "All Apps",
                font,
                text_color,
            );
        } else {
            ui.painter().text(
                pos2(rect.left() + 12.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                font,
                text_color,
            );
        }

        if resp.clicked() {
            on_click(self);
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // CENTRAL APPS WORKSPACE (Hero banner + 3-column card grid OR Detail View)
    // ══════════════════════════════════════════════════════════════════════
    fn render_apps_view(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        // If an app detail view is active, render the dedicated Creative Cloud style detail screen
        if let Some(ref sel_id) = self.selected_app_id.clone() {
            if let Some(app) = self.apps.iter().find(|a| a.id == *sel_id).cloned() {
                let mut install_action: Option<String> = None;
                let mut launch_action: Option<String> = None;
                let mut uninstall_action: Option<String> = None;
                let mut back_action = false;

                crate::ui::app_detail::render_app_detail_view(
                    ui,
                    &app,
                    &mut self.app_detail_state,
                    &mut install_action,
                    &mut launch_action,
                    &mut uninstall_action,
                    &mut back_action,
                    palette,
                    self.tokio_rt.clone(),
                );

                if back_action {
                    self.selected_app_id = None;
                }
                if let Some(id) = install_action {
                    self.start_install(id);
                }
                if let Some(id) = launch_action {
                    let app_name = self.apps.iter().find(|a| a.id == id).map(|a| a.name.clone()).unwrap_or_else(|| "App".into());
                    if let Err(e) = launch_app(&id, None) {
                        self.show_toast("Launch Failed", format!("Failed to launch {}: {}", app_name, e), ToastKind::Error);
                    }
                }
                if let Some(id) = uninstall_action {
                    let app_name = self.apps.iter().find(|a| a.id == id).map(|a| a.name.clone()).unwrap_or_else(|| "App".into());
                    if let Err(e) = uninstall_app(&id) {
                        self.show_toast("Uninstall Failed", format!("Failed to uninstall {}: {}", app_name, e), ToastKind::Error);
                    } else {
                        if let Some(app) = self.apps.iter_mut().find(|a| a.id == id) {
                            app.install_state = InstallState::NotInstalled;
                            app.installed_version = None;
                            app.executable_path = None;
                        }
                        self.show_toast("App Uninstalled", format!("{} was uninstalled successfully.", app_name), ToastKind::UninstallSuccess);
                    }
                }
                return;
            } else {
                self.selected_app_id = None;
            }
        }

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut install_action: Option<String> = None;
                let mut launch_action: Option<String> = None;
                let mut uninstall_action: Option<String> = None;
                let mut reveal_action: Option<String> = None;
                let mut select_action: Option<String> = None;

                let query = self.search_query.to_lowercase();
                let sel_cat = self.selected_category;
                let only_inst = self.filter_only_installed;
                let only_upd = self.filter_only_updates;

                // Title row: dynamic title for subcategories, search, or Updates (with Check for Updates button).
                // On default All Apps view, "Welcome to OpenCloud" hero banner already serves as the header.
                let show_title_header = sel_cat.is_some() || only_upd || !query.is_empty();

                if show_title_header {
                    let page_title = if let Some(cat) = sel_cat {
                        cat.display_name()
                    } else if only_upd {
                        "Updates"
                    } else {
                        "Search Results"
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(page_title)
                                .font(FontId::proportional(22.0))
                                .color(palette.text_primary)
                                .strong(),
                        );

                        if only_upd {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (btn_text, btn_color) = if self.is_checking_updates {
                                    ("Checking...", palette.accent_blue)
                                } else {
                                    ("Check for Updates", palette.text_primary)
                                };

                                let check_btn = egui::Button::new(
                                    RichText::new(btn_text)
                                        .font(FontId::proportional(12.0))
                                        .color(btn_color),
                                )
                                .fill(palette.button_secondary_bg)
                                .stroke(Stroke::new(1.0, palette.border_active))
                                .corner_radius(CornerRadius::same(14))
                                .min_size(vec2(130.0, 30.0));

                                if ui.add_enabled(!self.is_checking_updates, check_btn).clicked() {
                                    self.check_for_updates();
                                }
                            });
                        }
                    });

                    ui.add_space(16.0);
                }

                // ──────────────────────────────────────────────────────────
                // HERO BANNER: "Welcome to OpenCloud"
                // ──────────────────────────────────────────────────────────
                if query.is_empty() && sel_cat.is_none() && !only_upd {
                    self.render_hero_banner(ui, palette);
                    ui.add_space(24.0);
                }

                // Filter apps
                let filtered_indices: Vec<usize> = self
                    .apps
                    .iter()
                    .enumerate()
                    .filter(|(_, app)| {
                        if !query.is_empty()
                            && !app.name.to_lowercase().contains(&query)
                            && !app.description.to_lowercase().contains(&query)
                            && !app.file_formats.iter().any(|fmt| fmt.to_lowercase().contains(&query))
                        {
                            return false;
                        }
                        if let Some(cat) = sel_cat {
                            if app.category != cat {
                                return false;
                            }
                        }
                        if only_inst
                            && !matches!(
                                app.install_state,
                                InstallState::Installed | InstallState::UpdateAvailable
                            )
                        {
                            return false;
                        }
                        if only_upd
                            && !matches!(app.install_state, InstallState::UpdateAvailable)
                        {
                            return false;
                        }
                        true
                    })
                    .map(|(idx, _)| idx)
                    .collect();

                // Group 1: Installed Apps
                let installed_indices: Vec<usize> = filtered_indices
                    .iter()
                    .copied()
                    .filter(|&idx| matches!(self.apps[idx].install_state, InstallState::Installed))
                    .collect();

                // Group 2: Updates
                let update_indices: Vec<usize> = filtered_indices
                    .iter()
                    .copied()
                    .filter(|&idx| matches!(self.apps[idx].install_state, InstallState::UpdateAvailable))
                    .collect();

                // Group 3: Available apps to install
                let available_indices: Vec<usize> = filtered_indices
                    .iter()
                    .copied()
                    .filter(|&idx| matches!(self.apps[idx].install_state, InstallState::NotInstalled | InstallState::Downloading { .. } | InstallState::Installing))
                    .collect();

                // Render Updates Section
                if !update_indices.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Updates Available")
                                .font(FontId::proportional(16.0))
                                .color(palette.update_amber)
                                .strong(),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .button(RichText::new("Update All").color(egui::Color32::BLACK).strong())
                                .clicked()
                            {
                                self.update_all();
                            }
                        });
                    });
                    ui.add_space(10.0);

                    self.render_card_grid(
                        ui,
                        &update_indices,
                        &mut install_action,
                        &mut launch_action,
                        &mut uninstall_action,
                        &mut reveal_action,
                        &mut select_action,
                        palette,
                    );
                    ui.add_space(24.0);
                } else if only_upd {
                    let total_installed = self
                        .apps
                        .iter()
                        .filter(|a| matches!(a.install_state, InstallState::Installed | InstallState::UpdateAvailable))
                        .count();

                    if total_installed == 0 {
                        // User has no apps installed yet
                        ui.add_space(20.0);
                        let card_frame = egui::Frame::new()
                            .fill(palette.card_bg)
                            .stroke(Stroke::new(1.0, palette.border))
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::symmetric(40, 48));

                        card_frame.show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                // Draw a sleek vector download/package box
                                let (icon_rect, _) = ui.allocate_exact_size(vec2(52.0, 52.0), egui::Sense::hover());
                                ui.painter().rect_stroke(
                                    icon_rect,
                                    CornerRadius::same(10),
                                    Stroke::new(1.5, palette.border_active),
                                    egui::StrokeKind::Outside,
                                );
                                let c = icon_rect.center();
                                ui.painter().line_segment(
                                    [pos2(c.x, c.y - 10.0), pos2(c.x, c.y + 8.0)],
                                    Stroke::new(2.2, palette.accent_blue),
                                );
                                ui.painter().line_segment(
                                    [pos2(c.x - 6.0, c.y + 2.0), pos2(c.x, c.y + 8.0)],
                                    Stroke::new(2.2, palette.accent_blue),
                                );
                                ui.painter().line_segment(
                                    [pos2(c.x + 6.0, c.y + 2.0), pos2(c.x, c.y + 8.0)],
                                    Stroke::new(2.2, palette.accent_blue),
                                );

                                ui.add_space(18.0);
                                ui.label(
                                    RichText::new("No Applications Installed")
                                        .font(FontId::proportional(19.0))
                                        .color(palette.text_primary)
                                        .strong(),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("You don't have any OpenCloud applications installed yet.\nInstall creative apps from the suite to manage versions and receive automated updates.")
                                        .font(FontId::proportional(13.0))
                                        .color(palette.text_muted),
                                );
                                ui.add_space(20.0);

                                let browse_btn = egui::Button::new(
                                    RichText::new("Browse All Apps")
                                        .font(FontId::proportional(13.0))
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                )
                                .fill(palette.accent_blue)
                                .stroke(Stroke::new(1.0, palette.accent_blue))
                                .corner_radius(CornerRadius::same(14))
                                .min_size(vec2(150.0, 34.0));

                                if ui.add(browse_btn).clicked() {
                                    self.filter_only_updates = false;
                                    self.filter_only_installed = false;
                                    self.selected_category = None;
                                    self.selected_app_id = None;
                                }
                            });
                        });
                    } else {
                        // Installed apps exist and are all up to date
                        ui.add_space(20.0);
                        let card_frame = egui::Frame::new()
                            .fill(palette.card_bg)
                            .stroke(Stroke::new(1.0, palette.border))
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::symmetric(40, 48));

                        card_frame.show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                let (icon_rect, _) = ui.allocate_exact_size(vec2(44.0, 44.0), egui::Sense::hover());
                                ui.painter().circle_stroke(
                                    icon_rect.center(),
                                    20.0,
                                    Stroke::new(1.5, palette.success_green),
                                );
                                let c = icon_rect.center();
                                ui.painter().line_segment(
                                    [pos2(c.x - 7.0, c.y), pos2(c.x - 2.0, c.y + 5.0)],
                                    Stroke::new(2.2, palette.success_green),
                                );
                                ui.painter().line_segment(
                                    [pos2(c.x - 2.0, c.y + 5.0), pos2(c.x + 8.0, c.y - 5.0)],
                                    Stroke::new(2.2, palette.success_green),
                                );

                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new("All Installed Applications Up to Date")
                                        .font(FontId::proportional(18.0))
                                        .color(palette.text_primary)
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(format!(
                                        "All {} installed application{} running the latest available release.",
                                        total_installed,
                                        if total_installed == 1 { " is" } else { "s are" }
                                    ))
                                    .font(FontId::proportional(13.0))
                                    .color(palette.text_muted),
                                );
                                ui.add_space(16.0);
                                let check_btn = egui::Button::new(
                                    RichText::new("Check Again")
                                        .font(FontId::proportional(12.0))
                                        .color(palette.text_primary),
                                )
                                .fill(palette.button_secondary_bg)
                                .stroke(Stroke::new(1.0, palette.border))
                                .corner_radius(CornerRadius::same(12))
                                .min_size(vec2(120.0, 30.0));

                                if ui.add(check_btn).clicked() {
                                    self.check_for_updates();
                                }
                            });
                        });
                    }
                }

                // Render Installed Section (if any installed)
                if !installed_indices.is_empty() {
                    let installed_label = if let Some(cat) = sel_cat {
                        format!("Installed {} ({})", cat.display_name(), installed_indices.len())
                    } else if !query.is_empty() {
                        format!("Installed Matching Apps ({})", installed_indices.len())
                    } else {
                        format!("Installed ({})", installed_indices.len())
                    };

                    ui.label(
                        RichText::new(installed_label)
                            .font(FontId::proportional(16.0))
                            .color(palette.text_primary)
                            .strong(),
                    );
                    ui.add_space(10.0);

                    self.render_card_grid(
                        ui,
                        &installed_indices,
                        &mut install_action,
                        &mut launch_action,
                        &mut uninstall_action,
                        &mut reveal_action,
                        &mut select_action,
                        palette,
                    );
                    ui.add_space(24.0);
                }

                // Render Available Apps Section (all remaining suite applications)
                if !available_indices.is_empty() {
                    let section_label = if let Some(cat) = sel_cat {
                        if installed_indices.is_empty() && update_indices.is_empty() {
                            format!("{} ({})", cat.display_name(), available_indices.len())
                        } else {
                            format!("Available {} ({})", cat.display_name(), available_indices.len())
                        }
                    } else if !query.is_empty() {
                        format!("Available Search Results ({})", available_indices.len())
                    } else if installed_indices.is_empty() && update_indices.is_empty() {
                        format!("All Suite Applications ({})", available_indices.len())
                    } else {
                        format!("Available in OpenCloud ({})", available_indices.len())
                    };

                    ui.label(
                        RichText::new(section_label)
                            .font(FontId::proportional(16.0))
                            .color(palette.text_primary)
                            .strong(),
                    );
                    ui.add_space(10.0);

                    self.render_card_grid(
                        ui,
                        &available_indices,
                        &mut install_action,
                        &mut launch_action,
                        &mut uninstall_action,
                        &mut reveal_action,
                        &mut select_action,
                        palette,
                    );
                    ui.add_space(24.0);
                }

                // Handle app detail selection
                if let Some(id) = select_action {
                    self.selected_app_id = Some(id);
                }

                // Execute actions
                if let Some(id) = install_action {
                    self.start_install(id);
                }
                if let Some(id) = launch_action {
                    let app_name = self.apps.iter().find(|a| a.id == id).map(|a| a.name.clone()).unwrap_or_else(|| "App".into());
                    if let Err(e) = launch_app(&id, None) {
                        self.show_toast("Launch Failed", format!("Failed to launch {}: {}", app_name, e), ToastKind::Error);
                    }
                }
                if let Some(id) = uninstall_action {
                    let app_name = self.apps.iter().find(|a| a.id == id).map(|a| a.name.clone()).unwrap_or_else(|| "App".into());
                    if let Err(e) = uninstall_app(&id) {
                        self.show_toast("Uninstall Failed", format!("Failed to uninstall {}: {}", app_name, e), ToastKind::Error);
                    } else {
                        if let Some(app) = self.apps.iter_mut().find(|a| a.id == id) {
                            app.install_state = InstallState::NotInstalled;
                            app.installed_version = None;
                            app.executable_path = None;
                        }
                        self.show_toast("App Uninstalled", format!("{} was uninstalled successfully.", app_name), ToastKind::UninstallSuccess);
                    }
                }
                if let Some(path) = reveal_action {
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open").arg("-R").arg(&path).spawn();
                    }
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path)).spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open").arg(std::path::Path::new(&path).parent().unwrap_or(std::path::Path::new("/"))).spawn();
                    }
                }
            });
    }

    // ══════════════════════════════════════════════════════════════════════
    // HERO BANNER: Clean, compact welcome card
    // ══════════════════════════════════════════════════════════════════════
    fn render_hero_banner(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let banner_w = (ui.available_width() - 20.0).max(200.0);
        let banner_frame = egui::Frame::new()
            .fill(palette.banner_bg)
            .stroke(Stroke::new(1.0, palette.banner_border))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(24, 18));

        ui.allocate_ui_with_layout(
            vec2(banner_w, 74.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(banner_w);
                banner_frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Welcome to OpenCloud")
                                .font(FontId::proportional(20.0))
                                .color(palette.banner_text)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Find and update your creative apps, libraries and more.")
                                .font(FontId::proportional(13.0))
                                .color(egui::Color32::from_rgb(180, 182, 195)),
                        );
                    });
                });
            },
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // MULTI-COLUMN CARD GRID (3 columns responsive)
    // ══════════════════════════════════════════════════════════════════════
    fn render_card_grid(
        &mut self,
        ui: &mut Ui,
        indices: &[usize],
        install_action: &mut Option<String>,
        launch_action: &mut Option<String>,
        uninstall_action: &mut Option<String>,
        reveal_action: &mut Option<String>,
        select_action: &mut Option<String>,
        palette: &ThemePalette,
    ) {
        let avail_w = (ui.available_width() - 20.0).max(100.0);
        let gap = 16.0;
        let cols = if avail_w >= 820.0 {
            3
        } else if avail_w >= 540.0 {
            2
        } else {
            1
        };
        let total_gap = (cols - 1) as f32 * gap;
        let card_w = ((avail_w - total_gap) / cols as f32).floor().max(200.0);

        for chunk in indices.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &idx in chunk {
                    self.render_grid_app_card(
                        ui,
                        idx,
                        card_w,
                        install_action,
                        launch_action,
                        uninstall_action,
                        reveal_action,
                        select_action,
                        palette,
                    );
                }
            });
            ui.add_space(gap);
        }
    }

    fn render_grid_app_card(
        &mut self,
        ui: &mut Ui,
        idx: usize,
        card_w: f32,
        install_action: &mut Option<String>,
        launch_action: &mut Option<String>,
        uninstall_action: &mut Option<String>,
        reveal_action: &mut Option<String>,
        select_action: &mut Option<String>,
        palette: &ThemePalette,
    ) {
        let app = &mut self.apps[idx];
        let app_id = app.id.clone();
        let exec_path = app.executable_path.clone();

        let card_h = 168.0;
        let inner_w = (card_w - 28.0).max(120.0);
        let inner_h = 144.0;

        let card_frame = egui::Frame::new()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(14, 12));

        ui.allocate_ui_with_layout(
            vec2(card_w, card_h),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(card_w);
                ui.set_height(card_h);
                card_frame.show(ui, |ui| {
                    ui.set_width(inner_w);
                    ui.set_height(inner_h);

                    ui.vertical(|ui| {
                        // TOP ROW: Logo (40x40) + App Name + Version (Desktop icon removed)
                        ui.allocate_ui_with_layout(
                            vec2(inner_w, 42.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                let logo_resp = render_logo(ui, &app.id, &app.token, app.accent_color, 40.0);
                                if logo_resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    *select_action = Some(app_id.clone());
                                }

                                ui.add_space(8.0);

                                ui.vertical(|ui| {
                                    let title_btn = egui::Button::new(
                                        RichText::new(&app.name)
                                            .font(FontId::proportional(15.0))
                                            .color(palette.text_primary)
                                            .strong(),
                                    )
                                    .frame(false);

                                    let title_resp = ui.add(title_btn);
                                    if title_resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        *select_action = Some(app_id.clone());
                                    }

                                    if let Some(ref ver) = app.installed_version {
                                        ui.label(
                                            RichText::new(format!("v{}", ver))
                                                .font(FontId::proportional(10.0))
                                                .color(palette.success_green),
                                        );
                                    }
                                });
                            },
                        );

                        ui.add_space(6.0);

                        // MIDDLE ROW: Fixed-height description container so cards never vary in height
                        ui.allocate_ui_with_layout(
                            vec2(inner_w, 42.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.set_width(inner_w);
                                ui.set_height(42.0);
                                let desc_label = egui::Label::new(
                                    RichText::new(&app.description)
                                        .font(FontId::proportional(11.5))
                                        .color(palette.text_muted),
                                )
                                .wrap();
                                ui.add(desc_label);
                            },
                        );

                        ui.add_space(8.0);

                        // BOTTOM ROW: GitHub Icon + Pill Action Button [ Open / Install ] + More Options Menu
                        ui.allocate_ui_with_layout(
                            vec2(inner_w, 32.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                // GitHub icon button (replaces graduation hat)
                                let gh_id = format!("app_card_gh_{}", app_id);
                                let gh_resp = render_github_icon_with_id(ui, &gh_id, 16.0, palette.text_muted);
                                if gh_resp
                                    .on_hover_text(format!("View {} on GitHub", app.name))
                                    .clicked()
                                {
                                    let url = format!("https://github.com/{}", app.repo);
                                    ui.ctx().open_url(egui::OpenUrl::new_tab(&url));
                                    #[cfg(target_os = "macos")]
                                    let _ = std::process::Command::new("open").arg(&url).spawn();
                                }

                                // Action buttons right-aligned
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    // Context Menu button (...)
                                    ui.menu_button("...", |ui| {
                                        if let Some(ref p) = exec_path {
                                            if ui.button("Reveal in Finder").clicked() {
                                                *reveal_action = Some(p.clone());
                                                ui.close();
                                            }
                                        }

                                        let mut is_auto = self.settings_state.settings.apps.is_app_auto_update_enabled(&app_id);
                                        if ui.checkbox(&mut is_auto, "Auto-update this app").changed() {
                                            self.settings_state.settings.apps.set_app_auto_update(&app_id, is_auto);
                                            self.settings_state.save();
                                        }

                                        if ui.button("GitHub Repository").clicked() {
                                            let _ = std::process::Command::new("open")
                                                .arg(format!("https://github.com/{}", app.repo))
                                                .spawn();
                                            ui.close();
                                        }

                                        if matches!(app.install_state, InstallState::Installed | InstallState::UpdateAvailable) {
                                            if ui.button("Uninstall").clicked() {
                                                *uninstall_action = Some(app_id.clone());
                                                ui.close();
                                            }
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // Pill Action Button
                                    match &app.install_state {
                                        InstallState::NotInstalled => {
                                            let install_btn = egui::Button::new(
                                                RichText::new("Install")
                                                    .font(FontId::proportional(12.0))
                                                    .color(egui::Color32::WHITE)
                                                    .strong(),
                                            )
                                            .fill(palette.accent_blue)
                                            .stroke(Stroke::new(1.0, palette.accent_blue))
                                            .corner_radius(CornerRadius::same(14))
                                            .min_size(vec2(64.0, 26.0));

                                            if ui.add(install_btn).clicked() {
                                                *install_action = Some(app_id.clone());
                                            }
                                        }
                                        InstallState::Installed => {
                                            let open_btn = egui::Button::new(
                                                RichText::new("Open")
                                                    .font(FontId::proportional(12.0))
                                                    .color(palette.text_primary)
                                                    .strong(),
                                            )
                                            .fill(palette.button_secondary_bg)
                                            .stroke(Stroke::new(1.0, palette.border_active))
                                            .corner_radius(CornerRadius::same(14))
                                            .min_size(vec2(64.0, 26.0));

                                            if ui.add(open_btn).clicked() {
                                                *launch_action = Some(app_id.clone());
                                            }
                                        }
                                        InstallState::UpdateAvailable => {
                                            let update_btn = egui::Button::new(
                                                RichText::new("Update")
                                                    .font(FontId::proportional(12.0))
                                                    .color(egui::Color32::WHITE)
                                                    .strong(),
                                            )
                                            .fill(palette.update_amber)
                                            .stroke(Stroke::new(1.0, palette.update_amber))
                                            .corner_radius(CornerRadius::same(14))
                                            .min_size(vec2(64.0, 26.0));

                                            if ui.add(update_btn).clicked() {
                                                *install_action = Some(app_id.clone());
                                            }
                                        }
                                        InstallState::Downloading { percentage, phase } => {
                                            let time = ui.input(|i| i.time);
                                            ui.ctx().request_repaint();

                                            ui.horizontal(|ui| {
                                                // Animated rotating spinner dots
                                                let (spin_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                                                let center = spin_rect.center();
                                                let base_angle = (time * 8.0) as f32;
                                                for i in 0..4 {
                                                    let a = base_angle + (i as f32) * std::f32::consts::FRAC_PI_2;
                                                    let alpha = ((i as f32 + 1.0) / 4.0).powf(1.5);
                                                    let pos = center + vec2(a.cos(), a.sin()) * 5.5;
                                                    ui.painter().circle_filled(
                                                        pos,
                                                        1.4,
                                                        egui::Color32::from_rgba_unmultiplied(
                                                            palette.accent_blue.r(), palette.accent_blue.g(), palette.accent_blue.b(), (255.0 * alpha) as u8
                                                        ),
                                                    );
                                                }

                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(format!("{}: {:.0}%", phase, percentage))
                                                            .font(FontId::proportional(10.0))
                                                            .color(palette.accent_blue)
                                                            .strong(),
                                                    );
                                                    let p_fraction = (*percentage / 100.0).clamp(0.0, 1.0);
                                                    let (bar_rect, _) = ui.allocate_exact_size(vec2(60.0, 4.0), egui::Sense::hover());
                                                    ui.painter().rect_filled(bar_rect, CornerRadius::same(2), palette.button_secondary_bg);
                                                    let fill_rect = Rect::from_min_size(bar_rect.min, vec2(bar_rect.width() * p_fraction, bar_rect.height()));
                                                    ui.painter().rect_filled(fill_rect, CornerRadius::same(2), palette.accent_blue);
                                                });
                                            });
                                        }
                                        InstallState::Installing => {
                                            let time = ui.input(|i| i.time);
                                            ui.ctx().request_repaint();
                                            ui.horizontal(|ui| {
                                                let (spin_rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
                                                let center = spin_rect.center();
                                                let base_angle = (time * 9.0) as f32;
                                                for i in 0..4 {
                                                    let a = base_angle + (i as f32) * std::f32::consts::FRAC_PI_2;
                                                    let alpha = ((i as f32 + 1.0) / 4.0).powf(1.5);
                                                    let pos = center + vec2(a.cos(), a.sin()) * 5.0;
                                                    ui.painter().circle_filled(
                                                        pos,
                                                        1.4,
                                                        egui::Color32::from_rgba_unmultiplied(
                                                            palette.update_amber.r(), palette.update_amber.g(), palette.update_amber.b(), (255.0 * alpha) as u8
                                                        ),
                                                    );
                                                }
                                                ui.label(
                                                    RichText::new("Installing...")
                                                        .font(FontId::proportional(11.0))
                                                        .color(palette.update_amber)
                                                        .strong(),
                                                );
                                            });
                                        }
                                    }
                                });
                            },
                        );
                    });
                });
            },
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // "YOUR WORK" TAB (Recent creative project files fallback)
    // ══════════════════════════════════════════════════════════════════════
    fn render_recents_view(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Recent Project Files")
                        .font(FontId::proportional(22.0))
                        .color(palette.text_primary)
                        .strong(),
                );
                ui.label(
                    RichText::new("Open any creative project directly into its corresponding application.")
                        .font(FontId::proportional(13.0))
                        .color(palette.text_muted),
                );
                ui.add_space(16.0);

                if self.recent_projects.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        let (icon_rect, _) = ui.allocate_exact_size(vec2(48.0, 40.0), egui::Sense::hover());
                        ui.painter().rect_stroke(
                            icon_rect,
                            CornerRadius::same(6),
                            Stroke::new(1.5, palette.border_active),
                            egui::StrokeKind::Outside,
                        );
                        let tab_rect = Rect::from_min_size(icon_rect.min, vec2(18.0, 8.0));
                        ui.painter().rect_filled(tab_rect, CornerRadius::same(2), palette.border_active);
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("No Recent Project Files Found")
                                .font(FontId::proportional(18.0))
                                .color(palette.text_primary)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Files created with PhotoCraft, VectorCraft, CadCraft, or other suite apps will appear here.")
                                .font(FontId::proportional(13.0))
                                .color(palette.text_muted),
                        );
                        ui.add_space(16.0);
                        if ui
                            .button(RichText::new("Explore Apps").color(egui::Color32::WHITE).strong())
                            .clicked()
                        {
                            self.current_tab = NavTab::Apps;
                        }
                    });
                    return;
                }

                let mut launch_file: Option<(String, String)> = None;

                for proj in &self.recent_projects {
                    let item_frame = egui::Frame::new()
                        .fill(palette.card_bg)
                        .stroke(Stroke::new(1.0, palette.border))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(14, 12));

                    item_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&proj.name)
                                    .font(FontId::proportional(14.0))
                                    .color(palette.text_primary)
                                    .strong(),
                            );

                            ui.label(
                                RichText::new(format!("- {}", proj.app_name))
                                    .font(FontId::proportional(12.0))
                                    .color(palette.accent_blue),
                            );

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let open_btn = egui::Button::new(
                                    RichText::new(format!("Open in {}", proj.app_name))
                                        .font(FontId::proportional(12.0))
                                        .color(palette.text_primary)
                                        .strong(),
                                )
                                .fill(palette.button_secondary_bg)
                                .stroke(Stroke::new(1.0, palette.border_active))
                                .corner_radius(CornerRadius::same(14));

                                if ui.add(open_btn).clicked() {
                                    launch_file = Some((proj.app_id.clone(), proj.path.clone()));
                                }

                                ui.label(
                                    RichText::new(&proj.path)
                                        .font(FontId::proportional(11.0))
                                        .color(palette.text_muted),
                                );
                            });
                        });
                    });
                    ui.add_space(8.0);
                }

                if let Some((app_id, path)) = launch_file {
                    if let Err(e) = launch_app(&app_id, Some(&path)) {
                        self.show_toast("Launch Failed", format!("Failed to open file: {}", e), ToastKind::Error);
                    }
                }
            });
    }

    fn get_view_key(&self) -> u32 {
        match self.current_tab {
            NavTab::Settings => 1000,
            NavTab::YourWork => 5000,
            NavTab::Fonts => 6000,
            NavTab::Apps => {
                if let Some(ref id) = self.selected_app_id {
                    2000 + id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32))
                } else if self.filter_only_updates {
                    3000
                } else if let Some(cat) = self.selected_category {
                    4000 + cat as u32
                } else {
                    1
                }
            }
        }
    }

    fn render_toast(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let toast = match &self.toast {
            Some(t) => t.clone(),
            None => return,
        };

        let elapsed = toast.created_at.elapsed().as_secs_f32();
        if elapsed >= toast.duration_secs {
            self.toast = None;
            return;
        }

        // Request continuous repaint for smooth 60fps toast animation
        ui.ctx().request_repaint();

        // Animation easing: pure alpha fade in over 0.22s, fade out over 0.35s
        let enter_t = (elapsed / 0.22).clamp(0.0, 1.0);
        let enter_ease = 1.0 - (1.0 - enter_t).powi(2);

        // Exit: last 0.35s fade out
        let remaining = toast.duration_secs - elapsed;
        let exit_alpha = (remaining / 0.35).clamp(0.0, 1.0);

        let total_alpha = (enter_ease * exit_alpha).clamp(0.0, 1.0);

        let toast_width = 380.0;
        let toast_height = 62.0;
        let win_rect = ui.max_rect();
        let toast_rect = Rect::from_min_size(
            pos2(win_rect.max.x - toast_width - 24.0, win_rect.max.y - toast_height - 24.0),
            vec2(toast_width, toast_height),
        );

        // Floating drop shadow (alpha strictly synchronized)
        ui.painter().rect_filled(
            toast_rect.translate(vec2(0.0, 6.0)),
            CornerRadius::same(12),
            egui::Color32::from_black_alpha((55.0 * total_alpha) as u8),
        );

        // Elevated card background & border
        let bg_color = if palette.is_dark {
            egui::Color32::from_rgba_unmultiplied(26, 29, 36, (252.0 * total_alpha) as u8)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (252.0 * total_alpha) as u8)
        };

        let (accent_color, border_color) = match toast.kind {
            ToastKind::InstallSuccess => (
                palette.success_green,
                egui::Color32::from_rgba_unmultiplied(
                    palette.success_green.r(),
                    palette.success_green.g(),
                    palette.success_green.b(),
                    (200.0 * total_alpha) as u8,
                ),
            ),
            ToastKind::UninstallSuccess | ToastKind::Error => (
                egui::Color32::from_rgb(239, 68, 68),
                egui::Color32::from_rgba_unmultiplied(239, 68, 68, (220.0 * total_alpha) as u8),
            ),
            ToastKind::Info => (
                palette.accent_blue,
                egui::Color32::from_rgba_unmultiplied(
                    palette.border_active.r(),
                    palette.border_active.g(),
                    palette.border_active.b(),
                    (210.0 * total_alpha) as u8,
                ),
            ),
        };

        ui.painter().rect(
            toast_rect,
            CornerRadius::same(12),
            bg_color,
            Stroke::new(1.0, border_color),
            egui::StrokeKind::Inside,
        );

        // Vector Status Badge & Icon (100% Geometry, ZERO unicode fonts)
        let icon_center = pos2(toast_rect.left() + 28.0, toast_rect.center().y);
        let badge_bg = egui::Color32::from_rgba_unmultiplied(
            accent_color.r(),
            accent_color.g(),
            accent_color.b(),
            (240.0 * total_alpha) as u8,
        );
        let white_icon = egui::Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * total_alpha) as u8);

        ui.painter().circle_filled(icon_center, 12.0, badge_bg);

        match toast.kind {
            ToastKind::InstallSuccess => {
                // Crisp green checkmark lines with synchronized alpha
                let p1 = icon_center + vec2(-4.0, 0.0);
                let p2 = icon_center + vec2(-1.0, 3.5);
                let p3 = icon_center + vec2(4.5, -3.0);
                ui.painter().line_segment([p1, p2], Stroke::new(2.0, white_icon));
                ui.painter().line_segment([p2, p3], Stroke::new(2.0, white_icon));
            }
            ToastKind::UninstallSuccess => {
                // Crisp red minus / uninstall line with synchronized alpha
                let p1 = icon_center + vec2(-4.5, 0.0);
                let p2 = icon_center + vec2(4.5, 0.0);
                ui.painter().line_segment([p1, p2], Stroke::new(2.2, white_icon));
            }
            ToastKind::Error => {
                // Vector exclamation point with synchronized alpha
                ui.painter().line_segment(
                    [icon_center + vec2(0.0, -5.5), icon_center + vec2(0.0, 1.0)],
                    Stroke::new(2.2, white_icon),
                );
                ui.painter().circle_filled(icon_center + vec2(0.0, 4.0), 1.2, white_icon);
            }
            ToastKind::Info => {
                // Vector 'i' info symbol with synchronized alpha
                ui.painter().circle_filled(icon_center + vec2(0.0, -4.0), 1.2, white_icon);
                ui.painter().line_segment(
                    [icon_center + vec2(0.0, -1.5), icon_center + vec2(0.0, 4.5)],
                    Stroke::new(2.0, white_icon),
                );
            }
        }

        // Text Content (Title + Message) - synchronized alpha
        let text_left = toast_rect.left() + 50.0;
        let text_top = toast_rect.top() + 14.0;
        let title_color = egui::Color32::from_rgba_unmultiplied(
            palette.text_primary.r(),
            palette.text_primary.g(),
            palette.text_primary.b(),
            (255.0 * total_alpha) as u8,
        );
        let msg_color = egui::Color32::from_rgba_unmultiplied(
            palette.text_muted.r(),
            palette.text_muted.g(),
            palette.text_muted.b(),
            (230.0 * total_alpha) as u8,
        );

        ui.painter().text(
            pos2(text_left, text_top),
            egui::Align2::LEFT_TOP,
            &toast.title,
            FontId::proportional(13.0),
            title_color,
        );
        ui.painter().text(
            pos2(text_left, text_top + 18.0),
            egui::Align2::LEFT_TOP,
            &toast.message,
            FontId::proportional(12.0),
            msg_color,
        );

        // Close Button - synchronized alpha
        let close_rect = Rect::from_center_size(
            pos2(toast_rect.right() - 20.0, toast_rect.center().y),
            vec2(20.0, 20.0),
        );
        let close_resp = ui.allocate_rect(close_rect, egui::Sense::click());
        let close_color = if close_resp.hovered() {
            palette.text_primary
        } else {
            palette.text_muted
        };
        let c_color = egui::Color32::from_rgba_unmultiplied(
            close_color.r(),
            close_color.g(),
            close_color.b(),
            (190.0 * total_alpha) as u8,
        );
        let c_center = close_rect.center();
        ui.painter().line_segment(
            [c_center + vec2(-3.5, -3.5), c_center + vec2(3.5, 3.5)],
            Stroke::new(1.4, c_color),
        );
        ui.painter().line_segment(
            [c_center + vec2(-3.5, 3.5), c_center + vec2(3.5, -3.5)],
            Stroke::new(1.4, c_color),
        );

        if close_resp.clicked() {
            self.toast = None;
        }

        // Animated countdown timer line along the bottom of the toast - synchronized alpha
        let pct = ((toast.duration_secs - elapsed) / toast.duration_secs).clamp(0.0, 1.0);
        let line_w = (toast_width - 8.0) * pct;
        let bar_color = egui::Color32::from_rgba_unmultiplied(
            accent_color.r(),
            accent_color.g(),
            accent_color.b(),
            (170.0 * total_alpha) as u8,
        );
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(toast_rect.left() + 4.0, toast_rect.bottom() - 3.0), vec2(line_w, 2.0)),
            CornerRadius::same(1),
            bar_color,
        );
    }
}

impl eframe::App for OpenCloudApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.check_background_messages();
        let ctx = ui.ctx().clone();
        let current_theme_name = self.settings_state.settings.general.studio_theme.clone();
        let palette = ThemePalette::from_theme(&current_theme_name);
        // Only rebuild and push egui Style when the theme actually changes.
        if self.last_applied_theme != current_theme_name {
            apply_studio_theme(&ctx, &palette);
            self.last_applied_theme = current_theme_name;
        }

        // Fill entire window background canvas
        ui.painter().rect_filled(ui.max_rect(), CornerRadius::ZERO, palette.canvas_bg);

        // TOP NAVIGATION HEADER
        let header_frame = egui::Frame::new()
            .fill(palette.panel_bg)
            .inner_margin(Margin::symmetric(18, 12))
            .stroke(Stroke::new(1.0, palette.border));

        header_frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.render_header(ui, &palette);
        });

        // MAIN WORKSPACE: Left Sidebar + Central Content Canvas
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            // Left Navigation Sidebar
            let sidebar_frame = egui::Frame::new()
                .fill(palette.panel_bg)
                .inner_margin(Margin::symmetric(14, 16))
                .stroke(Stroke::new(1.0, palette.border));

            sidebar_frame.show(ui, |ui| {
                ui.set_width(220.0);
                ui.set_height(ui.available_height());
                ui.vertical(|ui| {
                    self.render_sidebar(ui, &palette);
                });
            });

            // Detect view change AFTER all header and sidebar interactions in this frame
            let view_key = self.get_view_key();
            if view_key != self.current_view_key {
                self.current_view_key = view_key;
                self.view_transition_start = ctx.input(|i| i.time);
            }

            let elapsed = (ctx.input(|i| i.time) - self.view_transition_start).max(0.0);
            let anim_duration = 0.14;
            let anim_t = (elapsed / anim_duration).min(1.0) as f32;
            let ease = anim_t; // Clean, instant linear alpha fade

            if anim_t < 1.0 {
                ctx.request_repaint();
            }

            // Central Content Canvas - allocate_ui to accurately bound the width and prevent right cutoff
            let total_avail_w = ui.available_width();
            let total_avail_h = ui.available_height();

            let central_response = ui.allocate_ui_with_layout(
                vec2(total_avail_w, total_avail_h),
                Layout::top_down(Align::Min),
                |ui| {
                    let central_frame = egui::Frame::new()
                        .fill(palette.canvas_bg)
                        .inner_margin(Margin::symmetric(24, 20));

                    central_frame.show(ui, |ui| {
                        match self.current_tab {
                            NavTab::Apps => self.render_apps_view(ui, &palette),
                            NavTab::YourWork => self.render_recents_view(ui, &palette),
                            NavTab::Fonts => {
                                self.settings_state.current_tab = SettingsTab::Fonts;
                                render_settings(ui, &mut self.settings_state, self.tokio_rt.clone(), &palette);
                            }
                            NavTab::Settings => {
                                render_settings(ui, &mut self.settings_state, self.tokio_rt.clone(), &palette);
                            }
                        }
                    });
                },
            );

            // True GPU alpha fade-in on tab/view switch without any 1-frame flash or double animation
            if ease < 0.999 {
                let overlay_alpha = ((1.0 - ease) * 255.0) as u8;
                ui.painter().rect_filled(
                    central_response.response.rect,
                    CornerRadius::ZERO,
                    egui::Color32::from_rgba_unmultiplied(
                        palette.canvas_bg.r(),
                        palette.canvas_bg.g(),
                        palette.canvas_bg.b(),
                        overlay_alpha,
                    ),
                );
            }
        });

        // Floating animated toast notification overlay
        self.render_toast(ui, &palette);
    }
}



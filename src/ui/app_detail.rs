use crate::models::{InstallState, SuiteApp};
use crate::ui::logos::{render_github_icon_with_id, render_logo};
use crate::ui::theme::ThemePalette;
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Layout, Margin, Rect, RichText,
    ScrollArea, Stroke, StrokeKind, TextureOptions, Ui,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct AppDetailState {
    pub cached_readmes: std::collections::HashMap<String, String>,
    pub is_fetching_readme: std::collections::HashSet<String>,
}

impl Default for AppDetailState {
    fn default() -> Self {
        Self {
            cached_readmes: std::collections::HashMap::new(),
            is_fetching_readme: std::collections::HashSet::new(),
        }
    }
}

pub fn load_hero_color_image(app_id: &str) -> Option<egui::ColorImage> {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
    let dot_cache = home.join(".cache/opencloud/hero");
    let lib_cache = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("opencloud/hero");
    let candidates = [
        dot_cache.join(format!("{}.jpg", app_id)),
        dot_cache.join(format!("{}.png", app_id)),
        lib_cache.join(format!("{}.jpg", app_id)),
        lib_cache.join(format!("{}.png", app_id)),
        std::env::temp_dir().join(format!("opencloud_hero_{}.jpg", app_id)),
        std::env::temp_dir().join(format!("opencloud_hero_{}.png", app_id)),
    ];

    for path in &candidates {
        if path.is_file() {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    return Some(egui::ColorImage::from_rgba_unmultiplied(
                        [w as usize, h as usize],
                        rgba.as_raw(),
                    ));
                }
            }
        }
    }
    None
}

pub fn render_hero_screenshot(ui: &mut Ui, app_id: &str, _accent_color: [u8; 3], palette: &ThemePalette) {
    let texture_id = format!("opencloud_detail_hero_{}", app_id);
    let texture = ui.ctx().data_mut(|d| {
        d.get_temp::<egui::TextureHandle>(egui::Id::new(&texture_id))
    });

    let texture = match texture {
        Some(t) => t,
        None => {
            if let Some(color_img) = load_hero_color_image(app_id) {
                let handle = ui.ctx().load_texture(&texture_id, color_img, TextureOptions::LINEAR);
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new(&texture_id), handle.clone());
                });
                handle
            } else {
                // If not yet cached, render a sleek placeholder preview
                let container_w = ui.available_width().max(200.0);
                let container_h = 240.0;
                let (rect, _) = ui.allocate_exact_size(vec2(container_w, container_h), egui::Sense::hover());
                ui.painter().rect_filled(rect, CornerRadius::same(10), palette.card_bg);
                ui.painter().rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0, palette.border), egui::StrokeKind::Outside);
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    format!("{} Studio Interface Preview", app_id),
                    FontId::proportional(15.0),
                    palette.text_muted,
                );
                return;
            }
        }
    };

    let avail_w = ui.available_width().max(300.0);
    let img_size = texture.size_vec2();
    let aspect_ratio = if img_size.x > 0.0 && img_size.y > 0.0 {
        img_size.x / img_size.y
    } else {
        16.0 / 9.0
    };

    let target_w = avail_w;
    let target_h = (target_w / aspect_ratio).min(380.0).max(180.0);

    let container_frame = egui::Frame::new()
        .fill(palette.card_bg)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(4));

    container_frame.show(ui, |ui| {
        ui.add(
            egui::Image::new(&texture)
                .fit_to_exact_size(vec2(target_w - 8.0, target_h - 8.0))
                .corner_radius(CornerRadius::same(8)),
        );
    });
}

pub fn render_app_detail_view(
    ui: &mut Ui,
    app: &SuiteApp,
    detail_state: &mut AppDetailState,
    install_action: &mut Option<String>,
    launch_action: &mut Option<String>,
    uninstall_action: &mut Option<String>,
    back_action: &mut bool,
    palette: &ThemePalette,
    rt: Arc<Runtime>,
) {
    let app_id = app.id.clone();
    let app_name = app.name.clone();
    let app_repo = app.repo.clone();

    // ──────────────────────────────────────────────────────────
    // 1. TOP BREADCRUMB / BACK BAR
    // ──────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        let (btn_rect, btn_resp) = ui.allocate_exact_size(vec2(96.0, 28.0), egui::Sense::click());
        let is_hovered = btn_resp.hovered();
        let bg = if is_hovered { palette.border_active } else { palette.button_secondary_bg };
        ui.painter().rect(btn_rect, CornerRadius::same(14), bg, Stroke::new(1.0, palette.border), StrokeKind::Inside);

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
            FontId::proportional(13.0),
            palette.accent_blue,
        );

        if btn_resp.clicked() {
            *back_action = true;
        }

        ui.add_space(8.0);
        ui.label(RichText::new("/").color(palette.text_muted));
        ui.add_space(8.0);
        ui.label(
            RichText::new(&app_name)
                .font(FontId::proportional(13.0))
                .color(palette.text_primary)
                .strong(),
        );
    });

    ui.add_space(14.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ──────────────────────────────────────────────────────────
            // 2. HERO HEADER BLOCK (Like Adobe Creative Cloud)
            // ──────────────────────────────────────────────────────────
            let header_frame = egui::Frame::new()
                .fill(palette.card_bg)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::symmetric(20, 18));

            header_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Large 64x64 App Logo
                    let _ = render_logo(ui, &app_id, &app.token, app.accent_color, 64.0);
                    ui.add_space(16.0);

                    // Title & Description Column
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&app_name)
                                    .font(FontId::proportional(24.0))
                                    .color(palette.text_primary)
                                    .strong(),
                            );

                            ui.add_space(10.0);
                            let cat_label = format!("{:?}", app.category);
                            let cat_frame = egui::Frame::new()
                                .fill(palette.category_pill_bg)
                                .stroke(Stroke::new(1.0, palette.border))
                                .corner_radius(CornerRadius::same(10))
                                .inner_margin(Margin::symmetric(8, 2));

                            cat_frame.show(ui, |ui| {
                                ui.label(
                                    RichText::new(cat_label)
                                        .font(FontId::proportional(10.0))
                                        .color(palette.text_muted)
                                        .strong(),
                                );
                            });

                            if let Some(ref ver) = app.installed_version {
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(format!("v{}", ver))
                                        .font(FontId::proportional(11.0))
                                        .color(palette.success_green)
                                        .strong(),
                                );
                            }
                        });

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&app.description)
                                .font(FontId::proportional(13.0))
                                .color(palette.text_muted),
                        );

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            // GitHub upstream link with official octocat mark
                            let gh_id = format!("detail_gh_{}", app_id);
                            let gh_resp = render_github_icon_with_id(ui, &gh_id, 16.0, palette.text_muted);
                            if gh_resp
                                .on_hover_text(format!("Open https://github.com/{} in browser", app_repo))
                                .clicked()
                            {
                                let url = format!("https://github.com/{}", app_repo);
                                ui.ctx().open_url(egui::OpenUrl::new_tab(&url));
                                #[cfg(target_os = "macos")]
                                let _ = std::process::Command::new("open").arg(&url).spawn();
                            }

                            let link_btn = ui.add(
                                egui::Button::new(
                                    RichText::new(format!("github.com/{}", app_repo))
                                        .font(FontId::proportional(11.0))
                                        .color(palette.text_muted),
                                )
                                .frame(false),
                            );
                            if link_btn.clicked() {
                                let url = format!("https://github.com/{}", app_repo);
                                ui.ctx().open_url(egui::OpenUrl::new_tab(&url));
                                #[cfg(target_os = "macos")]
                                let _ = std::process::Command::new("open").arg(&url).spawn();
                            }

                            ui.add_space(14.0);
                            ui.label(
                                RichText::new("100% Pure Rust  |  WGPU Native")
                                    .font(FontId::proportional(11.0))
                                    .color(palette.text_muted),
                            );
                        });
                    });

                    // Right-aligned Action Buttons
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        match &app.install_state {
                            InstallState::NotInstalled => {
                                let install_btn = egui::Button::new(
                                    RichText::new("Install App")
                                        .font(FontId::proportional(13.0))
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(palette.accent_blue)
                                .stroke(Stroke::new(1.0, palette.accent_blue))
                                .corner_radius(CornerRadius::same(16))
                                .min_size(vec2(120.0, 34.0));

                                if ui.add(install_btn).clicked() {
                                    *install_action = Some(app_id.clone());
                                }
                            }
                            InstallState::Installed => {
                                ui.horizontal(|ui| {
                                    let open_btn = egui::Button::new(
                                        RichText::new("Open")
                                            .font(FontId::proportional(13.0))
                                            .color(palette.text_primary)
                                            .strong(),
                                    )
                                    .fill(palette.button_secondary_bg)
                                    .stroke(Stroke::new(1.0, palette.border_active))
                                    .corner_radius(CornerRadius::same(16))
                                    .min_size(vec2(90.0, 34.0));

                                    if ui.add(open_btn).clicked() {
                                        *launch_action = Some(app_id.clone());
                                    }

                                    let uninst_btn = egui::Button::new(
                                        RichText::new("Uninstall")
                                            .font(FontId::proportional(12.0))
                                            .color(Color32::from_rgb(239, 68, 68)),
                                    )
                                    .fill(palette.button_secondary_bg)
                                    .stroke(Stroke::new(1.0, palette.border))
                                    .corner_radius(CornerRadius::same(16))
                                    .min_size(vec2(80.0, 34.0));

                                    if ui.add(uninst_btn).clicked() {
                                        *uninstall_action = Some(app_id.clone());
                                    }
                                });
                            }
                            InstallState::UpdateAvailable => {
                                let update_btn = egui::Button::new(
                                    RichText::new("Update Now")
                                        .font(FontId::proportional(13.0))
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(palette.update_amber)
                                .stroke(Stroke::new(1.0, palette.update_amber))
                                .corner_radius(CornerRadius::same(16))
                                .min_size(vec2(120.0, 34.0));

                                if ui.add(update_btn).clicked() {
                                    *install_action = Some(app_id.clone());
                                }
                            }
                            InstallState::Downloading { percentage, phase } => {
                                let time = ui.input(|i| i.time);
                                ui.ctx().request_repaint();

                                ui.horizontal(|ui| {
                                    let (spin_rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                                    let center = spin_rect.center();
                                    let base_angle = (time * 8.0) as f32;
                                    for i in 0..4 {
                                        let a = base_angle + (i as f32) * std::f32::consts::FRAC_PI_2;
                                        let alpha = ((i as f32 + 1.0) / 4.0).powf(1.5);
                                        let pos = center + vec2(a.cos(), a.sin()) * 6.5;
                                        ui.painter().circle_filled(
                                            pos,
                                            1.6,
                                            egui::Color32::from_rgba_unmultiplied(
                                                palette.accent_blue.r(), palette.accent_blue.g(), palette.accent_blue.b(), (255.0 * alpha) as u8
                                            ),
                                        );
                                    }

                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(format!("{}: {:.0}%", phase, percentage))
                                                .font(FontId::proportional(12.0))
                                                .color(palette.accent_blue)
                                                .strong(),
                                        );
                                        let p_fraction = (*percentage / 100.0).clamp(0.0, 1.0);
                                        let (bar_rect, _) = ui.allocate_exact_size(vec2(120.0, 5.0), egui::Sense::hover());
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
                                    let (spin_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                                    let center = spin_rect.center();
                                    let base_angle = (time * 9.0) as f32;
                                    for i in 0..4 {
                                        let a = base_angle + (i as f32) * std::f32::consts::FRAC_PI_2;
                                        let alpha = ((i as f32 + 1.0) / 4.0).powf(1.5);
                                        let pos = center + vec2(a.cos(), a.sin()) * 5.5;
                                        ui.painter().circle_filled(
                                            pos,
                                            1.5,
                                            egui::Color32::from_rgba_unmultiplied(
                                                palette.update_amber.r(), palette.update_amber.g(), palette.update_amber.b(), (255.0 * alpha) as u8
                                            ),
                                        );
                                    }
                                    ui.label(
                                        RichText::new("Installing...")
                                            .font(FontId::proportional(13.0))
                                            .color(palette.update_amber)
                                            .strong(),
                                    );
                                });
                            }

                        }
                    });
                });
            });

            ui.add_space(18.0);

            // ──────────────────────────────────────────────────────────
            // 3. SHOWCASE PREVIEW (Screenshot with Hero image from GitHub)
            // ──────────────────────────────────────────────────────────
            ui.label(
                RichText::new("Overview & Preview")
                    .font(FontId::proportional(16.0))
                    .color(palette.text_primary)
                    .strong(),
            );
            ui.add_space(6.0);

            render_hero_screenshot(ui, &app_id, app.accent_color, palette);

            ui.add_space(20.0);

            // ──────────────────────────────────────────────────────────
            // 4. "EVERYTHING IN THE BOX" FEATURE HIGHLIGHTS GRID
            // ──────────────────────────────────────────────────────────
            ui.label(
                RichText::new("Highlights & Capabilities")
                    .font(FontId::proportional(16.0))
                    .color(palette.text_primary)
                    .strong(),
            );
            ui.add_space(8.0);

            render_features_grid(ui, &app_id, palette);

            ui.add_space(20.0);

            // ──────────────────────────────────────────────────────────
            // 5. TECHNICAL SPECIFICATIONS & COMPATIBILITY
            // ──────────────────────────────────────────────────────────
            ui.label(
                RichText::new("Technical Specifications")
                    .font(FontId::proportional(16.0))
                    .color(palette.text_primary)
                    .strong(),
            );
            ui.add_space(8.0);

            let specs_frame = egui::Frame::new()
                .fill(palette.card_bg)
                .stroke(Stroke::new(1.0, palette.border))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(16, 14));

            specs_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Supported File Formats:").color(palette.text_muted));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            for fmt in &app.file_formats {
                                let chip = egui::Frame::new()
                                    .fill(palette.category_pill_bg)
                                    .stroke(Stroke::new(1.0, palette.border))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::symmetric(6, 2));

                                chip.show(ui, |ui| {
                                    ui.label(
                                        RichText::new(fmt)
                                            .font(FontId::proportional(11.0))
                                            .color(palette.accent_blue)
                                            .strong(),
                                    );
                                });
                            }
                        });
                    });

                    ui.add_space(40.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new("Graphics & Engine:").color(palette.text_muted));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("WGPU (Metal / Vulkan / DirectX 12)")
                                .color(palette.text_primary)
                                .strong(),
                        );
                    });

                    ui.add_space(40.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new("Software License:").color(palette.text_muted));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Open Source (MIT)")
                                .color(palette.text_primary)
                                .strong(),
                        );
                    });
                });
            });

            ui.add_space(20.0);

            // ──────────────────────────────────────────────────────────
            // 6. GITHUB README DOCUMENTATION
            // ──────────────────────────────────────────────────────────
            ui.label(
                RichText::new("GitHub Repository Documentation")
                    .font(FontId::proportional(16.0))
                    .color(palette.text_primary)
                    .strong(),
            );
            ui.add_space(8.0);

            render_readme_section(ui, &app_id, &app_repo, detail_state, palette, rt);

            ui.add_space(32.0);
        });
}

fn render_features_grid(ui: &mut Ui, app_id: &str, palette: &ThemePalette) {
    let features = get_app_features(app_id);
    let avail_w = ui.available_width().max(200.0);
    let gap = 12.0;
    let card_w = ((avail_w - gap) / 2.0).floor().max(180.0);

    for chunk in features.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (title, desc) in chunk {
                let card_frame = egui::Frame::new()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 12));

                ui.allocate_ui_with_layout(
                    vec2(card_w, 82.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(card_w);
                        card_frame.show(ui, |ui| {
                            ui.set_width(card_w - 28.0);
                            ui.horizontal(|ui| {
                                let (bullet_rect, _) = ui.allocate_exact_size(vec2(8.0, 14.0), egui::Sense::hover());
                                ui.painter().circle_filled(
                                    bullet_rect.center(),
                                    3.0,
                                    palette.accent_blue,
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(*title)
                                        .font(FontId::proportional(13.5))
                                        .color(palette.text_primary)
                                        .strong(),
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(*desc)
                                    .font(FontId::proportional(12.0))
                                    .color(palette.text_muted),
                            );
                        });
                    },
                );
            }
        });
        ui.add_space(gap);
    }
}

fn get_app_features(app_id: &str) -> &'static [(&'static str, &'static str)] {
    match app_id {
        "photocraft" => &[
            ("Familiar by Design", "Standard menus, shortcuts, adjustment layers, and tools where your hands expect them."),
            ("Native and Fast", "GPU compositor on wgpu (Metal, Vulkan, DX12), copy-on-write tiles, and multithreaded filters."),
            ("Real PSD Files", "Open, edit and save layered PSD documents with non-destructive adjustments and masks."),
            ("Agent-Ready", "Every action is a command, scriptable from the UI, CLI, or MCP servers."),
        ],
        "vectorcraft" => &[
            ("Bezier & Pen Tool", "Direct anchor point editing, handles, Pathfinder operations, and smooth vector curves."),
            ("Appearance & Blends", "Live blend ribbons, multiple fills and strokes per path, and gradient meshes."),
            ("Perspective & Artboards", "Two-point perspective grids, Plane Switching, and multi-artboard layouts."),
            ("GPU Geometry", "Sub-millisecond rasterization on native WGPU hardware acceleration."),
        ],
        "filmcraft" => &[
            ("Multi-Track Timeline", "Real-time 4K/8K playback, magnetic snapping, ripple edits, and nested sequences."),
            ("Lumetri Color Grading", "3-way color wheels, curves, LUT support, and live GPU waveform scopes."),
            ("Integrated Audio", "Sub-frame audio trimming, track ducking, loudness metering, and multi-bus routing."),
            ("Hardware Video Codecs", "Hardware decoding and encoding via VideoToolbox, NVENC, and VAAPI."),
        ],
        "lightcraft" => &[
            ("Non-Destructive RAW", "High-fidelity demosaicing, highlight recovery, and color profile mapping."),
            ("Catalog & Collections", "Fast thumbnail caching, star ratings, color labels, and EXIF metadata filtering."),
            ("Tone & Curve Controls", "Histogram-guided exposure, contrast, highlights, shadows, whites, and blacks."),
            ("Instant Previews", "SIMD and GPU accelerated image processing for instant loupe magnification."),
        ],
        "pdfcraft" => &[
            ("Page Manipulation", "Merge, split, rotate, extract, reorder, and watermark PDF pages seamlessly."),
            ("Forms & Signatures", "Interactive AcroForms support and cryptographic digital signatures."),
            ("Threaded Annotations", "Highlighting, sticky notes, callouts, and threaded review replies."),
            ("Security & Encryption", "AES-256 password protection, permission flags, and redaction tools."),
        ],
        "effectcraft" => &[
            ("Node & Layer Graph", "Seamless blending between 2D timeline layers and 3D spatial particles."),
            ("Bezier Keyframes", "Smooth easing graphs, velocity handles, and time remapping."),
            ("Compositing & Masks", "Roto bezier masks, motion tracking, chroma keying, and blend modes."),
            ("Shader Pipelines", "Custom GLSL/WGSL effect shaders running at display refresh rates."),
        ],
        "designcraft" => &[
            ("Threaded Text Frames", "Automatic text flow across pages, master spreads, and baseline grids."),
            ("Advanced Typography", "Kerning, tracking, ligatures, small caps, and optical margin alignment."),
            ("Preflight & Print", "Bleed margins, CMYK color separation, spot colors, and print-ready PDF/X."),
            ("500+ Page Support", "Smooth virtualized canvas capable of handling book-length publications."),
        ],
        "soundcraft" => &[
            ("Multi-Track Recording", "Low-latency CoreAudio, ASIO, and ALSA driver integration."),
            ("Parametric Dynamics", "Integrated compressor, limiter, gate, de-esser, and spectral analysis."),
            ("Mixer Bus Routing", "Aux sends, submixes, master limiter, and real-time LUFS loudness meters."),
            ("Non-Destructive Clips", "Crossfades, slip editing, pitch shifting, and sample-accurate automation."),
        ],
        "cadcraft" => &[
            ("Precision Geometry", "Endpoints, midpoints, intersections, perpendiculars, and tangent snaps."),
            ("Architectural Drawing", "Hatched walls, dimension styles with architectural ticks, and schedules."),
            ("Standard CAD Formats", "Native DWG and DXF reading/writing with full layer hierarchy."),
            ("Vector Math Engine", "High-precision floating point geometry engine built in Rust."),
        ],
        "deckcraft" => &[
            ("Slide & Layout Design", "Smart alignment guides, master slide templates, and animated transitions."),
            ("Dynamic Vector Charts", "Interactive data charts, process diagrams, and vector shapes."),
            ("Presenter Studio", "Speaker notes, live timers, slide previews, and remote presentation controls."),
            ("Universal Slide Export", "Export to interactive PDF, standalone HTML5, and standard PPTX."),
        ],
        "gridcraft" => &[
            ("Calculation Engine", "Dependency DAG with parallel evaluation supporting 300+ standard formulas."),
            ("Visual Analytics", "Interactive clustered charts, sparklines, data bars, and conditional formatting."),
            ("Massive Row Capacity", "Virtualized rendering handling millions of rows with sub-millisecond response."),
            ("Excel Compatibility", "Native XLSX, XLS, and CSV reading and writing with formula preservation."),
        ],
        "wordcraft" => &[
            ("Rich Document Styles", "Styles gallery, hierarchical headings, footnotes, tables, and images."),
            ("Navigation Outline", "Instant jump between sections, chapters, and bookmarks."),
            ("Real-Time Word Metrics", "Word counts, reading time estimates, and grammar analysis."),
            ("Format Interop", "Seamless import and export of DOCX, Markdown, and formatted plain text."),
        ],
        _ => &[
            ("High Performance", "Rebuilt from scratch in pure Rust with zero bloat."),
            ("Modern Workflow", "Clean-room implementation designed for creative professionals."),
            ("File Fidelity", "Native support for industry standard creative project formats."),
            ("Automation Ready", "Scriptable and automated from UI, CLI, and agent tooling."),
        ],
    }
}

fn render_readme_section(
    ui: &mut Ui,
    app_id: &str,
    app_repo: &str,
    detail_state: &mut AppDetailState,
    palette: &ThemePalette,
    rt: Arc<Runtime>,
) {
    let readme_content = detail_state.cached_readmes.get(app_id).cloned();

    let text_to_render = match readme_content {
        Some(t) => t,
        None => {
            // Trigger async fetch if not already in progress
            if !detail_state.is_fetching_readme.contains(app_id) {
                detail_state.is_fetching_readme.insert(app_id.to_string());
                let app_id_clone = app_id.to_string();
                let url = format!("https://raw.githubusercontent.com/{}/main/README.md", app_repo);
                let rt_clone = rt.clone();

                rt_clone.spawn(async move {
                    if let Ok(res) = reqwest::get(&url).await {
                        if res.status().is_success() {
                            if let Ok(body) = res.text().await {
                                let cache_file = dirs::cache_dir()
                                    .unwrap_or_else(std::env::temp_dir)
                                    .join(format!("opencloud/readme_{}.md", app_id_clone));
                                let _ = std::fs::create_dir_all(cache_file.parent().unwrap());
                                let _ = std::fs::write(&cache_file, &body);
                            }
                        }
                    }
                });
            }

            // Check if cached on disk
            let cache_file = dirs::cache_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(format!("opencloud/readme_{}.md", app_id));
            if cache_file.is_file() {
                if let Ok(content) = std::fs::read_to_string(&cache_file) {
                    detail_state.cached_readmes.insert(app_id.to_string(), content.clone());
                    content
                } else {
                    get_default_readme_summary(app_id)
                }
            } else {
                get_default_readme_summary(app_id)
            }
        }
    };

    let doc_frame = egui::Frame::new()
        .fill(palette.card_bg)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(20, 18));

    doc_frame.show(ui, |ui| {
        ui.vertical(|ui| {
            // Filter out raw html tags and format headers and lines cleanly
            for line in text_to_render.lines().take(60) {
                let trimmed = line.trim();
                if trimmed.starts_with("<") && trimmed.ends_with(">") {
                    continue; // Skip raw html tags like <p>, <div>, etc.
                }
                if trimmed.starts_with("# ") {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(trimmed.trim_start_matches("# "))
                            .font(FontId::proportional(18.0))
                            .color(palette.text_primary)
                            .strong(),
                    );
                    ui.add_space(4.0);
                } else if trimmed.starts_with("## ") {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(trimmed.trim_start_matches("## "))
                            .font(FontId::proportional(15.0))
                            .color(palette.text_primary)
                            .strong(),
                    );
                    ui.add_space(2.0);
                } else if trimmed.starts_with("### ") {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(trimmed.trim_start_matches("### "))
                            .font(FontId::proportional(13.0))
                            .color(palette.text_primary)
                            .strong(),
                    );
                } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    ui.horizontal(|ui| {
                        let (b_rect, _) = ui.allocate_exact_size(vec2(8.0, 14.0), egui::Sense::hover());
                        ui.painter().circle_filled(b_rect.center(), 2.5, palette.accent_blue);
                        ui.label(
                            RichText::new(trimmed[2..].trim())
                                .font(FontId::proportional(12.0))
                                .color(palette.text_primary),
                        );
                    });
                } else if !trimmed.is_empty() {
                    ui.label(
                        RichText::new(trimmed)
                            .font(FontId::proportional(12.0))
                            .color(palette.text_muted),
                    );
                }
            }
        });
    });
}

fn get_default_readme_summary(app_id: &str) -> String {
    format!(
        "# {name}\n\n\
        An open-source, high-performance clean-room reimplementation rebuilt in pure Rust.\n\n\
        ## Architectural Features\n\
        - Native GPU Compositor on WGPU (Metal, Vulkan, DirectX 12)\n\
        - Zero Electron, zero webview, and ultra-low memory footprint\n\
        - Full fidelity project file compatibility\n\
        - Scriptable JSON control protocol and agent MCP server\n\n\
        ## Platform Support\n\
        - macOS (Apple Silicon Universal)\n\
        - Windows 10/11 (x64 / ARM64)\n\
        - Linux & FreeBSD (x86_64 / AArch64)\n\n\
        ## License\n\
        Licensed under the MIT License.",
        name = match app_id {
            "photocraft" => "PhotoCraft",
            "vectorcraft" => "VectorCraft",
            "filmcraft" => "FilmCraft",
            "lightcraft" => "LightCraft",
            "pdfcraft" => "PdfCraft",
            "effectcraft" => "EffectCraft",
            "designcraft" => "DesignCraft",
            "soundcraft" => "SoundCraft",
            "cadcraft" => "CadCraft",
            "deckcraft" => "DeckCraft",
            "gridcraft" => "GridCraft",
            "wordcraft" => "WordCraft",
            _ => "Creative Studio App",
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_12_apps_have_features_and_readmes() {
        let app_ids = [
            "photocraft",
            "vectorcraft",
            "filmcraft",
            "lightcraft",
            "pdfcraft",
            "effectcraft",
            "designcraft",
            "soundcraft",
            "cadcraft",
            "deckcraft",
            "gridcraft",
            "wordcraft",
        ];

        for id in app_ids {
            let features = get_app_features(id);
            assert_eq!(features.len(), 4, "Every app must define 4 signature feature highlights: {}", id);
            let readme = get_default_readme_summary(id);
            assert!(readme.contains("Architectural Features"), "Readme must contain features: {}", id);
            assert!(readme.contains("Platform Support"), "Readme must contain platform support: {}", id);
        }
    }

    #[test]
    fn test_hero_screenshot_loader() {
        // Test loading photocraft hero image
        let img = load_hero_color_image("photocraft");
        assert!(img.is_some(), "Photocraft hero image should be loaded from cache");
        let unwrapped = img.unwrap();
        assert!(unwrapped.width() > 0 && unwrapped.height() > 0);
    }
}

